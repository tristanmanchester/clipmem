use std::io::Cursor;
use std::path::Path;

use anyhow::{ensure, Context, Result};
use image::{ExtendedColorType, ImageEncoder, ImageReader, Limits};
use rusqlite::{params, OptionalExtension, TransactionBehavior};

use super::config::{
    ImageOptimizationCandidate, IMAGE_OPTIMIZATION_FORMAT, IMAGE_OPTIMIZATION_MAX_DIMENSION,
    IMAGE_OPTIMIZATION_MIN_ABSOLUTE_SAVINGS, IMAGE_OPTIMIZATION_MIN_RELATIVE_SAVINGS_DENOMINATOR,
    IMAGE_OPTIMIZATION_MIN_RELATIVE_SAVINGS_NUMERATOR, WEBP_UTI,
};
use super::ocr::rebuild_snapshot_ocr_cache;
use super::rebuild::{
    rebuild_snapshot_summary, recompute_snapshot_fingerprint, snapshot_fingerprint_with_replacement,
};
use super::revision::bump_revision_tx;
use crate::db::core::{clamp_result_limit, pragma_usize, storage_file_sizes};
use crate::db::sqlite_helpers::{collect_rows, row_usize, usize_to_i64};
use crate::db::types::{
    ArchiveChangeKind, Database, ImageOptimizationCandidateSummary, ImageOptimizationProgressEvent,
    ImageOptimizationReport,
};
use crate::model::{hash_bytes, truncate_chars};

#[derive(Debug, Clone)]
pub(in crate::db) struct OptimizedImage {
    pub(in crate::db) bytes: Vec<u8>,
    pub(in crate::db) raw_sha256: String,
}

impl Database {
    pub(crate) fn optimize_images(
        &mut self,
        dry_run: bool,
        limit: usize,
        auto_compact: bool,
    ) -> Result<ImageOptimizationReport> {
        self.optimize_images_with_progress(dry_run, limit, auto_compact, |_| Ok(()))
    }

    pub(crate) fn optimize_images_with_progress(
        &mut self,
        dry_run: bool,
        limit: usize,
        auto_compact: bool,
        mut progress: impl FnMut(ImageOptimizationProgressEvent) -> Result<()>,
    ) -> Result<ImageOptimizationReport> {
        let initial_file_bytes = if database_path_is_file_backed(&self.path) {
            Some(storage_file_sizes(&self.path)?.total_bytes())
        } else {
            None
        };
        let candidates = load_image_optimization_candidates(&self.conn, limit)?;
        let total_rows = candidates.len();
        let mut report = ImageOptimizationReport {
            dry_run,
            format: IMAGE_OPTIMIZATION_FORMAT,
            scanned_rows: 0,
            compressed_rows: 0,
            skipped_rows: 0,
            conflict_count: 0,
            original_bytes: 0,
            optimized_bytes: 0,
            logical_saved_bytes: 0,
            compact_run: false,
            compact: None,
            compact_error: None,
            filesystem_saved_bytes: 0,
            filesystem_growth_bytes: 0,
            compact_recommended: false,
        };
        progress(ImageOptimizationProgressEvent::Started { total_rows })?;

        self.process_image_optimization_candidates(
            dry_run,
            candidates,
            total_rows,
            &mut report,
            &mut progress,
        )?;
        self.maybe_compact_after_image_optimization(
            dry_run,
            auto_compact,
            initial_file_bytes,
            total_rows,
            &mut report,
            &mut progress,
        )?;

        progress(ImageOptimizationProgressEvent::Complete {
            report: Box::new(report.clone()),
        })?;
        Ok(report)
    }

    pub(crate) fn image_optimization_candidate_summaries(
        &self,
        limit: usize,
    ) -> Result<Vec<ImageOptimizationCandidateSummary>> {
        load_image_optimization_candidates(&self.conn, limit)?
            .into_iter()
            .map(|candidate| {
                Ok(ImageOptimizationCandidateSummary::new(
                    candidate.snapshot_id,
                    candidate.item_index,
                    candidate.uti,
                    candidate.byte_len,
                    candidate.raw_sha256,
                ))
            })
            .collect()
    }

    fn process_image_optimization_candidates(
        &mut self,
        dry_run: bool,
        candidates: Vec<ImageOptimizationCandidate>,
        total_rows: usize,
        report: &mut ImageOptimizationReport,
        progress: &mut impl FnMut(ImageOptimizationProgressEvent) -> Result<()>,
    ) -> Result<()> {
        for candidate in candidates {
            process_image_optimization_candidate(&mut self.conn, dry_run, &candidate, report)?;
            Self::emit_image_optimization_scan_progress(progress, report, total_rows)?;
        }
        Ok(())
    }

    fn maybe_compact_after_image_optimization(
        &mut self,
        dry_run: bool,
        auto_compact: bool,
        initial_file_bytes: Option<u64>,
        total_rows: usize,
        report: &mut ImageOptimizationReport,
        progress: &mut impl FnMut(ImageOptimizationProgressEvent) -> Result<()>,
    ) -> Result<()> {
        if dry_run {
            return Ok(());
        }

        if auto_compact && database_path_is_file_backed(&self.path) {
            let compact_wanted = report.compact_recommended || storage_compaction_would_help(self)?;
            if compact_wanted {
                self.compact_after_image_optimization(
                    initial_file_bytes,
                    total_rows,
                    report,
                    progress,
                )?;
            } else {
                self.assign_current_image_optimization_filesystem_delta(
                    initial_file_bytes,
                    report,
                )?;
            }
        } else if report.compact_recommended && !auto_compact {
            self.assign_current_image_optimization_filesystem_delta(initial_file_bytes, report)?;
        }
        Ok(())
    }

    fn compact_after_image_optimization(
        &mut self,
        initial_file_bytes: Option<u64>,
        total_rows: usize,
        report: &mut ImageOptimizationReport,
        progress: &mut impl FnMut(ImageOptimizationProgressEvent) -> Result<()>,
    ) -> Result<()> {
        report.compact_run = true;
        progress(ImageOptimizationProgressEvent::Compacting {
            scanned_rows: report.scanned_rows,
            total_rows,
            compressed_rows: report.compressed_rows,
            skipped_rows: report.skipped_rows,
            conflict_count: report.conflict_count,
        })?;

        match self.compact_storage(false) {
            Ok(compact) => {
                assign_image_optimization_filesystem_delta(
                    report,
                    initial_file_bytes,
                    compact.total_after_bytes,
                );
                report.compact_recommended = false;
                report.compact = Some(compact);
            }
            Err(error) => {
                report.compact_error = Some(format_compaction_error(&error));
                report.compact_recommended = true;
            }
        }
        Ok(())
    }

    fn assign_current_image_optimization_filesystem_delta(
        &self,
        initial_file_bytes: Option<u64>,
        report: &mut ImageOptimizationReport,
    ) -> Result<()> {
        if let Some(initial) = initial_file_bytes {
            let final_bytes = storage_file_sizes(&self.path)?.total_bytes();
            assign_image_optimization_filesystem_delta(report, Some(initial), final_bytes);
        }
        Ok(())
    }

    fn emit_image_optimization_scan_progress(
        progress: &mut impl FnMut(ImageOptimizationProgressEvent) -> Result<()>,
        report: &ImageOptimizationReport,
        total_rows: usize,
    ) -> Result<()> {
        progress(ImageOptimizationProgressEvent::Scanning {
            scanned_rows: report.scanned_rows,
            total_rows,
            compressed_rows: report.compressed_rows,
            skipped_rows: report.skipped_rows,
            conflict_count: report.conflict_count,
        })
    }
}

fn process_image_optimization_candidate(
    conn: &mut rusqlite::Connection,
    dry_run: bool,
    candidate: &ImageOptimizationCandidate,
    report: &mut ImageOptimizationReport,
) -> Result<()> {
    report.scanned_rows += 1;
    let optimized = match encode_candidate_as_lossless_webp(candidate) {
        Ok(optimized) => optimized,
        Err(reason) => {
            record_image_optimization_skip(conn, dry_run, candidate, report, reason, false)?;
            return Ok(());
        }
    };

    if !image_optimization_is_beneficial(candidate.byte_len, optimized.bytes.len()) {
        record_image_optimization_skip(conn, dry_run, candidate, report, "not_smaller", false)?;
        return Ok(());
    }

    if image_optimization_would_conflict(conn, candidate, &optimized)? {
        record_image_optimization_skip(conn, dry_run, candidate, report, "conflict", true)?;
        return Ok(());
    }

    let saved_bytes = candidate.byte_len.saturating_sub(optimized.bytes.len());
    report.compressed_rows += 1;
    report.original_bytes += candidate.byte_len;
    report.optimized_bytes += optimized.bytes.len();
    report.logical_saved_bytes += saved_bytes;
    report.compact_recommended = true;

    if !dry_run {
        replace_image_with_optimized_webp(conn, candidate, optimized)?;
    }
    Ok(())
}

fn record_image_optimization_skip(
    conn: &rusqlite::Connection,
    dry_run: bool,
    candidate: &ImageOptimizationCandidate,
    report: &mut ImageOptimizationReport,
    reason: &str,
    is_conflict: bool,
) -> Result<()> {
    report.skipped_rows += 1;
    if is_conflict {
        report.conflict_count += 1;
    }
    if !dry_run {
        mark_image_optimization_skipped(conn, candidate, reason)?;
    }
    Ok(())
}

fn assign_image_optimization_filesystem_delta(
    report: &mut ImageOptimizationReport,
    initial_bytes: Option<u64>,
    final_bytes: u64,
) {
    if let Some(initial) = initial_bytes {
        report.filesystem_saved_bytes = initial.saturating_sub(final_bytes);
        report.filesystem_growth_bytes = final_bytes.saturating_sub(initial);
    }
}

pub(in crate::db) fn load_image_optimization_candidates(
    conn: &rusqlite::Connection,
    limit: usize,
) -> Result<Vec<ImageOptimizationCandidate>> {
    let limit = usize_to_i64(clamp_result_limit(limit))?;
    let mut stmt = conn
        .prepare(
            r"
                SELECT snapshot_id, item_index, uti, byte_len, raw_sha256, blob_value
                FROM item_representations
                WHERE kind = 'image'
                  AND image_compression_status = 'uncompressed'
                  AND length(blob_value) > 0
                ORDER BY byte_len DESC, snapshot_id ASC, item_index ASC, uti ASC
                LIMIT ?1
            ",
        )
        .context("prepare image optimization candidate query")?;
    let rows = stmt
        .query_map([limit], |row| {
            Ok(ImageOptimizationCandidate {
                snapshot_id: row.get(0)?,
                item_index: row.get(1)?,
                uti: row.get(2)?,
                byte_len: row_usize(row, 3)?,
                raw_sha256: row.get(4)?,
                blob_value: row.get(5)?,
            })
        })
        .context("execute image optimization candidate query")?;
    collect_rows(rows).context("collect image optimization candidates")
}

pub(in crate::db) fn database_path_is_file_backed(path: &Path) -> bool {
    path != Path::new(":memory:")
}

pub(in crate::db) fn storage_compaction_would_help(db: &Database) -> Result<bool> {
    let file_sizes = storage_file_sizes(&db.path)?;
    let freelist_count = pragma_usize(&db.conn, "freelist_count")?;
    Ok(freelist_count > 0 || file_sizes.wal > 0 || file_sizes.shm > 0)
}

pub(in crate::db) fn format_compaction_error(error: &anyhow::Error) -> String {
    let message = error.to_string();
    let lower = message.to_ascii_lowercase();
    if lower.contains("busy") || lower.contains("locked") {
        format!("{message}; retry after stopping capture or after the database is no longer busy")
    } else {
        message
    }
}

pub(in crate::db) fn encode_candidate_as_lossless_webp(
    candidate: &ImageOptimizationCandidate,
) -> std::result::Result<OptimizedImage, &'static str> {
    if candidate_looks_animated_or_unsupported(candidate) {
        return Err("animated_or_unsupported");
    }

    let mut reader = ImageReader::new(Cursor::new(&candidate.blob_value))
        .with_guessed_format()
        .map_err(|_| "unsupported")?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(IMAGE_OPTIMIZATION_MAX_DIMENSION);
    limits.max_image_height = Some(IMAGE_OPTIMIZATION_MAX_DIMENSION);
    reader.limits(limits);
    let image = reader.decode().map_err(|_| "corrupt_or_unsupported")?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    let mut bytes = Vec::new();
    image::codecs::webp::WebPEncoder::new_lossless(&mut bytes)
        .write_image(rgba.as_raw(), width, height, ExtendedColorType::Rgba8)
        .map_err(|_| "encode_failed")?;
    let raw_sha256 = hash_bytes(&bytes);
    Ok(OptimizedImage { bytes, raw_sha256 })
}

pub(in crate::db) fn candidate_looks_animated_or_unsupported(
    candidate: &ImageOptimizationCandidate,
) -> bool {
    let uti = candidate.uti.to_ascii_lowercase();
    if uti.contains("gif") || uti.contains("webp") {
        return true;
    }

    let bytes = candidate.blob_value.as_slice();
    bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") || webp_has_animation_flag(bytes)
}

pub(in crate::db) fn webp_has_animation_flag(bytes: &[u8]) -> bool {
    bytes.len() >= 21
        && &bytes[0..4] == b"RIFF"
        && &bytes[8..12] == b"WEBP"
        && &bytes[12..16] == b"VP8X"
        && (bytes[20] & 0b0000_0010) != 0
}

pub(in crate::db) fn image_optimization_is_beneficial(
    original_len: usize,
    optimized_len: usize,
) -> bool {
    if optimized_len >= original_len {
        return false;
    }
    let saved = original_len - optimized_len;
    saved >= IMAGE_OPTIMIZATION_MIN_ABSOLUTE_SAVINGS
        && saved * IMAGE_OPTIMIZATION_MIN_RELATIVE_SAVINGS_DENOMINATOR
            >= original_len * IMAGE_OPTIMIZATION_MIN_RELATIVE_SAVINGS_NUMERATOR
}

pub(in crate::db) fn mark_image_optimization_skipped(
    conn: &rusqlite::Connection,
    candidate: &ImageOptimizationCandidate,
    reason: &str,
) -> Result<()> {
    conn.execute(
        r"
            UPDATE item_representations
            SET image_compression_status = 'skipped',
                image_compression_reason = ?4
            WHERE snapshot_id = ?1
              AND item_index = ?2
              AND uti = ?3
              AND image_compression_status = 'uncompressed'
        ",
        params![
            candidate.snapshot_id,
            candidate.item_index,
            &candidate.uti,
            reason
        ],
    )
    .context("mark image optimization skipped")?;
    Ok(())
}

pub(in crate::db) fn image_optimization_would_conflict(
    conn: &rusqlite::Connection,
    candidate: &ImageOptimizationCandidate,
    optimized: &OptimizedImage,
) -> Result<bool> {
    let uti_conflict: i64 = conn
        .query_row(
            r"
                SELECT EXISTS(
                    SELECT 1
                    FROM item_representations
                    WHERE snapshot_id = ?1
                      AND item_index = ?2
                      AND uti = ?3
                      AND uti != ?4
                )
            ",
            params![
                candidate.snapshot_id,
                candidate.item_index,
                WEBP_UTI,
                &candidate.uti
            ],
            |row| row.get(0),
        )
        .context("check WebP representation conflict")?;
    if uti_conflict != 0 {
        return Ok(true);
    }

    let fingerprint =
        snapshot_fingerprint_with_replacement(conn, candidate, WEBP_UTI, &optimized.bytes)?;
    let snapshot_conflict: i64 = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM snapshots WHERE sha256 = ?1 AND id != ?2)",
            params![fingerprint, candidate.snapshot_id],
            |row| row.get(0),
        )
        .context("check optimized snapshot fingerprint conflict")?;
    Ok(snapshot_conflict != 0)
}

pub(in crate::db) fn replace_image_with_optimized_webp(
    conn: &mut rusqlite::Connection,
    candidate: &ImageOptimizationCandidate,
    optimized: OptimizedImage,
) -> Result<()> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .context("begin image optimization transaction")?;

    let updated = tx
        .execute(
            r"
            UPDATE item_representations
            SET uti = ?4,
                byte_len = ?5,
                raw_sha256 = ?6,
                blob_value = ?7,
                image_compression_status = 'compressed',
                image_compression_format = ?8,
                image_compressed_at = CURRENT_TIMESTAMP,
                image_original_byte_len = ?9,
                image_original_raw_sha256 = ?10,
                image_compression_reason = NULL
            WHERE snapshot_id = ?1
              AND item_index = ?2
              AND uti = ?3
              AND image_compression_status = 'uncompressed'
        ",
            params![
                candidate.snapshot_id,
                candidate.item_index,
                &candidate.uti,
                WEBP_UTI,
                usize_to_i64(optimized.bytes.len())?,
                &optimized.raw_sha256,
                &optimized.bytes,
                IMAGE_OPTIMIZATION_FORMAT,
                usize_to_i64(candidate.byte_len)?,
                &candidate.raw_sha256
            ],
        )
        .context("replace image representation with lossless WebP")?;
    ensure!(
        updated == 1,
        "image optimization target changed before replacement; retry optimize-images"
    );

    copy_ocr_result_for_optimized_image(&tx, &candidate.raw_sha256, &optimized.raw_sha256)?;
    rebuild_snapshot_item_summary(&tx, candidate.snapshot_id, candidate.item_index)?;
    rebuild_snapshot_summary(&tx, candidate.snapshot_id)?;
    let fingerprint = recompute_snapshot_fingerprint(&tx, candidate.snapshot_id)?;
    tx.execute(
        "UPDATE snapshots SET sha256 = ?2 WHERE id = ?1",
        params![candidate.snapshot_id, fingerprint],
    )
    .context("update optimized snapshot fingerprint")?;
    rebuild_snapshot_ocr_cache(&tx, candidate.snapshot_id)?;
    bump_revision_tx(
        &tx,
        &[
            ArchiveChangeKind::ArchiveContent,
            ArchiveChangeKind::Storage,
        ],
    )?;

    tx.commit()
        .context("commit image optimization transaction")?;
    Ok(())
}

pub(in crate::db) fn copy_ocr_result_for_optimized_image(
    tx: &rusqlite::Transaction<'_>,
    old_hash: &str,
    new_hash: &str,
) -> Result<()> {
    tx.execute(
        r"
            INSERT OR IGNORE INTO ocr_results (
                raw_sha256,
                status,
                engine,
                recognition_level,
                text_value,
                error,
                attempt_count,
                created_at,
                updated_at
            )
            SELECT
                ?2,
                status,
                engine,
                recognition_level,
                text_value,
                error,
                attempt_count,
                created_at,
                CURRENT_TIMESTAMP
            FROM ocr_results
            WHERE raw_sha256 = ?1
        ",
        params![old_hash, new_hash],
    )
    .context("copy OCR result for optimized image")?;
    Ok(())
}

pub(in crate::db) fn rebuild_snapshot_item_summary(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    item_index: i64,
) -> Result<()> {
    let total_bytes: usize = tx
        .query_row(
            r"
                SELECT COALESCE(SUM(byte_len), 0)
                FROM item_representations
                WHERE snapshot_id = ?1 AND item_index = ?2
            ",
            params![snapshot_id, item_index],
            |row| row_usize(row, 0),
        )
        .context("recompute snapshot item byte count")?;
    let primary = tx
        .query_row(
            r"
                SELECT kind, uti, byte_len
                FROM item_representations
                WHERE snapshot_id = ?1 AND item_index = ?2
                ORDER BY
                    CASE kind
                        WHEN 'plain_text' THEN 0
                        WHEN 'url' THEN 1
                        WHEN 'file_url' THEN 2
                        WHEN 'html' THEN 3
                        WHEN 'json' THEN 4
                        WHEN 'xml' THEN 5
                        WHEN 'rtf' THEN 6
                        WHEN 'pdf' THEN 7
                        WHEN 'image' THEN 8
                        WHEN 'binary' THEN 9
                        ELSE 10
                    END,
                    CASE WHEN text_value IS NULL OR text_value = '' THEN 1 ELSE 0 END,
                    uti ASC
                LIMIT 1
            ",
            params![snapshot_id, item_index],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row_usize(row, 2)?,
                ))
            },
        )
        .optional()
        .context("load optimized item primary representation")?;
    let search_text: String = tx
        .query_row(
            "SELECT search_text FROM snapshot_items WHERE snapshot_id = ?1 AND item_index = ?2",
            params![snapshot_id, item_index],
            |row| row.get(0),
        )
        .context("load optimized item search text")?;
    let (primary_kind, primary_uti, preview_text) =
        if let Some((kind, uti, primary_byte_len)) = primary {
            let preview_text = if search_text.is_empty() {
                truncate_chars(&format!("[{kind} · {primary_byte_len} bytes · {uti}]"), 200)
            } else {
                search_text.clone()
            };
            (kind, Some(uti), preview_text)
        } else {
            (
                "empty".to_string(),
                None,
                "[empty clipboard item]".to_string(),
            )
        };

    tx.execute(
        r"
            UPDATE snapshot_items
            SET primary_kind = ?3,
                primary_uti = ?4,
                preview_text = ?5,
                total_bytes = ?6
            WHERE snapshot_id = ?1 AND item_index = ?2
        ",
        params![
            snapshot_id,
            item_index,
            primary_kind,
            primary_uti,
            preview_text,
            usize_to_i64(total_bytes)?
        ],
    )
    .context("update optimized snapshot item summary")?;
    Ok(())
}

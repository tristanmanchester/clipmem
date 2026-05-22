use std::path::Path;

use anyhow::Result;

use crate::cli::formats::OutputFormat;
use crate::cli::formats::ProgressFormat;
use crate::cli::output::print_json_line;
use crate::cli::presentation::emit_json_or_text;
use crate::cli::presentation::{emit_image_optimization_output, emit_storage_compact_output};
use crate::cli::schema::{
    StorageArgs, StorageCommand, StorageCompactArgs, StorageImageCandidatesArgs,
    StorageOptimizeImagesArgs,
};
use crate::db::{ImageOptimizationCandidateSummary, ImageOptimizationReport};

use super::mutation_support::require_text_or_json;
use super::notify::notify_app_refresh;
use super::runtime::open_existing_db;

pub(in crate::cli) fn storage(db_path: &Path, args: &StorageArgs) -> Result<()> {
    match &args.command {
        StorageCommand::Compact(args) => storage_compact(db_path, args),
        StorageCommand::ImageCandidates(args) => storage_image_candidates(db_path, args),
        StorageCommand::OptimizeImages(args) => storage_optimize_images(db_path, args),
    }
}

fn storage_compact(db_path: &Path, args: &StorageCompactArgs) -> Result<()> {
    let format = require_text_or_json(args.output.resolved()?, "storage compact")?;
    let mut db = open_existing_db(db_path)?;
    let report = db.compact_storage(args.dry_run)?;
    emit_storage_compact_output(format, &report)?;
    if !report.dry_run && report.completed {
        notify_app_refresh();
    }
    Ok(())
}

fn storage_optimize_images(db_path: &Path, args: &StorageOptimizeImagesArgs) -> Result<()> {
    if matches!(args.progress, Some(ProgressFormat::Jsonl)) {
        let mut db = open_existing_db(db_path)?;
        db.optimize_images_with_progress(args.dry_run, args.limit, !args.no_compact, |event| {
            print_json_line(&event)
        })?;
        if !args.dry_run {
            notify_app_refresh();
        }
        return Ok(());
    }

    let format = require_text_or_json(args.output.resolved()?, "storage optimize-images")?;
    let mut db = open_existing_db(db_path)?;
    let report = db.optimize_images(args.dry_run, args.limit, !args.no_compact)?;
    emit_image_optimization_output(format, &report)?;
    if should_notify_after_image_optimization(&report) {
        notify_app_refresh();
    }
    Ok(())
}

fn should_notify_after_image_optimization(report: &ImageOptimizationReport) -> bool {
    !report.dry_run && (report.compressed_rows > 0 || report.compact_run)
}

fn storage_image_candidates(db_path: &Path, args: &StorageImageCandidatesArgs) -> Result<()> {
    let format = require_text_or_json(args.output.resolved()?, "storage image-candidates")?;
    let db = open_existing_db(db_path)?;
    let candidates = db.image_optimization_candidate_summaries(args.limit)?;
    match format {
        OutputFormat::Json => {
            emit_json_or_text(true, &candidates, |rows| render_image_candidates_text(rows))
        }
        OutputFormat::Human | OutputFormat::Text => {
            print!("{}", render_image_candidates_text(&candidates));
            Ok(())
        }
        _ => unreachable!("unsupported storage image-candidates format should be rejected earlier"),
    }
}

fn render_image_candidates_text(candidates: &[ImageOptimizationCandidateSummary]) -> String {
    if candidates.is_empty() {
        return "image candidates=0\n".to_string();
    }
    let mut out = format!("image candidates={}\n", candidates.len());
    for candidate in candidates {
        out.push_str(&format!(
            "snapshot={} item={} uti={} bytes={}\n",
            candidate.snapshot_id(),
            candidate.item_index(),
            candidate.uti(),
            candidate.byte_len()
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::should_notify_after_image_optimization;
    use crate::db::ImageOptimizationReport;

    fn report(dry_run: bool, compressed_rows: usize, compact_run: bool) -> ImageOptimizationReport {
        ImageOptimizationReport {
            dry_run,
            format: "webp_lossless",
            scanned_rows: 0,
            compressed_rows,
            skipped_rows: 0,
            conflict_count: 0,
            original_bytes: 0,
            optimized_bytes: 0,
            logical_saved_bytes: 0,
            compact_run,
            compact: None,
            compact_error: None,
            filesystem_saved_bytes: 0,
            filesystem_growth_bytes: 0,
            compact_recommended: false,
        }
    }

    #[test]
    fn image_optimization_notify_predicate_includes_compaction_only_mutation() {
        assert!(should_notify_after_image_optimization(&report(
            false, 0, true
        )));
        assert!(should_notify_after_image_optimization(&report(
            false, 1, false
        )));
        assert!(!should_notify_after_image_optimization(&report(
            true, 1, true
        )));
        assert!(!should_notify_after_image_optimization(&report(
            false, 0, false
        )));
    }
}

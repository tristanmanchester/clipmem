use anyhow::{bail, Context, Result};
use rusqlite::Connection;

use crate::model::{
    dedupe_text_fragments, html_to_text_lossy, is_searchable_text_fragment, normalize_whitespace,
    rtf_to_text_lossy, truncate_chars, ClipboardKind,
};

use super::sqlite_helpers::{collect_rows, row_enum};

pub(super) const SCHEMA: &str = include_str!("schema.sql");
pub(super) const CURRENT_SCHEMA_VERSION: i64 = 18;
const LEGACY_PRERELEASE_COLUMNS: &[&str] = &["classification", "is_text"];

pub(in crate::db) fn prepare_schema(conn: &mut Connection) -> Result<()> {
    let tx = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .context("begin schema transaction")?;

    tx.execute_batch(SCHEMA).context("apply database schema")?;

    let user_version: i64 = tx
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .context("read PRAGMA user_version")?;

    validate_supported_user_version(&tx, user_version)?;
    if user_version < CURRENT_SCHEMA_VERSION {
        run_schema_migration_steps(&tx, user_version)?;
        tx.pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION)
            .context("set PRAGMA user_version")?;
    }

    ensure_api_key_filter_setting_column(&tx)?;
    ensure_ocr_enabled_setting_column(&tx)?;
    ensure_image_compression_columns(&tx)?;
    ensure_image_optimization_queue_index(&tx)?;
    ensure_representation_cache_deferred_column(&tx)?;
    ensure_archive_revisions_table(&tx)?;
    tx.execute(
        "INSERT OR IGNORE INTO clipmem_settings (id, paused, retention_seconds, api_key_filter_enabled, ocr_enabled) VALUES (1, 0, NULL, 0, 0)",
        [],
    )
    .context("seed clipmem settings row")?;
    tx.execute(
        "INSERT OR IGNORE INTO archive_revisions (id) VALUES (1)",
        [],
    )
    .context("seed archive revision row")?;

    tx.commit().context("commit schema transaction")?;
    Ok(())
}

struct MigrationStep {
    name: &'static str,
    applies_to: fn(i64) -> bool,
    run: fn(&Connection) -> Result<()>,
}

const MIGRATION_STEPS: &[MigrationStep] = &[
    MigrationStep {
        name: "rebuild FTS5 index",
        applies_to: source_version_is_zero,
        run: rebuild_snapshots_fts,
    },
    MigrationStep {
        name: "rebuild snapshot stats",
        applies_to: source_version_through_1,
        run: rebuild_snapshot_stats,
    },
    MigrationStep {
        name: "rebuild snapshot projection cache",
        applies_to: source_version_through_2,
        run: rebuild_snapshot_projection_cache,
    },
    MigrationStep {
        name: "rebuild snapshot event filter cache",
        applies_to: source_version_through_3,
        run: rebuild_snapshot_event_filter_cache,
    },
    MigrationStep {
        name: "repair stored snapshot text projection",
        applies_to: source_version_needs_text_projection_repair,
        run: rebuild_snapshot_text_from_representations,
    },
    MigrationStep {
        name: "rebuild snapshot literal cache",
        applies_to: source_version_needs_literal_cache_rebuild,
        run: rebuild_snapshot_literal_cache,
    },
    MigrationStep {
        name: "rebuild snapshot file URL FTS",
        applies_to: source_version_through_5,
        run: rebuild_snapshot_file_url_fts,
    },
];

fn validate_supported_user_version(conn: &Connection, user_version: i64) -> Result<()> {
    if user_version > CURRENT_SCHEMA_VERSION {
        bail!(
            "database schema version {user_version} is newer than supported version {CURRENT_SCHEMA_VERSION}"
        );
    }
    if user_version < 0 {
        bail!("unsupported database schema version {user_version}");
    }
    if user_version > 0 && legacy_prerelease_schema_detected(conn)? {
        bail!(
            "database at the current user_version uses an incompatible prerelease schema; move it aside and run `clipmem setup` to initialize a fresh archive"
        );
    }
    Ok(())
}

fn run_schema_migration_steps(conn: &Connection, source_version: i64) -> Result<()> {
    for step in MIGRATION_STEPS
        .iter()
        .filter(|step| (step.applies_to)(source_version))
    {
        (step.run)(conn).with_context(|| format!("run schema migration step: {}", step.name))?;
    }
    Ok(())
}

fn source_version_is_zero(version: i64) -> bool {
    version == 0
}

fn source_version_through_1(version: i64) -> bool {
    version <= 1
}

fn source_version_through_2(version: i64) -> bool {
    version <= 2
}

fn source_version_through_3(version: i64) -> bool {
    version <= 3
}

fn source_version_through_5(version: i64) -> bool {
    version <= 5
}

fn source_version_needs_literal_cache_rebuild(version: i64) -> bool {
    version <= 4 || matches!(version, 8 | 9)
}

fn source_version_needs_text_projection_repair(version: i64) -> bool {
    matches!(version, 8 | 9)
}

fn rebuild_snapshots_fts(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT INTO snapshots_fts(snapshots_fts) VALUES ('rebuild')",
        [],
    )
    .context("rebuild FTS5 index")?;
    Ok(())
}

pub(in crate::db) fn ensure_api_key_filter_setting_column(conn: &Connection) -> Result<()> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(clipmem_settings)")
        .context("prepare clipmem_settings table info query")?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .context("query clipmem_settings columns")?;
    let columns = collect_rows(rows).context("collect clipmem_settings columns")?;

    if columns
        .iter()
        .any(|column| column == "api_key_filter_enabled")
    {
        return Ok(());
    }

    conn.execute(
        "ALTER TABLE clipmem_settings ADD COLUMN api_key_filter_enabled INTEGER NOT NULL DEFAULT 0 CHECK (api_key_filter_enabled IN (0, 1))",
        [],
    )
    .context("add api_key_filter_enabled column")?;
    Ok(())
}

pub(in crate::db) fn ensure_ocr_enabled_setting_column(conn: &Connection) -> Result<()> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(clipmem_settings)")
        .context("prepare clipmem_settings table info query")?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .context("query clipmem_settings columns")?;
    let columns = collect_rows(rows).context("collect clipmem_settings columns")?;

    if columns.iter().any(|column| column == "ocr_enabled") {
        return Ok(());
    }

    conn.execute(
        "ALTER TABLE clipmem_settings ADD COLUMN ocr_enabled INTEGER NOT NULL DEFAULT 0 CHECK (ocr_enabled IN (0, 1))",
        [],
    )
    .context("add ocr_enabled column")?;
    Ok(())
}

pub(in crate::db) fn ensure_image_compression_columns(conn: &Connection) -> Result<()> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(item_representations)")
        .context("prepare item_representations table info query")?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .context("query item_representations columns")?;
    let columns = collect_rows(rows).context("collect item_representations columns")?;

    let add_column = |name: &str, sql: &str| -> Result<()> {
        if columns.iter().any(|column| column == name) {
            return Ok(());
        }
        conn.execute(sql, [])
            .with_context(|| format!("add {name} column"))?;
        Ok(())
    };

    add_column(
        "image_compression_status",
        "ALTER TABLE item_representations ADD COLUMN image_compression_status TEXT NOT NULL DEFAULT 'uncompressed' CHECK (image_compression_status IN ('uncompressed', 'compressed', 'skipped'))",
    )?;
    add_column(
        "image_compression_format",
        "ALTER TABLE item_representations ADD COLUMN image_compression_format TEXT",
    )?;
    add_column(
        "image_compressed_at",
        "ALTER TABLE item_representations ADD COLUMN image_compressed_at TEXT",
    )?;
    add_column(
        "image_original_byte_len",
        "ALTER TABLE item_representations ADD COLUMN image_original_byte_len INTEGER",
    )?;
    add_column(
        "image_original_raw_sha256",
        "ALTER TABLE item_representations ADD COLUMN image_original_raw_sha256 TEXT",
    )?;
    add_column(
        "image_compression_reason",
        "ALTER TABLE item_representations ADD COLUMN image_compression_reason TEXT",
    )?;
    Ok(())
}

pub(in crate::db) fn ensure_image_optimization_queue_index(conn: &Connection) -> Result<()> {
    conn.execute(
        r"
        CREATE INDEX IF NOT EXISTS idx_item_representations_image_optimization_queue
            ON item_representations(
                image_compression_status,
                byte_len DESC,
                snapshot_id ASC,
                item_index ASC,
                uti ASC
            )
            WHERE kind = 'image' AND length(blob_value) > 0
        ",
        [],
    )
    .context("create image optimization queue index")?;
    Ok(())
}

pub(in crate::db) fn ensure_representation_cache_deferred_column(conn: &Connection) -> Result<()> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(clipmem_settings)")
        .context("prepare clipmem_settings table info query")?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .context("query clipmem_settings columns")?;
    let columns = collect_rows(rows).context("collect clipmem_settings columns")?;

    if columns
        .iter()
        .any(|column| column == "representation_cache_deferred")
    {
        return Ok(());
    }

    conn.execute(
        "ALTER TABLE clipmem_settings ADD COLUMN representation_cache_deferred INTEGER NOT NULL DEFAULT 0 CHECK (representation_cache_deferred IN (0, 1))",
        [],
    )
    .context("add representation_cache_deferred column")?;
    Ok(())
}

pub(in crate::db) fn ensure_archive_revisions_table(conn: &Connection) -> Result<()> {
    conn.execute(
        r"
            CREATE TABLE IF NOT EXISTS archive_revisions (
                id                         INTEGER PRIMARY KEY CHECK (id = 1),
                revision                   INTEGER NOT NULL DEFAULT 0 CHECK (revision >= 0),
                archive_content_revision   INTEGER NOT NULL DEFAULT 0 CHECK (archive_content_revision >= 0),
                settings_revision          INTEGER NOT NULL DEFAULT 0 CHECK (settings_revision >= 0),
                ocr_revision               INTEGER NOT NULL DEFAULT 0 CHECK (ocr_revision >= 0),
                storage_revision           INTEGER NOT NULL DEFAULT 0 CHECK (storage_revision >= 0),
                service_revision           INTEGER NOT NULL DEFAULT 0 CHECK (service_revision >= 0),
                app_preferences_revision   INTEGER NOT NULL DEFAULT 0 CHECK (app_preferences_revision >= 0),
                last_change_kind           TEXT NOT NULL DEFAULT 'initialized',
                updated_at                 TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
        ",
        [],
    )
    .context("create archive_revisions table")?;
    Ok(())
}

pub(in crate::db) fn rebuild_snapshot_stats(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM snapshot_stats", [])
        .context("clear snapshot stats")?;
    conn.execute_batch(
        r"
        INSERT INTO snapshot_stats (
            snapshot_id,
            capture_count,
            first_observed_at,
            last_observed_at,
            last_event_id,
            last_frontmost_app_bundle_id,
            last_frontmost_app_name
        )
        SELECT
            ce.snapshot_id,
            COUNT(*) AS capture_count,
            MIN(ce.observed_at) AS first_observed_at,
            MAX(ce.observed_at) AS last_observed_at,
            (
                SELECT latest.id
                FROM capture_events latest
                WHERE latest.snapshot_id = ce.snapshot_id
                ORDER BY latest.observed_at DESC, latest.id DESC
                LIMIT 1
            ) AS last_event_id,
            (
                SELECT latest.frontmost_app_bundle_id
                FROM capture_events latest
                WHERE latest.snapshot_id = ce.snapshot_id
                ORDER BY latest.observed_at DESC, latest.id DESC
                LIMIT 1
            ) AS last_frontmost_app_bundle_id,
            (
                SELECT latest.frontmost_app_name
                FROM capture_events latest
                WHERE latest.snapshot_id = ce.snapshot_id
                ORDER BY latest.observed_at DESC, latest.id DESC
                LIMIT 1
            ) AS last_frontmost_app_name
        FROM capture_events ce
        GROUP BY ce.snapshot_id;
        ",
    )
    .context("rebuild snapshot stats")?;
    Ok(())
}

pub(in crate::db) fn rebuild_snapshot_projection_cache(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM snapshot_projection_cache", [])
        .context("clear snapshot projection cache")?;
    conn.execute_batch(
        r"
        WITH url_values AS (
            SELECT
                snapshot_id,
                GROUP_CONCAT(text_value, char(31)) AS urls
            FROM (
                SELECT DISTINCT snapshot_id, text_value
                FROM item_representations
                WHERE kind = 'url' AND text_value IS NOT NULL AND text_value != ''
                ORDER BY text_value
            )
            GROUP BY snapshot_id
        ),
        file_url_values AS (
            SELECT
                snapshot_id,
                GROUP_CONCAT(text_value, char(31)) AS file_urls
            FROM (
                SELECT DISTINCT snapshot_id, text_value
                FROM item_representations
                WHERE kind = 'file_url' AND text_value IS NOT NULL AND text_value != ''
                ORDER BY text_value
            )
            GROUP BY snapshot_id
        )
        INSERT INTO snapshot_projection_cache (snapshot_id, urls, file_urls)
        SELECT
            s.id,
            COALESCE(uv.urls, ''),
            COALESCE(fv.file_urls, '')
        FROM snapshots s
        LEFT JOIN url_values uv ON uv.snapshot_id = s.id
        LEFT JOIN file_url_values fv ON fv.snapshot_id = s.id;
        ",
    )
    .context("rebuild snapshot projection cache")?;
    Ok(())
}

pub(in crate::db) fn rebuild_snapshot_event_filter_cache(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM snapshot_event_filter_cache", [])
        .context("clear snapshot event filter cache")?;
    conn.execute_batch(
        r"
        INSERT INTO snapshot_event_filter_cache (snapshot_id, app_names_lower, bundle_ids_lower)
        SELECT
            s.id,
            COALESCE((
                SELECT GROUP_CONCAT(app_name, char(31))
                FROM (
                    SELECT DISTINCT lower(ce.frontmost_app_name) AS app_name
                    FROM capture_events ce
                    WHERE ce.snapshot_id = s.id
                      AND ce.frontmost_app_name IS NOT NULL
                      AND ce.frontmost_app_name != ''
                    ORDER BY app_name
                )
            ), '') AS app_names_lower,
            COALESCE((
                SELECT GROUP_CONCAT(bundle_id, char(31))
                FROM (
                    SELECT DISTINCT lower(ce.frontmost_app_bundle_id) AS bundle_id
                    FROM capture_events ce
                    WHERE ce.snapshot_id = s.id
                      AND ce.frontmost_app_bundle_id IS NOT NULL
                      AND ce.frontmost_app_bundle_id != ''
                    ORDER BY bundle_id
                )
            ), '') AS bundle_ids_lower
        FROM snapshots s;
        ",
    )
    .context("rebuild snapshot event filter cache")?;
    Ok(())
}

#[derive(Debug)]
pub(in crate::db) struct StoredProjectionItem {
    item_index: i64,
    representations: Vec<StoredProjectionRepresentation>,
}

#[derive(Debug)]
pub(in crate::db) struct StoredProjectionRepresentation {
    uti: String,
    kind: ClipboardKind,
    byte_len: i64,
    text_value: Option<String>,
}

pub(in crate::db) fn rebuild_snapshot_text_from_representations(conn: &Connection) -> Result<()> {
    let snapshot_ids = {
        let mut stmt = conn
            .prepare("SELECT id FROM snapshots ORDER BY id ASC")
            .context("prepare snapshot ids query")?;
        let rows = stmt
            .query_map([], |row| row.get::<_, i64>(0))
            .context("query snapshot ids")?;
        collect_rows(rows).context("collect snapshot ids")?
    };

    for snapshot_id in snapshot_ids {
        let items = load_stored_projection_items(conn, snapshot_id)?;
        let mut snapshot_previews = Vec::new();
        let mut snapshot_search_fragments = Vec::new();

        for item in items {
            let search_text = rebuilt_item_search_text(&item.representations);
            let preview_text = rebuilt_item_preview_text(&item.representations, &search_text);
            conn.execute(
                "UPDATE snapshot_items
                 SET primary_kind = ?1,
                     primary_uti = ?2,
                     preview_text = ?3,
                     search_text = ?4
                 WHERE snapshot_id = ?5 AND item_index = ?6",
                rusqlite::params![
                    rebuilt_primary_kind(&item.representations).as_str(),
                    rebuilt_primary_uti(&item.representations),
                    preview_text,
                    search_text,
                    snapshot_id,
                    item.item_index,
                ],
            )
            .with_context(|| {
                format!(
                    "update text projection for snapshot {snapshot_id} item {}",
                    item.item_index
                )
            })?;

            if !preview_text.trim().is_empty() {
                snapshot_previews.push(preview_text);
            }
            if !search_text.trim().is_empty() {
                snapshot_search_fragments.push(search_text);
            }
        }

        let preview_text = if snapshot_previews.is_empty() {
            "[empty clipboard]".to_string()
        } else {
            truncate_chars(&snapshot_previews.join(" | "), 280)
        };
        let search_text = snapshot_search_fragments.join("\n\n");

        conn.execute(
            "UPDATE snapshots SET preview_text = ?1, search_text = ?2 WHERE id = ?3",
            rusqlite::params![preview_text, search_text, snapshot_id],
        )
        .with_context(|| format!("update text projection for snapshot {snapshot_id}"))?;
    }

    Ok(())
}

pub(in crate::db) fn load_stored_projection_items(
    conn: &Connection,
    snapshot_id: i64,
) -> Result<Vec<StoredProjectionItem>> {
    let mut stmt = conn
        .prepare(
            r"
            SELECT item_index, uti, kind, byte_len, text_value
            FROM item_representations
            WHERE snapshot_id = ?1
            ORDER BY item_index ASC, uti ASC
            ",
        )
        .context("prepare stored representation projection query")?;
    let rows = stmt
        .query_map([snapshot_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                StoredProjectionRepresentation {
                    uti: row.get(1)?,
                    kind: row_enum(row, 2)?,
                    byte_len: row.get(3)?,
                    text_value: row.get(4)?,
                },
            ))
        })
        .context("query stored representation projection rows")?;

    let mut items = Vec::<StoredProjectionItem>::new();
    for row in rows {
        let (item_index, representation) = row?;
        if let Some(item) = items
            .iter_mut()
            .find(|candidate| candidate.item_index == item_index)
        {
            item.representations.push(representation);
        } else {
            items.push(StoredProjectionItem {
                item_index,
                representations: vec![representation],
            });
        }
    }
    Ok(items)
}

pub(in crate::db) fn rebuilt_primary_representation(
    representations: &[StoredProjectionRepresentation],
) -> Option<&StoredProjectionRepresentation> {
    representations.iter().min_by_key(|representation| {
        (
            representation.kind.priority(),
            !representation
                .text_value
                .as_deref()
                .is_some_and(is_searchable_text_fragment),
            representation.uti.as_str(),
        )
    })
}

pub(in crate::db) fn rebuilt_primary_kind(
    representations: &[StoredProjectionRepresentation],
) -> ClipboardKind {
    rebuilt_primary_representation(representations).map_or(ClipboardKind::Empty, |rep| rep.kind)
}

pub(in crate::db) fn rebuilt_primary_uti(
    representations: &[StoredProjectionRepresentation],
) -> Option<&str> {
    rebuilt_primary_representation(representations).map(|rep| rep.uti.as_str())
}

pub(in crate::db) fn rebuilt_item_search_text(
    representations: &[StoredProjectionRepresentation],
) -> String {
    dedupe_text_fragments(
        representations
            .iter()
            .filter_map(rebuilt_search_fragment_for_representation),
    )
    .join("\n\n")
}

pub(in crate::db) fn rebuilt_search_fragment_for_representation(
    representation: &StoredProjectionRepresentation,
) -> Option<String> {
    if !representation.kind.is_textual() {
        return None;
    }

    let text = representation.text_value.as_deref()?;
    if !is_searchable_text_fragment(text) {
        return None;
    }

    let projected = match representation.kind {
        ClipboardKind::Html => html_to_text_lossy(text),
        ClipboardKind::Rtf => rtf_to_text_lossy(text),
        _ => text.to_string(),
    };
    let normalized = normalize_whitespace(&projected);
    is_searchable_text_fragment(&normalized).then_some(normalized)
}

pub(in crate::db) fn rebuilt_item_preview_text(
    representations: &[StoredProjectionRepresentation],
    search_text: &str,
) -> String {
    if !search_text.is_empty() {
        return truncate_chars(&search_text.replace('\n', " "), 200);
    }

    if let Some(rep) = rebuilt_primary_representation(representations) {
        return truncate_chars(
            &format!("[{} · {} bytes · {}]", rep.kind, rep.byte_len, rep.uti),
            200,
        );
    }

    "[empty clipboard item]".to_string()
}

pub(in crate::db) fn rebuild_snapshot_literal_cache(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM snapshot_literal_cache", [])
        .context("clear snapshot literal cache")?;
    conn.execute("DELETE FROM snapshots_literal_fts", [])
        .context("clear literal FTS cache")?;
    conn.execute_batch(
        r"
        INSERT INTO snapshot_literal_cache (snapshot_id, haystack)
        SELECT
            s.id,
            lower(
                COALESCE(NULLIF(s.preview_text, ''), s.search_text, '') || char(31) ||
                COALESCE(s.preview_text, '') || char(31) ||
                COALESCE(s.search_text, '') || char(31) ||
                COALESCE(sp.urls, '') || char(31) ||
                COALESCE(sp.file_urls, '') || char(31) ||
                COALESCE(ss.last_frontmost_app_name, '') || char(31) ||
                COALESCE(ss.last_frontmost_app_bundle_id, '')
            )
        FROM snapshots s
        LEFT JOIN snapshot_projection_cache sp ON sp.snapshot_id = s.id
        LEFT JOIN snapshot_stats ss ON ss.snapshot_id = s.id;
        ",
    )
    .context("rebuild snapshot literal cache")?;
    Ok(())
}

pub(in crate::db) fn rebuild_snapshot_file_url_fts(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT INTO snapshot_file_url_fts(snapshot_file_url_fts) VALUES ('rebuild')",
        [],
    )
    .context("rebuild snapshot file-url FTS")?;
    Ok(())
}

pub(in crate::db) fn legacy_prerelease_schema_detected(conn: &Connection) -> Result<bool> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(item_representations)")
        .context("prepare PRAGMA table_info(item_representations)")?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .context("read item_representations columns")?;
    let columns = collect_rows(rows).context("collect item_representations columns")?;
    if columns.is_empty() {
        return Ok(false);
    }

    let has_kind = columns.iter().any(|column| column == "kind");
    let has_legacy_marker = LEGACY_PRERELEASE_COLUMNS
        .iter()
        .any(|legacy| columns.iter().any(|column| column == legacy));
    Ok(!has_kind || has_legacy_marker)
}

#[cfg(test)]
pub(crate) fn explain_query_plan(
    conn: &Connection,
    sql: &str,
    params: &[&dyn rusqlite::ToSql],
) -> Result<Vec<String>> {
    let explain = format!("EXPLAIN QUERY PLAN {sql}");
    let mut stmt = conn
        .prepare(&explain)
        .context("prepare EXPLAIN QUERY PLAN")?;
    let rows = stmt
        .query_map(params, |row| row.get::<_, String>(3))
        .context("execute EXPLAIN QUERY PLAN")?;
    collect_rows(rows).context("collect EXPLAIN QUERY PLAN rows")
}

use anyhow::{Context, Result};
use rusqlite::params;

use crate::db::sqlite_helpers::row_u64;
use crate::db::types::{ArchiveChangeKind, ArchiveRevision, Database};

impl Database {
    pub fn archive_revision(&self) -> Result<ArchiveRevision> {
        self.conn
            .query_row(
                r"
                    SELECT
                        revision,
                        archive_content_revision,
                        settings_revision,
                        ocr_revision,
                        storage_revision,
                        service_revision,
                        app_preferences_revision,
                        last_change_kind,
                        updated_at
                    FROM archive_revisions
                    WHERE id = 1
                ",
                [],
                |row| {
                    Ok(ArchiveRevision::new(
                        row_u64(row, 0)?,
                        row_u64(row, 1)?,
                        row_u64(row, 2)?,
                        row_u64(row, 3)?,
                        row_u64(row, 4)?,
                        row_u64(row, 5)?,
                        row_u64(row, 6)?,
                        row.get(7)?,
                        row.get(8)?,
                    ))
                },
            )
            .context("load archive revision")
    }

    pub(crate) fn bump_service_revision(&self) -> Result<ArchiveRevision> {
        bump_revision(&self.conn, &[ArchiveChangeKind::Service])?;
        self.archive_revision()
    }

    pub(crate) fn bump_app_preferences_revision(&self) -> Result<ArchiveRevision> {
        bump_revision(&self.conn, &[ArchiveChangeKind::AppPreferences])?;
        self.archive_revision()
    }
}

pub(in crate::db) fn bump_revision(
    conn: &rusqlite::Connection,
    kinds: &[ArchiveChangeKind],
) -> Result<()> {
    if kinds.is_empty() {
        return Ok(());
    }

    let assignments = kinds
        .iter()
        .map(|kind| {
            format!(
                "{} = {} + 1",
                kind.revision_column(),
                kind.revision_column()
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let kind_label = change_kind_label(kinds);
    let sql = format!(
        r"
            INSERT INTO archive_revisions (id, revision, {first_column}, last_change_kind, updated_at)
            VALUES (1, 1, 1, ?1, CURRENT_TIMESTAMP)
            ON CONFLICT(id) DO UPDATE SET
                revision = revision + 1,
                {assignments},
                last_change_kind = excluded.last_change_kind,
                updated_at = CURRENT_TIMESTAMP
        ",
        first_column = kinds[0].revision_column(),
    );
    conn.execute(&sql, params![kind_label])
        .context("bump archive revision")?;
    Ok(())
}

pub(in crate::db) fn bump_revision_tx(
    tx: &rusqlite::Transaction<'_>,
    kinds: &[ArchiveChangeKind],
) -> Result<()> {
    bump_revision(tx, kinds)
}

fn change_kind_label(kinds: &[ArchiveChangeKind]) -> String {
    kinds
        .iter()
        .map(|kind| kind.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

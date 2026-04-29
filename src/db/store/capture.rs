use anyhow::{Context, Result};
use rusqlite::{params, OptionalExtension, TransactionBehavior};

use super::config::RESTORE_SUPPRESSION_WINDOW_SECONDS;
use super::rebuild::{
    insert_item, rebuild_snapshot_projection_cache_for_snapshot, set_representation_cache_deferred,
};
use super::revision::bump_revision_tx;
use crate::db::sqlite_helpers::usize_to_i64;
use crate::db::types::{ArchiveChangeKind, CaptureSkipReason, CaptureStoreOutcome, Database};
use crate::model::{CaptureStoreResult, ClipboardSnapshot};
use crate::sensitive;

impl Database {
    /// Store a captured clipboard snapshot and append a capture event for the observation.
    ///
    /// # Errors
    ///
    /// Returns an error if any transaction step fails, including snapshot, item,
    /// representation, or capture-event inserts, or the final commit.
    pub fn store_capture(&mut self, snapshot: &ClipboardSnapshot) -> Result<CaptureStoreResult> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .context("begin capture transaction")?;

        let inserted_snapshot_id: Option<i64> = tx
            .query_row(
                "INSERT INTO snapshots (
                    sha256,
                    snapshot_kind,
                    preview_text,
                    search_text,
                    item_count,
                    total_bytes
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ON CONFLICT(sha256) DO NOTHING
                RETURNING id",
                params![
                    snapshot.fingerprint(),
                    snapshot.snapshot_kind().as_str(),
                    snapshot.preview_text(),
                    snapshot.search_text(),
                    usize_to_i64(snapshot.item_count())?,
                    usize_to_i64(snapshot.total_bytes())?,
                ],
                |row| row.get(0),
            )
            .optional()
            .context("insert snapshot row")?;

        let snapshot_id = if let Some(id) = inserted_snapshot_id {
            set_representation_cache_deferred(&tx, true)?;
            for item in snapshot.items() {
                insert_item(&tx, id, item)
                    .with_context(|| format!("insert snapshot item {}", item.item_index()))?;
            }
            rebuild_snapshot_projection_cache_for_snapshot(&tx, id)?;
            set_representation_cache_deferred(&tx, false)?;

            id
        } else {
            tx.query_row(
                "SELECT id FROM snapshots WHERE sha256 = ?1",
                [snapshot.fingerprint()],
                |row| row.get(0),
            )
            .context("lookup existing snapshot by fingerprint")?
        };

        let inserted_event_rows = tx
            .execute(
                "INSERT INTO capture_events (
                snapshot_id,
                change_count,
                frontmost_app_bundle_id,
                frontmost_app_name
            ) VALUES (?1, ?2, ?3, ?4)",
                params![
                    snapshot_id,
                    snapshot.change_count(),
                    snapshot.frontmost_app_bundle_id(),
                    snapshot.frontmost_app_name(),
                ],
            )
            .context("insert capture event row")?;
        if inserted_event_rows == 0 {
            tx.commit()
                .context("commit suppressed capture transaction")?;
            anyhow::bail!("capture event suppressed by pending restore marker");
        }
        let event_id = tx.last_insert_rowid();

        bump_revision_tx(&tx, &[ArchiveChangeKind::ArchiveContent])?;
        tx.commit().context("commit capture transaction")?;

        Ok(CaptureStoreResult::new(
            snapshot_id,
            event_id,
            inserted_snapshot_id.is_some(),
        ))
    }

    pub(crate) fn store_capture_if_allowed(
        &mut self,
        snapshot: &ClipboardSnapshot,
    ) -> Result<CaptureStoreOutcome> {
        let settings = self.capture_settings()?;
        if settings.api_key_filter_enabled()
            && sensitive::should_skip_snapshot_for_api_key_filter(snapshot)
        {
            return Ok(CaptureStoreOutcome::Skipped(
                CaptureSkipReason::ApiKeyFilter,
            ));
        }

        self.store_capture(snapshot)
            .map(CaptureStoreOutcome::Stored)
    }

    pub(crate) fn store_watched_capture_if_allowed(
        &mut self,
        snapshot: &ClipboardSnapshot,
    ) -> Result<CaptureStoreOutcome> {
        if self.consume_pending_restore(snapshot.fingerprint())? {
            return Ok(CaptureStoreOutcome::Skipped(
                CaptureSkipReason::RestoredSnapshot,
            ));
        }

        self.store_capture_if_allowed(snapshot)
    }

    pub(crate) fn register_pending_restore(&self, snapshot_sha256: &str) -> Result<()> {
        self.delete_expired_pending_restores()?;
        self.conn
            .execute(
                "INSERT INTO pending_restores (snapshot_sha256, created_at)
                 VALUES (?1, CURRENT_TIMESTAMP)
                 ON CONFLICT(snapshot_sha256) DO UPDATE SET created_at = CURRENT_TIMESTAMP",
                [snapshot_sha256],
            )
            .context("register pending restore")?;
        Ok(())
    }

    pub(crate) fn clear_pending_restore(&self, snapshot_sha256: &str) -> Result<()> {
        self.conn
            .execute(
                "DELETE FROM pending_restores WHERE snapshot_sha256 = ?1",
                [snapshot_sha256],
            )
            .context("clear pending restore")?;
        Ok(())
    }

    fn consume_pending_restore(&mut self, snapshot_sha256: &str) -> Result<bool> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .context("begin pending restore transaction")?;
        delete_expired_pending_restores(&tx)?;
        let consumed = tx
            .execute(
                "DELETE FROM pending_restores WHERE snapshot_sha256 = ?1",
                [snapshot_sha256],
            )
            .context("consume pending restore")?
            != 0;
        tx.commit().context("commit pending restore transaction")?;
        Ok(consumed)
    }

    fn delete_expired_pending_restores(&self) -> Result<()> {
        delete_expired_pending_restores(&self.conn)
    }
}

fn delete_expired_pending_restores(conn: &rusqlite::Connection) -> Result<()> {
    let expiry_window = format!("-{RESTORE_SUPPRESSION_WINDOW_SECONDS} seconds");
    conn.execute(
        "DELETE FROM pending_restores
         WHERE datetime(created_at) < datetime('now', ?1)",
        [expiry_window],
    )
    .context("delete expired pending restores")?;
    Ok(())
}

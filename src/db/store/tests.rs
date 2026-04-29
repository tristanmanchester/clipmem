use anyhow::Result;

use crate::db::{Database, RetrievalFilters};
use crate::model::{build_item, build_representation, build_snapshot, CaptureContext};

pub(in crate::db) fn fake_snapshot(
    change_count: i64,
    text: &str,
) -> crate::model::ClipboardSnapshot {
    build_snapshot(
        CaptureContext::new(change_count).with_frontmost_app_name("Editor"),
        vec![build_item(
            0,
            vec![build_representation(
                "public.utf8-plain-text".to_string(),
                None,
                text.as_bytes().to_vec(),
            )],
        )],
    )
}

#[test]
pub(in crate::db) fn empty_snapshots_store_without_placeholder_rows() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    let snapshot = build_snapshot(CaptureContext::new(9), Vec::new());

    let store = db.store_capture(&snapshot)?;
    let stored_item_count: i64 = db.conn.query_row(
        "SELECT item_count FROM snapshots WHERE id = ?1",
        [store.snapshot_id()],
        |row| row.get(0),
    )?;
    let stored_row_count: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM snapshot_items WHERE snapshot_id = ?1",
        [store.snapshot_id()],
        |row| row.get(0),
    )?;

    assert_eq!(stored_item_count, 0);
    assert_eq!(stored_row_count, 0);
    Ok(())
}

#[test]
pub(in crate::db) fn duplicate_snapshots_reuse_content_row_and_append_events() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    let first = fake_snapshot(1, "git status");
    let second = fake_snapshot(2, "git status");

    let first_store = db.store_capture(&first)?;
    let second_store = db.store_capture(&second)?;
    let event_count: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM capture_events WHERE snapshot_id = ?1",
        [first_store.snapshot_id()],
        |row| row.get(0),
    )?;

    assert!(first_store.inserted_new_snapshot());
    assert!(!second_store.inserted_new_snapshot());
    assert_eq!(first_store.snapshot_id(), second_store.snapshot_id());
    assert_eq!(event_count, 2);
    Ok(())
}

#[test]
pub(in crate::db) fn store_capture_bumps_archive_revision_once_per_event() -> Result<()> {
    let mut db = Database::open_in_memory()?;

    let initial = db.archive_revision()?;
    db.store_capture(&fake_snapshot(1, "git status"))?;
    let after_first = db.archive_revision()?;
    db.store_capture(&fake_snapshot(2, "git status"))?;
    let after_second = db.archive_revision()?;

    assert_eq!(initial.revision(), 0);
    assert_eq!(after_first.revision(), 1);
    assert_eq!(after_first.archive_content_revision(), 1);
    assert_eq!(after_first.last_change_kind(), "archive_content");
    assert_eq!(after_second.revision(), 2);
    assert_eq!(after_second.archive_content_revision(), 2);
    Ok(())
}

#[test]
pub(in crate::db) fn settings_revision_only_changes_for_real_setting_mutations() -> Result<()> {
    let mut db = Database::open_in_memory()?;

    db.set_paused(true)?;
    let changed = db.archive_revision()?;
    db.set_paused(true)?;
    let repeated = db.archive_revision()?;
    db.add_ignored_bundle_id("com.apple.Terminal")?;
    let ignored = db.archive_revision()?;

    assert_eq!(changed.revision(), 1);
    assert_eq!(changed.settings_revision(), 1);
    assert_eq!(changed.last_change_kind(), "settings");
    assert_eq!(repeated.revision(), changed.revision());
    assert_eq!(ignored.revision(), 2);
    assert_eq!(ignored.settings_revision(), 2);
    Ok(())
}

#[test]
pub(in crate::db) fn deferred_representation_cache_keeps_file_urls_searchable() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    let snapshot = build_snapshot(
        CaptureContext::new(1)
            .with_frontmost_app_name("Finder")
            .with_frontmost_app_bundle_id("com.apple.finder"),
        vec![build_item(
            0,
            vec![
                build_representation(
                    "public.file-url-a".to_string(),
                    Some("file:///tmp/clipmem-one.txt".to_string()),
                    b"file:///tmp/clipmem-one.txt".to_vec(),
                ),
                build_representation(
                    "public.file-url-b".to_string(),
                    Some("file:///tmp/clipmem-two.txt".to_string()),
                    b"file:///tmp/clipmem-two.txt".to_vec(),
                ),
            ],
        )],
    );

    let stored = db.store_capture(&snapshot)?;
    let (file_urls, deferred): (String, i64) = db.conn.query_row(
        "SELECT sp.file_urls, cs.representation_cache_deferred
         FROM snapshot_projection_cache sp
         CROSS JOIN clipmem_settings cs
         WHERE sp.snapshot_id = ?1 AND cs.id = 1",
        [stored.snapshot_id()],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let results = db.search_auto("/tmp/clipmem-two.txt", 10, &RetrievalFilters::default())?;

    assert_eq!(deferred, 0);
    assert!(file_urls.contains("file:///tmp/clipmem-one.txt"));
    assert!(file_urls.contains("file:///tmp/clipmem-two.txt"));
    assert_eq!(results.hits().len(), 1);
    assert_eq!(results.hits()[0].snapshot_id(), stored.snapshot_id());
    Ok(())
}

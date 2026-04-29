use super::*;
use rusqlite::params;

#[test]
fn duplicate_snapshots_share_content_row() -> Result<()> {
    let mut db = Database::open_in_memory()?;

    let first = fake_snapshot(1, "git clone https://example.com/repo");
    let second = fake_snapshot(2, "git clone https://example.com/repo");

    let first_store = db.store_capture(&first)?;
    let second_store = db.store_capture(&second)?;

    assert!(first_store.inserted_new_snapshot());
    assert!(!second_store.inserted_new_snapshot());
    assert_eq!(first_store.snapshot_id(), second_store.snapshot_id());

    let results = db.search_auto("git", 10, &unfiltered())?;
    assert_eq!(results.mode_used(), SearchMode::Fts);
    assert_eq!(results.hits().len(), 1);
    assert_eq!(results.hits()[0].capture_count(), 2);

    let details = db
        .find_snapshot(first_store.snapshot_id(), 10)?
        .context("expected stored snapshot details")?;
    assert_eq!(details.capture_count(), 2);
    assert_eq!(details.items().len(), 1);

    Ok(())
}

#[test]
fn open_existing_does_not_create_missing_database_paths() {
    let path = temp_db_path("open-existing-missing");

    let result = Database::open_existing(&path);

    assert!(result.is_err());
    assert!(!path.exists());
}

#[cfg(unix)]
#[test]
fn hardening_sqlite_sidecars_applies_private_file_permissions() -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let path = temp_db_path("sidecar-permissions");
    std::fs::create_dir_all(path.parent().unwrap())?;

    for suffix in ["-wal", "-shm"] {
        let sidecar = super::super::sidecar_path(&path, suffix);
        std::fs::write(&sidecar, b"sidecar")?;
        std::fs::set_permissions(&sidecar, std::fs::Permissions::from_mode(0o666))?;
    }

    super::super::harden_existing_sqlite_sidecar_permissions(&path)?;

    for suffix in ["-wal", "-shm"] {
        let sidecar = super::super::sidecar_path(&path, suffix);
        let mode = std::fs::metadata(&sidecar)?.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    cleanup_db(&path);
    Ok(())
}

#[test]
fn search_like_treats_percent_as_literal() -> Result<()> {
    let mut db = Database::open_in_memory()?;

    db.store_capture(&fake_snapshot(1, "Discount: 50 percent off"))?;
    db.store_capture(&fake_snapshot(2, "Discount: 50%"))?;

    let hits = db.search_literal("50%", 10, &unfiltered())?;
    let previews: Vec<_> = hits
        .hits()
        .iter()
        .map(crate::model::SearchHit::preview_text)
        .collect();

    assert_eq!(previews, vec!["Discount: 50%"]);
    Ok(())
}

#[test]
fn stats_empty_result_returns_zeroes_and_empty_leaderboards() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    db.store_capture(&fake_snapshot(1, "hello"))?;

    let stats = db.stats(&filters_with_app("No Such App"))?;

    assert_eq!(stats.snapshot_count(), 0);
    assert_eq!(stats.capture_event_count(), 0);
    assert_eq!(stats.unique_app_count(), 0);
    assert_eq!(stats.total_bytes(), 0);
    assert_eq!(stats.average_bytes_per_snapshot(), 0.0);
    assert_eq!(stats.average_captures_per_snapshot(), 0.0);
    assert_eq!(stats.dedupe_ratio(), 0.0);
    assert_eq!(stats.first_observed_at(), None);
    assert_eq!(stats.last_observed_at(), None);
    assert_eq!(stats.archive_span_seconds(), None);
    assert!(stats.most_recopied_snapshot().is_none());
    assert!(stats.kind_breakdown().is_empty());
    assert!(stats.top_apps().is_empty());
    assert_eq!(stats.busiest_hours().len(), 24);
    assert_eq!(stats.busiest_weekdays().len(), 7);
    assert!(stats.largest_snapshots().is_empty());
    assert!(stats.most_captured_snapshots().is_empty());
    Ok(())
}

#[test]
fn stats_uses_event_and_snapshot_filtering_semantics() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    let first = db.store_capture(&fake_snapshot(1, "repeat text"))?;
    let second = db.store_capture(&fake_snapshot(2, "repeat text"))?;
    let third = db.store_capture(&fake_snapshot(3, "other text"))?;
    set_event_app(
        &db,
        first.event_id(),
        Some("Terminal"),
        Some("com.apple.Terminal"),
    )?;
    set_event_app(
        &db,
        second.event_id(),
        Some("Safari"),
        Some("com.apple.Safari"),
    )?;
    set_event_app(
        &db,
        third.event_id(),
        Some("Safari"),
        Some("com.apple.Safari"),
    )?;

    let stats = db.stats(&filters_with_app("safari"))?;

    assert_eq!(stats.capture_event_count(), 2);
    assert_eq!(stats.snapshot_count(), 2);
    assert_eq!(stats.most_recopied_snapshot().unwrap().capture_count(), 1);
    assert_eq!(stats.top_apps()[0].app(), "Safari");
    assert_eq!(stats.top_apps()[0].capture_event_count(), 2);

    let text_stats = db.stats(&filters_with_kind(RetrievalKind::Text))?;
    assert_eq!(text_stats.capture_event_count(), 3);
    assert_eq!(text_stats.snapshot_count(), 2);
    Ok(())
}

#[test]
fn stats_app_grouping_falls_back_to_bundle_and_unknown() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    let first = db.store_capture(&fake_snapshot(1, "first"))?;
    let second = db.store_capture(&fake_snapshot(2, "second"))?;
    let third = db.store_capture(&fake_snapshot(3, "third"))?;
    set_event_app(
        &db,
        first.event_id(),
        Some("Named App"),
        Some("com.example.named"),
    )?;
    set_event_app(&db, second.event_id(), None, Some("com.example.bundle"))?;
    set_event_app(&db, third.event_id(), None, None)?;

    let stats = db.stats(&unfiltered())?;
    let apps: Vec<_> = stats.top_apps().iter().map(|entry| entry.app()).collect();

    assert!(apps.contains(&"Named App"));
    assert!(apps.contains(&"com.example.bundle"));
    assert!(apps.contains(&"Unknown"));
    assert_eq!(stats.unique_app_count(), 3);
    Ok(())
}

#[test]
fn stats_ordering_ties_are_deterministic() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    let first = db.store_capture(&fake_snapshot(1, "alpha"))?;
    let second = db.store_capture(&fake_snapshot(2, "bravo"))?;
    set_event_observed_at(&db, first.event_id(), "2026-04-17T10:00:00Z")?;
    set_event_observed_at(&db, second.event_id(), "2026-04-17T10:00:00Z")?;

    let stats = db.stats(&unfiltered())?;

    assert_eq!(
        stats.largest_snapshots()[0].snapshot_id(),
        first.snapshot_id()
    );
    assert_eq!(
        stats.most_captured_snapshots()[0].snapshot_id(),
        first.snapshot_id()
    );
    assert_eq!(stats.busiest_hours()[10].capture_event_count(), 2);
    assert_eq!(
        stats
            .busiest_weekdays()
            .iter()
            .find(|entry| entry.bucket() == "Friday")
            .unwrap()
            .capture_event_count(),
        2
    );
    Ok(())
}

#[test]
fn capture_policy_persists_across_reopen() -> Result<()> {
    let path = temp_db_path("capture-policy-persistence");

    {
        let mut db = Database::open_or_init(&path)?;
        db.set_paused(true)?;
        db.set_api_key_filter_enabled(true)?;
        db.set_retention_seconds(Some(30 * 24 * 60 * 60))?;
        assert!(db.add_ignored_bundle_id("Com.Apple.Terminal")?);
    }

    let reopened = Database::open_existing(&path)?;
    let policy = reopened.capture_policy()?;

    assert!(policy.settings().paused());
    assert!(policy.settings().api_key_filter_enabled());
    assert_eq!(
        policy.settings().retention_seconds(),
        Some(30 * 24 * 60 * 60)
    );
    assert_eq!(
        policy.ignored_bundle_ids(),
        &["com.apple.terminal".to_string()]
    );

    cleanup_db(&path);
    Ok(())
}

#[test]
fn ocr_setting_defaults_to_off_and_toggles() -> Result<()> {
    let mut db = Database::open_in_memory()?;

    assert!(!db.capture_settings()?.ocr_enabled());
    db.set_ocr_enabled(true)?;
    assert!(db.capture_settings()?.ocr_enabled());
    db.set_ocr_enabled(false)?;
    assert!(!db.capture_settings()?.ocr_enabled());

    Ok(())
}

#[test]
fn ready_ocr_text_is_cached_deduped_and_searchable() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    let stored = db.store_capture(&image_snapshot(
        1,
        vec![
            ("public.png", b"same-image-bytes".to_vec()),
            ("public.jpeg", b"same-image-bytes".to_vec()),
        ],
    ))?;

    assert_eq!(db.enqueue_ocr_for_snapshot(stored.snapshot_id())?, 1);
    assert_eq!(db.archive_revision()?.ocr_revision(), 1);
    let candidates = db.next_ocr_candidates(25, None, false)?;
    assert_eq!(candidates.len(), 1);
    db.store_ocr_text(
        candidates[0].raw_sha256(),
        "fake",
        "fast",
        "Invoice Total 42",
    )?;
    assert_eq!(db.archive_revision()?.ocr_revision(), 2);

    let details = db
        .find_snapshot(stored.snapshot_id(), 10)?
        .expect("stored image snapshot should exist");
    assert_eq!(details.ocr_text(), Some("Invoice Total 42"));
    assert_eq!(details.ocr_status(), Some("ready"));
    assert_eq!(details.best_text(), "Invoice Total 42");
    assert_eq!(details.best_text_uti(), Some("com.clipmem.ocr.text"));

    let results = db.search_auto("Invoice", 10, &unfiltered())?;
    assert_eq!(results.hits().len(), 1);
    assert_eq!(results.hits()[0].snapshot_id(), stored.snapshot_id());
    assert_eq!(
        results.hits()[0].matched_fields(),
        &["ocr_text".to_string()]
    );
    assert_eq!(db.ocr_status_report()?.snapshots_with_ocr_text(), 1);

    Ok(())
}

#[test]
fn failed_ocr_results_do_not_enter_search() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    let stored = db.store_capture(&image_snapshot(
        1,
        vec![("public.png", b"not-a-supported-image".to_vec())],
    ))?;
    db.enqueue_ocr_for_snapshot(stored.snapshot_id())?;
    let candidates = db.next_ocr_candidates(25, None, false)?;
    assert_eq!(candidates.len(), 1);

    db.store_ocr_failure(candidates[0].raw_sha256(), "fake", "fast", "decode failed")?;

    let details = db
        .find_snapshot(stored.snapshot_id(), 10)?
        .expect("stored image snapshot should exist");
    assert_eq!(details.ocr_text(), None);
    assert_eq!(details.ocr_status(), Some("failed"));
    assert!(db
        .search_auto("decode", 10, &unfiltered())?
        .hits()
        .is_empty());
    assert_eq!(db.ocr_status_report()?.failed(), 1);

    Ok(())
}

#[test]
fn store_capture_if_allowed_skips_api_key_like_snapshots_when_enabled() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    db.set_api_key_filter_enabled(true)?;

    let outcome = db.store_capture_if_allowed(&fake_snapshot(
        1,
        "Authorization: Bearer 8JfA-2mQpV_4tLz9XnR6cH0wKdS7yBu3",
    ))?;

    assert!(matches!(
        outcome,
        super::CaptureStoreOutcome::Skipped(super::CaptureSkipReason::ApiKeyFilter)
    ));
    assert!(db.recent(10, &unfiltered())?.hits().is_empty());
    Ok(())
}

#[test]
fn store_capture_if_allowed_stores_sensitive_text_when_filter_is_disabled() -> Result<()> {
    let mut db = Database::open_in_memory()?;

    let outcome = db.store_capture_if_allowed(&fake_snapshot(
        1,
        "Authorization: Bearer 8JfA-2mQpV_4tLz9XnR6cH0wKdS7yBu3",
    ))?;

    assert!(matches!(outcome, super::CaptureStoreOutcome::Stored(_)));
    assert_eq!(db.recent(10, &unfiltered())?.hits().len(), 1);
    Ok(())
}

#[test]
fn recent_reports_whether_more_results_are_available() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    db.store_capture(&fake_snapshot(1, "first recent item"))?;
    db.store_capture(&fake_snapshot(2, "second recent item"))?;

    let first_page = db.recent(1, &unfiltered())?;
    let full_page = db.recent(2, &unfiltered())?;

    assert_eq!(first_page.hits().len(), 1);
    assert!(first_page.has_more());
    assert_eq!(full_page.hits().len(), 2);
    assert!(!full_page.has_more());
    Ok(())
}

#[test]
fn timeline_exposes_public_events_and_pagination_state() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    db.store_capture(&fake_snapshot(1, "first timeline item"))?;
    db.store_capture(&fake_snapshot(2, "second timeline item"))?;

    let first_page = db.timeline(1, &unfiltered(), TimelineSort::Desc)?;
    let full_page = db.timeline(2, &unfiltered(), TimelineSort::Desc)?;

    assert_eq!(first_page.events().len(), 1);
    assert!(first_page.has_more());
    assert_eq!(full_page.events().len(), 2);
    assert!(!full_page.has_more());
    Ok(())
}

#[test]
fn watched_capture_skips_pending_restored_snapshot_once() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    let stored = db.store_capture(&fake_snapshot(1, "git status"))?;

    db.register_pending_restore(db.find_snapshot(stored.snapshot_id(), 1)?.unwrap().sha256())?;
    let outcome = db.store_watched_capture_if_allowed(&fake_snapshot(2, "git status"))?;

    assert!(matches!(
        outcome,
        super::CaptureStoreOutcome::Skipped(super::CaptureSkipReason::RestoredSnapshot)
    ));
    let event_count: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM capture_events", [], |row| row.get(0))?;
    assert_eq!(event_count, 1);

    let outcome = db.store_watched_capture_if_allowed(&fake_snapshot(3, "git status"))?;

    assert!(matches!(outcome, super::CaptureStoreOutcome::Stored(_)));
    let event_count: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM capture_events", [], |row| row.get(0))?;
    assert_eq!(event_count, 2);
    Ok(())
}

#[test]
fn pending_restore_trigger_suppresses_direct_capture_event_once() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    let stored = db.store_capture(&fake_snapshot(1, "git status"))?;
    let snapshot = db
        .find_snapshot(stored.snapshot_id(), 1)?
        .expect("stored snapshot should exist");

    db.register_pending_restore(snapshot.sha256())?;
    let inserted = db.conn.execute(
        "INSERT INTO capture_events (snapshot_id, change_count) VALUES (?1, ?2)",
        params![stored.snapshot_id(), 2_i64],
    )?;

    assert_eq!(inserted, 0);
    let event_count: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM capture_events", [], |row| row.get(0))?;
    let pending_count: i64 =
        db.conn
            .query_row("SELECT COUNT(*) FROM pending_restores", [], |row| {
                row.get(0)
            })?;
    let capture_count: i64 = db.conn.query_row(
        "SELECT capture_count FROM snapshot_stats WHERE snapshot_id = ?1",
        [stored.snapshot_id()],
        |row| row.get(0),
    )?;

    assert_eq!(event_count, 1);
    assert_eq!(pending_count, 0);
    assert_eq!(capture_count, 1);

    let inserted = db.conn.execute(
        "INSERT INTO capture_events (snapshot_id, change_count) VALUES (?1, ?2)",
        params![stored.snapshot_id(), 3_i64],
    )?;
    let event_count: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM capture_events", [], |row| row.get(0))?;
    let capture_count: i64 = db.conn.query_row(
        "SELECT capture_count FROM snapshot_stats WHERE snapshot_id = ?1",
        [stored.snapshot_id()],
        |row| row.get(0),
    )?;

    assert_eq!(inserted, 1);
    assert_eq!(event_count, 2);
    assert_eq!(capture_count, 2);
    Ok(())
}

#[test]
fn store_capture_reports_restore_trigger_suppression_without_stale_event_id() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    let stored = db.store_capture(&fake_snapshot(1, "git status"))?;
    let snapshot = db
        .find_snapshot(stored.snapshot_id(), 1)?
        .expect("stored snapshot should exist");

    db.register_pending_restore(snapshot.sha256())?;
    let err = db
        .store_capture(&fake_snapshot(2, "git status"))
        .expect_err("trigger suppression should not return a stale event id");

    assert!(err
        .to_string()
        .contains("capture event suppressed by pending restore marker"));
    let event_count: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM capture_events", [], |row| row.get(0))?;
    assert_eq!(event_count, 1);
    Ok(())
}

#[test]
fn expired_pending_restore_does_not_suppress_capture() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    let stored = db.store_capture(&fake_snapshot(1, "git status"))?;
    let sha256 = db
        .find_snapshot(stored.snapshot_id(), 1)?
        .unwrap()
        .sha256()
        .to_string();
    db.register_pending_restore(&sha256)?;
    db.conn.execute(
        "UPDATE pending_restores SET created_at = datetime('now', '-31 seconds') WHERE snapshot_sha256 = ?1",
        [&sha256],
    )?;

    let outcome = db.store_watched_capture_if_allowed(&fake_snapshot(2, "git status"))?;

    assert!(matches!(outcome, super::CaptureStoreOutcome::Stored(_)));
    let pending_count: i64 =
        db.conn
            .query_row("SELECT COUNT(*) FROM pending_restores", [], |row| {
                row.get(0)
            })?;
    assert_eq!(pending_count, 0);
    Ok(())
}

#[test]
fn expired_pending_restore_trigger_allows_direct_capture_event() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    let stored = db.store_capture(&fake_snapshot(1, "git status"))?;
    let sha256 = db
        .find_snapshot(stored.snapshot_id(), 1)?
        .unwrap()
        .sha256()
        .to_string();
    db.register_pending_restore(&sha256)?;
    db.conn.execute(
        "UPDATE pending_restores SET created_at = datetime('now', '-31 seconds') WHERE snapshot_sha256 = ?1",
        [&sha256],
    )?;

    let inserted = db.conn.execute(
        "INSERT INTO capture_events (snapshot_id, change_count) VALUES (?1, ?2)",
        params![stored.snapshot_id(), 2_i64],
    )?;

    let event_count: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM capture_events", [], |row| row.get(0))?;
    let pending_count: i64 =
        db.conn
            .query_row("SELECT COUNT(*) FROM pending_restores", [], |row| {
                row.get(0)
            })?;

    assert_eq!(inserted, 1);
    assert_eq!(event_count, 2);
    assert_eq!(pending_count, 0);
    Ok(())
}

#[test]
fn forget_snapshot_cascades_and_removes_search_visibility() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    let stored = db.store_capture(&fake_snapshot(1, "git status"))?;

    let report = db
        .forget_snapshot(stored.snapshot_id())?
        .expect("stored snapshot should be forgettable");

    assert_eq!(report.snapshot_id(), stored.snapshot_id());
    assert_eq!(report.item_count(), 1);
    assert_eq!(report.representation_count(), 1);
    assert_eq!(report.capture_event_count(), 1);
    assert_eq!(report.total_bytes(), "git status".len());
    assert!(db.find_snapshot(stored.snapshot_id(), 10)?.is_none());
    assert!(db.search_auto("git", 10, &unfiltered())?.hits().is_empty());

    let snapshot_count: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM snapshots", [], |row| row.get(0))?;
    let item_count: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM snapshot_items", [], |row| row.get(0))?;
    let representation_count: i64 =
        db.conn
            .query_row("SELECT COUNT(*) FROM item_representations", [], |row| {
                row.get(0)
            })?;
    let event_count: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM capture_events", [], |row| row.get(0))?;

    assert_eq!(snapshot_count, 0);
    assert_eq!(item_count, 0);
    assert_eq!(representation_count, 0);
    assert_eq!(event_count, 0);
    Ok(())
}

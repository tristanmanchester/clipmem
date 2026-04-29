use super::*;
use rusqlite::params;

#[test]
fn purge_uses_last_observed_at_and_dry_run_does_not_delete() -> Result<()> {
    let mut db = Database::open_in_memory()?;

    let old = db.store_capture(&fake_snapshot(1, "old clipboard entry"))?;
    let fresh = db.store_capture(&fake_snapshot(2, "fresh clipboard entry"))?;

    set_event_observed_at(&db, old.event_id(), "2000-01-01 00:00:00")?;
    db.conn.execute(
        "UPDATE snapshots SET created_at = '2000-01-01 00:00:00' WHERE id = ?1",
        [fresh.snapshot_id()],
    )?;

    let dry_run = db.purge_snapshots_older_than(30 * 24 * 60 * 60, true)?;
    assert!(dry_run.dry_run());
    assert_eq!(dry_run.snapshot_count(), 1);
    assert_eq!(dry_run.item_count(), 1);
    assert_eq!(dry_run.representation_count(), 1);
    assert_eq!(dry_run.capture_event_count(), 1);
    assert!(db.find_snapshot(old.snapshot_id(), 10)?.is_some());
    assert!(db.find_snapshot(fresh.snapshot_id(), 10)?.is_some());

    let deleted = db.purge_snapshots_older_than(30 * 24 * 60 * 60, false)?;
    assert!(!deleted.dry_run());
    assert_eq!(deleted.snapshot_count(), 1);
    assert!(db.find_snapshot(old.snapshot_id(), 10)?.is_none());
    assert!(db.find_snapshot(fresh.snapshot_id(), 10)?.is_some());
    assert!(db
        .search_auto("old clipboard", 10, &unfiltered())?
        .hits()
        .is_empty());
    assert_eq!(
        db.search_auto("fresh clipboard", 10, &unfiltered())?
            .hits()
            .len(),
        1
    );

    Ok(())
}

#[test]
fn search_like_treats_underscore_as_literal() -> Result<()> {
    let mut db = Database::open_in_memory()?;

    db.store_capture(&fake_snapshot(1, "configXtest"))?;
    db.store_capture(&fake_snapshot(2, "config test"))?;
    db.store_capture(&fake_snapshot(3, "config_test"))?;

    let hits = db.search_literal("config_test", 10, &unfiltered())?;
    let previews: Vec<_> = hits
        .hits()
        .iter()
        .map(crate::model::SearchHit::preview_text)
        .collect();

    assert_eq!(previews, vec!["config_test"]);
    Ok(())
}

#[test]
fn search_like_treats_escape_character_as_literal() -> Result<()> {
    let mut db = Database::open_in_memory()?;

    db.store_capture(&fake_snapshot(1, r"logs\2024\archive"))?;
    db.store_capture(&fake_snapshot(2, "logs/2024/archive"))?;

    let hits = db.search_literal(r"logs\2024", 10, &unfiltered())?;
    let previews: Vec<_> = hits
        .hits()
        .iter()
        .map(crate::model::SearchHit::preview_text)
        .collect();

    assert_eq!(previews, vec![r"logs\2024\archive"]);
    Ok(())
}

#[test]
fn search_literal_matches_file_url_paths() -> Result<()> {
    let mut db = Database::open_in_memory()?;

    let snapshot = build_snapshot(
        CaptureContext::new(1)
            .with_frontmost_app_name("Finder")
            .with_frontmost_app_bundle_id("com.apple.finder"),
        vec![build_item(
            0,
            vec![build_representation(
                "public.file-url".to_string(),
                Some("file:///tmp/repo/42/Cargo.toml".to_string()),
                b"file:///tmp/repo/42/Cargo.toml".to_vec(),
            )],
        )],
    );
    db.store_capture(&snapshot)?;

    let hits = db.search_literal("/tmp/repo/42/Cargo.toml", 10, &unfiltered())?;

    assert_eq!(hits.hits().len(), 1);
    assert_eq!(
        hits.hits()[0].preview_text(),
        "file:///tmp/repo/42/Cargo.toml"
    );

    let filtered_hits =
        db.search_literal("/tmp/repo/42/Cargo.toml", 10, &filters_with_app("finder"))?;

    assert_eq!(filtered_hits.hits().len(), 1);
    assert_eq!(
        filtered_hits.hits()[0].snapshot_id(),
        hits.hits()[0].snapshot_id()
    );
    Ok(())
}

#[test]
fn search_percent_literal_returns_only_exact_matches() -> Result<()> {
    let mut db = Database::open_in_memory()?;

    db.store_capture(&fake_snapshot(1, "Discount: 50 percent off"))?;
    db.store_capture(&fake_snapshot(2, "Discount: 50%"))?;

    let results = db.search_auto("50%", 10, &unfiltered())?;
    let previews: Vec<_> = results
        .hits()
        .iter()
        .map(crate::model::SearchHit::preview_text)
        .collect();

    assert_eq!(results.mode_used(), SearchMode::Literal);
    assert_eq!(previews, vec!["Discount: 50%"]);
    Ok(())
}

#[test]
fn search_underscore_literal_returns_only_exact_matches() -> Result<()> {
    let mut db = Database::open_in_memory()?;

    db.store_capture(&fake_snapshot(1, "configXtest"))?;
    db.store_capture(&fake_snapshot(2, "config test"))?;
    db.store_capture(&fake_snapshot(3, "config_test"))?;

    let results = db.search_auto("config_test", 10, &unfiltered())?;
    let previews: Vec<_> = results
        .hits()
        .iter()
        .map(crate::model::SearchHit::preview_text)
        .collect();

    assert_eq!(results.mode_used(), SearchMode::Literal);
    assert_eq!(previews, vec!["config_test"]);
    Ok(())
}

#[test]
fn search_propagates_non_syntax_fts_failures() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    db.store_capture(&fake_snapshot(1, "git clone https://example.com/repo"))?;
    db.conn.execute_batch("DROP TABLE snapshots_fts;")?;

    assert!(db.search_auto("git", 10, &unfiltered()).is_err());
    Ok(())
}

#[test]
fn schema_keeps_fts_index_and_triggers() {
    assert!(SCHEMA.contains("CREATE VIRTUAL TABLE IF NOT EXISTS snapshots_fts"));
    assert!(SCHEMA.contains("CREATE TRIGGER IF NOT EXISTS snapshots_ai"));
    assert!(SCHEMA.contains("CREATE TRIGGER IF NOT EXISTS snapshots_ad"));
    assert!(SCHEMA.contains("CREATE TRIGGER IF NOT EXISTS snapshots_au"));
    assert!(SCHEMA.contains("capture_events_restore_suppression_bi"));
    assert!(SCHEMA.contains("idx_capture_events_snapshot_observed_id"));
    assert!(SCHEMA.contains("idx_capture_events_observed_id"));
}

#[test]
fn timeline_returns_real_events_in_stable_descending_order() -> Result<()> {
    let mut db = Database::open_in_memory()?;

    let first = db.store_capture(&fake_snapshot(1, "git status"))?;
    let second = db.store_capture(&fake_snapshot(2, "git status"))?;
    let third = db.store_capture(&fake_snapshot(3, "cargo test"))?;

    set_event_observed_at(&db, first.event_id(), "2026-04-16 10:00:00")?;
    set_event_observed_at(&db, second.event_id(), "2026-04-16 10:00:00")?;
    set_event_observed_at(&db, third.event_id(), "2026-04-16 11:00:00")?;

    let page = db.timeline_page(10, &unfiltered(), TimelineSort::Desc, None)?;
    let ids = page
        .items()
        .iter()
        .map(|event| event.event_id())
        .collect::<Vec<_>>();

    assert_eq!(
        ids,
        vec![third.event_id(), second.event_id(), first.event_id()]
    );
    assert_eq!(page.items()[1].snapshot_id(), first.snapshot_id());
    assert_eq!(page.items()[1].change_count(), 2);
    Ok(())
}

#[test]
fn timeline_paging_respects_sort_and_cursor_boundaries() -> Result<()> {
    let mut db = Database::open_in_memory()?;

    let first = db.store_capture(&fake_snapshot(1, "alpha"))?;
    let second = db.store_capture(&fake_snapshot(2, "beta"))?;
    let third = db.store_capture(&fake_snapshot(3, "gamma"))?;

    set_event_observed_at(&db, first.event_id(), "2026-04-16 09:00:00")?;
    set_event_observed_at(&db, second.event_id(), "2026-04-16 10:00:00")?;
    set_event_observed_at(&db, third.event_id(), "2026-04-16 11:00:00")?;

    let first_page = db.timeline_page(2, &unfiltered(), TimelineSort::Asc, None)?;
    assert!(first_page.has_more());
    let cursor = super::TimelineCursorState::new(
        first_page
            .items()
            .last()
            .expect("timeline page should not be empty")
            .observed_at()
            .to_string(),
        first_page
            .items()
            .last()
            .expect("timeline page should not be empty")
            .event_id(),
    );

    let second_page = db.timeline_page(2, &unfiltered(), TimelineSort::Asc, Some(&cursor))?;
    let ids = second_page
        .items()
        .iter()
        .map(|event| event.event_id())
        .collect::<Vec<_>>();

    assert!(!second_page.has_more());
    assert_eq!(ids, vec![third.event_id()]);
    Ok(())
}

#[test]
fn shared_filters_constrain_search_recent_and_timeline_consistently() -> Result<()> {
    let mut db = Database::open_in_memory()?;

    let rich = build_snapshot(
        CaptureContext::new(1)
            .with_frontmost_app_name("Terminal")
            .with_frontmost_app_bundle_id("com.apple.Terminal"),
        vec![build_item(
            0,
            vec![
                build_representation(
                    "public.utf8-plain-text".to_string(),
                    Some("git clone https://example.com/repo".to_string()),
                    b"git clone https://example.com/repo".to_vec(),
                ),
                build_representation(
                    "public.url".to_string(),
                    Some("https://example.com/repo".to_string()),
                    b"https://example.com/repo".to_vec(),
                ),
            ],
        )],
    );
    let preview = fake_snapshot(2, "meeting notes");

    let first = db.store_capture(&rich)?;
    let second = db.store_capture(&rich)?;
    let _third = db.store_capture(&preview)?;

    set_event_observed_at(&db, first.event_id(), "2026-04-16 09:00:00")?;
    set_event_observed_at(&db, second.event_id(), "2026-04-16 10:00:00")?;

    let filters = RetrievalFilters::default()
        .with_app(Some("terminal".to_string()))
        .with_kind(Some(RetrievalKind::Url))
        .requiring_url()
        .with_min_bytes(Some(20));

    let search = db.search_auto("example.com", 10, &filters)?;
    let recent = db.recent(10, &filters)?;
    let timeline = db.timeline_page(10, &filters, TimelineSort::Desc, None)?;

    assert_eq!(search.hits().len(), 1);
    assert_eq!(recent.hits().len(), 1);
    assert_eq!(timeline.items().len(), 2);
    assert_eq!(search.hits()[0].snapshot_id(), first.snapshot_id());
    assert_eq!(recent.hits()[0].snapshot_id(), first.snapshot_id());
    assert!(timeline
        .items()
        .iter()
        .all(|event| event.snapshot_id() == first.snapshot_id()));
    Ok(())
}

#[test]
fn configure_connection_enables_foreign_keys() -> Result<()> {
    let conn = rusqlite::Connection::open_in_memory()?;
    configure_connection(&conn)?;

    let foreign_keys_enabled: i64 = conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
    assert_eq!(foreign_keys_enabled, 1);
    Ok(())
}

#[test]
fn search_auto_falls_back_for_url_queries() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    db.store_capture(&fake_snapshot(1, "git clone https://example.com/repo"))?;

    let results = db.search_auto("https://example.com/repo", 10, &unfiltered())?;

    assert_eq!(results.mode_used(), SearchMode::Literal);
    assert_eq!(results.hits().len(), 1);
    assert_eq!(
        results.hits()[0].preview_text(),
        "git clone https://example.com/repo"
    );
    Ok(())
}

#[test]
fn search_auto_falls_back_for_colon_queries() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    db.store_capture(&fake_snapshot(1, "foo:bar"))?;

    let results = db.search_auto("foo:bar", 10, &unfiltered())?;

    assert_eq!(results.mode_used(), SearchMode::Literal);
    assert_eq!(results.hits().len(), 1);
    assert_eq!(results.hits()[0].preview_text(), "foo:bar");
    Ok(())
}

#[test]
fn search_auto_falls_back_for_bundle_id_queries() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    db.store_capture(&fake_snapshot(1, "com.apple.Terminal"))?;

    let results = db.search_auto("com.apple.Terminal", 10, &unfiltered())?;

    assert_eq!(results.mode_used(), SearchMode::Literal);
    assert_eq!(results.hits().len(), 1);
    assert_eq!(results.hits()[0].preview_text(), "com.apple.Terminal");
    Ok(())
}

#[test]
fn search_auto_falls_back_for_slashy_path_queries() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    db.store_capture(&fake_snapshot(1, "logs/2024/archive"))?;

    let results = db.search_auto("logs/2024", 10, &unfiltered())?;

    assert_eq!(results.mode_used(), SearchMode::Literal);
    assert_eq!(results.hits().len(), 1);
    assert_eq!(results.hits()[0].preview_text(), "logs/2024/archive");
    Ok(())
}

#[test]
fn search_auto_literal_phrase_uses_unquoted_escaped_phrase() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    db.store_capture(&fake_snapshot(1, "configXtest"))?;
    db.store_capture(&fake_snapshot(2, "config_test"))?;

    let results = db.search_auto("\"config_test\"", 10, &unfiltered())?;
    let previews: Vec<_> = results
        .hits()
        .iter()
        .map(crate::model::SearchHit::preview_text)
        .collect();

    assert_eq!(results.mode_used(), SearchMode::Literal);
    assert_eq!(previews, vec!["config_test"]);
    Ok(())
}

#[test]
fn search_auto_handles_leading_colon_queries_without_error() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    db.store_capture(&fake_snapshot(1, "git clone https://example.com/repo"))?;

    let results = db.search_auto(":leading", 10, &unfiltered())?;

    assert_eq!(results.mode_used(), SearchMode::Literal);
    assert!(results.hits().is_empty());
    Ok(())
}

#[test]
fn search_auto_keeps_valid_quoted_fts_queries_in_fts_mode() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    db.store_capture(&fake_snapshot(1, "launchctl bootstrap"))?;

    let results = db.search_auto("\"launchctl\" AND bootstrap", 10, &unfiltered())?;

    assert_eq!(results.mode_used(), SearchMode::Fts);
    assert_eq!(results.hits().len(), 1);
    Ok(())
}

#[test]
fn open_existing_migrates_legacy_database_and_rebuilds_fts() -> Result<()> {
    let path = temp_db_path("legacy-migration");
    let parent = path.parent().expect("temporary path should have a parent");
    std::fs::create_dir_all(parent)?;

    let conn = rusqlite::Connection::open(&path)?;
    conn.execute_batch(
        r"
        CREATE TABLE snapshots (
            id INTEGER PRIMARY KEY,
            sha256 TEXT NOT NULL UNIQUE,
            snapshot_kind TEXT NOT NULL,
            preview_text TEXT NOT NULL,
            search_text TEXT NOT NULL,
            item_count INTEGER NOT NULL,
            total_bytes INTEGER NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE capture_events (
            id INTEGER PRIMARY KEY,
            snapshot_id INTEGER NOT NULL,
            observed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            change_count INTEGER NOT NULL,
            frontmost_app_bundle_id TEXT,
            frontmost_app_name TEXT
        );
        INSERT INTO snapshots (
            id, sha256, snapshot_kind, preview_text, search_text, item_count, total_bytes, created_at
        ) VALUES (
            1, 'legacy-sha', 'plain_text', 'git status', 'git status', 1, 10, '2026-04-16 10:00:00'
        );
        INSERT INTO capture_events (
            id, snapshot_id, observed_at, change_count, frontmost_app_bundle_id, frontmost_app_name
        ) VALUES (
            1, 1, '2026-04-16 10:00:00', 1, 'com.example.test', 'Test App'
        );
        PRAGMA user_version = 0;
    ",
    )?;
    drop(conn);

    let db = Database::open_existing(&path)?;
    let version: i64 = db
        .conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let results = db.search_auto("git", 10, &unfiltered())?;

    assert_eq!(version, CURRENT_SCHEMA_VERSION);
    let api_key_filter_enabled: i64 = db.conn.query_row(
        "SELECT api_key_filter_enabled FROM clipmem_settings WHERE id = 1",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(results.mode_used(), SearchMode::Fts);
    assert_eq!(results.hits().len(), 1);
    assert_eq!(api_key_filter_enabled, 0);

    std::fs::remove_file(&path)?;
    Ok(())
}

#[test]
fn migration_repairs_embedded_nul_snapshot_text_projection() -> Result<()> {
    let path = temp_db_path("embedded-nul-projection-migration");
    let parent = path.parent().expect("temporary path should have a parent");
    std::fs::create_dir_all(parent)?;

    let marker = "clipmem repaired projection";
    let bad_utf16_text = marker.chars().flat_map(|ch| [ch, '\0']).collect::<String>();
    let utf16_bytes = marker
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    let control_text = "\u{1}\0\0\0\0\0\u{10}\0";

    let conn = rusqlite::Connection::open(&path)?;
    configure_connection(&conn)?;
    conn.execute_batch(SCHEMA)?;
    conn.execute(
        "INSERT INTO snapshots (
            id, sha256, snapshot_kind, preview_text, search_text, item_count, total_bytes, created_at
        ) VALUES (1, 'bad-text-projection', 'plain_text', ?1, ?1, 1, 128, '2026-04-16 10:00:00')",
        [&bad_utf16_text],
    )?;
    conn.execute(
        "INSERT INTO snapshot_items (
            snapshot_id, item_index, primary_kind, primary_uti, preview_text, search_text, total_bytes
        ) VALUES (1, 0, 'plain_text', 'public.utf16-plain-text', ?1, ?1, 128)",
        [&bad_utf16_text],
    )?;
    conn.execute(
        "INSERT INTO item_representations (
            snapshot_id, item_index, uti, kind, byte_len, raw_sha256, text_value, blob_value
        ) VALUES (1, 0, 'public.utf16-plain-text', 'plain_text', ?1, 'utf16-sha', ?2, ?3)",
        params![utf16_bytes.len() as i64, bad_utf16_text, utf16_bytes],
    )?;
    conn.execute(
        "INSERT INTO item_representations (
            snapshot_id, item_index, uti, kind, byte_len, raw_sha256, text_value, blob_value
        ) VALUES (1, 0, 'public.utf8-plain-text', 'plain_text', ?1, 'utf8-sha', ?2, ?3)",
        params![marker.len() as i64, marker, marker.as_bytes()],
    )?;
    conn.execute(
        "INSERT INTO item_representations (
            snapshot_id, item_index, uti, kind, byte_len, raw_sha256, text_value, blob_value
        ) VALUES (1, 0, 'dyn.ah62d4rv4gk81g7d3ru', 'plain_text', ?1, 'dyn-sha', ?2, ?3)",
        params![
            control_text.len() as i64,
            control_text,
            control_text.as_bytes()
        ],
    )?;
    conn.execute(
        "INSERT INTO capture_events (
            id, snapshot_id, observed_at, change_count, frontmost_app_bundle_id, frontmost_app_name
        ) VALUES (1, 1, '2026-04-16 10:00:00', 1, 'com.example.test', 'Test App')",
        [],
    )?;
    conn.pragma_update(None, "user_version", 9)?;
    drop(conn);

    let db = Database::open_existing(&path)?;
    let version: i64 = db
        .conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let (preview_text, search_text): (String, String) = db.conn.query_row(
        "SELECT preview_text, search_text FROM snapshots WHERE id = 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let results = db.search_auto(marker, 10, &unfiltered())?;

    assert_eq!(version, CURRENT_SCHEMA_VERSION);
    assert_eq!(preview_text, marker);
    assert_eq!(search_text, marker);
    assert_eq!(results.hits().len(), 1);

    std::fs::remove_file(&path)?;
    Ok(())
}

#[test]
fn schema_version_11_adds_image_compression_metadata() -> Result<()> {
    let path = temp_db_path("image-compression-metadata-migration");
    let parent = path.parent().expect("temporary path should have a parent");
    std::fs::create_dir_all(parent)?;

    let conn = rusqlite::Connection::open(&path)?;
    conn.execute_batch(
        r"
        CREATE TABLE item_representations (
            snapshot_id INTEGER NOT NULL,
            item_index INTEGER NOT NULL,
            uti TEXT NOT NULL,
            kind TEXT NOT NULL,
            byte_len INTEGER NOT NULL,
            raw_sha256 TEXT NOT NULL,
            text_value TEXT,
            blob_value BLOB NOT NULL,
            PRIMARY KEY (snapshot_id, item_index, uti)
        );
        PRAGMA user_version = 10;
    ",
    )?;
    drop(conn);

    let db = Database::open_existing(&path)?;
    let version: i64 = db
        .conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let columns = db
        .conn
        .prepare("PRAGMA table_info(item_representations)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    assert_eq!(version, CURRENT_SCHEMA_VERSION);
    assert!(columns.contains(&"image_compression_status".to_string()));
    assert!(columns.contains(&"image_compression_format".to_string()));
    assert!(columns.contains(&"image_original_raw_sha256".to_string()));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn schema_version_13_adds_pending_restore_backstop() -> Result<()> {
    let path = temp_db_path("pending-restore-marker-migration");
    let parent = path.parent().expect("temporary path should have a parent");
    std::fs::create_dir_all(parent)?;

    let conn = rusqlite::Connection::open(&path)?;
    conn.execute_batch(
        r"
        CREATE TABLE item_representations (
            snapshot_id INTEGER NOT NULL,
            item_index INTEGER NOT NULL,
            uti TEXT NOT NULL,
            kind TEXT NOT NULL,
            byte_len INTEGER NOT NULL,
            raw_sha256 TEXT NOT NULL,
            text_value TEXT,
            blob_value BLOB NOT NULL,
            image_compression_status TEXT NOT NULL DEFAULT 'uncompressed',
            image_compression_format TEXT,
            image_compressed_at TEXT,
            image_original_byte_len INTEGER,
            image_original_raw_sha256 TEXT,
            image_compression_reason TEXT,
            PRIMARY KEY (snapshot_id, item_index, uti)
        );
        PRAGMA user_version = 12;
    ",
    )?;
    drop(conn);

    let db = Database::open_existing(&path)?;
    let version: i64 = db
        .conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let table_exists: bool = db.conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'pending_restores')",
        [],
        |row| row.get::<_, i64>(0).map(|value| value != 0),
    )?;
    let trigger_exists: bool = db.conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'trigger' AND name = 'capture_events_restore_suppression_bi')",
        [],
        |row| row.get::<_, i64>(0).map(|value| value != 0),
    )?;

    assert_eq!(version, CURRENT_SCHEMA_VERSION);
    assert!(table_exists);
    assert!(trigger_exists);

    cleanup_db(&path);
    Ok(())
}

#[test]
fn schema_version_14_adds_representation_cache_deferral_column() -> Result<()> {
    let path = temp_db_path("representation-cache-deferral-migration");
    let parent = path.parent().expect("temporary path should have a parent");
    std::fs::create_dir_all(parent)?;

    let conn = rusqlite::Connection::open(&path)?;
    conn.execute_batch(
        r"
        CREATE TABLE clipmem_settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            paused INTEGER NOT NULL DEFAULT 0 CHECK (paused IN (0, 1)),
            retention_seconds INTEGER CHECK (retention_seconds IS NULL OR retention_seconds >= 0),
            api_key_filter_enabled INTEGER NOT NULL DEFAULT 0 CHECK (api_key_filter_enabled IN (0, 1)),
            ocr_enabled INTEGER NOT NULL DEFAULT 0 CHECK (ocr_enabled IN (0, 1))
        );
        CREATE TABLE item_representations (
            snapshot_id INTEGER NOT NULL,
            item_index INTEGER NOT NULL,
            uti TEXT NOT NULL,
            kind TEXT NOT NULL,
            byte_len INTEGER NOT NULL,
            raw_sha256 TEXT NOT NULL,
            text_value TEXT,
            blob_value BLOB NOT NULL,
            image_compression_status TEXT NOT NULL DEFAULT 'uncompressed',
            image_compression_format TEXT,
            image_compressed_at TEXT,
            image_original_byte_len INTEGER,
            image_original_raw_sha256 TEXT,
            image_compression_reason TEXT,
            PRIMARY KEY (snapshot_id, item_index, uti)
        );
        CREATE TRIGGER item_representations_ai AFTER INSERT ON item_representations BEGIN
            SELECT 1;
        END;
        PRAGMA user_version = 13;
    ",
    )?;
    drop(conn);

    let db = Database::open_existing(&path)?;
    let version: i64 = db
        .conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let columns = db
        .conn
        .prepare("PRAGMA table_info(clipmem_settings)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let deferred: i64 = db.conn.query_row(
        "SELECT representation_cache_deferred FROM clipmem_settings WHERE id = 1",
        [],
        |row| row.get(0),
    )?;
    let trigger_sql: String = db.conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type = 'trigger' AND name = 'item_representations_ai'",
        [],
        |row| row.get(0),
    )?;

    assert_eq!(version, CURRENT_SCHEMA_VERSION);
    assert!(columns.contains(&"representation_cache_deferred".to_string()));
    assert_eq!(deferred, 0);
    assert!(trigger_sql.contains("representation_cache_deferred"));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn schema_version_18_adds_archive_revisions_table() -> Result<()> {
    let path = temp_db_path("archive-revisions-migration");
    let parent = path.parent().expect("temporary path should have a parent");
    std::fs::create_dir_all(parent)?;

    let conn = rusqlite::Connection::open(&path)?;
    conn.execute_batch(
        r"
        CREATE TABLE clipmem_settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            paused INTEGER NOT NULL DEFAULT 0 CHECK (paused IN (0, 1)),
            retention_seconds INTEGER CHECK (retention_seconds IS NULL OR retention_seconds >= 0),
            api_key_filter_enabled INTEGER NOT NULL DEFAULT 0 CHECK (api_key_filter_enabled IN (0, 1)),
            ocr_enabled INTEGER NOT NULL DEFAULT 0 CHECK (ocr_enabled IN (0, 1)),
            representation_cache_deferred INTEGER NOT NULL DEFAULT 0 CHECK (representation_cache_deferred IN (0, 1))
        );
        INSERT INTO clipmem_settings (id) VALUES (1);
        PRAGMA user_version = 17;
    ",
    )?;
    drop(conn);

    let db = Database::open_existing(&path)?;
    let version: i64 = db
        .conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let revision = db.archive_revision()?;

    assert_eq!(version, CURRENT_SCHEMA_VERSION);
    assert_eq!(revision.revision(), 0);
    assert_eq!(revision.last_change_kind(), "initialized");

    cleanup_db(&path);
    Ok(())
}

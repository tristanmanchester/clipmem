use super::*;

#[test]
fn settings_commands_persist_policy_and_support_json_views() -> Result<()> {
    let path = temp_db_path("settings-policy");

    let pause_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "settings",
        "pause",
        "on",
    ]);
    assert!(pause_output.status.success());

    let retention_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "settings",
        "retention",
        "30d",
    ]);
    assert!(retention_output.status.success());

    let api_key_filter_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "settings",
        "api-key-filter",
        "on",
    ]);
    assert!(api_key_filter_output.status.success());

    let ocr_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "settings",
        "ocr",
        "on",
    ]);
    assert!(ocr_output.status.success());

    let add_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "settings",
        "ignore",
        "add",
        "Com.Apple.Terminal",
    ]);
    assert!(add_output.status.success());

    let show_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "settings",
        "show",
        "--format",
        "json",
    ]);
    let show_payload: Value =
        serde_json::from_slice(&show_output.stdout).expect("settings show JSON should parse");

    assert!(show_output.status.success());
    assert_eq!(show_payload["paused"].as_bool(), Some(true));
    assert_eq!(show_payload["api_key_filter_enabled"].as_bool(), Some(true));
    assert_eq!(show_payload["ocr_enabled"].as_bool(), Some(true));
    assert_eq!(
        show_payload["retention_seconds"].as_u64(),
        Some(30 * 24 * 60 * 60)
    );
    assert_eq!(show_payload["retention"].as_str(), Some("30d"));
    assert_eq!(
        show_payload["ignored_bundle_ids"][0].as_str(),
        Some("com.apple.terminal")
    );

    let list_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "settings",
        "ignore",
        "list",
        "--format",
        "json",
    ]);
    let list_payload: Value =
        serde_json::from_slice(&list_output.stdout).expect("ignore list JSON should parse");

    assert!(list_output.status.success());
    assert_eq!(
        list_payload["ignored_bundle_ids"][0].as_str(),
        Some("com.apple.terminal")
    );

    let reset_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "settings",
        "reset",
        "--format",
        "json",
    ]);
    let reset_payload: Value =
        serde_json::from_slice(&reset_output.stdout).expect("settings reset JSON should parse");

    assert!(reset_output.status.success());
    assert_eq!(reset_payload["paused"].as_bool(), Some(false));
    assert_eq!(
        reset_payload["api_key_filter_enabled"].as_bool(),
        Some(false)
    );
    assert_eq!(reset_payload["ocr_enabled"].as_bool(), Some(false));
    assert!(reset_payload["retention_seconds"].is_null());
    assert_eq!(reset_payload["retention"].as_str(), Some("forever"));
    assert_eq!(
        reset_payload["ignored_bundle_ids"]
            .as_array()
            .unwrap()
            .len(),
        0
    );

    let db = Database::open_existing(&path)?;
    let revision = db.archive_revision()?;
    assert_eq!(revision.settings_revision(), 6);
    assert_eq!(revision.last_change_kind(), "settings");

    cleanup_db(&path);
    Ok(())
}

#[test]
fn forget_command_hard_deletes_snapshot() -> Result<()> {
    let path = temp_db_path("forget-snapshot");
    let ids = seed_database(&path, &[text_snapshot(1, "temporary clipboard text")])?;

    let forget_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "forget",
        &ids[0].to_string(),
    ]);
    let stdout = stdout_text(&forget_output);

    assert!(forget_output.status.success());
    assert!(stdout.contains(&format!("forgot snapshot={}", ids[0])));

    let db = Database::open_existing(&path)?;
    assert!(db.find_snapshot(ids[0], 10)?.is_none());
    assert!(db
        .search_auto("temporary clipboard", 10, &Default::default())?
        .hits()
        .is_empty());
    assert_eq!(db.archive_revision()?.archive_content_revision(), 2);

    cleanup_db(&path);
    Ok(())
}

#[test]
fn forget_json_reports_deleted_snapshot_counts() -> Result<()> {
    let path = temp_db_path("forget-json");
    let ids = seed_database(&path, &[text_snapshot(1, "temporary clipboard text")])?;

    let forget_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "forget",
        &ids[0].to_string(),
        "--format",
        "json",
    ]);
    let payload: Value =
        serde_json::from_slice(&forget_output.stdout).expect("forget JSON should parse");

    assert!(
        forget_output.status.success(),
        "{}",
        stderr_text(&forget_output)
    );
    assert_eq!(payload["snapshot_id"].as_i64(), Some(ids[0]));
    assert_eq!(payload["item_count"].as_u64(), Some(1));
    assert_eq!(payload["representation_count"].as_u64(), Some(1));
    assert_eq!(payload["capture_event_count"].as_u64(), Some(1));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn purge_command_reports_dry_run_then_deletes_old_snapshots() -> Result<()> {
    let path = temp_db_path("purge-snapshots");
    let events = seed_events(
        &path,
        &[
            text_snapshot(1, "expired snapshot"),
            text_snapshot(2, "fresh snapshot"),
        ],
    )?;
    set_event_observed_at(&path, events[0].1, "2000-01-01 00:00:00")?;

    let dry_run = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "purge",
        "--older-than",
        "30d",
        "--dry-run",
    ]);
    let dry_run_stdout = stdout_text(&dry_run);

    assert!(dry_run.status.success());
    assert!(dry_run_stdout.contains("purge dry-run older_than=30d snapshots=1"));

    let delete = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "purge",
        "--older-than",
        "30d",
    ]);
    let delete_stdout = stdout_text(&delete);

    assert!(delete.status.success());
    assert!(delete_stdout.contains("purged older_than=30d snapshots=1"));

    let db = Database::open_existing(&path)?;
    assert!(db.find_snapshot(events[0].0, 10)?.is_none());
    assert!(db.find_snapshot(events[1].0, 10)?.is_some());
    assert_eq!(db.archive_revision()?.archive_content_revision(), 3);

    cleanup_db(&path);
    Ok(())
}

#[test]
fn purge_json_reports_dry_run_counts() -> Result<()> {
    let path = temp_db_path("purge-json");
    let events = seed_events(
        &path,
        &[
            text_snapshot(1, "expired snapshot"),
            text_snapshot(2, "fresh snapshot"),
        ],
    )?;
    set_event_observed_at(&path, events[0].1, "2000-01-01 00:00:00")?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "purge",
        "--older-than",
        "30d",
        "--dry-run",
        "--format",
        "json",
    ]);
    let payload: Value = serde_json::from_slice(&output.stdout).expect("purge JSON should parse");

    assert!(output.status.success(), "{}", stderr_text(&output));
    assert_eq!(
        payload["older_than_seconds"].as_u64(),
        Some(30 * 24 * 60 * 60)
    );
    assert_eq!(payload["dry_run"].as_bool(), Some(true));
    assert_eq!(payload["snapshot_count"].as_u64(), Some(1));
    assert_eq!(payload["capture_event_count"].as_u64(), Some(1));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn storage_compact_json_reports_file_sizes() -> Result<()> {
    let path = temp_db_path("storage-compact-json");
    seed_database(&path, &[text_snapshot(1, "compact me")])?;
    let before = fs::metadata(&path)?.len();

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "storage",
        "compact",
        "--dry-run",
        "--format",
        "json",
    ]);
    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("storage compact JSON should parse");

    assert!(output.status.success(), "{}", stderr_text(&output));
    assert_eq!(payload["dry_run"].as_bool(), Some(true));
    assert_eq!(payload["completed"].as_bool(), Some(false));
    assert_eq!(payload["db_path"].as_str(), path.to_str());
    assert!(payload["before"]["db"].as_u64().unwrap_or_default() >= before);
    assert!(payload["after"]["db"].as_u64().unwrap_or_default() >= before);
    assert!(payload["page_count"].as_u64().unwrap_or_default() > 0);
    assert!(payload["estimated_reclaimable_bytes"].as_u64().is_some());
    assert_eq!(
        Database::open_existing(&path)?
            .archive_revision()?
            .storage_revision(),
        0
    );

    let compact = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "storage",
        "compact",
        "--format",
        "json",
    ]);
    let compact_payload: Value =
        serde_json::from_slice(&compact.stdout).expect("storage compact JSON should parse");

    assert!(compact.status.success(), "{}", stderr_text(&compact));
    assert_eq!(compact_payload["completed"].as_bool(), Some(true));
    assert_eq!(
        Database::open_existing(&path)?
            .archive_revision()?
            .storage_revision(),
        1
    );

    cleanup_db(&path);
    Ok(())
}

#[test]
fn storage_optimize_images_json_reports_dry_run_without_marking_rows() -> Result<()> {
    let path = temp_db_path("storage-optimize-images-json");
    seed_database(&path, &[image_snapshot(1, b"not actually a png")])?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "storage",
        "optimize-images",
        "--dry-run",
        "--format",
        "json",
    ]);
    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("image optimization JSON should parse");

    assert!(output.status.success(), "{}", stderr_text(&output));
    assert_eq!(payload["dry_run"].as_bool(), Some(true));
    assert_eq!(payload["format"].as_str(), Some("webp_lossless"));
    assert_eq!(payload["scanned_rows"].as_u64(), Some(1));
    assert_eq!(payload["skipped_rows"].as_u64(), Some(1));
    assert_eq!(payload["compact_run"].as_bool(), Some(false));
    assert!(payload["compact"].is_null());

    let conn = Connection::open(&path)?;
    let status: String = conn.query_row(
        "SELECT image_compression_status FROM item_representations",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(status, "uncompressed");

    cleanup_db(&path);
    Ok(())
}

#[test]
fn storage_image_candidates_lists_eligible_rows_without_mutation() -> Result<()> {
    let path = temp_db_path("storage-image-candidates");
    seed_database(&path, &[image_snapshot(1, b"not actually a png")])?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "storage",
        "image-candidates",
        "--limit",
        "1",
        "--format",
        "json",
    ]);
    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("image candidates JSON should parse");

    assert!(output.status.success(), "{}", stderr_text(&output));
    assert_eq!(payload.as_array().expect("array").len(), 1);
    assert_eq!(payload[0]["snapshot_id"].as_u64(), Some(1));
    assert_eq!(payload[0]["item_index"].as_u64(), Some(0));
    assert_eq!(payload[0]["uti"].as_str(), Some("public.png"));

    let status: String = Connection::open(&path)?.query_row(
        "SELECT image_compression_status FROM item_representations",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(status, "uncompressed");

    cleanup_db(&path);
    Ok(())
}

#[test]
fn storage_optimize_images_json_compacts_by_default() -> Result<()> {
    let path = temp_db_path("storage-optimize-images-compacts");
    let original = lossless_test_tiff()?;
    seed_database(&path, &[image_snapshot(1, &original)])?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "storage",
        "optimize-images",
        "--format",
        "json",
    ]);
    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("image optimization JSON should parse");

    assert!(output.status.success(), "{}", stderr_text(&output));
    assert_eq!(payload["compressed_rows"].as_u64(), Some(1));
    assert_eq!(payload["compact_run"].as_bool(), Some(true));
    assert!(payload["compact"].is_object());
    assert_eq!(payload["compact_recommended"].as_bool(), Some(false));
    assert!(payload["filesystem_growth_bytes"].as_u64().is_some());
    assert!(payload["filesystem_saved_bytes"].as_u64().is_some());

    cleanup_db(&path);
    Ok(())
}

#[test]
fn storage_optimize_images_no_compact_reports_recommendation() -> Result<()> {
    let path = temp_db_path("storage-optimize-images-no-compact");
    let original = lossless_test_tiff()?;
    seed_database(&path, &[image_snapshot(1, &original)])?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "storage",
        "optimize-images",
        "--no-compact",
        "--format",
        "json",
    ]);
    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("image optimization JSON should parse");

    assert!(output.status.success(), "{}", stderr_text(&output));
    assert_eq!(payload["compressed_rows"].as_u64(), Some(1));
    assert_eq!(payload["compact_run"].as_bool(), Some(false));
    assert!(payload["compact"].is_null());
    assert_eq!(payload["compact_recommended"].as_bool(), Some(true));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn storage_optimize_images_limit_processes_uncompressed_rows() -> Result<()> {
    let path = temp_db_path("storage-optimize-images-limit");
    seed_database(
        &path,
        &[
            image_snapshot(1, b"not actually a png"),
            image_snapshot(2, b"also not actually a png"),
        ],
    )?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "storage",
        "optimize-images",
        "--limit",
        "1",
        "--format",
        "json",
    ]);
    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("image optimization JSON should parse");

    assert!(output.status.success(), "{}", stderr_text(&output));
    assert_eq!(payload["scanned_rows"].as_u64(), Some(1));
    assert_eq!(payload["skipped_rows"].as_u64(), Some(1));

    let conn = Connection::open(&path)?;
    let skipped_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM item_representations WHERE image_compression_status = 'skipped'",
        [],
        |row| row.get(0),
    )?;
    let uncompressed_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM item_representations WHERE image_compression_status = 'uncompressed'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(skipped_count, 1);
    assert_eq!(uncompressed_count, 1);

    cleanup_db(&path);
    Ok(())
}

#[test]
fn storage_optimize_images_progress_jsonl_reports_scan_progress() -> Result<()> {
    let path = temp_db_path("storage-optimize-images-progress-jsonl");
    seed_database(
        &path,
        &[
            image_snapshot(1, b"not actually a png"),
            image_snapshot(2, b"also not actually a png"),
        ],
    )?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "storage",
        "optimize-images",
        "--dry-run",
        "--progress",
        "jsonl",
    ]);
    let stdout = stdout_text(&output);
    let events = stdout
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("progress line should parse"))
        .collect::<Vec<_>>();

    assert!(output.status.success(), "{}", stderr_text(&output));
    assert_eq!(events.len(), 4);
    assert_eq!(events[0]["type"].as_str(), Some("started"));
    assert_eq!(events[0]["total_rows"].as_u64(), Some(2));
    assert_eq!(events[1]["type"].as_str(), Some("scanning"));
    assert_eq!(events[1]["scanned_rows"].as_u64(), Some(1));
    assert_eq!(events[1]["total_rows"].as_u64(), Some(2));
    assert_eq!(events[1]["skipped_rows"].as_u64(), Some(1));
    assert_eq!(events[2]["type"].as_str(), Some("scanning"));
    assert_eq!(events[2]["scanned_rows"].as_u64(), Some(2));
    assert_eq!(events[2]["skipped_rows"].as_u64(), Some(2));
    assert_eq!(events[3]["type"].as_str(), Some("complete"));
    assert_eq!(events[3]["report"]["scanned_rows"].as_u64(), Some(2));
    assert_eq!(events[3]["report"]["skipped_rows"].as_u64(), Some(2));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn storage_optimize_images_progress_jsonl_handles_empty_candidates() -> Result<()> {
    let path = temp_db_path("storage-optimize-images-progress-empty");
    let _db = Database::open_or_init(&path)?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "storage",
        "optimize-images",
        "--progress",
        "jsonl",
    ]);
    let stdout = stdout_text(&output);
    let events = stdout
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("progress line should parse"))
        .collect::<Vec<_>>();

    assert!(output.status.success(), "{}", stderr_text(&output));
    assert_eq!(events.len(), 3);
    assert_eq!(events[0]["type"].as_str(), Some("started"));
    assert_eq!(events[0]["total_rows"].as_u64(), Some(0));
    assert_eq!(events[1]["type"].as_str(), Some("compacting"));
    assert_eq!(events[1]["scanned_rows"].as_u64(), Some(0));
    assert_eq!(events[1]["total_rows"].as_u64(), Some(0));
    assert_eq!(events[2]["type"].as_str(), Some("complete"));
    assert_eq!(events[2]["report"]["scanned_rows"].as_u64(), Some(0));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn storage_optimize_images_progress_jsonl_rejects_format_flags() {
    let output = run_cli(&[
        "storage",
        "optimize-images",
        "--progress",
        "jsonl",
        "--format",
        "json",
    ]);

    assert_eq!(status_code(&output), 2);
    assert!(stderr_text(&output).contains("`--progress jsonl` cannot be combined"));
    assert!(stdout_text(&output).is_empty());
}

#[test]
#[cfg(target_os = "macos")]
fn service_status_reports_capture_policy_in_text_and_json() -> Result<()> {
    let path = temp_db_path("service-status-policy");

    let pause_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "settings",
        "pause",
        "on",
    ]);
    assert!(pause_output.status.success());

    let retention_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "settings",
        "retention",
        "30d",
    ]);
    assert!(retention_output.status.success());

    let api_key_filter_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "settings",
        "api-key-filter",
        "on",
    ]);
    assert!(api_key_filter_output.status.success());

    let ignore_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "settings",
        "ignore",
        "add",
        "com.apple.Terminal",
    ]);
    assert!(ignore_output.status.success());

    let json_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "service",
        "status",
        "--json",
    ]);
    let json_payload: Value =
        serde_json::from_slice(&json_output.stdout).expect("service status JSON should parse");

    assert!(json_output.status.success());
    assert_eq!(json_payload["paused"].as_bool(), Some(true));
    assert_eq!(json_payload["api_key_filter_enabled"].as_bool(), Some(true));
    assert_eq!(
        json_payload["retention_seconds"].as_u64(),
        Some(30 * 24 * 60 * 60)
    );
    assert_eq!(json_payload["retention"].as_str(), Some("30d"));
    assert_eq!(json_payload["ignored_bundle_id_count"].as_u64(), Some(1));
    assert!(
        json_payload["db_size_bytes"].as_u64().unwrap_or_default() > 0,
        "service status should report the database file size"
    );

    let text_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "service",
        "status",
    ]);
    let text = stdout_text(&text_output);

    assert!(text_output.status.success());
    assert!(text.contains("paused: true"));
    assert!(text.contains("api key filter: true"));
    assert!(text.contains("retention: 30d"));
    assert!(text.contains("ignored bundle ids: 1"));
    assert!(text.contains("database size: "));

    cleanup_db(&path);
    Ok(())
}

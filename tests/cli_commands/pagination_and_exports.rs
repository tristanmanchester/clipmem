use super::*;

#[test]
fn status_action_and_settings_commands_support_human_output() -> Result<()> {
    let path = temp_db_path("status-action-settings-human");
    let _db = Database::open_or_init(&path)?;

    let settings = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "settings",
        "show",
        "--human",
    ]);
    let settings_stdout = stdout_text(&settings);
    assert!(settings.status.success());
    assert_human_output(&settings_stdout, "clipmem Settings");
    assert!(settings_stdout.contains("API key filter"));
    assert!(settings_stdout.contains("Retention"));

    let ocr = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "ocr",
        "status",
        "--human",
    ]);
    let ocr_stdout = stdout_text(&ocr);
    assert!(ocr.status.success());
    assert_human_output(&ocr_stdout, "clipmem OCR Status");
    assert!(ocr_stdout.contains("Pending"));
    assert!(ocr_stdout.contains("Snapshots w/ OCR"));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn human_flag_rejects_conflicting_output_flags() {
    let accepted = run_cli(&["search", "git", "--human", "--format", "human"]);
    assert_ne!(status_code(&accepted), 2);

    let json_conflict = run_cli(&["search", "git", "--human", "--json"]);
    assert_eq!(status_code(&json_conflict), 2);
    assert!(stderr_text(&json_conflict).contains("cannot be combined"));

    let format_conflict = run_cli(&["search", "git", "--human", "--format", "json"]);
    assert_eq!(status_code(&format_conflict), 2);
    assert!(stderr_text(&format_conflict).contains("`--human` is only compatible"));

    let recall_conflict = run_cli(&["recall", "git", "--human", "--format", "json"]);
    assert_eq!(status_code(&recall_conflict), 2);
    assert!(stderr_text(&recall_conflict).contains("`--human` is only compatible"));
}

#[test]
fn list_commands_include_truncation_metadata_and_resume_with_cursor() -> Result<()> {
    let path = temp_db_path("search-pagination");
    seed_database(
        &path,
        &[
            text_snapshot(1, "git one"),
            text_snapshot(2, "git two"),
            text_snapshot(3, "git three"),
        ],
    )?;

    let first_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "search",
        "--mode",
        "literal",
        "--limit",
        "2",
        "--format",
        "json",
        "git",
    ]);
    let first_payload: Value =
        serde_json::from_slice(&first_output.stdout).expect("first page should parse");
    let next_cursor = first_payload["next_cursor"]
        .as_str()
        .expect("first page should include a cursor")
        .to_string();
    let first_ids = first_payload["results"]
        .as_array()
        .expect("results should be an array")
        .iter()
        .map(|row| row["snapshot_id"].as_i64().unwrap())
        .collect::<Vec<_>>();

    assert!(first_output.status.success());
    assert_eq!(first_payload["truncated"].as_bool(), Some(true));
    assert_eq!(first_payload["results"].as_array().map(Vec::len), Some(2));

    let second_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "search",
        "--mode",
        "literal",
        "--limit",
        "2",
        "--cursor",
        &next_cursor,
        "--format",
        "json",
        "git",
    ]);
    let second_payload: Value =
        serde_json::from_slice(&second_output.stdout).expect("second page should parse");
    let second_ids = second_payload["results"]
        .as_array()
        .expect("results should be an array")
        .iter()
        .map(|row| row["snapshot_id"].as_i64().unwrap())
        .collect::<Vec<_>>();

    assert!(second_output.status.success());
    assert_eq!(second_payload["truncated"].as_bool(), Some(false));
    assert!(second_payload["next_cursor"].is_null());
    assert_eq!(second_ids.len(), 1);
    assert!(!first_ids.contains(&second_ids[0]));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn search_rejects_cursor_filter_mismatches() -> Result<()> {
    let path = temp_db_path("search-cursor-mismatch");
    seed_database(
        &path,
        &[
            text_snapshot(1, "git status"),
            text_snapshot(2, "git commit"),
        ],
    )?;

    let first_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "search",
        "--mode",
        "literal",
        "--limit",
        "1",
        "--format",
        "json",
        "git",
    ]);
    let first_payload: Value =
        serde_json::from_slice(&first_output.stdout).expect("first page should parse");
    let next_cursor = first_payload["next_cursor"]
        .as_str()
        .expect("first page should include a cursor")
        .to_string();

    let mismatched = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "search",
        "--mode",
        "literal",
        "--limit",
        "1",
        "--cursor",
        &next_cursor,
        "--format",
        "json",
        "cargo",
    ]);
    let stderr = stderr_text(&mismatched);

    assert!(!mismatched.status.success());
    assert!(stderr.contains("cursor does not match the active search query or mode"));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn timeline_rejects_cursor_filter_mismatches() -> Result<()> {
    let path = temp_db_path("timeline-cursor-mismatch");
    let events = seed_events(
        &path,
        &[text_snapshot(1, "alpha"), text_snapshot(2, "beta")],
    )?;
    set_event_observed_at(&path, events[0].1, "2026-04-16 09:00:00")?;
    set_event_observed_at(&path, events[1].1, "2026-04-16 10:00:00")?;

    let first_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "timeline",
        "--sort",
        "asc",
        "--limit",
        "1",
        "--format",
        "json",
    ]);
    let first_payload: Value =
        serde_json::from_slice(&first_output.stdout).expect("timeline page should parse");
    let next_cursor = first_payload["next_cursor"]
        .as_str()
        .expect("timeline page should include a cursor")
        .to_string();

    let mismatched = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "timeline",
        "--sort",
        "desc",
        "--limit",
        "1",
        "--cursor",
        &next_cursor,
        "--format",
        "json",
    ]);
    let stderr = stderr_text(&mismatched);

    assert!(!mismatched.status.success());
    assert!(stderr.contains("cursor does not match the active timeline filters or sort"));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn search_help_mentions_new_format_and_cursor_flags() {
    let output = run_cli(&["help", "search"]);
    let help_text = format!("{}{}", stdout_text(&output), stderr_text(&output));

    assert!(help_text.contains("--format <FORMAT>"));
    assert!(help_text.contains("--json"));
    assert!(help_text.contains("--cursor <CURSOR>"));
}

#[test]
fn recent_and_timeline_help_make_the_distinction_clear() {
    let recent = run_cli(&["help", "recent"]);
    let recent_help = format!("{}{}", stdout_text(&recent), stderr_text(&recent));
    let timeline = run_cli(&["help", "timeline"]);
    let timeline_help = format!("{}{}", stdout_text(&timeline), stderr_text(&timeline));

    assert!(recent_help.contains("recent unique clipboard states"));
    assert!(recent_help.contains("deduplicated by snapshot"));
    assert!(timeline_help.contains("chronological clipboard capture events"));
    assert!(timeline_help.contains("one row per observation"));
    assert!(timeline_help.contains("--since <SINCE>"));
    assert!(timeline_help.contains("--sort <SORT>"));
}

#[test]
fn json_alias_rejects_non_json_format() {
    let output = run_cli(&["search", "git", "--json", "--format", "md"]);
    let stderr = stderr_text(&output);

    assert!(!output.status.success());
    assert!(stderr.contains("`--json` is only compatible with `--format json`"));
}

#[test]
fn action_json_alias_rejects_non_json_format() {
    let cases = [
        vec!["restore", "1", "--json", "--format", "md"],
        vec!["forget", "1", "--json", "--format", "toon"],
        vec![
            "purge",
            "--older-than",
            "30d",
            "--json",
            "--format",
            "jsonl",
        ],
        vec![
            "export",
            "1",
            "--item",
            "0",
            "--uti",
            "public.utf8-plain-text",
            "--out",
            "/tmp/clipmem-export.txt",
            "--json",
            "--format",
            "md",
        ],
    ];

    for args in cases {
        let output = run_cli(&args);
        let stderr = stderr_text(&output);

        assert_eq!(status_code(&output), 2, "args: {args:?}");
        assert!(
            stderr.contains("`--json` is only compatible with `--format json`"),
            "stderr for {args:?}: {stderr}"
        );
    }
}

#[test]
fn doctor_command_reports_database_capabilities() {
    let path = temp_db_path("doctor-text");
    let db = Database::open_or_init(&path).expect("test database should open");
    drop(db);
    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "doctor",
    ]);
    let stdout = stdout_text(&output);

    assert!(output.status.success());
    assert!(stdout.contains("database:"));
    assert!(stdout.contains("sqlite version:"));
    assert!(stdout.contains("fts5 temp table creation works:"));

    cleanup_db(&path);
}

#[test]
fn recent_command_rejects_zero_limit() {
    let path = temp_db_path("recent-zero-limit");
    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "recent",
        "--limit",
        "0",
    ]);
    let stderr = stderr_text(&output);

    assert!(!output.status.success());
    assert!(stderr.contains('0'));

    cleanup_db(&path);
}

#[test]
fn ocr_commands_report_status_and_empty_backfill_runs() -> Result<()> {
    let path = temp_db_path("ocr-status");
    seed_database(&path, &[text_snapshot(1, "git status")])?;

    let status_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "ocr",
        "status",
        "--format",
        "json",
    ]);
    let status_payload: Value =
        serde_json::from_slice(&status_output.stdout).expect("ocr status JSON should parse");

    assert!(
        status_output.status.success(),
        "{}",
        stderr_text(&status_output)
    );
    assert_eq!(status_payload["pending"].as_u64(), Some(0));
    assert_eq!(status_payload["ready"].as_u64(), Some(0));
    assert_eq!(status_payload["failed"].as_u64(), Some(0));
    assert_eq!(status_payload["snapshots_with_ocr_text"].as_u64(), Some(0));

    let run_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "ocr",
        "run",
        "--limit",
        "1",
        "--format",
        "json",
    ]);
    let run_payload: Value =
        serde_json::from_slice(&run_output.stdout).expect("ocr run JSON should parse");

    assert!(run_output.status.success(), "{}", stderr_text(&run_output));
    assert_eq!(run_payload["processed"].as_u64(), Some(0));
    assert_eq!(run_payload["remaining_pending"].as_u64(), Some(0));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn ocr_candidates_lists_pending_hashes_without_processing() -> Result<()> {
    let path = temp_db_path("ocr-candidates");
    seed_database(&path, &[image_snapshot(1, b"not actually a png")])?;
    let conn = Connection::open(&path)?;
    let raw_sha: String = conn.query_row(
        "SELECT raw_sha256 FROM item_representations WHERE kind = 'image'",
        [],
        |row| row.get(0),
    )?;
    conn.execute(
        "INSERT INTO ocr_results (raw_sha256, status) VALUES (?1, 'pending')",
        [&raw_sha],
    )?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "ocr",
        "candidates",
        "--limit",
        "1",
        "--format",
        "json",
    ]);
    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("ocr candidates JSON should parse");

    assert!(output.status.success(), "{}", stderr_text(&output));
    assert_eq!(payload.as_array().expect("array").len(), 1);
    assert_eq!(payload[0]["raw_sha256"].as_str(), Some(raw_sha.as_str()));
    assert_eq!(payload[0]["snapshot_count"].as_u64(), Some(1));

    let get_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "ocr",
        "get",
        raw_sha.as_str(),
        "--format",
        "json",
    ]);
    let get_payload: Value =
        serde_json::from_slice(&get_output.stdout).expect("ocr get JSON should parse");

    assert!(get_output.status.success(), "{}", stderr_text(&get_output));
    assert_eq!(get_payload["raw_sha256"].as_str(), Some(raw_sha.as_str()));
    assert_eq!(get_payload["status"].as_str(), Some("pending"));

    let clear_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "ocr",
        "clear",
        raw_sha.as_str(),
        "--format",
        "json",
    ]);
    let clear_payload: Value =
        serde_json::from_slice(&clear_output.stdout).expect("ocr clear JSON should parse");

    assert!(
        clear_output.status.success(),
        "{}",
        stderr_text(&clear_output)
    );
    assert_eq!(clear_payload["pending"].as_u64(), Some(0));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn ocr_read_commands_do_not_initialize_missing_database() {
    let path = temp_db_path("ocr-missing-read");

    let candidates = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "ocr",
        "candidates",
        "--format",
        "json",
    ]);
    let get = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "ocr",
        "get",
        "abc123",
        "--format",
        "json",
    ]);
    let clear = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "ocr",
        "clear",
        "abc123",
        "--format",
        "json",
    ]);

    assert!(!candidates.status.success());
    assert!(!get.status.success());
    assert!(!clear.status.success());
    assert!(stderr_text(&candidates).contains("Run `clipmem setup`"));
    assert!(stderr_text(&get).contains("Run `clipmem setup`"));
    assert!(stderr_text(&clear).contains("Run `clipmem setup`"));
    assert!(!path.exists());
}

#[test]
fn export_command_writes_raw_representation_bytes() -> Result<()> {
    let path = temp_db_path("export-bytes");
    let output_path = temp_artifact_path("export-bytes", ".bin");
    let snapshot = build_snapshot(
        CaptureContext::new(1)
            .with_frontmost_app_name("Preview")
            .with_frontmost_app_bundle_id("com.apple.Preview"),
        vec![build_item(
            0,
            vec![build_representation(
                "public.png".to_string(),
                None,
                vec![0x89, b'P', b'N', b'G'],
            )],
        )],
    );
    let ids = seed_database(&path, &[snapshot])?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "export",
        &ids[0].to_string(),
        "--item",
        "0",
        "--uti",
        "public.png",
        "--out",
        output_path.to_str().expect("output path should be UTF-8"),
    ]);
    let stdout = stdout_text(&output);
    let bytes = fs::read(&output_path)?;

    assert!(output.status.success());
    assert_eq!(bytes, vec![0x89, b'P', b'N', b'G']);
    assert!(stdout.contains("snapshot="));
    assert!(stdout.contains("uti=public.png"));

    cleanup_temp_artifact(&output_path);
    cleanup_db(&path);
    Ok(())
}

#[test]
fn export_command_creates_parent_directories_for_nested_output() -> Result<()> {
    let path = temp_db_path("export-nested");
    let artifact_root = temp_artifact_path("export-nested", "");
    let output_path = artifact_root.join("nested").join("out.txt");
    let snapshot = text_snapshot(1, "nested export");
    let ids = seed_database(&path, &[snapshot])?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "export",
        &ids[0].to_string(),
        "--item",
        "0",
        "--uti",
        "public.utf8-plain-text",
        "--out",
        output_path.to_str().expect("output path should be UTF-8"),
    ]);

    assert!(output.status.success(), "{}", stderr_text(&output));
    assert_eq!(fs::read(&output_path)?, b"nested export");

    cleanup_temp_artifact(&artifact_root);
    cleanup_db(&path);
    Ok(())
}

#[test]
fn export_command_rejects_existing_file_without_force() -> Result<()> {
    let path = temp_db_path("export-existing");
    let output_path = temp_artifact_path("export-existing", ".txt");
    let snapshot = text_snapshot(1, "updated");
    let ids = seed_database(&path, &[snapshot])?;
    fs::create_dir_all(
        output_path
            .parent()
            .expect("temp artifact path should have a parent"),
    )?;
    fs::write(&output_path, b"original")?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "export",
        &ids[0].to_string(),
        "--item",
        "0",
        "--uti",
        "public.utf8-plain-text",
        "--out",
        output_path.to_str().expect("output path should be UTF-8"),
    ]);
    let stderr = stderr_text(&output);
    let bytes = fs::read(&output_path)?;

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr.contains("already exists"));
    assert!(stderr.contains("--force"));
    assert_eq!(bytes, b"original");

    cleanup_temp_artifact(&output_path);
    cleanup_db(&path);
    Ok(())
}

#[test]
fn export_command_overwrites_existing_file_with_force() -> Result<()> {
    let path = temp_db_path("export-force");
    let output_path = temp_artifact_path("export-force", ".txt");
    let snapshot = text_snapshot(1, "updated");
    let ids = seed_database(&path, &[snapshot])?;
    fs::create_dir_all(
        output_path
            .parent()
            .expect("temp artifact path should have a parent"),
    )?;
    fs::write(&output_path, b"original")?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "export",
        &ids[0].to_string(),
        "--item",
        "0",
        "--uti",
        "public.utf8-plain-text",
        "--out",
        output_path.to_str().expect("output path should be UTF-8"),
        "--force",
    ]);
    let stdout = stdout_text(&output);
    let bytes = fs::read(&output_path)?;

    assert!(output.status.success());
    assert_eq!(bytes, b"updated");
    assert!(stdout.contains("snapshot="));

    cleanup_temp_artifact(&output_path);
    cleanup_db(&path);
    Ok(())
}

#[test]
fn export_command_rejects_directory_destination_without_partial_file() -> Result<()> {
    let path = temp_db_path("export-directory");
    let output_dir = temp_artifact_path("export-directory", "");
    let snapshot = text_snapshot(1, "updated");
    let ids = seed_database(&path, &[snapshot])?;
    fs::create_dir_all(&output_dir)?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "export",
        &ids[0].to_string(),
        "--item",
        "0",
        "--uti",
        "public.utf8-plain-text",
        "--out",
        output_dir.to_str().expect("output path should be UTF-8"),
    ]);
    let stderr = stderr_text(&output);

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr.contains("not a regular file"));
    assert!(fs::read_dir(&output_dir)?.next().is_none());

    cleanup_temp_artifact(&output_dir);
    cleanup_db(&path);
    Ok(())
}

#[test]
#[cfg(unix)]
fn export_command_rejects_symlink_destination_even_with_force() -> Result<()> {
    let path = temp_db_path("export-symlink");
    let link_path = temp_artifact_path("export-symlink", ".txt");
    let victim_path = temp_artifact_path("export-symlink-victim", ".txt");
    let snapshot = text_snapshot(1, "updated");
    let ids = seed_database(&path, &[snapshot])?;
    fs::create_dir_all(
        link_path
            .parent()
            .expect("temp artifact path should have a parent"),
    )?;
    fs::write(&victim_path, b"victim")?;
    std::os::unix::fs::symlink(&victim_path, &link_path)?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "export",
        &ids[0].to_string(),
        "--item",
        "0",
        "--uti",
        "public.utf8-plain-text",
        "--out",
        link_path.to_str().expect("output path should be UTF-8"),
    ]);
    let stderr = stderr_text(&output);

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr.contains("symbolic link"));
    assert_eq!(fs::read(&victim_path)?, b"victim");

    let forced_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "export",
        &ids[0].to_string(),
        "--item",
        "0",
        "--uti",
        "public.utf8-plain-text",
        "--out",
        link_path.to_str().expect("output path should be UTF-8"),
        "--force",
    ]);
    let forced_stderr = stderr_text(&forced_output);

    assert_eq!(forced_output.status.code(), Some(2));
    assert!(forced_stderr.contains("symbolic link"));
    assert_eq!(fs::read(&victim_path)?, b"victim");

    cleanup_temp_artifact(&link_path);
    cleanup_temp_artifact(&victim_path);
    cleanup_db(&path);
    Ok(())
}

#[test]
fn export_command_fails_for_unknown_representation() -> Result<()> {
    let path = temp_db_path("export-missing");
    let snapshot = text_snapshot(1, "git status");
    let ids = seed_database(&path, &[snapshot])?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "export",
        &ids[0].to_string(),
        "--item",
        "0",
        "--uti",
        "public.png",
        "--out",
        "/tmp/clipmem-missing.bin",
    ]);
    let stderr = stderr_text(&output);

    assert!(!output.status.success());
    assert!(stderr.contains("representation not found"));

    cleanup_db(&path);
    Ok(())
}

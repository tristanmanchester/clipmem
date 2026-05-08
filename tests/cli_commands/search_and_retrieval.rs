use super::*;

#[test]
fn invalid_args_exit_with_code_2_and_write_only_stderr() {
    let output = run_cli(&["recent", "--limit", "0"]);
    assert_eq!(status_code(&output), 2);
    assert!(stdout_text(&output).is_empty());
    assert!(stderr_text(&output).contains("between 1 and 250"));
}

#[test]
fn legacy_prerelease_database_schema_points_users_at_setup() -> Result<()> {
    let path = temp_db_path("legacy-prerelease-schema");
    let parent = path.parent().expect("temp db path should have a parent");
    fs::create_dir_all(parent)?;

    let conn = Connection::open(&path)?;
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
        CREATE TABLE snapshot_items (
            snapshot_id INTEGER NOT NULL,
            item_index INTEGER NOT NULL,
            preview_text TEXT NOT NULL,
            PRIMARY KEY (snapshot_id, item_index)
        );
        CREATE TABLE item_representations (
            snapshot_id INTEGER NOT NULL,
            item_index INTEGER NOT NULL,
            uti TEXT NOT NULL,
            classification TEXT NOT NULL,
            is_text INTEGER NOT NULL CHECK (is_text IN (0, 1)),
            byte_len INTEGER NOT NULL,
            raw_sha256 TEXT NOT NULL,
            text_value TEXT,
            blob_value BLOB NOT NULL,
            PRIMARY KEY (snapshot_id, item_index, uti)
        );
        CREATE TABLE capture_events (
            id INTEGER PRIMARY KEY,
            snapshot_id INTEGER NOT NULL,
            observed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            change_count INTEGER NOT NULL,
            frontmost_app_bundle_id TEXT,
            frontmost_app_name TEXT
        );
        PRAGMA user_version = 1;
    ",
    )?;
    drop(conn);

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be utf-8"),
        "recent",
    ]);
    assert!(!output.status.success());
    let stderr = stderr_text(&output);
    assert!(stderr.contains("database operation failed"));
    assert!(stderr.contains("clipmem setup"));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn not_found_exit_with_code_3_and_write_only_stderr() -> Result<()> {
    let path = temp_db_path("get-not-found-exit-code");
    let _db = Database::open_or_init(&path)?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "get",
        "42",
    ]);

    assert_eq!(status_code(&output), 3);
    assert!(stdout_text(&output).is_empty());
    let stderr = stderr_text(&output);
    assert!(
        stderr.contains("snapshot 42 was not found")
            || stderr.contains("get failed for snapshot 42"),
        "unexpected stderr: {stderr}"
    );

    cleanup_db(&path);
    Ok(())
}

#[test]
fn unsupported_format_exit_with_code_4_and_write_only_stderr() -> Result<()> {
    let path = temp_db_path("get-unsupported-format");
    let ids = seed_database(&path, &[text_snapshot(1, "git status")])?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "get",
        &ids[0].to_string(),
        "--format",
        "toon",
    ]);

    assert_eq!(status_code(&output), 4);
    assert!(stdout_text(&output).is_empty());
    assert!(
        stderr_text(&output).contains("format toon is only supported for flattened list outputs")
    );

    cleanup_db(&path);
    Ok(())
}

#[test]
fn get_rejects_unsupported_format_before_opening_database() -> Result<()> {
    let path = temp_db_path("get-unsupported-format-missing-db");
    cleanup_db(&path);

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "get",
        "1",
        "--format",
        "toon",
    ]);

    assert_eq!(status_code(&output), 4);
    assert!(stdout_text(&output).is_empty());
    assert!(
        stderr_text(&output).contains("format toon is only supported for flattened list outputs")
    );

    cleanup_db(&path);
    Ok(())
}

#[test]
fn db_error_exit_with_code_5_and_write_only_stderr() -> Result<()> {
    let dir = temp_test_dir("db-error-dir");
    fs::create_dir_all(&dir)?;

    let output = run_cli(&[
        "--db",
        dir.to_str().expect("dir path should be UTF-8"),
        "recent",
    ]);

    assert_eq!(status_code(&output), 5);
    assert!(stdout_text(&output).is_empty());
    assert!(stderr_text(&output).contains("database does not exist"));

    let _ = fs::remove_dir_all(&dir);
    Ok(())
}

#[test]
fn search_command_prints_literal_results_in_text_mode() -> Result<()> {
    let path = temp_db_path("search-text");
    seed_database(
        &path,
        &[
            text_snapshot(1, "Discount: 50 percent off"),
            text_snapshot(2, "Discount: 50%"),
        ],
    )?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "search",
        "--mode",
        "literal",
        "50%",
    ]);
    let stdout = stdout_text(&output);

    assert!(output.status.success());
    assert!(stdout.contains("preview: Discount: 50%"));
    assert!(!stdout.contains("Discount: 50 percent off"));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn search_json_envelope_includes_agent_friendly_rows() -> Result<()> {
    let path = temp_db_path("search-json");
    let ids = seed_database(
        &path,
        &[rich_snapshot(
            1,
            "git status",
            "https://example.com/repo",
            "file:///Users/test/report.txt",
        )],
    )?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "search",
        "--mode",
        "literal",
        "--format",
        "json",
        "git",
    ]);
    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("search JSON output should parse");

    assert!(output.status.success());
    assert_eq!(payload["schema_version"].as_u64(), Some(2));
    assert_eq!(payload["command"].as_str(), Some("search"));
    assert_eq!(
        payload["applied_filters"]["requested_mode"].as_str(),
        Some("literal")
    );
    assert_eq!(
        payload["applied_filters"]["mode_used"].as_str(),
        Some("literal")
    );
    assert_eq!(payload["truncated"].as_bool(), Some(false));
    assert_eq!(payload["results"][0]["snapshot_id"].as_i64(), Some(ids[0]));
    assert_eq!(payload["results"][0]["event_id"].as_i64(), Some(1));
    assert!(payload["results"][0]["best_text"]
        .as_str()
        .unwrap_or_default()
        .contains("git status"));
    assert!(payload["results"][0]["ocr_text"].is_null());
    assert!(payload["results"][0]["ocr_status"].is_null());
    assert_eq!(
        payload["results"][0]["best_text_uti"].as_str(),
        Some("public.utf8-plain-text")
    );
    let fragments = payload["results"][0]["text_fragments"]
        .as_array()
        .expect("text_fragments should be an array");
    assert!(fragments.iter().any(|fragment| {
        fragment["text"].as_str() == Some("git status")
            && fragment["uti"].as_str() == Some("public.utf8-plain-text")
    }));
    assert_eq!(
        payload["results"][0]["why_matched"].as_str(),
        Some("Prefix match in best text")
    );
    assert!(payload["results"][0]["matched_fields"]
        .as_array()
        .expect("matched_fields should be an array")
        .iter()
        .any(|field| field.as_str() == Some("best_text")));
    assert!(payload["results"][0]["matched_fields"]
        .as_array()
        .expect("matched_fields should be an array")
        .iter()
        .any(|field| field.as_str() == Some("search_text")));
    assert_eq!(
        payload["results"][0]["urls"][0].as_str(),
        Some("https://example.com/repo")
    );
    assert_eq!(
        payload["results"][0]["file_paths"][0].as_str(),
        Some("/Users/test/report.txt")
    );
    assert!(payload["results"][0]["text_summary"]
        .as_str()
        .unwrap_or_default()
        .contains("git status"));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn search_handles_url_bundle_id_path_and_shell_queries_with_explanations() -> Result<()> {
    let path = temp_db_path("search-robust-queries");
    seed_database(
        &path,
        &[
            app_text_snapshot(
                1,
                "Terminal",
                "com.apple.Terminal",
                "launchctl bootstrap gui/501 ~/Library/LaunchAgents/io.example.agent.plist",
            ),
            rich_snapshot(
                2,
                "repository link",
                "https://example.com/repo",
                "file:///Users/test/path/with/slashes",
            ),
            text_snapshot(3, "foo:bar"),
            text_snapshot(4, "git commit -m"),
        ],
    )?;

    let cases = [
        ("https://example.com/repo", "Exact URL match", "urls"),
        ("com.apple.Terminal", "Bundle ID match", "app_bundle_id"),
        (
            "~/path/with/slashes",
            "Path fragment match in file paths",
            "file_paths",
        ),
        ("foo:bar", "Exact text match in best text", "best_text"),
        (
            "git commit -m",
            "Exact text match in best text",
            "best_text",
        ),
    ];

    for (query, why, field) in cases {
        let output = run_cli(&[
            "--db",
            path.to_str().expect("db path should be UTF-8"),
            "search",
            "--format",
            "json",
            query,
        ]);
        assert!(output.status.success(), "search should succeed for {query}");
        let payload: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(
            payload["applied_filters"]["mode_used"].as_str(),
            Some("literal")
        );
        assert_eq!(payload["results"][0]["why_matched"].as_str(), Some(why));
        assert!(payload["results"][0]["matched_fields"]
            .as_array()
            .expect("matched_fields should be an array")
            .iter()
            .any(|value| value.as_str() == Some(field)));
    }

    cleanup_db(&path);
    Ok(())
}

#[test]
fn literal_search_includes_app_name_matches_when_text_fast_path_hits() -> Result<()> {
    let path = temp_db_path("search-app-name-with-text-hit");
    let ids = seed_database(
        &path,
        &[
            app_text_snapshot(
                1,
                "Visual Studio Code",
                "com.microsoft.VSCode",
                "release notes",
            ),
            text_snapshot(2, "Visual Studio Code settings"),
        ],
    )?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "search",
        "--mode",
        "literal",
        "--format",
        "json",
        "Visual Studio Code",
    ]);
    let payload: Value = serde_json::from_slice(&output.stdout)?;
    let results = payload["results"]
        .as_array()
        .expect("results should be an array");

    assert!(output.status.success(), "{}", stderr_text(&output));
    assert!(results
        .iter()
        .any(|row| row["snapshot_id"].as_i64() == Some(ids[1])));
    let app_name_row = results
        .iter()
        .find(|row| row["snapshot_id"].as_i64() == Some(ids[0]))
        .expect("literal search should include app-name-only matches");
    assert_eq!(app_name_row["why_matched"].as_str(), Some("App name match"));
    assert!(app_name_row["matched_fields"]
        .as_array()
        .expect("matched_fields should be an array")
        .iter()
        .any(|value| value.as_str() == Some("app_name")));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn search_exact_phrase_query_prefers_exact_phrase_hits() -> Result<()> {
    let path = temp_db_path("search-exact-phrase");
    let ids = seed_database(
        &path,
        &[
            text_snapshot(1, "status git"),
            text_snapshot(2, "git status"),
        ],
    )?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "search",
        "--format",
        "json",
        "\"git status\"",
    ]);
    assert!(output.status.success());
    let payload: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        payload["applied_filters"]["mode_used"].as_str(),
        Some("fts")
    );
    assert_eq!(payload["results"][0]["snapshot_id"].as_i64(), Some(ids[1]));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn recent_json_alias_uses_new_envelope_shape() -> Result<()> {
    let path = temp_db_path("recent-json");
    let ids = seed_database(&path, &[text_snapshot(1, "git status")])?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "recent",
        "--json",
    ]);
    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("recent JSON output should parse");

    assert!(output.status.success());
    assert_eq!(payload["command"].as_str(), Some("recent"));
    assert_eq!(payload["results"].as_array().map(Vec::len), Some(1));
    assert_eq!(payload["results"][0]["snapshot_id"].as_i64(), Some(ids[0]));
    assert_eq!(
        payload["results"][0]["best_text"].as_str(),
        Some("git status")
    );
    assert_eq!(
        payload["results"][0]["best_text_uti"].as_str(),
        Some("public.utf8-plain-text")
    );
    assert!(payload["results"][0]["why_matched"].is_null());

    cleanup_db(&path);
    Ok(())
}

#[test]
fn timeline_json_envelope_returns_event_rows_in_descending_order() -> Result<()> {
    let path = temp_db_path("timeline-json");
    let events = seed_events(
        &path,
        &[
            text_snapshot(1, "git status"),
            text_snapshot(2, "git status"),
            rich_snapshot(
                3,
                "cargo test",
                "https://example.com/repo",
                "file:///Users/test/report.txt",
            ),
        ],
    )?;
    set_event_observed_at(&path, events[0].1, "2026-04-16 09:00:00")?;
    set_event_observed_at(&path, events[1].1, "2026-04-16 10:00:00")?;
    set_event_observed_at(&path, events[2].1, "2026-04-16 11:00:00")?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "timeline",
        "--format",
        "json",
        "--limit",
        "3",
    ]);
    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("timeline JSON output should parse");

    assert!(output.status.success());
    assert_eq!(payload["command"].as_str(), Some("timeline"));
    assert_eq!(payload["applied_filters"]["sort"].as_str(), Some("desc"));
    assert_eq!(payload["results"].as_array().map(Vec::len), Some(3));
    assert_eq!(
        payload["results"][0]["event_id"].as_i64(),
        Some(events[2].1)
    );
    assert_eq!(
        payload["results"][1]["event_id"].as_i64(),
        Some(events[1].1)
    );
    assert_eq!(
        payload["results"][2]["event_id"].as_i64(),
        Some(events[0].1)
    );
    assert_eq!(payload["results"][0]["change_count"].as_i64(), Some(3));
    assert_eq!(
        payload["results"][0]["best_text"].as_str(),
        Some("cargo test")
    );
    assert_eq!(
        payload["results"][0]["best_text_uti"].as_str(),
        Some("public.utf8-plain-text")
    );
    assert_eq!(
        payload["results"][0]["urls"][0].as_str(),
        Some("https://example.com/repo")
    );
    assert_eq!(
        payload["results"][0]["file_paths"][0].as_str(),
        Some("/Users/test/report.txt")
    );

    cleanup_db(&path);
    Ok(())
}

#[test]
fn timeline_paginates_with_cursor_and_sort_asc() -> Result<()> {
    let path = temp_db_path("timeline-pagination");
    let events = seed_events(
        &path,
        &[
            text_snapshot(1, "alpha"),
            text_snapshot(2, "beta"),
            text_snapshot(3, "gamma"),
        ],
    )?;
    set_event_observed_at(&path, events[0].1, "2026-04-16 09:00:00")?;
    set_event_observed_at(&path, events[1].1, "2026-04-16 10:00:00")?;
    set_event_observed_at(&path, events[2].1, "2026-04-16 11:00:00")?;

    let first_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "timeline",
        "--sort",
        "asc",
        "--limit",
        "2",
        "--format",
        "json",
    ]);
    let first_payload: Value =
        serde_json::from_slice(&first_output.stdout).expect("first timeline page should parse");
    let next_cursor = first_payload["next_cursor"]
        .as_str()
        .expect("first timeline page should include a cursor")
        .to_string();

    assert!(first_output.status.success());
    assert_eq!(first_payload["truncated"].as_bool(), Some(true));
    assert_eq!(
        first_payload["results"][0]["event_id"].as_i64(),
        Some(events[0].1)
    );
    assert_eq!(
        first_payload["results"][1]["event_id"].as_i64(),
        Some(events[1].1)
    );

    let second_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "timeline",
        "--sort",
        "asc",
        "--limit",
        "2",
        "--cursor",
        &next_cursor,
        "--format",
        "json",
    ]);
    let second_payload: Value =
        serde_json::from_slice(&second_output.stdout).expect("second timeline page should parse");

    assert!(second_output.status.success());
    assert_eq!(second_payload["truncated"].as_bool(), Some(false));
    assert!(second_payload["next_cursor"].is_null());
    assert_eq!(second_payload["results"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        second_payload["results"][0]["event_id"].as_i64(),
        Some(events[2].1)
    );

    cleanup_db(&path);
    Ok(())
}

#[test]
fn search_filters_are_combined_and_echoed_in_structured_output() -> Result<()> {
    let path = temp_db_path("search-shared-filters");
    seed_database(
        &path,
        &[
            rich_snapshot(
                1,
                "git clone https://example.com/repo",
                "https://example.com/repo",
                "file:///Users/test/report.txt",
            ),
            app_text_snapshot(2, "Preview", "com.apple.Preview", "git clone local mirror"),
        ],
    )?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "search",
        "--mode",
        "literal",
        "--app",
        "terminal",
        "--kind",
        "url",
        "--has-url",
        "--min-bytes",
        "20",
        "--format",
        "json",
        "example.com",
    ]);
    let payload: Value = serde_json::from_slice(&output.stdout).expect("search JSON should parse");

    assert!(output.status.success());
    assert_eq!(payload["results"].as_array().map(Vec::len), Some(1));
    assert_eq!(payload["applied_filters"]["app"].as_str(), Some("terminal"));
    assert_eq!(payload["applied_filters"]["kind"].as_str(), Some("url"));
    assert_eq!(payload["applied_filters"]["has_url"].as_bool(), Some(true));
    assert_eq!(payload["applied_filters"]["min_bytes"].as_u64(), Some(20));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn recall_respects_shared_filters() -> Result<()> {
    let path = temp_db_path("recall-shared-filters");
    seed_database(
        &path,
        &[
            app_text_snapshot(1, "Terminal", "com.apple.Terminal", "deploy checklist"),
            app_text_snapshot(2, "Preview", "com.apple.Preview", "invoice draft"),
        ],
    )?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "recall",
        "--prefer-recent",
        "--app",
        "preview",
        "--format",
        "json",
    ]);
    let payload: Value = serde_json::from_slice(&output.stdout).expect("recall JSON should parse");

    assert!(output.status.success());
    assert_eq!(
        payload["best_candidate"]["app_name"].as_str(),
        Some("Preview")
    );
    assert_eq!(payload["applied_filters"]["app"].as_str(), Some("preview"));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn get_and_export_use_shared_filters_as_guards() -> Result<()> {
    let path = temp_db_path("get-export-guards");
    let ids = seed_database(
        &path,
        &[rich_snapshot(
            1,
            "git status",
            "https://example.com/repo",
            "file:///Users/test/report.txt",
        )],
    )?;

    let get_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "get",
        &ids[0].to_string(),
        "--app",
        "terminal",
        "--has-url",
        "--format",
        "json",
    ]);
    let get_payload: Value =
        serde_json::from_slice(&get_output.stdout).expect("get JSON should parse");
    assert!(get_output.status.success());
    assert_eq!(
        get_payload["applied_filters"]["app"].as_str(),
        Some("terminal")
    );
    assert_eq!(
        get_payload["applied_filters"]["has_url"].as_bool(),
        Some(true)
    );

    let rejected_get = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "get",
        &ids[0].to_string(),
        "--app",
        "preview",
    ]);
    assert!(!rejected_get.status.success());
    assert!(stderr_text(&rejected_get).contains("does not satisfy the active filters"));

    let output_path =
        std::env::temp_dir().join(format!("clipmem-filter-export-{}-{}.txt", process::id(), 1));
    let rejected_export = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "export",
        &ids[0].to_string(),
        "--item",
        "0",
        "--uti",
        "public.utf8-plain-text",
        "--out",
        output_path.to_str().expect("export path should be UTF-8"),
        "--app",
        "preview",
    ]);
    assert!(!rejected_export.status.success());
    assert!(stderr_text(&rejected_export).contains("does not satisfy the active filters"));
    let _ = fs::remove_file(&output_path);

    cleanup_db(&path);
    Ok(())
}

#[test]
fn export_json_reports_written_representation() -> Result<()> {
    let path = temp_db_path("export-json");
    let ids = seed_database(&path, &[text_snapshot(1, "git status")])?;
    let output_path = temp_artifact_path("export-json", ".txt");

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
        output_path.to_str().expect("export path should be UTF-8"),
        "--format",
        "json",
    ]);
    let payload: Value = serde_json::from_slice(&output.stdout).expect("export JSON should parse");

    assert!(output.status.success(), "{}", stderr_text(&output));
    assert_eq!(payload["snapshot_id"].as_i64(), Some(ids[0]));
    assert_eq!(payload["item_index"].as_u64(), Some(0));
    assert_eq!(payload["uti"].as_str(), Some("public.utf8-plain-text"));
    assert_eq!(
        payload["byte_count"].as_u64(),
        Some("git status".len() as u64)
    );
    assert_eq!(payload["out"].as_str(), output_path.to_str());
    assert_eq!(fs::read_to_string(&output_path)?, "git status");

    cleanup_temp_artifact(&output_path);
    cleanup_db(&path);
    Ok(())
}

#[test]
fn export_accepts_bare_relative_output_path() -> Result<()> {
    let path = temp_db_path("export-relative-out");
    let ids = seed_database(&path, &[text_snapshot(1, "git status")])?;
    let cwd = temp_test_dir("export-relative-cwd");
    fs::create_dir_all(&cwd)?;

    let output = run_cli_in_dir(
        &[
            "--db",
            path.to_str().expect("db path should be UTF-8"),
            "export",
            &ids[0].to_string(),
            "--item",
            "0",
            "--uti",
            "public.utf8-plain-text",
            "--out",
            "relative-export.txt",
        ],
        &cwd,
    );

    assert!(output.status.success(), "{}", stderr_text(&output));
    assert_eq!(
        fs::read_to_string(cwd.join("relative-export.txt"))?,
        "git status"
    );

    let _ = fs::remove_dir_all(&cwd);
    cleanup_db(&path);
    Ok(())
}

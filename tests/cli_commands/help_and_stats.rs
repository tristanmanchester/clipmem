use super::*;

#[test]
fn root_help_prints_to_stdout_only_and_exits_successfully() {
    let output = run_cli(&["--help"]);
    let stdout = stdout_text(&output);
    let stderr = stderr_text(&output);

    assert_eq!(status_code(&output), 0);
    assert!(stdout.contains("Usage: clipmem"));
    assert!(stdout.contains("Examples:"));
    assert!(stdout.contains("Agent-first flow:"));
    assert!(stdout.contains("docs/action-parity.md"));
    assert!(stderr.is_empty());
}

#[test]
fn command_help_includes_examples_and_pagination_guidance() {
    let cases = [
        (
            vec!["search", "--help"],
            vec!["Examples:", "--cursor", "page size", "bounded 1-250"],
        ),
        (
            vec!["recent", "--help"],
            vec![
                "Examples:",
                "deduplicated by snapshot",
                "--cursor",
                "bounded 1-250",
            ],
        ),
        (
            vec!["timeline", "--help"],
            vec![
                "Examples:",
                "one row per real capture event",
                "--cursor",
                "bounded 1-250",
            ],
        ),
        (
            vec!["stats", "--help"],
            vec![
                "Examples:",
                "clipmem stats --hours 24",
                "shared retrieval filters",
                "text",
                "json",
            ],
        ),
        (
            vec!["recall", "--help"],
            vec![
                "Examples:",
                "primary best-first retrieval command",
                "--quote",
                "--full",
            ],
        ),
        (
            vec!["get", "--help"],
            vec![
                "Examples:",
                "nested snapshot inspection",
                "--events",
                "toon",
            ],
        ),
    ];

    for (args, needles) in cases {
        let output = run_cli(&args);
        let stdout = stdout_text(&output);
        assert_eq!(status_code(&output), 0, "help should succeed for {args:?}");
        assert!(
            stderr_text(&output).is_empty(),
            "help should stay off stderr for {args:?}"
        );
        for needle in needles {
            assert!(
                stdout.contains(needle),
                "expected help for {args:?} to contain {needle:?}\n{stdout}"
            );
        }
    }
}

#[test]
fn root_help_mentions_stats_examples() {
    let output = run_cli(&["--help"]);
    let stdout = stdout_text(&output);

    assert_eq!(status_code(&output), 0);
    assert!(stdout.contains("clipmem stats"));
}

#[test]
fn stats_text_output_shows_overview_and_leaderboards() -> Result<()> {
    let db_path = temp_db_path("stats-text-output");
    seed_database(
        &db_path,
        &[
            app_text_snapshot(1, "Terminal", "com.apple.Terminal", "git status"),
            app_text_snapshot(2, "Terminal", "com.apple.Terminal", "git status"),
            app_text_snapshot(3, "Safari", "com.apple.Safari", "https://example.com"),
        ],
    )?;

    let output = run_cli(&["--db", &db_path.display().to_string(), "stats"]);
    let stdout = stdout_text(&output);

    assert_eq!(status_code(&output), 0, "{}", stderr_text(&output));
    assert!(stdout.contains("Overview"));
    assert!(stdout.contains("Content mix"));
    assert!(stdout.contains("Top apps"));
    assert!(stdout.contains("Activity patterns"));
    assert!(stdout.contains("Leaderboards"));
    assert!(stdout.contains("Most re-copied snapshot"));

    cleanup_db(&db_path);
    Ok(())
}

#[test]
fn stats_json_output_uses_stable_envelope() -> Result<()> {
    let db_path = temp_db_path("stats-json-output");
    seed_database(&db_path, &[text_snapshot(1, "git status")])?;

    let output = run_cli(&[
        "--db",
        &db_path.display().to_string(),
        "stats",
        "--format",
        "json",
    ]);
    assert_eq!(status_code(&output), 0, "{}", stderr_text(&output));
    let payload: Value = serde_json::from_slice(&output.stdout)?;

    assert_eq!(payload["schema_version"], 2);
    assert_eq!(payload["command"], "stats");
    assert!(payload["generated_at"].is_string());
    assert!(payload["applied_filters"].is_object());
    assert_eq!(payload["stats"]["snapshot_count"], 1);
    assert_eq!(payload["stats"]["capture_event_count"], 1);

    cleanup_db(&db_path);
    Ok(())
}

#[test]
fn stats_filters_apply_to_events_and_snapshots() -> Result<()> {
    let db_path = temp_db_path("stats-filters");
    let ids = seed_events(
        &db_path,
        &[
            app_text_snapshot(1, "Terminal", "com.apple.Terminal", "git status"),
            app_text_snapshot(2, "Terminal", "com.apple.Terminal", "git status"),
            app_rich_snapshot(
                3,
                "Safari",
                "com.apple.Safari",
                "Example",
                "https://example.com",
                "file:///Users/tristan/example.txt",
            ),
            html_snapshot(4, "<p>hello</p>"),
            image_snapshot(5, &[1, 2, 3, 4]),
        ],
    )?;
    set_event_observed_at(&db_path, ids[0].1, "2026-04-17T10:00:00Z")?;
    set_event_observed_at(&db_path, ids[1].1, "2026-04-17T11:00:00Z")?;
    set_event_observed_at(&db_path, ids[2].1, "2026-04-17T12:00:00Z")?;

    let db = db_path.display().to_string();
    let cases = [
        (vec!["stats", "--since", "2026-04-17T10:30:00Z"], 4, 4),
        (vec!["stats", "--app", "safari"], 2, 2),
        (vec!["stats", "--bundle-id", "com.apple.Terminal"], 1, 2),
        (vec!["stats", "--kind", "html"], 1, 1),
        (vec!["stats", "--has-url"], 1, 1),
        (vec!["stats", "--has-image"], 1, 1),
    ];

    for (mut args, expected_snapshots, expected_events) in cases {
        let mut command = vec!["--db", db.as_str()];
        command.append(&mut args);
        command.extend(["--format", "json"]);
        let output = run_cli(&command);
        assert_eq!(status_code(&output), 0, "{}", stderr_text(&output));
        let payload: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(
            payload["stats"]["snapshot_count"], expected_snapshots,
            "args={command:?}"
        );
        assert_eq!(
            payload["stats"]["capture_event_count"], expected_events,
            "args={command:?}"
        );
    }

    cleanup_db(&db_path);
    Ok(())
}

#[test]
fn stats_dedupe_ratio_and_most_recopied_snapshot_are_correct() -> Result<()> {
    let db_path = temp_db_path("stats-dedupe");
    let ids = seed_database(
        &db_path,
        &[
            text_snapshot(1, "repeat me"),
            text_snapshot(2, "repeat me"),
            text_snapshot(3, "repeat me"),
            text_snapshot(4, "single"),
        ],
    )?;

    let output = run_cli(&[
        "--db",
        &db_path.display().to_string(),
        "stats",
        "--format",
        "json",
    ]);
    assert_eq!(status_code(&output), 0, "{}", stderr_text(&output));
    let payload: Value = serde_json::from_slice(&output.stdout)?;

    assert_eq!(payload["stats"]["snapshot_count"], 2);
    assert_eq!(payload["stats"]["capture_event_count"], 4);
    assert_eq!(payload["stats"]["dedupe_ratio"], 0.5);
    assert_eq!(
        payload["stats"]["most_recopied_snapshot"]["snapshot_id"],
        ids[0]
    );
    assert_eq!(
        payload["stats"]["most_recopied_snapshot"]["capture_count"],
        3
    );

    cleanup_db(&db_path);
    Ok(())
}

#[test]
fn stats_rejects_unsupported_formats() -> Result<()> {
    let db_path = temp_db_path("stats-unsupported-format");
    seed_database(&db_path, &[text_snapshot(1, "git status")])?;

    for format in ["jsonl", "toon", "md"] {
        let output = run_cli(&[
            "--db",
            &db_path.display().to_string(),
            "stats",
            "--format",
            format,
        ]);
        assert_eq!(status_code(&output), 2);
        assert!(stderr_text(&output).contains("invalid value"));
    }

    cleanup_db(&db_path);
    Ok(())
}

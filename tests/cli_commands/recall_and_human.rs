use super::*;

#[test]
fn agents_openclaw_install_print_and_uninstall_skill_work() -> Result<()> {
    let test_dir = temp_test_dir("openclaw-install");
    let bin_dir = test_dir.join("bin");
    let workspace_dir = test_dir.join("workspace");
    let install_dir = workspace_dir.join("skills").join("clipboard-memory");
    fs::create_dir_all(&bin_dir)?;
    fs::create_dir_all(&workspace_dir)?;

    let openclaw_path = bin_dir.join("openclaw");
    write_executable(
        &openclaw_path,
        &format!(
            "#!/bin/sh\nif [ \"$1\" = \"config\" ] && [ \"$2\" = \"get\" ] && [ \"$3\" = \"agents.defaults.workspace\" ]; then\n  printf '%s\\n' '{}'\n  exit 0\nfi\nif [ \"$1\" = \"sandbox\" ] && [ \"$2\" = \"explain\" ]; then\n  printf 'sandbox disabled\\n'\n  exit 0\nfi\nexit 1\n",
            workspace_dir.display()
        ),
    )?;

    let clipmem_link = bin_dir.join("clipmem");
    #[cfg(unix)]
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_clipmem"), &clipmem_link)?;

    let path_value = bin_dir.display().to_string();

    let install = run_cli_with_env(
        &["agents", "openclaw", "install-skill"],
        &[("PATH", &path_value), ("HOME", test_dir.to_str().unwrap())],
    );
    assert!(install.status.success());
    assert!(install_dir.join("SKILL.md").is_file());
    assert!(install_dir.join("references/commands.md").is_file());
    assert!(install_dir.join("references/troubleshooting.md").is_file());
    assert!(install_dir.join("references/json-schema.md").is_file());
    assert!(install_dir.join("references/examples.md").is_file());
    assert!(install_dir.join("references/setup-check.md").is_file());
    assert!(install_dir.join("scripts/check-setup.sh").is_file());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mode = fs::metadata(install_dir.join("scripts/check-setup.sh"))?
            .permissions()
            .mode()
            & 0o777;
        assert!(mode & 0o111 != 0);
    }

    let printed = run_cli_with_env(
        &["agents", "openclaw", "print-skill"],
        &[("PATH", &path_value), ("HOME", test_dir.to_str().unwrap())],
    );
    assert!(printed.status.success());
    assert_eq!(
        stdout_text(&printed),
        fs::read_to_string(install_dir.join("SKILL.md"))?
    );

    let uninstall = run_cli_with_env(
        &["agents", "openclaw", "uninstall-skill"],
        &[("PATH", &path_value), ("HOME", test_dir.to_str().unwrap())],
    );
    assert!(uninstall.status.success());
    assert!(!install_dir.exists());

    let _ = fs::remove_dir_all(&test_dir);
    Ok(())
}

#[test]
fn agents_openclaw_install_force_replaces_existing_skill_directory() -> Result<()> {
    let test_dir = temp_test_dir("openclaw-install-force");
    let bin_dir = test_dir.join("bin");
    let workspace_dir = test_dir.join("workspace");
    let install_dir = workspace_dir.join("skills").join("clipboard-memory");
    fs::create_dir_all(&bin_dir)?;
    fs::create_dir_all(&install_dir)?;
    fs::write(install_dir.join("SKILL.md"), "stale skill")?;
    fs::write(install_dir.join("old-file.txt"), "remove me")?;

    let openclaw_path = bin_dir.join("openclaw");
    write_executable(
        &openclaw_path,
        &format!(
            "#!/bin/sh\nif [ \"$1\" = \"config\" ] && [ \"$2\" = \"get\" ] && [ \"$3\" = \"agents.defaults.workspace\" ]; then\n  printf '%s\\n' '{}'\n  exit 0\nfi\nexit 1\n",
            workspace_dir.display()
        ),
    )?;

    let path_value = bin_dir.display().to_string();
    let install = run_cli_with_env(
        &["agents", "openclaw", "install-skill", "--force"],
        &[("PATH", &path_value), ("HOME", test_dir.to_str().unwrap())],
    );

    assert!(install.status.success(), "{}", stderr_text(&install));
    assert!(!install_dir.join("old-file.txt").exists());
    assert!(fs::read_to_string(install_dir.join("SKILL.md"))?.contains("clipboard-memory"));
    assert!(install_dir.join("references/commands.md").is_file());
    assert!(install_dir.join("scripts/check-setup.sh").is_file());

    let _ = fs::remove_dir_all(&test_dir);
    Ok(())
}

#[test]
fn agents_openclaw_doctor_reports_missing_clipmem_with_next_steps() -> Result<()> {
    let test_dir = temp_test_dir("openclaw-doctor-missing-clipmem");
    let bin_dir = test_dir.join("bin");
    let workspace_dir = test_dir.join("workspace");
    let skill_dir = workspace_dir.join("skills").join("clipboard-memory");
    fs::create_dir_all(&bin_dir)?;
    write_openclaw_skill_package(&skill_dir)?;

    let openclaw_path = bin_dir.join("openclaw");
    write_executable(
        &openclaw_path,
        &format!(
            "#!/bin/sh\nif [ \"$1\" = \"config\" ] && [ \"$2\" = \"get\" ] && [ \"$3\" = \"agents.defaults.workspace\" ]; then\n  printf '%s\\n' '{}'\n  exit 0\nfi\nif [ \"$1\" = \"sandbox\" ] && [ \"$2\" = \"explain\" ]; then\n  printf 'sandbox disabled\\n'\n  exit 0\nfi\nexit 1\n",
            workspace_dir.display()
        ),
    )?;

    let output = run_cli_with_env(
        &["agents", "openclaw", "doctor"],
        &[
            ("PATH", bin_dir.to_str().unwrap()),
            ("HOME", test_dir.to_str().unwrap()),
        ],
    );
    assert!(!output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("[FAIL] Host clipmem on PATH"));
    assert!(stdout.contains("brew install tristanmanchester/tap/clipmem"));

    let _ = fs::remove_dir_all(&test_dir);
    Ok(())
}

#[test]
fn agents_openclaw_doctor_fails_when_reference_file_is_missing() -> Result<()> {
    let test_dir = temp_test_dir("openclaw-doctor-missing-reference");
    let bin_dir = test_dir.join("bin");
    let workspace_dir = test_dir.join("workspace");
    let skill_dir = workspace_dir.join("skills").join("clipboard-memory");
    fs::create_dir_all(&bin_dir)?;
    write_openclaw_skill_package(&skill_dir)?;
    fs::remove_file(skill_dir.join("references/troubleshooting.md"))?;

    let openclaw_path = bin_dir.join("openclaw");
    write_executable(
        &openclaw_path,
        &format!(
            "#!/bin/sh\nif [ \"$1\" = \"config\" ] && [ \"$2\" = \"get\" ] && [ \"$3\" = \"agents.defaults.workspace\" ]; then\n  printf '%s\\n' '{}'\n  exit 0\nfi\nif [ \"$1\" = \"sandbox\" ] && [ \"$2\" = \"explain\" ]; then\n  printf 'sandbox disabled\\n'\n  exit 0\nfi\nexit 1\n",
            workspace_dir.display()
        ),
    )?;

    let clipmem_link = bin_dir.join("clipmem");
    #[cfg(unix)]
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_clipmem"), &clipmem_link)?;

    let output = run_cli_with_env(
        &["agents", "openclaw", "doctor"],
        &[
            ("PATH", bin_dir.to_str().unwrap()),
            ("HOME", test_dir.to_str().unwrap()),
        ],
    );
    assert!(!output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("[FAIL] SKILL.md metadata"));
    assert!(stdout.contains("packaged file is missing"));

    let _ = fs::remove_dir_all(&test_dir);
    Ok(())
}

#[test]
fn agents_openclaw_doctor_fails_when_setup_script_is_missing() -> Result<()> {
    let test_dir = temp_test_dir("openclaw-doctor-missing-setup-script");
    let bin_dir = test_dir.join("bin");
    let workspace_dir = test_dir.join("workspace");
    let skill_dir = workspace_dir.join("skills").join("clipboard-memory");
    fs::create_dir_all(&bin_dir)?;
    write_openclaw_skill_package(&skill_dir)?;
    fs::remove_file(skill_dir.join("scripts/check-setup.sh"))?;

    let openclaw_path = bin_dir.join("openclaw");
    write_executable(
        &openclaw_path,
        &format!(
            "#!/bin/sh\nif [ \"$1\" = \"config\" ] && [ \"$2\" = \"get\" ] && [ \"$3\" = \"agents.defaults.workspace\" ]; then\n  printf '%s\\n' '{}'\n  exit 0\nfi\nif [ \"$1\" = \"sandbox\" ] && [ \"$2\" = \"explain\" ]; then\n  printf 'sandbox disabled\\n'\n  exit 0\nfi\nexit 1\n",
            workspace_dir.display()
        ),
    )?;

    let clipmem_link = bin_dir.join("clipmem");
    #[cfg(unix)]
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_clipmem"), &clipmem_link)?;

    let output = run_cli_with_env(
        &["agents", "openclaw", "doctor"],
        &[
            ("PATH", bin_dir.to_str().unwrap()),
            ("HOME", test_dir.to_str().unwrap()),
        ],
    );
    assert!(!output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("[FAIL] SKILL.md metadata"));
    assert!(stdout.contains("packaged file is missing"));
    assert!(stdout.contains("scripts/check-setup.sh"));

    let _ = fs::remove_dir_all(&test_dir);
    Ok(())
}

#[cfg(unix)]
#[test]
fn agents_openclaw_doctor_fails_when_setup_script_is_not_executable() -> Result<()> {
    let test_dir = temp_test_dir("openclaw-doctor-nonexec-setup-script");
    let bin_dir = test_dir.join("bin");
    let workspace_dir = test_dir.join("workspace");
    let skill_dir = workspace_dir.join("skills").join("clipboard-memory");
    fs::create_dir_all(&bin_dir)?;
    write_openclaw_skill_package(&skill_dir)?;
    set_mode(&skill_dir.join("scripts/check-setup.sh"), 0o644)?;

    let openclaw_path = bin_dir.join("openclaw");
    write_executable(
        &openclaw_path,
        &format!(
            "#!/bin/sh\nif [ \"$1\" = \"config\" ] && [ \"$2\" = \"get\" ] && [ \"$3\" = \"agents.defaults.workspace\" ]; then\n  printf '%s\\n' '{}'\n  exit 0\nfi\nif [ \"$1\" = \"sandbox\" ] && [ \"$2\" = \"explain\" ]; then\n  printf 'sandbox disabled\\n'\n  exit 0\nfi\nexit 1\n",
            workspace_dir.display()
        ),
    )?;

    let clipmem_link = bin_dir.join("clipmem");
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_clipmem"), &clipmem_link)?;

    let output = run_cli_with_env(
        &["agents", "openclaw", "doctor"],
        &[
            ("PATH", bin_dir.to_str().unwrap()),
            ("HOME", test_dir.to_str().unwrap()),
        ],
    );
    assert!(!output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("[FAIL] SKILL.md metadata"));
    assert!(stdout.contains("packaged script is not executable"));
    assert!(stdout.contains("scripts/check-setup.sh"));

    let _ = fs::remove_dir_all(&test_dir);
    Ok(())
}

#[test]
fn portable_and_canonical_skill_packages_are_present() -> Result<()> {
    for root in [
        Path::new("skills/clipboard-memory"),
        Path::new("extras/agent-skills/clipboard-memory"),
        Path::new("extras/hermes/clipboard-memory"),
    ] {
        assert!(root.join("SKILL.md").is_file());
        assert!(root.join("references/commands.md").is_file());
        assert!(root.join("references/troubleshooting.md").is_file());
        assert!(root.join("references/json-schema.md").is_file());
        assert!(root.join("references/examples.md").is_file());
        assert!(root.join("references/setup-check.md").is_file());
        assert!(root.join("scripts/check-setup.sh").is_file());
    }
    Ok(())
}

#[test]
fn recall_json_prefers_a_strong_query_match() -> Result<()> {
    let path = temp_db_path("recall-strong-query");
    let ids = seed_database(
        &path,
        &[
            text_snapshot(1, "git status"),
            text_snapshot(2, "cargo test"),
            text_snapshot(3, "git commit"),
        ],
    )?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "recall",
        "git status",
        "--format",
        "json",
    ]);
    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("recall JSON output should parse");

    assert!(output.status.success());
    assert_eq!(payload["command"].as_str(), Some("recall"));
    assert_eq!(payload["query"].as_str(), Some("git status"));
    assert_eq!(
        payload["best_candidate"]["snapshot_id"].as_i64(),
        Some(ids[0])
    );
    assert_eq!(
        payload["best_candidate"]["best_text"].as_str(),
        Some("git status")
    );
    assert_eq!(
        payload["best_candidate"]["best_text_uti"].as_str(),
        Some("public.utf8-plain-text")
    );
    assert_eq!(
        payload["best_candidate"]["text_fragments"][0]["text"].as_str(),
        Some("git status")
    );
    assert_eq!(payload["best_match_confidence"].as_str(), Some("high"));
    assert!(payload["why_selected"]
        .as_str()
        .unwrap_or_default()
        .contains("strongest search match"));
    assert!(payload["best_candidate"]["why_matched"]
        .as_str()
        .unwrap_or_default()
        .contains("match"));
    assert!(payload["best_candidate"]["matched_fields"]
        .as_array()
        .expect("matched_fields should be an array")
        .iter()
        .any(|value| value.as_str() == Some("best_text")));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn recall_json_alias_prefers_a_strong_query_match() -> Result<()> {
    let path = temp_db_path("recall-json-alias");
    let ids = seed_database(
        &path,
        &[
            text_snapshot(1, "git status"),
            text_snapshot(2, "cargo test"),
        ],
    )?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "recall",
        "git status",
        "--json",
    ]);
    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("recall JSON alias output should parse");

    assert!(output.status.success());
    assert_eq!(payload["command"].as_str(), Some("recall"));
    assert_eq!(
        payload["best_candidate"]["snapshot_id"].as_i64(),
        Some(ids[0])
    );

    cleanup_db(&path);
    Ok(())
}

#[test]
fn recall_json_falls_back_to_recent_when_search_is_weak() -> Result<()> {
    let path = temp_db_path("recall-weak-search");
    let ids = seed_database(
        &path,
        &[
            text_snapshot(1, "git status"),
            app_text_snapshot(
                2,
                "Preview",
                "com.apple.Preview",
                "Meeting notes from today",
            ),
        ],
    )?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "recall",
        "git",
        "--mode",
        "literal",
        "--format",
        "json",
        "--min-score",
        "0.95",
        "--prefer-recent",
    ]);
    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("recall fallback JSON should parse");

    assert!(output.status.success());
    assert_eq!(
        payload["best_candidate"]["snapshot_id"].as_i64(),
        Some(ids[1])
    );
    assert!(payload["why_selected"]
        .as_str()
        .unwrap_or_default()
        .contains("Fell back to recent clipboard items"));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn recall_limit_counts_best_candidate_when_falling_back_to_recent() -> Result<()> {
    let path = temp_db_path("recall-fallback-limit");
    seed_database(
        &path,
        &[
            text_snapshot(1, "git status"),
            app_text_snapshot(
                2,
                "Preview",
                "com.apple.Preview",
                "Meeting notes from today",
            ),
        ],
    )?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "recall",
        "git",
        "--mode",
        "literal",
        "--format",
        "json",
        "--limit",
        "1",
        "--min-score",
        "0.95",
        "--prefer-recent",
    ]);
    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("recall fallback JSON should parse");

    assert!(output.status.success());
    assert_eq!(payload["alternatives"].as_array().map(Vec::len), Some(0));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn recall_without_query_returns_recent_candidates() -> Result<()> {
    let path = temp_db_path("recall-no-query");
    let ids = seed_database(
        &path,
        &[
            text_snapshot(1, "older text"),
            text_snapshot(2, "newest text"),
        ],
    )?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "recall",
        "--format",
        "json",
    ]);
    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("recall no-query JSON should parse");

    assert!(output.status.success());
    assert!(payload["query"].is_null());
    assert_eq!(
        payload["best_candidate"]["snapshot_id"].as_i64(),
        Some(ids[1])
    );
    assert_eq!(
        payload["best_candidate"]["best_text"].as_str(),
        Some("newest text")
    );
    assert!(payload["why_selected"]
        .as_str()
        .unwrap_or_default()
        .contains("most likely useful recent clipboard item"));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn get_json_exposes_html_and_rtf_plain_text_projections() -> Result<()> {
    let path = temp_db_path("get-rich-text-projections");
    let ids = seed_database(
        &path,
        &[
            html_snapshot(1, "<p>Hello <strong>world</strong></p>"),
            rtf_snapshot(2, r"{\rtf1\ansi hello\par world}"),
        ],
    )?;

    let html_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "get",
        &ids[0].to_string(),
        "--format",
        "json",
    ]);
    let html_payload: Value =
        serde_json::from_slice(&html_output.stdout).expect("html get JSON should parse");
    assert!(html_output.status.success());
    assert_eq!(
        html_payload["snapshot"]["html_text"].as_str(),
        Some("Hello world")
    );
    assert_eq!(
        html_payload["snapshot"]["best_text"].as_str(),
        Some("Hello world")
    );

    let rtf_output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "get",
        &ids[1].to_string(),
        "--format",
        "json",
    ]);
    let rtf_payload: Value =
        serde_json::from_slice(&rtf_output.stdout).expect("rtf get JSON should parse");
    assert!(rtf_output.status.success());
    assert_eq!(
        rtf_payload["snapshot"]["rtf_text"].as_str(),
        Some("hello world")
    );
    assert_eq!(
        rtf_payload["snapshot"]["best_text"].as_str(),
        Some("hello world")
    );

    cleanup_db(&path);
    Ok(())
}

#[test]
fn recall_prefer_app_boosts_matching_candidates() -> Result<()> {
    let path = temp_db_path("recall-prefer-app");
    let ids = seed_database(
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
        "--format",
        "json",
        "--prefer-app",
        "terminal",
    ]);
    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("recall prefer-app JSON should parse");

    assert!(output.status.success());
    assert_eq!(
        payload["best_candidate"]["snapshot_id"].as_i64(),
        Some(ids[0])
    );
    assert!(payload["why_selected"]
        .as_str()
        .unwrap_or_default()
        .contains("preferred app"));
    assert!(payload["best_candidate"]["matched_fields"].is_array());

    cleanup_db(&path);
    Ok(())
}

#[test]
fn recall_rejects_empty_prefer_app_before_opening_database() -> Result<()> {
    let path = temp_db_path("recall-empty-prefer-app-corrupt-db");
    fs::write(&path, b"not a sqlite database")?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "recall",
        "--prefer-app",
        "",
    ]);

    assert_eq!(status_code(&output), 2);
    assert!(stdout_text(&output).is_empty());
    assert!(stderr_text(&output).contains("preferred app cannot be empty"));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn recall_markdown_quotes_and_expands_best_text() -> Result<()> {
    let path = temp_db_path("recall-md-quote");
    let long_text = "git status --short && git log --oneline && cargo test --package clipmem";
    seed_database(&path, &[text_snapshot(1, long_text)])?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "recall",
        "git status",
        "--full",
        "--quote",
    ]);
    let stdout = stdout_text(&output);

    assert!(output.status.success());
    assert!(stdout.contains("# Best Match"));
    assert!(stdout.contains("> git status"));
    assert!(stdout.contains("Why This Match"));
    assert!(stdout.contains("Alternatives") || !stdout.contains("## Alternatives"));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn recall_toon_output_is_flattened() -> Result<()> {
    let path = temp_db_path("recall-toon");
    seed_database(&path, &[text_snapshot(1, "git status")])?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "recall",
        "git status",
        "--format",
        "toon",
    ]);
    let stdout = stdout_text(&output);

    assert!(output.status.success());
    assert!(stdout.contains(
        "best_candidate[#1\t]{snapshot_id\tevent_id\tobserved_at\tfirst_seen_at\tlast_seen_at\tkind\tapp_name\tapp_bundle_id\tdisplay_text\tcapture_count\titem_count\ttotal_bytes\tscore\twhy_matched}:"
    ));
    assert!(stdout.contains("alternatives[#0\t]{snapshot_id\tevent_id\tobserved_at\tfirst_seen_at\tlast_seen_at\tkind\tapp_name\tapp_bundle_id\tdisplay_text\tcapture_count\titem_count\ttotal_bytes\tscore\twhy_matched}:"));
    assert!(!stdout.contains("matched_fields"));
    assert!(!stdout.contains("snippet\t"));
    assert!(!stdout.contains("sha256"));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn retrieval_commands_support_human_output() -> Result<()> {
    let path = temp_db_path("retrieval-human");
    seed_database(
        &path,
        &[
            app_text_snapshot(1, "Terminal", "com.apple.Terminal", "git status --short"),
            app_text_snapshot(2, "Safari", "com.apple.Safari", "https://example.com/docs"),
        ],
    )?;

    let search = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "search",
        "git status",
        "--human",
    ]);
    let search_stdout = stdout_text(&search);
    assert!(search.status.success());
    assert_human_output(&search_stdout, "clipmem Search");
    assert!(search_stdout.contains("ID"));
    assert!(search_stdout.contains("git status"));
    assert!(!search_stdout.contains("-200"));

    let recent = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "recent",
        "--format",
        "human",
    ]);
    let recent_stdout = stdout_text(&recent);
    assert!(recent.status.success());
    assert_human_output(&recent_stdout, "clipmem Recent");
    assert!(recent_stdout.contains("Terminal") || recent_stdout.contains("Safari"));
    assert!(recent_stdout.contains("Preview"));

    let timeline = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "timeline",
        "--limit",
        "5",
        "--human",
        "--format",
        "human",
    ]);
    let timeline_stdout = stdout_text(&timeline);
    assert!(timeline.status.success());
    assert_human_output(&timeline_stdout, "clipmem Timeline");
    assert!(timeline_stdout.contains("Event"));
    assert!(timeline_stdout.contains("Snapshot"));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn recall_stats_and_get_support_human_output() -> Result<()> {
    let path = temp_db_path("recall-stats-get-human");
    let ids = seed_database(
        &path,
        &[
            text_snapshot(1, "cargo test --package clipmem"),
            app_text_snapshot(2, "Safari", "com.apple.Safari", "release notes draft"),
        ],
    )?;

    let recall = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "recall",
        "cargo test",
        "--human",
    ]);
    let recall_stdout = stdout_text(&recall);
    assert!(recall.status.success());
    assert_human_output(&recall_stdout, "clipmem Recall");
    assert!(recall_stdout.contains("Best Match"));
    assert!(recall_stdout.contains("cargo test"));
    assert!(recall_stdout.contains("Provenance"));

    let stats = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "stats",
        "--human",
    ]);
    let stats_stdout = stdout_text(&stats);
    assert!(stats.status.success());
    assert_human_output(&stats_stdout, "clipmem Archive Stats");
    assert!(stats_stdout.contains("Dedupe meter"));
    assert!(stats_stdout.contains("Content Mix"));
    assert!(stats_stdout.contains("Top Apps"));

    let get = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "get",
        &ids[0].to_string(),
        "--human",
    ]);
    let get_stdout = stdout_text(&get);
    assert!(get.status.success());
    assert_human_output(&get_stdout, "clipmem Snapshot");
    assert!(get_stdout.contains("Items"));
    assert!(get_stdout.contains("cargo test"));

    cleanup_db(&path);
    Ok(())
}

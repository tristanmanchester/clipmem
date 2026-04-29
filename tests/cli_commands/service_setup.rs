use super::*;

#[cfg(unix)]
#[test]
fn setup_check_script_treats_loaded_but_not_running_launchagent_as_stale() -> Result<()> {
    let output = run_setup_check_script_with_launchctl(
        Path::new("extras/openclaw/clipboard-memory/scripts/check-setup.sh"),
        Some("- 0 io.openclaw.clipmem.watch"),
    )?;

    assert_eq!(status_code(&output), 1);
    let stdout = stdout_text(&output);
    assert!(stdout.contains("loaded but not running"));
    assert!(stdout.contains("STALE: no recent captures and no background watcher is running"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn setup_check_script_treats_running_launchagent_as_soft_warning_only() -> Result<()> {
    let output = run_setup_check_script_with_launchctl(
        Path::new("extras/openclaw/clipboard-memory/scripts/check-setup.sh"),
        Some("123 0 io.openclaw.clipmem.watch"),
    )?;

    assert_eq!(status_code(&output), 0);
    let stdout = stdout_text(&output);
    assert!(stdout.contains("LaunchAgent io.openclaw.clipmem.watch is running"));
    assert!(stdout.contains("All checks passed."));
    Ok(())
}

#[cfg(unix)]
#[test]
fn setup_check_script_treats_missing_launchagent_as_stale() -> Result<()> {
    let output = run_setup_check_script_with_launchctl(
        Path::new("extras/openclaw/clipboard-memory/scripts/check-setup.sh"),
        None,
    )?;

    assert_eq!(status_code(&output), 1);
    let stdout = stdout_text(&output);
    assert!(stdout.contains("no clipmem background service is loaded"));
    assert!(stdout.contains("clipmem setup"));
    assert!(stdout.contains("STALE: no recent captures and no background watcher is running"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn setup_check_script_treats_running_homebrew_service_as_healthy() -> Result<()> {
    let output = run_setup_check_script_with_launchctl(
        Path::new("extras/openclaw/clipboard-memory/scripts/check-setup.sh"),
        Some("homebrew"),
    )?;

    assert_eq!(status_code(&output), 0);
    let stdout = stdout_text(&output);
    assert!(stdout.contains("Homebrew service homebrew.mxcl.clipmem is running"));
    assert!(stdout.contains("All checks passed."));
    Ok(())
}

#[test]
fn openclaw_command_help_includes_examples() {
    let cases = [
        vec!["agents", "openclaw", "install-skill", "--help"],
        vec!["agents", "openclaw", "print-skill", "--help"],
        vec!["agents", "openclaw", "doctor", "--help"],
        vec!["agents", "openclaw", "uninstall-skill", "--help"],
    ];

    for args in cases {
        let output = run_cli(&args);
        let stdout = stdout_text(&output);
        assert_eq!(status_code(&output), 0);
        assert!(stdout.contains("Examples:"));
        assert!(stderr_text(&output).is_empty());
    }
}

#[test]
fn setup_and_service_help_include_examples() {
    let cases = [
        vec!["setup", "--help"],
        vec!["service", "--help"],
        vec!["service", "status", "--help"],
    ];

    for args in cases {
        let output = run_cli(&args);
        assert_eq!(status_code(&output), 0, "help should succeed for {args:?}");
        assert!(stdout_text(&output).contains("Examples:"));
        assert!(stderr_text(&output).is_empty());
    }
}

#[test]
fn service_revision_reports_revision_without_service_probe() -> Result<()> {
    let path = temp_db_path("service-revision");
    seed_database(&path, &[text_snapshot(1, "revision fixture")])?;

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "service",
        "revision",
        "--format",
        "json",
    ]);
    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("service revision JSON should parse");

    assert!(output.status.success(), "{}", stderr_text(&output));
    assert_eq!(payload["revision"].as_u64(), Some(1));
    assert_eq!(payload["archive_content_revision"].as_u64(), Some(1));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn agents_context_reports_revision_stats_and_capabilities() -> Result<()> {
    let path = temp_db_path("agents-context");
    let store_path = temp_artifact_path("agents-context-app-store", ".json");
    fs::write(
        &store_path,
        r#"{
  "defaultRecentHours": 12,
  "binaryPathOverride": "",
  "databasePathOverride": "",
  "defaultQueryMode": "recall",
  "hotkeyEnabled": false,
  "launchAtLoginEnabled": true,
  "didConfigureLaunchAtLogin": true,
  "cachedLatestVersion": "v999.0.0",
  "cachedLatestReleaseURL": "https://github.com/tristanmanchester/clipmem/releases/tag/v999.0.0",
  "lastUpdateCheckAt": 1800000000.0
}"#,
    )?;
    let envs = vec![(
        "CLIPMEM_APP_SETTINGS_STORE".to_string(),
        store_path.display().to_string(),
    )];
    seed_database(&path, &[text_snapshot(1, "agent context fixture")])?;

    let output = run_cli_with_owned_env(
        &[
            "--db",
            path.to_str().expect("db path should be UTF-8"),
            "agents",
            "context",
            "--format",
            "json",
        ],
        &envs,
    );
    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("agents context JSON should parse");

    assert!(output.status.success(), "{}", stderr_text(&output));
    assert_eq!(payload["schema_version"].as_u64(), Some(1));
    assert!(payload["generated_at"].as_str().is_some());
    assert_eq!(payload["db_exists"].as_bool(), Some(true));
    assert_eq!(
        payload["revision"]["archive_content_revision"].as_u64(),
        Some(1)
    );
    assert_eq!(payload["stats"]["snapshot_count"].as_u64(), Some(1));
    assert_eq!(
        payload["app"]["settings"]["default_recent_hours"].as_u64(),
        Some(12)
    );
    assert_eq!(
        payload["app"]["settings"]["default_query_mode"].as_str(),
        Some("recall")
    );
    assert!(payload["app"]["settings"]["binary_path_override"].is_null());
    assert!(payload["app"]["settings"]["database_path_override"].is_null());
    assert_eq!(
        payload["app"]["launch_at_login"]["desired_enabled"].as_bool(),
        Some(true)
    );
    assert_eq!(
        payload["app"]["update_check"]["latest_version"].as_str(),
        Some("v999.0.0")
    );
    assert_eq!(
        payload["privacy"]["includes_raw_clipboard_content"].as_bool(),
        Some(false)
    );
    assert!(payload["privacy"]["includes_metadata"]
        .as_array()
        .expect("metadata disclosure")
        .iter()
        .any(|value| value.as_str() == Some("top app names")));
    assert!(payload["privacy"]["note"]
        .as_str()
        .expect("privacy note")
        .contains("operational metadata"));
    assert_eq!(
        payload["recent_activity"]["last_24h"]["snapshot_count"].as_u64(),
        Some(1)
    );
    assert!(payload["capabilities"]["app_state"]
        .as_array()
        .expect("app_state capabilities")
        .iter()
        .any(|value| value.as_str() == Some("app update-check run")));
    assert!(payload["capabilities"]["primitive_reads"]
        .as_array()
        .expect("primitive_reads capabilities")
        .iter()
        .any(|value| value.as_str() == Some("storage image-candidates")));
    assert_eq!(
        payload["capabilities"]["action_parity_doc"].as_str(),
        Some("docs/action-parity.md")
    );

    cleanup_db(&path);
    cleanup_temp_artifact(&store_path);
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn setup_reenables_disabled_direct_launchagent_before_bootstrap() -> Result<()> {
    let test_dir = temp_test_dir("service-setup-disabled-direct");
    let home_dir = test_dir.join("home");
    let bin_dir = test_dir.join("bin");
    let state_dir = test_dir.join("state");
    fs::create_dir_all(&home_dir)?;
    fs::create_dir_all(&bin_dir)?;
    fs::create_dir_all(&state_dir)?;
    write_stateful_launchctl_stub(&bin_dir, &state_dir)?;
    write_executable(&bin_dir.join("id"), "#!/bin/sh\nprintf '501\\n'\n")?;
    fs::write(state_dir.join("direct.disabled"), "")?;

    let db_path = home_dir
        .join("Library/Application Support/clipmem/clipmem.sqlite3")
        .display()
        .to_string();
    let path_value = format!("{}:/usr/bin:/bin", bin_dir.display());
    let active_binary = home_dir.join(".cargo/bin/clipmem").display().to_string();
    let envs = vec![
        ("HOME".to_string(), home_dir.display().to_string()),
        ("PATH".to_string(), path_value),
        ("CLIPMEM_TEST_ACTIVE_BINARY".to_string(), active_binary),
        (
            "CLIPMEM_TEST_SKIP_SETUP_CAPTURE_ONCE".to_string(),
            "1".to_string(),
        ),
    ];

    let output = run_cli_with_owned_env(&["--db", &db_path, "setup"], &envs);
    assert!(output.status.success(), "{}", stderr_text(&output));
    assert!(!state_dir.join("direct.disabled").exists());

    let status = run_cli_with_owned_env(&["--db", &db_path, "service", "status", "--json"], &envs);
    assert!(status.status.success(), "{}", stderr_text(&status));
    let payload: Value = serde_json::from_slice(&status.stdout)?;
    assert_eq!(payload["preferred_provider"], "launchagent");
    assert_eq!(payload["launchagent"]["running"], true);

    let _ = fs::remove_dir_all(&test_dir);
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn setup_uses_direct_launchagent_provider_when_homebrew_is_not_detected() -> Result<()> {
    let test_dir = temp_test_dir("service-setup-direct");
    let home_dir = test_dir.join("home");
    let bin_dir = test_dir.join("bin");
    let state_dir = test_dir.join("state");
    fs::create_dir_all(&home_dir)?;
    fs::create_dir_all(&bin_dir)?;
    fs::create_dir_all(&state_dir)?;
    write_stateful_launchctl_stub(&bin_dir, &state_dir)?;
    write_executable(&bin_dir.join("id"), "#!/bin/sh\nprintf '501\\n'\n")?;

    let db_path = home_dir
        .join("Library/Application Support/clipmem/clipmem.sqlite3")
        .display()
        .to_string();
    let path_value = format!("{}:/usr/bin:/bin", bin_dir.display());
    let active_binary = home_dir.join(".cargo/bin/clipmem").display().to_string();
    let envs = vec![
        ("HOME".to_string(), home_dir.display().to_string()),
        ("PATH".to_string(), path_value),
        (
            "CLIPMEM_TEST_ACTIVE_BINARY".to_string(),
            active_binary.clone(),
        ),
        (
            "CLIPMEM_TEST_SKIP_SETUP_CAPTURE_ONCE".to_string(),
            "1".to_string(),
        ),
    ];

    let output = run_cli_with_owned_env(&["--db", &db_path, "setup"], &envs);
    assert!(output.status.success(), "{}", stderr_text(&output));
    let stdout = stdout_text(&output);
    assert!(stdout.contains("provider: launchagent"));
    assert!(stdout.contains("seed_capture: not_attempted"));

    let plist_path = home_dir.join("Library/LaunchAgents/io.openclaw.clipmem.watch.plist");
    assert!(plist_path.is_file());
    let plist = fs::read_to_string(&plist_path)?;
    assert!(plist.contains(&active_binary));
    assert!(plist.contains(&db_path));

    let status = run_cli_with_owned_env(&["--db", &db_path, "service", "status", "--json"], &envs);
    assert!(status.status.success(), "{}", stderr_text(&status));
    let payload: Value = serde_json::from_slice(&status.stdout)?;
    assert_eq!(payload["preferred_provider"], "launchagent");
    assert_eq!(payload["launchagent"]["running"], true);
    assert_eq!(payload["conflict"], false);

    let stop = run_cli_with_owned_env(&["--db", &db_path, "service", "stop"], &envs);
    assert!(stop.status.success(), "{}", stderr_text(&stop));
    assert!(plist_path.is_file());

    let uninstall = run_cli_with_owned_env(&["--db", &db_path, "service", "uninstall"], &envs);
    assert!(uninstall.status.success(), "{}", stderr_text(&uninstall));
    assert!(!plist_path.exists());

    let _ = fs::remove_dir_all(&test_dir);
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn service_stop_reports_direct_launchctl_disable_failure() -> Result<()> {
    let test_dir = temp_test_dir("service-stop-disable-failure");
    let home_dir = test_dir.join("home");
    let bin_dir = test_dir.join("bin");
    let state_dir = test_dir.join("state");
    fs::create_dir_all(home_dir.join("Library/LaunchAgents"))?;
    fs::create_dir_all(&bin_dir)?;
    fs::create_dir_all(&state_dir)?;
    write_stateful_launchctl_stub(&bin_dir, &state_dir)?;
    write_executable(&bin_dir.join("id"), "#!/bin/sh\nprintf '501\\n'\n")?;
    fs::write(
        state_dir.join("direct.state"),
        "123 0 io.openclaw.clipmem.watch\n",
    )?;
    fs::write(state_dir.join("disable.fail"), "")?;

    let db_path = home_dir
        .join("Library/Application Support/clipmem/clipmem.sqlite3")
        .display()
        .to_string();
    let path_value = format!("{}:/usr/bin:/bin", bin_dir.display());
    let envs = vec![
        ("HOME".to_string(), home_dir.display().to_string()),
        ("PATH".to_string(), path_value),
        (
            "CLIPMEM_TEST_ACTIVE_BINARY".to_string(),
            home_dir.join(".cargo/bin/clipmem").display().to_string(),
        ),
    ];

    let output = run_cli_with_owned_env(&["--db", &db_path, "service", "stop"], &envs);
    assert!(!output.status.success());
    let stderr = stderr_text(&output);
    assert!(stderr.contains("launchctl disable"));
    assert!(stderr.contains("forced disable failure"));

    let _ = fs::remove_dir_all(&test_dir);
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn service_uninstall_reports_direct_plist_removal_failure() -> Result<()> {
    let test_dir = temp_test_dir("service-uninstall-plist-failure");
    let home_dir = test_dir.join("home");
    let bin_dir = test_dir.join("bin");
    let state_dir = test_dir.join("state");
    let plist_path = home_dir.join("Library/LaunchAgents/io.openclaw.clipmem.watch.plist");
    fs::create_dir_all(&plist_path)?;
    fs::create_dir_all(&bin_dir)?;
    fs::create_dir_all(&state_dir)?;
    write_stateful_launchctl_stub(&bin_dir, &state_dir)?;
    write_executable(&bin_dir.join("id"), "#!/bin/sh\nprintf '501\\n'\n")?;
    fs::write(
        state_dir.join("direct.state"),
        "123 0 io.openclaw.clipmem.watch\n",
    )?;

    let db_path = home_dir
        .join("Library/Application Support/clipmem/clipmem.sqlite3")
        .display()
        .to_string();
    let path_value = format!("{}:/usr/bin:/bin", bin_dir.display());
    let envs = vec![
        ("HOME".to_string(), home_dir.display().to_string()),
        ("PATH".to_string(), path_value),
        (
            "CLIPMEM_TEST_ACTIVE_BINARY".to_string(),
            home_dir.join(".cargo/bin/clipmem").display().to_string(),
        ),
    ];

    let output = run_cli_with_owned_env(&["--db", &db_path, "service", "uninstall"], &envs);
    assert!(!output.status.success());
    let stderr = stderr_text(&output);
    assert!(stderr.contains("remove direct LaunchAgent plist"));
    assert!(stderr.contains("io.openclaw.clipmem.watch.plist"));

    let _ = fs::remove_dir_all(&test_dir);
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn setup_ignores_path_poisoning_when_persisting_service_binary() -> Result<()> {
    let test_dir = temp_test_dir("service-setup-path-poisoning");
    let home_dir = test_dir.join("home");
    let bin_dir = test_dir.join("bin");
    let state_dir = test_dir.join("state");
    fs::create_dir_all(&home_dir)?;
    fs::create_dir_all(&bin_dir)?;
    fs::create_dir_all(&state_dir)?;
    write_stateful_launchctl_stub(&bin_dir, &state_dir)?;
    write_executable(&bin_dir.join("id"), "#!/bin/sh\nprintf '501\\n'\n")?;
    write_executable(
        &bin_dir.join("clipmem"),
        "#!/bin/sh\nprintf 'poisoned clipmem should not run\\n' >&2\nexit 99\n",
    )?;

    let db_path = home_dir
        .join("Library/Application Support/clipmem/clipmem.sqlite3")
        .display()
        .to_string();
    let trusted_binary = env!("CARGO_BIN_EXE_clipmem").to_string();
    let path_value = format!("{}:/usr/bin:/bin", bin_dir.display());
    let envs = vec![
        ("HOME".to_string(), home_dir.display().to_string()),
        ("PATH".to_string(), path_value),
        (
            "CLIPMEM_TEST_SKIP_SETUP_CAPTURE_ONCE".to_string(),
            "1".to_string(),
        ),
    ];

    let output = run_cli_with_owned_env(&["--db", &db_path, "setup"], &envs);
    assert!(output.status.success(), "{}", stderr_text(&output));

    let plist_path = home_dir.join("Library/LaunchAgents/io.openclaw.clipmem.watch.plist");
    assert!(plist_path.is_file());
    let plist = fs::read_to_string(&plist_path)?;
    assert!(plist.contains(&trusted_binary));
    assert!(!plist.contains(&bin_dir.join("clipmem").display().to_string()));

    let status = run_cli_with_owned_env(&["--db", &db_path, "service", "status", "--json"], &envs);
    assert!(status.status.success(), "{}", stderr_text(&status));
    let payload: Value = serde_json::from_slice(&status.stdout)?;
    assert_eq!(payload["binary_path"], trusted_binary);
    assert_eq!(payload["preferred_provider"], "launchagent");

    let _ = fs::remove_dir_all(&test_dir);
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn setup_prefers_homebrew_provider_when_homebrew_binary_and_services_are_available() -> Result<()> {
    let test_dir = temp_test_dir("service-setup-homebrew");
    let home_dir = test_dir.join("home");
    let bin_dir = test_dir.join("bin");
    let state_dir = test_dir.join("state");
    fs::create_dir_all(&home_dir)?;
    fs::create_dir_all(&bin_dir)?;
    fs::create_dir_all(&state_dir)?;
    write_stateful_launchctl_stub(&bin_dir, &state_dir)?;
    write_stateful_brew_stub(&bin_dir, &state_dir, true)?;
    write_executable(&bin_dir.join("id"), "#!/bin/sh\nprintf '501\\n'\n")?;

    let path_value = format!("{}:/usr/bin:/bin", bin_dir.display());
    let envs = vec![
        ("HOME".to_string(), home_dir.display().to_string()),
        ("PATH".to_string(), path_value),
        (
            "CLIPMEM_TEST_ACTIVE_BINARY".to_string(),
            "/opt/homebrew/bin/clipmem".to_string(),
        ),
        (
            "CLIPMEM_TEST_SKIP_SETUP_CAPTURE_ONCE".to_string(),
            "1".to_string(),
        ),
    ];

    let output = run_cli_with_owned_env(&["setup"], &envs);
    assert!(output.status.success(), "{}", stderr_text(&output));
    let stdout = stdout_text(&output);
    assert!(stdout.contains("provider: homebrew"));

    let brew_log = fs::read_to_string(state_dir.join("brew.log"))?;
    assert!(brew_log.contains("services info clipmem --json"));

    let status = run_cli_with_owned_env(&["service", "status", "--json"], &envs);
    assert!(status.status.success(), "{}", stderr_text(&status));
    let payload: Value = serde_json::from_slice(&status.stdout)?;
    assert_eq!(payload["preferred_provider"], "homebrew");
    assert_eq!(payload["homebrew"]["running"], true);

    let _ = fs::remove_dir_all(&test_dir);
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn setup_falls_back_to_direct_launchagent_when_homebrew_formula_has_no_service() -> Result<()> {
    let test_dir = temp_test_dir("service-homebrew-no-service");
    let home_dir = test_dir.join("home");
    let bin_dir = test_dir.join("bin");
    let state_dir = test_dir.join("state");
    fs::create_dir_all(&home_dir)?;
    fs::create_dir_all(&bin_dir)?;
    fs::create_dir_all(&state_dir)?;
    write_stateful_launchctl_stub(&bin_dir, &state_dir)?;
    write_brew_stub_without_clipmem_service(&bin_dir, &state_dir)?;
    write_executable(&bin_dir.join("id"), "#!/bin/sh\nprintf '501\\n'\n")?;

    let path_value = format!("{}:/usr/bin:/bin", bin_dir.display());
    let active_binary = "/opt/homebrew/bin/clipmem".to_string();
    let envs = vec![
        ("HOME".to_string(), home_dir.display().to_string()),
        ("PATH".to_string(), path_value),
        (
            "CLIPMEM_TEST_ACTIVE_BINARY".to_string(),
            active_binary.clone(),
        ),
        (
            "CLIPMEM_TEST_SKIP_SETUP_CAPTURE_ONCE".to_string(),
            "1".to_string(),
        ),
    ];

    let output = run_cli_with_owned_env(&["setup"], &envs);
    assert!(output.status.success(), "{}", stderr_text(&output));
    let stdout = stdout_text(&output);
    assert!(stdout.contains("provider: launchagent"));
    assert!(stdout.contains("Homebrew install detected"));
    assert!(stdout.contains("cannot manage the clipmem formula"));

    let brew_log = fs::read_to_string(state_dir.join("brew.log"))?;
    assert!(brew_log.contains("services info clipmem --json"));

    let plist_path = home_dir.join("Library/LaunchAgents/io.openclaw.clipmem.watch.plist");
    assert!(plist_path.is_file());
    let plist = fs::read_to_string(&plist_path)?;
    assert!(plist.contains(&active_binary));

    let status = run_cli_with_owned_env(&["service", "status", "--json"], &envs);
    assert!(status.status.success(), "{}", stderr_text(&status));
    let payload: Value = serde_json::from_slice(&status.stdout)?;
    assert_eq!(payload["preferred_provider"], "launchagent");
    assert_eq!(payload["launchagent"]["running"], true);

    let _ = fs::remove_dir_all(&test_dir);
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn service_start_fails_with_clear_guidance_when_both_providers_are_installed() -> Result<()> {
    let test_dir = temp_test_dir("service-conflict");
    let home_dir = test_dir.join("home");
    let bin_dir = test_dir.join("bin");
    let state_dir = test_dir.join("state");
    fs::create_dir_all(home_dir.join("Library/LaunchAgents"))?;
    fs::create_dir_all(&bin_dir)?;
    fs::create_dir_all(&state_dir)?;
    write_stateful_launchctl_stub(&bin_dir, &state_dir)?;
    write_stateful_brew_stub(&bin_dir, &state_dir, true)?;
    write_executable(&bin_dir.join("id"), "#!/bin/sh\nprintf '501\\n'\n")?;
    fs::write(
        state_dir.join("homebrew.state"),
        "456 0 homebrew.mxcl.clipmem\n",
    )?;
    fs::write(
        state_dir.join("direct.state"),
        "123 0 io.openclaw.clipmem.watch\n",
    )?;
    fs::write(
        home_dir.join("Library/LaunchAgents/io.openclaw.clipmem.watch.plist"),
        "<plist/>",
    )?;
    fs::write(
        home_dir.join("Library/LaunchAgents/homebrew.mxcl.clipmem.plist"),
        "<plist/>",
    )?;

    let path_value = format!("{}:/usr/bin:/bin", bin_dir.display());
    let envs = vec![
        ("HOME".to_string(), home_dir.display().to_string()),
        ("PATH".to_string(), path_value),
        (
            "CLIPMEM_TEST_ACTIVE_BINARY".to_string(),
            "/opt/homebrew/bin/clipmem".to_string(),
        ),
    ];

    let output = run_cli_with_owned_env(&["service", "start"], &envs);
    assert!(!output.status.success());
    let stderr = stderr_text(&output);
    assert!(stderr.contains("Both the Homebrew service and the direct LaunchAgent are installed"));
    assert!(stderr.contains("brew services stop clipmem"));
    assert!(stderr.contains("clipmem service uninstall"));

    let _ = fs::remove_dir_all(&test_dir);
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn service_status_reports_stale_when_captures_are_old_and_no_service_is_running() -> Result<()> {
    let test_dir = temp_test_dir("service-status-stale");
    let home_dir = test_dir.join("home");
    let bin_dir = test_dir.join("bin");
    let state_dir = test_dir.join("state");
    fs::create_dir_all(&home_dir)?;
    fs::create_dir_all(&bin_dir)?;
    fs::create_dir_all(&state_dir)?;
    write_stateful_launchctl_stub(&bin_dir, &state_dir)?;
    write_executable(&bin_dir.join("id"), "#!/bin/sh\nprintf '501\\n'\n")?;

    let db_path = home_dir.join("Library/Application Support/clipmem/clipmem.sqlite3");
    let seeded = seed_events(&db_path, &[text_snapshot(1, "git status")])?;
    let (_, event_id) = seeded[0];
    set_event_observed_at(&db_path, event_id, "2026-04-10 10:00:00")?;

    let path_value = format!("{}:/usr/bin:/bin", bin_dir.display());
    let envs = vec![
        ("HOME".to_string(), home_dir.display().to_string()),
        ("PATH".to_string(), path_value),
        (
            "CLIPMEM_TEST_ACTIVE_BINARY".to_string(),
            home_dir.join(".cargo/bin/clipmem").display().to_string(),
        ),
    ];

    let output = run_cli_with_owned_env(
        &[
            "--db",
            db_path.to_str().expect("utf-8 db path"),
            "service",
            "status",
            "--json",
        ],
        &envs,
    );
    assert!(output.status.success(), "{}", stderr_text(&output));
    let payload: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(payload["stale"], true);
    assert_eq!(payload["launchagent"]["running"], false);
    assert_eq!(payload["recent_capture_within_last_hour"], false);

    let _ = fs::remove_dir_all(&test_dir);
    Ok(())
}

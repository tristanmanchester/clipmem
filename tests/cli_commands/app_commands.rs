use super::*;

#[test]
fn app_settings_show_reports_defaults() -> Result<()> {
    let path = temp_db_path("app-settings-defaults");
    let store_path = temp_artifact_path("app-settings-defaults", ".json");
    let envs = vec![(
        "CLIPMEM_APP_SETTINGS_STORE".to_string(),
        store_path.display().to_string(),
    )];

    let output = run_cli_with_owned_env(
        &[
            "--db",
            path.to_str().expect("db path should be UTF-8"),
            "app",
            "settings",
            "show",
            "--format",
            "json",
        ],
        &envs,
    );
    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("app settings JSON should parse");

    assert!(output.status.success(), "{}", stderr_text(&output));
    assert!(payload["binary_path_override"].is_null());
    assert_eq!(payload["default_recent_hours"].as_u64(), Some(24));
    assert_eq!(payload["default_query_mode"].as_str(), Some("recent"));
    assert_eq!(payload["hotkey_enabled"].as_bool(), Some(true));

    cleanup_db(&path);
    cleanup_temp_artifact(&store_path);
    Ok(())
}

#[cfg(not(target_os = "macos"))]
#[test]
fn app_settings_show_uses_defaults_without_macos_defaults_command() -> Result<()> {
    let path = temp_db_path("app-settings-non-macos-defaults");

    let output = run_cli(&[
        "--db",
        path.to_str().expect("db path should be UTF-8"),
        "app",
        "settings",
        "show",
        "--format",
        "json",
    ]);
    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("app settings JSON should parse");

    assert!(output.status.success(), "{}", stderr_text(&output));
    assert!(payload["binary_path_override"].is_null());
    assert!(payload["database_path_override"].is_null());
    assert_eq!(payload["default_recent_hours"].as_u64(), Some(24));
    assert_eq!(payload["default_query_mode"].as_str(), Some("recent"));
    assert_eq!(payload["hotkey_enabled"].as_bool(), Some(true));

    cleanup_db(&path);
    Ok(())
}

#[test]
fn app_settings_set_and_clear_preferences_bump_revision() -> Result<()> {
    let path = temp_db_path("app-settings-set-clear");
    let store_path = temp_artifact_path("app-settings-set-clear", ".json");
    let envs = vec![(
        "CLIPMEM_APP_SETTINGS_STORE".to_string(),
        store_path.display().to_string(),
    )];

    let set_binary = run_cli_with_owned_env(
        &[
            "--db",
            path.to_str().expect("db path should be UTF-8"),
            "app",
            "settings",
            "set",
            "binary-path-override",
            "/tmp/clipmem",
            "--format",
            "json",
        ],
        &envs,
    );
    let set_hours = run_cli_with_owned_env(
        &[
            "--db",
            path.to_str().expect("db path should be UTF-8"),
            "app",
            "settings",
            "set",
            "default-recent-hours",
            "12",
        ],
        &envs,
    );
    let clear_binary = run_cli_with_owned_env(
        &[
            "--db",
            path.to_str().expect("db path should be UTF-8"),
            "app",
            "settings",
            "clear",
            "binary-path-override",
        ],
        &envs,
    );

    assert!(set_binary.status.success(), "{}", stderr_text(&set_binary));
    assert!(set_hours.status.success(), "{}", stderr_text(&set_hours));
    assert!(
        clear_binary.status.success(),
        "{}",
        stderr_text(&clear_binary)
    );

    let show = run_cli_with_owned_env(
        &[
            "--db",
            path.to_str().expect("db path should be UTF-8"),
            "app",
            "settings",
            "show",
            "--format",
            "json",
        ],
        &envs,
    );
    let payload: Value =
        serde_json::from_slice(&show.stdout).expect("app settings JSON should parse");
    let db = Database::open_existing(&path)?;

    assert!(payload["binary_path_override"].is_null());
    assert_eq!(payload["default_recent_hours"].as_u64(), Some(12));
    assert_eq!(db.archive_revision()?.app_preferences_revision(), 3);

    cleanup_db(&path);
    cleanup_temp_artifact(&store_path);
    Ok(())
}

#[test]
fn app_settings_reject_invalid_recent_hours_and_query_mode() {
    let path = temp_db_path("app-settings-invalid");
    let store_path = temp_artifact_path("app-settings-invalid", ".json");
    let envs = vec![(
        "CLIPMEM_APP_SETTINGS_STORE".to_string(),
        store_path.display().to_string(),
    )];

    let hours = run_cli_with_owned_env(
        &[
            "--db",
            path.to_str().expect("db path should be UTF-8"),
            "app",
            "settings",
            "set",
            "default-recent-hours",
            "0",
        ],
        &envs,
    );
    let mode = run_cli_with_owned_env(
        &[
            "--db",
            path.to_str().expect("db path should be UTF-8"),
            "app",
            "settings",
            "set",
            "default-query-mode",
            "unknown",
        ],
        &envs,
    );

    assert!(!hours.status.success());
    assert!(stderr_text(&hours).contains("greater than zero"));
    assert!(!mode.status.success());
    assert!(stderr_text(&mode).contains("default-query-mode"));

    cleanup_db(&path);
    cleanup_temp_artifact(&store_path);
}

#[test]
fn app_launch_at_login_set_and_clear_bumps_revision() -> Result<()> {
    let path = temp_db_path("app-launch-at-login");
    let store_path = temp_artifact_path("app-launch-at-login", ".json");
    let envs = vec![(
        "CLIPMEM_APP_SETTINGS_STORE".to_string(),
        store_path.display().to_string(),
    )];

    let set = run_cli_with_owned_env(
        &[
            "--db",
            path.to_str().expect("db path should be UTF-8"),
            "app",
            "launch-at-login",
            "set",
            "on",
            "--format",
            "json",
        ],
        &envs,
    );
    let payload: Value =
        serde_json::from_slice(&set.stdout).expect("launch-at-login JSON should parse");

    assert!(set.status.success(), "{}", stderr_text(&set));
    assert_eq!(payload["desired_enabled"].as_bool(), Some(true));
    assert_eq!(payload["status"].as_str(), Some("requested_enabled"));
    assert_eq!(payload["requires_app_apply"].as_bool(), Some(true));

    let clear = run_cli_with_owned_env(
        &[
            "--db",
            path.to_str().expect("db path should be UTF-8"),
            "app",
            "launch-at-login",
            "clear",
            "--format",
            "json",
        ],
        &envs,
    );
    let payload: Value =
        serde_json::from_slice(&clear.stdout).expect("launch-at-login JSON should parse");
    let db = Database::open_existing(&path)?;

    assert!(clear.status.success(), "{}", stderr_text(&clear));
    assert!(payload["desired_enabled"].is_null());
    assert_eq!(payload["status"].as_str(), Some("default"));
    assert_eq!(db.archive_revision()?.app_preferences_revision(), 2);

    cleanup_db(&path);
    cleanup_temp_artifact(&store_path);
    Ok(())
}

#[test]
fn app_update_check_show_and_clear_cached_state() -> Result<()> {
    let path = temp_db_path("app-update-check");
    let store_path = temp_artifact_path("app-update-check", ".json");
    fs::write(
        &store_path,
        r#"{
  "cachedLatestVersion": "v999.0.0",
  "cachedLatestReleaseURL": "https://github.com/tristanmanchester/clipmem/releases/tag/v999.0.0",
  "lastUpdateCheckAt": 1800000000.0
}"#,
    )?;
    let envs = vec![(
        "CLIPMEM_APP_SETTINGS_STORE".to_string(),
        store_path.display().to_string(),
    )];

    let show = run_cli_with_owned_env(
        &[
            "--db",
            path.to_str().expect("db path should be UTF-8"),
            "app",
            "update-check",
            "show",
            "--format",
            "json",
        ],
        &envs,
    );
    let payload: Value =
        serde_json::from_slice(&show.stdout).expect("update-check JSON should parse");

    assert!(show.status.success(), "{}", stderr_text(&show));
    assert_eq!(payload["latest_version"].as_str(), Some("v999.0.0"));
    assert_eq!(payload["is_update_available"].as_bool(), Some(true));

    let clear = run_cli_with_owned_env(
        &[
            "--db",
            path.to_str().expect("db path should be UTF-8"),
            "app",
            "update-check",
            "clear",
            "--format",
            "json",
        ],
        &envs,
    );
    let payload: Value =
        serde_json::from_slice(&clear.stdout).expect("update-check JSON should parse");
    let db = Database::open_existing(&path)?;

    assert!(clear.status.success(), "{}", stderr_text(&clear));
    assert!(payload["latest_version"].is_null());
    assert_eq!(payload["is_update_available"].as_bool(), Some(false));
    assert_eq!(db.archive_revision()?.app_preferences_revision(), 1);

    cleanup_db(&path);
    cleanup_temp_artifact(&store_path);
    Ok(())
}

#[test]
fn app_update_check_run_refreshes_cached_state_and_bumps_revision() -> Result<()> {
    let path = temp_db_path("app-update-check-run");
    let store_path = temp_artifact_path("app-update-check-run", ".json");
    let envs = vec![
        (
            "CLIPMEM_APP_SETTINGS_STORE".to_string(),
            store_path.display().to_string(),
        ),
        (
            "CLIPMEM_UPDATE_CHECK_RESPONSE".to_string(),
            r#"{
  "tag_name": "v999.0.0",
  "html_url": "https://github.com/tristanmanchester/clipmem/releases/tag/v999.0.0",
  "prerelease": false,
  "draft": false
}"#
            .to_string(),
        ),
    ];

    let run = run_cli_with_owned_env(
        &[
            "--db",
            path.to_str().expect("db path should be UTF-8"),
            "app",
            "update-check",
            "run",
            "--format",
            "json",
        ],
        &envs,
    );
    let payload: Value =
        serde_json::from_slice(&run.stdout).expect("update-check run JSON should parse");
    let db = Database::open_existing(&path)?;

    assert!(run.status.success(), "{}", stderr_text(&run));
    assert_eq!(payload["latest_version"].as_str(), Some("v999.0.0"));
    assert_eq!(
        payload["release_url"].as_str(),
        Some("https://github.com/tristanmanchester/clipmem/releases/tag/v999.0.0")
    );
    assert_eq!(payload["is_update_available"].as_bool(), Some(true));
    assert!(payload["last_checked_at_unix"].as_f64().is_some());
    assert_eq!(db.archive_revision()?.app_preferences_revision(), 1);
    let store: Value = serde_json::from_slice(
        &fs::read(&store_path).expect("app settings store should exist after update check"),
    )
    .expect("app settings store should parse");
    assert!(store["lastUpdateCheckAt"].as_u64().is_some());

    cleanup_db(&path);
    cleanup_temp_artifact(&store_path);
    Ok(())
}

#[test]
fn app_preference_mutations_bump_existing_database_override_revision() -> Result<()> {
    let invocation_path = temp_db_path("app-pref-invocation-db");
    let override_path = temp_db_path("app-pref-override-db");
    let store_path = temp_artifact_path("app-pref-override-store", ".json");
    seed_database(&override_path, &[text_snapshot(1, "override database")])?;
    fs::write(
        &store_path,
        format!(
            r#"{{
  "databasePathOverride": "{}"
}}"#,
            override_path.display()
        ),
    )?;
    let envs = vec![(
        "CLIPMEM_APP_SETTINGS_STORE".to_string(),
        store_path.display().to_string(),
    )];

    let output = run_cli_with_owned_env(
        &[
            "--db",
            invocation_path.to_str().expect("db path should be UTF-8"),
            "app",
            "settings",
            "set",
            "default-recent-hours",
            "48",
            "--format",
            "json",
        ],
        &envs,
    );

    assert!(output.status.success(), "{}", stderr_text(&output));
    let invocation_db = Database::open_existing(&invocation_path)?;
    let override_db = Database::open_existing(&override_path)?;
    assert_eq!(
        invocation_db.archive_revision()?.app_preferences_revision(),
        1
    );
    assert_eq!(
        override_db.archive_revision()?.app_preferences_revision(),
        1
    );

    cleanup_db(&invocation_path);
    cleanup_db(&override_path);
    cleanup_temp_artifact(&store_path);
    Ok(())
}

#[test]
fn app_update_check_run_rejects_network_errors_without_clearing_cache() -> Result<()> {
    let path = temp_db_path("app-update-check-run-error");
    let store_path = temp_artifact_path("app-update-check-run-error", ".json");
    fs::write(
        &store_path,
        r#"{
  "cachedLatestVersion": "v888.0.0",
  "cachedLatestReleaseURL": "https://example.com/release",
  "lastUpdateCheckAt": 1700000000.0
}"#,
    )
    .expect("write app settings cache");
    let envs = vec![
        (
            "CLIPMEM_APP_SETTINGS_STORE".to_string(),
            store_path.display().to_string(),
        ),
        (
            "CLIPMEM_UPDATE_CHECK_RESPONSE".to_string(),
            "not json".to_string(),
        ),
    ];

    let run = run_cli_with_owned_env(
        &[
            "--db",
            path.to_str().expect("db path should be UTF-8"),
            "app",
            "update-check",
            "run",
            "--format",
            "json",
        ],
        &envs,
    );
    assert!(!run.status.success());
    assert!(stderr_text(&run).contains("parse update check response"));

    let show = run_cli_with_owned_env(
        &[
            "--db",
            path.to_str().expect("db path should be UTF-8"),
            "app",
            "update-check",
            "show",
            "--format",
            "json",
        ],
        &envs[..1],
    );
    let payload: Value =
        serde_json::from_slice(&show.stdout).expect("update-check show JSON should parse");
    assert!(show.status.success(), "{}", stderr_text(&show));
    assert_eq!(payload["latest_version"].as_str(), Some("v888.0.0"));

    cleanup_db(&path);
    cleanup_temp_artifact(&store_path);
    Ok(())
}

#[test]
fn app_quit_reports_request_with_test_override() -> Result<()> {
    let path = temp_db_path("app-quit");
    let envs = vec![("CLIPMEM_APP_QUIT_SKIP_OS".to_string(), "1".to_string())];

    let output = run_cli_with_owned_env(
        &[
            "--db",
            path.to_str().expect("db path should be UTF-8"),
            "app",
            "quit",
            "--format",
            "json",
        ],
        &envs,
    );
    let payload: Value = serde_json::from_slice(&output.stdout).expect("quit JSON should parse");

    assert!(output.status.success(), "{}", stderr_text(&output));
    assert_eq!(payload["requested"].as_bool(), Some(true));
    assert_eq!(payload["method"].as_str(), Some("test_override"));

    cleanup_db(&path);
    Ok(())
}

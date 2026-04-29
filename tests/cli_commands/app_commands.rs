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

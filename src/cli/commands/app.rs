use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use serde_json::Value;

use crate::cli::formats::{OutputFormat, ToggleState};
use crate::cli::presentation::emit_json_or_text;
use crate::cli::schema::{
    AppArgs, AppCommand, AppLaunchAtLoginArgs, AppLaunchAtLoginClearArgs, AppLaunchAtLoginCommand,
    AppLaunchAtLoginSetArgs, AppLaunchAtLoginShowArgs, AppPreferenceKey, AppSettingsArgs,
    AppSettingsClearArgs, AppSettingsCommand, AppSettingsSetArgs, AppSettingsShowArgs,
    AppUpdateCheckArgs, AppUpdateCheckClearArgs, AppUpdateCheckCommand, AppUpdateCheckShowArgs,
};
use crate::db::Database;

const APP_DOMAIN: &str = "io.openclaw.clipmem.menubar";
const LAUNCH_AT_LOGIN_ENABLED_KEY: &str = "launchAtLoginEnabled";
const DID_CONFIGURE_LAUNCH_AT_LOGIN_KEY: &str = "didConfigureLaunchAtLogin";
const CACHED_LATEST_VERSION_KEY: &str = "cachedLatestVersion";
const CACHED_LATEST_RELEASE_URL_KEY: &str = "cachedLatestReleaseURL";
const LAST_UPDATE_CHECK_AT_KEY: &str = "lastUpdateCheckAt";

#[derive(Debug, Clone, Serialize)]
struct AppSettingsOutput {
    binary_path_override: Option<String>,
    database_path_override: Option<String>,
    default_recent_hours: u32,
    default_query_mode: String,
    hotkey_enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
struct AppLaunchAtLoginOutput {
    desired_enabled: Option<bool>,
    did_configure: bool,
    status: &'static str,
    requires_app_apply: bool,
    bridge: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct AppUpdateCheckOutput {
    current_version: &'static str,
    latest_version: Option<String>,
    release_url: Option<String>,
    last_checked_at_unix: Option<f64>,
    is_update_available: bool,
    install_command: &'static str,
}

pub(in crate::cli) fn app(db_path: &Path, args: &AppArgs) -> Result<()> {
    match &args.command {
        AppCommand::Settings(args) => app_settings(db_path, args),
        AppCommand::LaunchAtLogin(args) => app_launch_at_login(db_path, args),
        AppCommand::UpdateCheck(args) => app_update_check(db_path, args),
    }
}

fn app_settings(db_path: &Path, args: &AppSettingsArgs) -> Result<()> {
    match &args.command {
        AppSettingsCommand::Show(args) => app_settings_show(args),
        AppSettingsCommand::Set(args) => app_settings_set(db_path, args),
        AppSettingsCommand::Clear(args) => app_settings_clear(db_path, args),
    }
}

fn app_settings_show(args: &AppSettingsShowArgs) -> Result<()> {
    let format = require_app_settings_format(args.output.resolved()?)?;
    let output = load_app_settings()?;
    emit_json_or_text(
        format == OutputFormat::Json,
        &output,
        render_app_settings_text,
    )
}

fn app_settings_set(db_path: &Path, args: &AppSettingsSetArgs) -> Result<()> {
    let format = require_app_settings_format(args.output.resolved()?)?;
    let value = parse_app_preference_value(args.key, &args.value)?;
    set_preference(args.key, value)?;
    bump_app_preferences_revision(db_path)?;
    let output = load_app_settings()?;
    emit_json_or_text(
        format == OutputFormat::Json,
        &output,
        render_app_settings_text,
    )
}

fn app_settings_clear(db_path: &Path, args: &AppSettingsClearArgs) -> Result<()> {
    let format = require_app_settings_format(args.output.resolved()?)?;
    clear_preference(args.key)?;
    bump_app_preferences_revision(db_path)?;
    let output = load_app_settings()?;
    emit_json_or_text(
        format == OutputFormat::Json,
        &output,
        render_app_settings_text,
    )
}

fn app_launch_at_login(db_path: &Path, args: &AppLaunchAtLoginArgs) -> Result<()> {
    match &args.command {
        AppLaunchAtLoginCommand::Show(args) => app_launch_at_login_show(args),
        AppLaunchAtLoginCommand::Set(args) => app_launch_at_login_set(db_path, args),
        AppLaunchAtLoginCommand::Clear(args) => app_launch_at_login_clear(db_path, args),
    }
}

fn app_launch_at_login_show(args: &AppLaunchAtLoginShowArgs) -> Result<()> {
    let format = require_app_settings_format(args.output.resolved()?)?;
    let output = load_launch_at_login()?;
    emit_json_or_text(
        format == OutputFormat::Json,
        &output,
        render_launch_at_login_text,
    )
}

fn app_launch_at_login_set(db_path: &Path, args: &AppLaunchAtLoginSetArgs) -> Result<()> {
    let format = require_app_settings_format(args.output.resolved()?)?;
    let enabled = matches!(args.state, ToggleState::On);
    set_named_preference(LAUNCH_AT_LOGIN_ENABLED_KEY, Value::Bool(enabled))?;
    set_named_preference(DID_CONFIGURE_LAUNCH_AT_LOGIN_KEY, Value::Bool(true))?;
    bump_app_preferences_revision(db_path)?;
    let output = load_launch_at_login()?;
    emit_json_or_text(
        format == OutputFormat::Json,
        &output,
        render_launch_at_login_text,
    )
}

fn app_launch_at_login_clear(db_path: &Path, args: &AppLaunchAtLoginClearArgs) -> Result<()> {
    let format = require_app_settings_format(args.output.resolved()?)?;
    clear_named_preference(LAUNCH_AT_LOGIN_ENABLED_KEY)?;
    clear_named_preference(DID_CONFIGURE_LAUNCH_AT_LOGIN_KEY)?;
    bump_app_preferences_revision(db_path)?;
    let output = load_launch_at_login()?;
    emit_json_or_text(
        format == OutputFormat::Json,
        &output,
        render_launch_at_login_text,
    )
}

fn app_update_check(db_path: &Path, args: &AppUpdateCheckArgs) -> Result<()> {
    match &args.command {
        AppUpdateCheckCommand::Show(args) => app_update_check_show(args),
        AppUpdateCheckCommand::Clear(args) => app_update_check_clear(db_path, args),
    }
}

fn app_update_check_show(args: &AppUpdateCheckShowArgs) -> Result<()> {
    let format = require_app_settings_format(args.output.resolved()?)?;
    let output = load_update_check()?;
    emit_json_or_text(
        format == OutputFormat::Json,
        &output,
        render_update_check_text,
    )
}

fn app_update_check_clear(db_path: &Path, args: &AppUpdateCheckClearArgs) -> Result<()> {
    let format = require_app_settings_format(args.output.resolved()?)?;
    clear_named_preference(CACHED_LATEST_VERSION_KEY)?;
    clear_named_preference(CACHED_LATEST_RELEASE_URL_KEY)?;
    clear_named_preference(LAST_UPDATE_CHECK_AT_KEY)?;
    bump_app_preferences_revision(db_path)?;
    let output = load_update_check()?;
    emit_json_or_text(
        format == OutputFormat::Json,
        &output,
        render_update_check_text,
    )
}

fn require_app_settings_format(format: OutputFormat) -> Result<OutputFormat> {
    match format {
        OutputFormat::Text | OutputFormat::Json | OutputFormat::Human => Ok(format),
        other => Err(crate::cli::errors::UnsupportedFormatError::new(format!(
            "app settings only supports `text`, `json`, and `human` output, got `{}`",
            other.as_str()
        ))
        .into()),
    }
}

fn load_app_settings() -> Result<AppSettingsOutput> {
    Ok(AppSettingsOutput {
        binary_path_override: read_string(AppPreferenceKey::BinaryPathOverride)?,
        database_path_override: read_string(AppPreferenceKey::DatabasePathOverride)?,
        default_recent_hours: read_u32(AppPreferenceKey::DefaultRecentHours)?.unwrap_or(24),
        default_query_mode: read_string(AppPreferenceKey::DefaultQueryMode)?
            .unwrap_or_else(|| "recent".to_string()),
        hotkey_enabled: read_bool(AppPreferenceKey::HotkeyEnabled)?.unwrap_or(true),
    })
}

fn load_launch_at_login() -> Result<AppLaunchAtLoginOutput> {
    let desired_enabled =
        read_named_preference(LAUNCH_AT_LOGIN_ENABLED_KEY)?.and_then(|value| value.as_bool());
    let did_configure = read_named_preference(DID_CONFIGURE_LAUNCH_AT_LOGIN_KEY)?
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let status = match desired_enabled {
        Some(true) => "requested_enabled",
        Some(false) => "requested_disabled",
        None => "default",
    };
    Ok(AppLaunchAtLoginOutput {
        desired_enabled,
        did_configure,
        status,
        requires_app_apply: true,
        bridge: "The menu bar app applies this preference through SMAppService.",
    })
}

fn load_update_check() -> Result<AppUpdateCheckOutput> {
    let latest_version = read_named_preference(CACHED_LATEST_VERSION_KEY)?
        .and_then(|value| value.as_str().map(ToOwned::to_owned));
    let release_url = read_named_preference(CACHED_LATEST_RELEASE_URL_KEY)?
        .and_then(|value| value.as_str().map(ToOwned::to_owned));
    let last_checked_at_unix = read_named_preference(LAST_UPDATE_CHECK_AT_KEY)?.and_then(|value| {
        value
            .as_f64()
            .or_else(|| value.as_u64().map(|value| value as f64))
    });
    let is_update_available = latest_version
        .as_deref()
        .is_some_and(|latest| version_is_newer(env!("CARGO_PKG_VERSION"), latest));
    Ok(AppUpdateCheckOutput {
        current_version: env!("CARGO_PKG_VERSION"),
        latest_version,
        release_url,
        last_checked_at_unix,
        is_update_available,
        install_command: "brew update && brew upgrade tristanmanchester/tap/clipmem && brew upgrade --cask tristanmanchester/tap/clipmem-app",
    })
}

fn parse_app_preference_value(key: AppPreferenceKey, value: &str) -> Result<Value> {
    let trimmed = value.trim();
    match key {
        AppPreferenceKey::BinaryPathOverride | AppPreferenceKey::DatabasePathOverride => {
            if trimmed.is_empty() {
                bail!("{} cannot be empty; use `clear` to remove it", key.as_str());
            }
            Ok(Value::String(trimmed.to_string()))
        }
        AppPreferenceKey::DefaultRecentHours => {
            let hours = trimmed
                .parse::<u32>()
                .map_err(|_| anyhow!("default-recent-hours must be a positive integer"))?;
            if hours == 0 {
                bail!("default-recent-hours must be greater than zero");
            }
            Ok(Value::from(hours))
        }
        AppPreferenceKey::DefaultQueryMode => {
            if !matches!(
                trimmed,
                "recall" | "search" | "recent" | "timeline" | "diagnostics"
            ) {
                bail!(
                    "default-query-mode must be recall, search, recent, timeline, or diagnostics"
                );
            }
            Ok(Value::String(trimmed.to_string()))
        }
        AppPreferenceKey::HotkeyEnabled => match trimmed {
            "true" | "on" | "1" => Ok(Value::Bool(true)),
            "false" | "off" | "0" => Ok(Value::Bool(false)),
            _ => bail!("hotkey-enabled must be true/false, on/off, or 1/0"),
        },
    }
}

fn read_string(key: AppPreferenceKey) -> Result<Option<String>> {
    read_preference(key).map(|value| value.and_then(|value| value.as_str().map(ToOwned::to_owned)))
}

fn read_u32(key: AppPreferenceKey) -> Result<Option<u32>> {
    read_preference(key).map(|value| {
        value.and_then(|value| value.as_u64().and_then(|number| u32::try_from(number).ok()))
    })
}

fn read_bool(key: AppPreferenceKey) -> Result<Option<bool>> {
    read_preference(key).map(|value| value.and_then(|value| value.as_bool()))
}

fn read_preference(key: AppPreferenceKey) -> Result<Option<Value>> {
    read_named_preference(defaults_key(key))
}

fn read_named_preference(key: &str) -> Result<Option<Value>> {
    if let Some(path) = override_store_path() {
        let store = read_override_store(&path)?;
        return Ok(store.get(key).cloned());
    }

    let output = Command::new("defaults")
        .args(["read", APP_DOMAIN, key])
        .output()
        .context("read app preference with defaults")?;
    if !output.status.success() {
        return Ok(None);
    }
    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(Some(match key {
        "defaultRecentHours" => Value::from(raw.parse::<u64>().unwrap_or(24)),
        "hotkeyEnabled" | "launchAtLoginEnabled" | "didConfigureLaunchAtLogin" => {
            Value::Bool(matches!(raw.as_str(), "1" | "true" | "TRUE" | "YES"))
        }
        "lastUpdateCheckAt" => raw
            .parse::<f64>()
            .map(Value::from)
            .unwrap_or_else(|_| Value::String(raw)),
        _ => Value::String(raw),
    }))
}

fn set_preference(key: AppPreferenceKey, value: Value) -> Result<()> {
    set_named_preference(defaults_key(key), value)
}

fn set_named_preference(key: &str, value: Value) -> Result<()> {
    if let Some(path) = override_store_path() {
        let mut store = read_override_store(&path)?;
        store.insert(key.to_string(), value);
        return write_override_store(&path, &store);
    }

    let mut command = Command::new("defaults");
    command.args(["write", APP_DOMAIN, key]);
    match value {
        Value::String(value) => {
            command.arg(value);
        }
        Value::Number(value) => {
            command.args(["-int", &value.to_string()]);
        }
        Value::Bool(value) => {
            command.args(["-bool", if value { "true" } else { "false" }]);
        }
        _ => bail!("unsupported preference value"),
    }
    run_defaults(command, "write app preference")
}

fn clear_preference(key: AppPreferenceKey) -> Result<()> {
    clear_named_preference(defaults_key(key))
}

fn clear_named_preference(key: &str) -> Result<()> {
    if let Some(path) = override_store_path() {
        let mut store = read_override_store(&path)?;
        store.remove(key);
        return write_override_store(&path, &store);
    }

    let output = Command::new("defaults")
        .args(["delete", APP_DOMAIN, key])
        .output()
        .context("delete app preference with defaults")?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("does not exist") || stderr.contains("Domain") {
        return Ok(());
    }
    Err(anyhow!("delete app preference failed: {}", stderr.trim()))
}

fn run_defaults(mut command: Command, label: &str) -> Result<()> {
    let output = command.output().with_context(|| label.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow!(
            "{label} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn defaults_key(key: AppPreferenceKey) -> &'static str {
    match key {
        AppPreferenceKey::BinaryPathOverride => "binaryPathOverride",
        AppPreferenceKey::DatabasePathOverride => "databasePathOverride",
        AppPreferenceKey::DefaultRecentHours => "defaultRecentHours",
        AppPreferenceKey::DefaultQueryMode => "defaultQueryMode",
        AppPreferenceKey::HotkeyEnabled => "hotkeyEnabled",
    }
}

fn override_store_path() -> Option<PathBuf> {
    std::env::var_os("CLIPMEM_APP_SETTINGS_STORE").map(PathBuf::from)
}

fn read_override_store(path: &Path) -> Result<BTreeMap<String, Value>> {
    match std::fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).context("parse app settings override store"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
        Err(error) => Err(error).with_context(|| format!("read {}", path.display())),
    }
}

fn write_override_store(path: &Path, store: &BTreeMap<String, Value>) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    std::fs::write(path, serde_json::to_vec_pretty(store)?)
        .with_context(|| format!("write {}", path.display()))
}

fn bump_app_preferences_revision(db_path: &Path) -> Result<()> {
    Database::open_or_init(db_path)?
        .bump_app_preferences_revision()
        .map(|_| ())
        .context("record app preference revision")
}

fn render_app_settings_text(output: &AppSettingsOutput) -> String {
    format!(
        "binary_path_override={}\ndatabase_path_override={}\ndefault_recent_hours={}\ndefault_query_mode={}\nhotkey_enabled={}\n",
        output.binary_path_override.as_deref().unwrap_or(""),
        output.database_path_override.as_deref().unwrap_or(""),
        output.default_recent_hours,
        output.default_query_mode,
        output.hotkey_enabled
    )
}

fn render_launch_at_login_text(output: &AppLaunchAtLoginOutput) -> String {
    format!(
        "desired_enabled={}\ndid_configure={}\nstatus={}\nrequires_app_apply={}\nbridge={}\n",
        output
            .desired_enabled
            .map(|value| value.to_string())
            .unwrap_or_else(|| "default".to_string()),
        output.did_configure,
        output.status,
        output.requires_app_apply,
        output.bridge
    )
}

fn render_update_check_text(output: &AppUpdateCheckOutput) -> String {
    format!(
        "current_version={}\nlatest_version={}\nrelease_url={}\nlast_checked_at_unix={}\nis_update_available={}\ninstall_command={}\n",
        output.current_version,
        output.latest_version.as_deref().unwrap_or(""),
        output.release_url.as_deref().unwrap_or(""),
        output
            .last_checked_at_unix
            .map(|value| value.to_string())
            .unwrap_or_default(),
        output.is_update_available,
        output.install_command
    )
}

fn version_is_newer(current: &str, latest: &str) -> bool {
    let Some(current) = parse_stable_version(current) else {
        return false;
    };
    let Some(latest) = parse_stable_version(latest) else {
        return false;
    };
    latest > current
}

fn parse_stable_version(value: &str) -> Option<Vec<u64>> {
    let trimmed = value.trim().trim_start_matches(['v', 'V']);
    if trimmed.is_empty() || trimmed.contains('-') {
        return None;
    }
    let mut parts = Vec::new();
    for part in trimmed.split('.') {
        if part.is_empty() || !part.chars().all(|ch| ch.is_ascii_digit()) {
            return None;
        }
        parts.push(part.parse().ok()?);
    }
    Some(parts)
}

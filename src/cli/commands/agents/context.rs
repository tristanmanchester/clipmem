use std::path::Path;

use anyhow::Result;
use serde::Serialize;

use crate::cli::formats::OutputFormat;
use crate::cli::output::print_json;
use crate::cli::presentation::generated_at_now;
use crate::db::{ArchiveRevision, Database, RetrievalFilters, StatsReport};

#[cfg(target_os = "macos")]
use crate::cli::service::{status_report, ServiceStatusReport};

use super::super::app::{load_app_settings, load_launch_at_login, load_update_check};

#[derive(Debug, Serialize)]
struct AgentContextOutput {
    schema_version: u32,
    generated_at: String,
    clipmem_version: &'static str,
    db_path: String,
    db_exists: bool,
    service: AgentServiceSummary,
    settings: AgentSettingsSummary,
    app: AgentAppSummary,
    revision: Option<ArchiveRevision>,
    stats: Option<AgentStatsSummary>,
    recent_activity: Option<AgentRecentActivitySummary>,
    privacy: AgentPrivacySummary,
    capabilities: AgentCapabilitySummary,
}

#[derive(Debug, Serialize)]
struct AgentServiceSummary {
    health: String,
    preferred_provider: String,
    stale: bool,
    recent_capture_at: Option<String>,
    recent_capture_within_last_hour: Option<bool>,
    watcher_running: bool,
    watcher_binary_mismatch: bool,
}

#[derive(Debug, Serialize)]
struct AgentSettingsSummary {
    paused: Option<bool>,
    api_key_filter_enabled: Option<bool>,
    retention_seconds: Option<u64>,
    retention: Option<String>,
    ignored_bundle_id_count: Option<usize>,
}

#[derive(Debug, Serialize)]
struct AgentStatsSummary {
    snapshot_count: usize,
    capture_event_count: usize,
    unique_app_count: usize,
    total_bytes: usize,
    last_observed_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct AgentAppSummary {
    settings: AgentAppSettingsSummary,
    launch_at_login: AgentLaunchAtLoginSummary,
    update_check: AgentUpdateCheckSummary,
}

#[derive(Debug, Serialize)]
struct AgentAppSettingsSummary {
    binary_path_override: Option<String>,
    database_path_override: Option<String>,
    default_recent_hours: u32,
    default_query_mode: String,
    hotkey_enabled: bool,
}

#[derive(Debug, Serialize)]
struct AgentLaunchAtLoginSummary {
    desired_enabled: Option<bool>,
    did_configure: bool,
    status: &'static str,
    requires_app_apply: bool,
}

#[derive(Debug, Serialize)]
struct AgentUpdateCheckSummary {
    current_version: &'static str,
    latest_version: Option<String>,
    release_url: Option<String>,
    last_checked_at_unix: Option<f64>,
    is_update_available: bool,
}

#[derive(Debug, Serialize)]
struct AgentRecentActivitySummary {
    last_24h: AgentActivityWindowSummary,
    last_7d: AgentActivityWindowSummary,
    has_ocr_text: bool,
    has_images: bool,
    has_pdfs: bool,
}

#[derive(Debug, Serialize)]
struct AgentActivityWindowSummary {
    hours: u32,
    snapshot_count: usize,
    capture_event_count: usize,
    unique_app_count: usize,
    latest_observed_at: Option<String>,
    top_apps: Vec<AgentTopAppSummary>,
    kind_breakdown: Vec<AgentKindSummary>,
}

#[derive(Debug, Serialize)]
struct AgentTopAppSummary {
    app: String,
    capture_event_count: usize,
}

#[derive(Debug, Serialize)]
struct AgentKindSummary {
    kind: String,
    snapshot_count: usize,
}

#[derive(Debug, Serialize)]
struct AgentPrivacySummary {
    includes_raw_clipboard_content: bool,
    includes_metadata: &'static [&'static str],
    content_retrieval_commands: &'static [&'static str],
    note: &'static str,
}

#[derive(Debug, Serialize)]
struct AgentCapabilitySummary {
    primary_retrieval: &'static [&'static str],
    primitive_reads: &'static [&'static str],
    mutation: &'static [&'static str],
    maintenance: &'static [&'static str],
    app_state: &'static [&'static str],
    service: &'static [&'static str],
    agent_integration: &'static [&'static str],
    destructive_commands: &'static [&'static str],
    stable_formats: &'static [&'static str],
    action_parity_doc: &'static str,
}

struct AgentContextStatus {
    db_exists: bool,
    preferred_provider: String,
    stale: bool,
    recent_capture_at: Option<String>,
    recent_capture_within_last_hour: Option<bool>,
    watcher_running: bool,
    watcher_binary_mismatch: bool,
    paused: Option<bool>,
    api_key_filter_enabled: Option<bool>,
    retention_seconds: Option<u64>,
    retention: Option<String>,
    ignored_bundle_id_count: Option<usize>,
    health: &'static str,
}

pub(in crate::cli) fn agent_context(db_path: &Path, format: OutputFormat) -> Result<()> {
    let format = require_agent_context_output_format(format)?;
    let context = build_agent_context(db_path)?;
    match format {
        OutputFormat::Json => print_json(&context),
        OutputFormat::Text | OutputFormat::Human | OutputFormat::Md => {
            print!("{}", render_agent_context_text(&context));
            Ok(())
        }
        OutputFormat::Jsonl | OutputFormat::Toon => {
            unreachable!("unsupported agents context format should be rejected earlier")
        }
    }
}

fn require_agent_context_output_format(format: OutputFormat) -> Result<OutputFormat> {
    match format {
        OutputFormat::Text | OutputFormat::Json | OutputFormat::Md | OutputFormat::Human => {
            Ok(format)
        }
        OutputFormat::Jsonl | OutputFormat::Toon => {
            Err(crate::cli::errors::UnsupportedFormatError::new(
                "agents context only supports `text`, `json`, `md`, and `human` output",
            )
            .into())
        }
    }
}

fn build_agent_context(db_path: &Path) -> Result<AgentContextOutput> {
    let status = load_agent_context_status(db_path)?;
    let generated_at = generated_at_now()?;
    let db_snapshot = if db_path.is_file() {
        let db = Database::open_existing(db_path)?;
        Some((
            db.archive_revision()?,
            db.stats(&RetrievalFilters::default())?,
            build_recent_activity(&db)?,
        ))
    } else {
        None
    };

    let (revision, stats, recent_activity) = db_snapshot
        .map(|(revision, stats, recent_activity)| {
            (
                Some(revision),
                Some(AgentStatsSummary {
                    snapshot_count: stats.snapshot_count(),
                    capture_event_count: stats.capture_event_count(),
                    unique_app_count: stats.unique_app_count(),
                    total_bytes: stats.total_bytes(),
                    last_observed_at: stats.last_observed_at().map(ToOwned::to_owned),
                }),
                Some(recent_activity),
            )
        })
        .unwrap_or((None, None, None));

    let app_settings = load_app_settings()?;
    let launch_at_login = load_launch_at_login()?;
    let update_check = load_update_check()?;

    Ok(AgentContextOutput {
        schema_version: 1,
        generated_at,
        clipmem_version: env!("CARGO_PKG_VERSION"),
        db_path: db_path.display().to_string(),
        db_exists: status.db_exists,
        service: AgentServiceSummary {
            health: status.health.to_string(),
            preferred_provider: status.preferred_provider,
            stale: status.stale,
            recent_capture_at: status.recent_capture_at,
            recent_capture_within_last_hour: status.recent_capture_within_last_hour,
            watcher_running: status.watcher_running,
            watcher_binary_mismatch: status.watcher_binary_mismatch,
        },
        settings: AgentSettingsSummary {
            paused: status.paused,
            api_key_filter_enabled: status.api_key_filter_enabled,
            retention_seconds: status.retention_seconds,
            retention: status.retention,
            ignored_bundle_id_count: status.ignored_bundle_id_count,
        },
        app: AgentAppSummary {
            settings: AgentAppSettingsSummary {
                binary_path_override: nonempty_option(app_settings.binary_path_override),
                database_path_override: nonempty_option(app_settings.database_path_override),
                default_recent_hours: app_settings.default_recent_hours,
                default_query_mode: app_settings.default_query_mode,
                hotkey_enabled: app_settings.hotkey_enabled,
            },
            launch_at_login: AgentLaunchAtLoginSummary {
                desired_enabled: launch_at_login.desired_enabled,
                did_configure: launch_at_login.did_configure,
                status: launch_at_login.status,
                requires_app_apply: launch_at_login.requires_app_apply,
            },
            update_check: AgentUpdateCheckSummary {
                current_version: update_check.current_version,
                latest_version: update_check.latest_version,
                release_url: update_check.release_url,
                last_checked_at_unix: update_check.last_checked_at_unix,
                is_update_available: update_check.is_update_available,
            },
        },
        revision,
        stats,
        recent_activity,
        privacy: AgentPrivacySummary {
            includes_raw_clipboard_content: false,
            includes_metadata: &[
                "database path",
                "service health",
                "capture settings",
                "revision counters",
                "archive statistics",
                "recent activity counts",
                "top app names",
                "content kind counts",
                "menu bar app preferences",
                "update-check cache",
            ],
            content_retrieval_commands: &[
                "recall", "search", "recent", "timeline", "get", "export",
            ],
            note: "Context excludes raw clipboard content but includes operational metadata such as app names, timestamps, counts, paths, and app preference state. Run retrieval commands explicitly for clipboard content.",
        },
        capabilities: AgentCapabilitySummary {
            primary_retrieval: &["recall", "timeline", "recent", "search", "get"],
            primitive_reads: &[
                "agents context",
                "service status",
                "service providers",
                "settings show",
                "settings ignore list",
                "stats",
                "storage image-candidates",
                "ocr status",
                "ocr candidates",
                "ocr get",
                "app settings show",
                "app launch-at-login show",
                "app update-check show",
            ],
            mutation: &[
                "restore",
                "export",
                "forget",
                "purge",
                "settings pause",
                "settings api-key-filter",
                "settings ocr",
                "settings retention",
                "settings reset",
                "settings ignore add",
                "settings ignore remove",
                "ocr clear",
            ],
            maintenance: &[
                "doctor",
                "capture-once",
                "watch",
                "storage compact",
                "storage image-candidates",
                "storage optimize-images",
                "ocr run",
            ],
            app_state: &[
                "app settings show",
                "app settings set",
                "app settings clear",
                "app launch-at-login show",
                "app launch-at-login set",
                "app launch-at-login clear",
                "app update-check show",
                "app update-check run",
                "app update-check clear",
                "app quit",
            ],
            service: &[
                "setup",
                "service providers",
                "service status",
                "service start",
                "service stop",
                "service uninstall",
            ],
            agent_integration: &[
                "agents context",
                "agents openclaw doctor",
                "agents openclaw install-skill",
                "agents openclaw print-skill",
                "agents openclaw uninstall-skill",
                "agents hermes doctor",
                "agents hermes install-skill",
                "agents hermes print-skill",
                "agents hermes uninstall-skill",
            ],
            destructive_commands: &[
                "forget",
                "purge --dry-run first",
                "settings reset",
                "ocr clear",
                "app settings clear",
                "app update-check clear",
            ],
            stable_formats: &["json", "text", "md", "human"],
            action_parity_doc: "docs/action-parity.md",
        },
    })
}

#[cfg(target_os = "macos")]
fn load_agent_context_status(db_path: &Path) -> Result<AgentContextStatus> {
    let status = status_report(db_path)?;
    let health = service_health_label(&status);
    Ok(AgentContextStatus {
        db_exists: status.db_exists,
        preferred_provider: status.preferred_provider,
        stale: status.stale,
        recent_capture_at: status.recent_capture_at,
        recent_capture_within_last_hour: status.recent_capture_within_last_hour,
        watcher_running: status.homebrew.running || status.launchagent.running,
        watcher_binary_mismatch: status.watcher_binary_mismatch,
        paused: status.paused,
        api_key_filter_enabled: status.api_key_filter_enabled,
        retention_seconds: status.retention_seconds,
        retention: status.retention,
        ignored_bundle_id_count: status.ignored_bundle_id_count,
        health,
    })
}

#[cfg(not(target_os = "macos"))]
fn load_agent_context_status(db_path: &Path) -> Result<AgentContextStatus> {
    let db_exists = db_path.is_file();
    if !db_exists {
        return Ok(AgentContextStatus {
            db_exists,
            preferred_provider: "unsupported".to_string(),
            stale: false,
            recent_capture_at: None,
            recent_capture_within_last_hour: None,
            watcher_running: false,
            watcher_binary_mismatch: false,
            paused: None,
            api_key_filter_enabled: None,
            retention_seconds: None,
            retention: None,
            ignored_bundle_id_count: None,
            health: "setup_needed",
        });
    }

    let db = Database::open_existing(db_path)?;
    let policy = db.capture_policy()?;
    let settings = policy.settings();
    let paused = settings.paused();
    let recent_capture_within_last_hour = db.has_capture_within_hours(1)?;
    let health = if paused {
        "capture_paused"
    } else {
        "service_unsupported"
    };

    Ok(AgentContextStatus {
        db_exists,
        preferred_provider: "unsupported".to_string(),
        stale: false,
        recent_capture_at: db.latest_capture_observed_at()?,
        recent_capture_within_last_hour: Some(recent_capture_within_last_hour),
        watcher_running: false,
        watcher_binary_mismatch: false,
        paused: Some(paused),
        api_key_filter_enabled: Some(settings.api_key_filter_enabled()),
        retention_seconds: settings.retention_seconds(),
        retention: Some(
            settings
                .retention_seconds()
                .map_or_else(|| "forever".to_string(), |seconds| format!("{seconds}s")),
        ),
        ignored_bundle_id_count: Some(policy.ignored_bundle_id_count()),
        health,
    })
}

fn nonempty_option(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(value)
        }
    })
}

fn build_recent_activity(db: &Database) -> Result<AgentRecentActivitySummary> {
    let last_24h = activity_window(db, 24)?;
    let last_7d = activity_window(db, 24 * 7)?;
    let image_stats = db.stats(&RetrievalFilters::default().requiring_image())?;
    let pdf_stats = db.stats(&RetrievalFilters::default().requiring_pdf())?;
    let ocr_status = db.ocr_status_report()?;
    Ok(AgentRecentActivitySummary {
        last_24h,
        last_7d,
        has_ocr_text: ocr_status.snapshots_with_ocr_text() > 0,
        has_images: image_stats.snapshot_count() > 0,
        has_pdfs: pdf_stats.snapshot_count() > 0,
    })
}

fn activity_window(db: &Database, hours: u32) -> Result<AgentActivityWindowSummary> {
    let stats = db.stats(&RetrievalFilters::default().with_hours(Some(hours)))?;
    Ok(AgentActivityWindowSummary {
        hours,
        snapshot_count: stats.snapshot_count(),
        capture_event_count: stats.capture_event_count(),
        unique_app_count: stats.unique_app_count(),
        latest_observed_at: stats.last_observed_at().map(ToOwned::to_owned),
        top_apps: top_apps(&stats),
        kind_breakdown: kind_breakdown(&stats),
    })
}

fn top_apps(stats: &StatsReport) -> Vec<AgentTopAppSummary> {
    stats
        .top_apps()
        .iter()
        .take(5)
        .map(|entry| AgentTopAppSummary {
            app: entry.app().to_string(),
            capture_event_count: entry.capture_event_count(),
        })
        .collect()
}

fn kind_breakdown(stats: &StatsReport) -> Vec<AgentKindSummary> {
    stats
        .kind_breakdown()
        .iter()
        .map(|entry| AgentKindSummary {
            kind: entry.kind().as_str().to_string(),
            snapshot_count: entry.snapshot_count(),
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn service_health_label(status: &ServiceStatusReport) -> &'static str {
    if status.conflict {
        "conflict"
    } else if status.db_error.is_some() {
        "error"
    } else if !status.db_exists {
        "setup_needed"
    } else if status.paused == Some(true) {
        "capture_paused"
    } else if status.stale {
        "stale"
    } else if status.homebrew.running || status.launchagent.running {
        if status.recent_capture_within_last_hour == Some(false) {
            "no_recent_captures"
        } else {
            "healthy"
        }
    } else if status.homebrew.installed
        || status.homebrew.loaded
        || status.launchagent.installed
        || status.launchagent.loaded
    {
        "watcher_stopped"
    } else {
        "setup_needed"
    }
}

fn render_agent_context_text(context: &AgentContextOutput) -> String {
    let mut out = String::new();
    out.push_str("clipmem agent context\n");
    out.push_str(&format!("generated_at: {}\n", context.generated_at));
    out.push_str(&format!("clipmem_version: {}\n", context.clipmem_version));
    out.push_str(&format!("db_path: {}\n", context.db_path));
    out.push_str(&format!("db_exists: {}\n", context.db_exists));
    out.push_str(&format!("health: {}\n", context.service.health));
    out.push_str(&format!(
        "watcher_running: {}\n",
        context.service.watcher_running
    ));
    out.push_str(&format!("stale: {}\n", context.service.stale));
    if let Some(revision) = &context.revision {
        out.push_str(&format!("revision: {}\n", revision.revision()));
        out.push_str(&format!(
            "last_change_kind: {}\n",
            revision.last_change_kind()
        ));
    }
    if let Some(stats) = &context.stats {
        out.push_str(&format!("snapshots: {}\n", stats.snapshot_count));
        out.push_str(&format!("capture_events: {}\n", stats.capture_event_count));
    }
    out.push_str(&format!(
        "app_default_query_mode: {}\n",
        context.app.settings.default_query_mode
    ));
    out.push_str(&format!("privacy: {}\n", context.privacy.note));
    out.push_str(&format!(
        "retrieval: {}\n",
        context.capabilities.primary_retrieval.join(", ")
    ));
    out.push_str(&format!(
        "primitive_reads: {}\n",
        context.capabilities.primitive_reads.join(", ")
    ));
    out.push_str(&format!(
        "mutations: {}\n",
        context.capabilities.mutation.join(", ")
    ));
    out.push_str(&format!(
        "maintenance: {}\n",
        context.capabilities.maintenance.join(", ")
    ));
    out.push_str(&format!(
        "app_state: {}\n",
        context.capabilities.app_state.join(", ")
    ));
    out.push_str(&format!(
        "service: {}\n",
        context.capabilities.service.join(", ")
    ));
    out.push_str(&format!(
        "agent_integration: {}\n",
        context.capabilities.agent_integration.join(", ")
    ));
    out.push_str(&format!(
        "destructive_commands: {}\n",
        context.capabilities.destructive_commands.join(", ")
    ));
    out.push_str(&format!(
        "stable_formats: {}\n",
        context.capabilities.stable_formats.join(", ")
    ));
    out.push_str(&format!(
        "action_parity_doc: {}\n",
        context.capabilities.action_parity_doc
    ));
    out
}

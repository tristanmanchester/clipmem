use std::path::Path;

use anyhow::Result;
use serde::Serialize;

use crate::cli::formats::OutputFormat;
use crate::cli::output::print_json;
use crate::cli::service::{status_report, ServiceStatusReport};
use crate::db::{ArchiveRevision, Database, RetrievalFilters};

#[derive(Debug, Serialize)]
struct AgentContextOutput {
    schema_version: u32,
    clipmem_version: &'static str,
    db_path: String,
    db_exists: bool,
    service: AgentServiceSummary,
    settings: AgentSettingsSummary,
    revision: Option<ArchiveRevision>,
    stats: Option<AgentStatsSummary>,
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
struct AgentCapabilitySummary {
    primary_retrieval: &'static [&'static str],
    mutation: &'static [&'static str],
    maintenance: &'static [&'static str],
    stable_formats: &'static [&'static str],
    action_parity_doc: &'static str,
}

pub(in crate::cli) fn agent_context(db_path: &Path, format: OutputFormat) -> Result<()> {
    let context = build_agent_context(db_path)?;
    match format {
        OutputFormat::Json => print_json(&context),
        OutputFormat::Text | OutputFormat::Human | OutputFormat::Md => {
            print!("{}", render_agent_context_text(&context));
            Ok(())
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
    let status = status_report(db_path)?;
    let db_snapshot = if db_path.is_file() {
        let db = Database::open_existing(db_path)?;
        Some((
            db.archive_revision()?,
            db.stats(&RetrievalFilters::default())?,
        ))
    } else {
        None
    };

    let (revision, stats) = db_snapshot
        .map(|(revision, stats)| {
            (
                Some(revision),
                Some(AgentStatsSummary {
                    snapshot_count: stats.snapshot_count(),
                    capture_event_count: stats.capture_event_count(),
                    unique_app_count: stats.unique_app_count(),
                    total_bytes: stats.total_bytes(),
                    last_observed_at: stats.last_observed_at().map(ToOwned::to_owned),
                }),
            )
        })
        .unwrap_or((None, None));

    Ok(AgentContextOutput {
        schema_version: 1,
        clipmem_version: env!("CARGO_PKG_VERSION"),
        db_path: db_path.display().to_string(),
        db_exists: status.db_exists,
        service: AgentServiceSummary {
            health: service_health_label(&status).to_string(),
            preferred_provider: status.preferred_provider,
            stale: status.stale,
            recent_capture_at: status.recent_capture_at,
            recent_capture_within_last_hour: status.recent_capture_within_last_hour,
            watcher_running: status.homebrew.running || status.launchagent.running,
            watcher_binary_mismatch: status.watcher_binary_mismatch,
        },
        settings: AgentSettingsSummary {
            paused: status.paused,
            api_key_filter_enabled: status.api_key_filter_enabled,
            retention_seconds: status.retention_seconds,
            retention: status.retention,
            ignored_bundle_id_count: status.ignored_bundle_id_count,
        },
        revision,
        stats,
        capabilities: AgentCapabilitySummary {
            primary_retrieval: &["recall", "timeline", "recent", "search", "get"],
            mutation: &["restore", "export", "forget", "purge", "settings"],
            maintenance: &[
                "doctor",
                "service status",
                "storage compact",
                "storage optimize-images",
                "ocr run",
            ],
            stable_formats: &["json", "jsonl", "toon"],
            action_parity_doc: "docs/action-parity.md",
        },
    })
}

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
    out.push_str(&format!("version: {}\n", context.clipmem_version));
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
    out.push_str("retrieval: recall, timeline, recent, search, get\n");
    out.push_str("mutations: restore, export, forget, purge, settings\n");
    out.push_str("action_parity_doc: docs/action-parity.md\n");
    out
}

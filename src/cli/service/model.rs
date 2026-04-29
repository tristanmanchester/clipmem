use std::path::PathBuf;

use serde::Serialize;

use crate::db::{ArchiveRevision, CaptureSkipReason};

pub(in crate::cli) const DIRECT_LABEL: &str = "io.openclaw.clipmem.watch";
pub(in crate::cli) const HOMEBREW_LABEL: &str = "homebrew.mxcl.clipmem";
pub(in crate::cli) const DEFAULT_INTERVAL_MS: u64 = 350;
pub(in crate::cli) const SERVICE_FRESHNESS_HOURS: u32 = 1;
pub(in crate::cli) const DIRECT_PLIST_TEMPLATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/extras/launchd/io.openclaw.clipmem.watch.plist.template"
));

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::cli) enum ServiceProvider {
    Homebrew,
    Launchagent,
}

impl ServiceProvider {
    pub(in crate::cli) const fn as_str(self) -> &'static str {
        match self {
            Self::Homebrew => "homebrew",
            Self::Launchagent => "launchagent",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::cli) enum ServiceState {
    NotInstalled,
    Installed,
    Loaded,
    Running,
}

#[derive(Debug, Clone, Serialize)]
pub(in crate::cli) struct ServiceProviderStatus {
    pub(in crate::cli) provider: ServiceProvider,
    pub(in crate::cli) label: String,
    pub(in crate::cli) state: ServiceState,
    pub(in crate::cli) installed: bool,
    pub(in crate::cli) loaded: bool,
    pub(in crate::cli) running: bool,
    pub(in crate::cli) pid: Option<i64>,
    pub(in crate::cli) plist_path: Option<String>,
    pub(in crate::cli) configured_binary_path: Option<String>,
    pub(in crate::cli) running_command: Option<String>,
    pub(in crate::cli) running_binary_path: Option<String>,
    pub(in crate::cli) stdout_log_path: Option<String>,
    pub(in crate::cli) stderr_log_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(in crate::cli) struct ServiceStatusReport {
    pub(in crate::cli) binary_path: String,
    pub(in crate::cli) db_path: String,
    pub(in crate::cli) preferred_provider: String,
    pub(in crate::cli) preferred_provider_reason: String,
    pub(in crate::cli) conflict: bool,
    pub(in crate::cli) homebrew: ServiceProviderStatus,
    pub(in crate::cli) launchagent: ServiceProviderStatus,
    pub(in crate::cli) db_exists: bool,
    pub(in crate::cli) db_size_bytes: Option<u64>,
    pub(in crate::cli) recent_capture_at: Option<String>,
    pub(in crate::cli) recent_capture_within_last_hour: Option<bool>,
    pub(in crate::cli) paused: Option<bool>,
    pub(in crate::cli) api_key_filter_enabled: Option<bool>,
    pub(in crate::cli) retention_seconds: Option<u64>,
    pub(in crate::cli) retention: Option<String>,
    pub(in crate::cli) ignored_bundle_id_count: Option<usize>,
    pub(in crate::cli) revision: Option<ArchiveRevision>,
    pub(in crate::cli) stale: bool,
    pub(in crate::cli) db_error: Option<String>,
    pub(in crate::cli) watcher_binary_mismatch: bool,
    pub(in crate::cli) watcher_binary_mismatch_note: Option<String>,
    pub(in crate::cli) notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub(in crate::cli) struct ServiceActionReport {
    pub(in crate::cli) action: &'static str,
    pub(in crate::cli) provider: ServiceProvider,
    pub(in crate::cli) binary_path: PathBuf,
    pub(in crate::cli) db_path: PathBuf,
    pub(in crate::cli) label: &'static str,
    pub(in crate::cli) notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub(in crate::cli) struct SetupReport {
    pub(in crate::cli) seed_capture: SeedCaptureOutcome,
    pub(in crate::cli) action: ServiceActionReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::cli) enum SeedCaptureOutcome {
    Stored,
    Skipped(CaptureSkipReason),
    NotAttempted,
}

#[derive(Debug, Clone)]
pub(in crate::cli) struct ServiceContext {
    pub(in crate::cli) binary_path: PathBuf,
    pub(in crate::cli) db_path: PathBuf,
    pub(in crate::cli) default_db_path: PathBuf,
    pub(in crate::cli) direct_plist_path: PathBuf,
    pub(in crate::cli) homebrew_plist_path: PathBuf,
    pub(in crate::cli) direct_stdout_path: PathBuf,
    pub(in crate::cli) direct_stderr_path: PathBuf,
    pub(in crate::cli) brew_path: Option<PathBuf>,
    pub(in crate::cli) homebrew_prefix: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub(in crate::cli) struct ProviderSelection {
    pub(in crate::cli) provider: ServiceProvider,
    pub(in crate::cli) reason: String,
    pub(in crate::cli) notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub(in crate::cli) struct LaunchctlRow {
    pub(in crate::cli) pid: Option<i64>,
}

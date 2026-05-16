use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::db::{RetrievalFilters, RetrievalKind, SearchMode, TimelineSort};

use super::formats::{OutputArgs, ProgressFormat, RecallOutputArgs, StatsOutputArgs, ToggleState};
use super::help::{
    AGENTS_CONTEXT_AFTER_HELP, APP_AFTER_HELP, APP_SETTINGS_AFTER_HELP, CAPTURE_ONCE_AFTER_HELP,
    DOCTOR_AFTER_HELP, EXPORT_AFTER_HELP, FORGET_AFTER_HELP, GET_AFTER_HELP,
    HERMES_DOCTOR_AFTER_HELP, HERMES_INSTALL_AFTER_HELP, HERMES_PRINT_AFTER_HELP,
    HERMES_UNINSTALL_AFTER_HELP, OCR_AFTER_HELP, OPENCLAW_DOCTOR_AFTER_HELP,
    OPENCLAW_INSTALL_AFTER_HELP, OPENCLAW_PRINT_AFTER_HELP, OPENCLAW_UNINSTALL_AFTER_HELP,
    PURGE_AFTER_HELP, RECALL_AFTER_HELP, RECENT_AFTER_HELP, RESTORE_AFTER_HELP, ROOT_AFTER_HELP,
    SEARCH_AFTER_HELP, SERVICE_AFTER_HELP, SERVICE_REVISION_AFTER_HELP, SERVICE_STATUS_AFTER_HELP,
    SETTINGS_AFTER_HELP, SETUP_AFTER_HELP, STATS_AFTER_HELP, STORAGE_AFTER_HELP,
    TIMELINE_AFTER_HELP, WATCH_AFTER_HELP,
};
use super::parsing::{
    parse_bounded_limit, parse_bundle_id, parse_duration_value, parse_item_index,
    parse_nonnegative_bytes, parse_normalized_score, parse_preferred_app, parse_representation_uti,
    parse_retention_value, parse_retrieval_kind, parse_rfc3339_timestamp, parse_search_mode,
    parse_sha256_hash, parse_timeline_sort, DurationValue, RetentionValue,
};
use super::value_validation::{
    normalize_nonempty_filter_value, validate_byte_window, validate_positive_hours,
    validate_time_window,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AppPreferenceKey {
    BinaryPathOverride,
    DatabasePathOverride,
    DefaultRecentHours,
    DefaultQueryMode,
    HotkeyEnabled,
}

impl AppPreferenceKey {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::BinaryPathOverride => "binary-path-override",
            Self::DatabasePathOverride => "database-path-override",
            Self::DefaultRecentHours => "default-recent-hours",
            Self::DefaultQueryMode => "default-query-mode",
            Self::HotkeyEnabled => "hotkey-enabled",
        }
    }
}

fn parse_app_preference_key(value: &str) -> Result<AppPreferenceKey, String> {
    match value {
        "binary-path-override" => Ok(AppPreferenceKey::BinaryPathOverride),
        "database-path-override" => Ok(AppPreferenceKey::DatabasePathOverride),
        "default-recent-hours" => Ok(AppPreferenceKey::DefaultRecentHours),
        "default-query-mode" => Ok(AppPreferenceKey::DefaultQueryMode),
        "hotkey-enabled" => Ok(AppPreferenceKey::HotkeyEnabled),
        _ => Err(format!(
            "unsupported app preference key `{value}`; expected binary-path-override, database-path-override, default-recent-hours, default-query-mode, or hotkey-enabled"
        )),
    }
}

#[derive(Debug, Parser)]
#[command(name = "clipmem")]
#[command(version)]
#[command(about = "macOS clipboard memory backed by SQLite")]
#[command(after_help = ROOT_AFTER_HELP)]
#[command(next_line_help = true)]
pub(super) struct Cli {
    /// Path to the `SQLite` database.
    #[arg(long, global = true)]
    pub(super) db: Option<PathBuf>,

    #[command(subcommand)]
    pub(super) command: Command,
}

#[derive(Debug, Subcommand)]
pub(super) enum Command {
    /// Manage agent-harness integrations.
    Agents(AgentsArgs),
    /// Initialize the database, seed one capture, and start background capture.
    Setup(SetupArgs),
    /// Manage the background clipmem watcher service.
    Service(ServiceArgs),
    /// View and update menu bar app preferences.
    App(AppArgs),
    /// Continuously poll the clipboard and archive observed changes.
    Watch(WatchArgs),
    /// Capture the current clipboard state once.
    CaptureOnce(CaptureOnceArgs),
    /// Search the clipboard archive.
    Search(SearchArgs),
    /// Show recent unique clipboard states (deduplicated by snapshot).
    Recent(RecentArgs),
    /// Show chronological clipboard capture events (one row per observation).
    Timeline(TimelineArgs),
    /// Report archive aggregates and leaderboards.
    Stats(StatsArgs),
    /// Recall the most likely clipboard item for an agent query.
    Recall(RecallArgs),
    /// Show a stored snapshot in detail.
    Get(GetArgs),
    /// Export one stored representation as raw bytes.
    Export(ExportArgs),
    /// Restore a stored snapshot back onto the clipboard.
    Restore(RestoreArgs),
    /// Irreversibly delete one stored snapshot and its capture history.
    Forget(ForgetArgs),
    /// Delete stored snapshots older than a duration.
    Purge(PurgeArgs),
    /// Compact database storage and optimize archived images.
    Storage(StorageArgs),
    /// Backfill and inspect local image OCR.
    Ocr(OcrArgs),
    /// View and update persistent capture policy.
    Settings(SettingsArgs),
    /// Print `SQLite` and FTS5 diagnostics.
    Doctor(DoctorArgs),
}

#[derive(Debug, Args)]
#[command(after_help = WATCH_AFTER_HELP)]
pub(super) struct WatchArgs {
    /// Poll interval in milliseconds.
    #[arg(long, default_value_t = 400)]
    pub(super) interval_ms: u64,

    /// Do not print one-line status messages for each capture.
    #[arg(long, default_value_t = false)]
    pub(super) quiet: bool,

    /// Skip capturing the clipboard state that already exists when the watcher starts.
    #[arg(long, default_value_t = false)]
    pub(super) skip_initial: bool,
}

#[derive(Debug, Args)]
#[command(after_help = SETUP_AFTER_HELP)]
pub(super) struct SetupArgs {}

#[derive(Debug, Args)]
#[command(after_help = SERVICE_AFTER_HELP)]
pub(super) struct ServiceArgs {
    #[command(subcommand)]
    pub(super) command: ServiceCommand,
}

#[derive(Debug, Subcommand)]
pub(super) enum ServiceCommand {
    /// List service providers and their current state without changing them.
    Providers(ServiceProvidersArgs),
    /// Print the current archive revision without probing service providers.
    Revision(ServiceRevisionArgs),
    /// Start background capture using the preferred service provider.
    Start,
    /// Stop background capture without uninstalling the service definition when possible.
    Stop,
    /// Report provider state, freshness, and service wiring.
    Status(ServiceStatusArgs),
    /// Remove the managed service definition.
    Uninstall,
}

#[derive(Debug, Args)]
pub(super) struct ServiceProvidersArgs {
    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
#[command(after_help = SERVICE_REVISION_AFTER_HELP)]
pub(super) struct ServiceRevisionArgs {
    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
#[command(after_help = SERVICE_STATUS_AFTER_HELP)]
pub(super) struct ServiceStatusArgs {
    /// Emit service status as JSON.
    #[arg(long, default_value_t = false)]
    pub(super) json: bool,

    /// Emit service status as polished terminal output.
    #[arg(long, default_value_t = false)]
    pub(super) human: bool,
}

#[derive(Debug, Args)]
#[command(after_help = APP_AFTER_HELP)]
pub(super) struct AppArgs {
    #[command(subcommand)]
    pub(super) command: AppCommand,
}

#[derive(Debug, Subcommand)]
pub(super) enum AppCommand {
    /// View and update menu bar app preferences.
    Settings(AppSettingsArgs),
    /// View and update the app-owned launch-at-login preference bridge.
    LaunchAtLogin(AppLaunchAtLoginArgs),
    /// View or clear cached menu bar app update-check state.
    UpdateCheck(AppUpdateCheckArgs),
    /// Request that the menu bar app quit.
    Quit(AppQuitArgs),
}

#[derive(Debug, Args)]
#[command(after_help = APP_SETTINGS_AFTER_HELP)]
pub(super) struct AppSettingsArgs {
    #[command(subcommand)]
    pub(super) command: AppSettingsCommand,
}

#[derive(Debug, Subcommand)]
pub(super) enum AppSettingsCommand {
    /// Show menu bar app preferences.
    Show(AppSettingsShowArgs),
    /// Set one menu bar app preference.
    Set(AppSettingsSetArgs),
    /// Clear one menu bar app preference.
    Clear(AppSettingsClearArgs),
}

#[derive(Debug, Args)]
pub(super) struct AppSettingsShowArgs {
    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct AppSettingsSetArgs {
    /// Preference key to set.
    #[arg(value_parser = parse_app_preference_key)]
    pub(super) key: AppPreferenceKey,

    /// Preference value.
    pub(super) value: String,

    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct AppSettingsClearArgs {
    /// Preference key to clear.
    #[arg(value_parser = parse_app_preference_key)]
    pub(super) key: AppPreferenceKey,

    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct AppLaunchAtLoginArgs {
    #[command(subcommand)]
    pub(super) command: AppLaunchAtLoginCommand,
}

#[derive(Debug, Subcommand)]
pub(super) enum AppLaunchAtLoginCommand {
    /// Show requested launch-at-login state.
    Show(AppLaunchAtLoginShowArgs),
    /// Set requested launch-at-login state.
    Set(AppLaunchAtLoginSetArgs),
    /// Clear requested launch-at-login state.
    Clear(AppLaunchAtLoginClearArgs),
}

#[derive(Debug, Args)]
pub(super) struct AppLaunchAtLoginShowArgs {
    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct AppLaunchAtLoginSetArgs {
    /// Requested launch-at-login state.
    pub(super) state: ToggleState,

    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct AppLaunchAtLoginClearArgs {
    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct AppUpdateCheckArgs {
    #[command(subcommand)]
    pub(super) command: AppUpdateCheckCommand,
}

#[derive(Debug, Subcommand)]
pub(super) enum AppUpdateCheckCommand {
    /// Show cached update-check state.
    Show(AppUpdateCheckShowArgs),
    /// Run a live update check and refresh cached update state.
    Run(AppUpdateCheckRunArgs),
    /// Clear cached update-check state.
    Clear(AppUpdateCheckClearArgs),
}

#[derive(Debug, Args)]
pub(super) struct AppUpdateCheckShowArgs {
    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct AppUpdateCheckRunArgs {
    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct AppUpdateCheckClearArgs {
    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct AppQuitArgs {
    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
#[command(after_help = CAPTURE_ONCE_AFTER_HELP)]
pub(super) struct CaptureOnceArgs {
    /// Emit the captured snapshot as JSON.
    #[arg(long, default_value_t = false)]
    pub(super) json: bool,

    /// Emit the captured snapshot as polished terminal output.
    #[arg(long, default_value_t = false)]
    pub(super) human: bool,
}

#[derive(Debug, Args)]
#[command(after_help = SEARCH_AFTER_HELP)]
pub(super) struct SearchArgs {
    /// Query string for the selected search mode. Auto mode handles URLs, paths, bundle ids, exact phrases, and shell fragments more robustly.
    pub(super) query: String,

    /// Search mode to execute.
    #[arg(long, value_parser = parse_search_mode, default_value = "auto")]
    pub(super) mode: SearchMode,

    /// Maximum number of results.
    #[arg(long, default_value_t = 10, value_parser = parse_bounded_limit)]
    pub(super) limit: usize,

    /// Resume a paginated result set using the opaque `next_cursor` from a prior response.
    #[arg(long)]
    pub(super) cursor: Option<String>,

    #[command(flatten)]
    pub(super) filters: RetrievalFilterArgs,

    #[command(flatten)]
    pub(super) output: OutputArgs,
}

impl SearchArgs {
    pub(super) fn search_mode(&self) -> SearchMode {
        self.mode
    }
}

#[derive(Debug, Clone, Args)]
pub(super) struct RetrievalFilterArgs {
    /// Include captures observed at or after this RFC3339 timestamp. When combined with `--hours`, this takes precedence.
    #[arg(long, value_parser = parse_rfc3339_timestamp)]
    pub(super) since: Option<String>,

    /// Include captures observed at or before this RFC3339 timestamp.
    #[arg(long, value_parser = parse_rfc3339_timestamp)]
    pub(super) until: Option<String>,

    /// Restrict results to the most recent N hours unless `--since` is provided.
    #[arg(long)]
    pub(super) hours: Option<u32>,

    /// Filter by recorded frontmost app name using a case-insensitive substring match.
    #[arg(long)]
    pub(super) app: Option<String>,

    /// Filter by recorded frontmost bundle id using a case-insensitive exact match.
    #[arg(long)]
    pub(super) bundle_id: Option<String>,

    /// Filter by clipboard content shape. `file` means file URLs; `other` means mixed or empty snapshots.
    #[arg(long, value_parser = parse_retrieval_kind)]
    pub(super) kind: Option<RetrievalKind>,

    /// Require at least one non-empty text-like representation.
    #[arg(long, default_value_t = false)]
    pub(super) has_text: bool,

    /// Require at least one URL representation.
    #[arg(long, default_value_t = false)]
    pub(super) has_url: bool,

    /// Require at least one file URL representation.
    #[arg(long, default_value_t = false)]
    pub(super) has_file_url: bool,

    /// Require at least one image representation.
    #[arg(long, default_value_t = false)]
    pub(super) has_image: bool,

    /// Require at least one PDF representation.
    #[arg(long, default_value_t = false)]
    pub(super) has_pdf: bool,

    /// Require snapshot size to be at least this many bytes.
    #[arg(long, value_parser = parse_nonnegative_bytes)]
    pub(super) min_bytes: Option<usize>,

    /// Require snapshot size to be at most this many bytes.
    #[arg(long, value_parser = parse_nonnegative_bytes)]
    pub(super) max_bytes: Option<usize>,
}

impl RetrievalFilterArgs {
    pub(super) fn normalized(
        &self,
    ) -> std::result::Result<RetrievalFilters, super::errors::CliValueError> {
        validate_time_window(self.since.as_deref(), self.until.as_deref())?;
        validate_positive_hours(self.hours)?;
        validate_byte_window(self.min_bytes, self.max_bytes)?;

        let app = normalize_nonempty_filter_value(self.app.as_deref(), "--app")?;
        let bundle_id = normalize_nonempty_filter_value(self.bundle_id.as_deref(), "--bundle-id")?;
        let since = self.since.clone();
        let hours = if self.since.is_some() {
            None
        } else {
            self.hours
        };

        let mut filters = RetrievalFilters::default()
            .with_since(since)
            .with_until(self.until.clone())
            .with_hours(hours)
            .with_app(app)
            .with_bundle_id(bundle_id)
            .with_kind(self.kind)
            .with_min_bytes(self.min_bytes)
            .with_max_bytes(self.max_bytes);

        if self.has_text {
            filters = filters.requiring_text();
        }
        if self.has_url {
            filters = filters.requiring_url();
        }
        if self.has_file_url {
            filters = filters.requiring_file_url();
        }
        if self.has_image {
            filters = filters.requiring_image();
        }
        if self.has_pdf {
            filters = filters.requiring_pdf();
        }

        Ok(filters)
    }
}

#[derive(Debug, Args)]
#[command(after_help = RECENT_AFTER_HELP)]
pub(super) struct RecentArgs {
    /// Maximum number of results.
    #[arg(long, default_value_t = 10, value_parser = parse_bounded_limit)]
    pub(super) limit: usize,

    /// Resume a paginated result set using the opaque `next_cursor` from a prior response.
    #[arg(long)]
    pub(super) cursor: Option<String>,

    #[command(flatten)]
    pub(super) filters: RetrievalFilterArgs,

    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
#[command(after_help = TIMELINE_AFTER_HELP)]
pub(super) struct TimelineArgs {
    /// Maximum number of results.
    #[arg(long, default_value_t = 10, value_parser = parse_bounded_limit)]
    pub(super) limit: usize,

    /// Resume a paginated result set using the opaque `next_cursor` from a prior response.
    #[arg(long)]
    pub(super) cursor: Option<String>,

    /// Sort timeline events chronologically ascending or descending.
    #[arg(long, value_parser = parse_timeline_sort, default_value = "desc")]
    pub(super) sort: TimelineSort,

    #[command(flatten)]
    pub(super) filters: RetrievalFilterArgs,

    #[command(flatten)]
    pub(super) output: OutputArgs,
}

impl TimelineArgs {
    pub(super) fn timeline_sort(&self) -> TimelineSort {
        self.sort
    }
}

#[derive(Debug, Args)]
#[command(after_help = STATS_AFTER_HELP)]
pub(super) struct StatsArgs {
    #[command(flatten)]
    pub(super) filters: RetrievalFilterArgs,

    #[command(flatten)]
    pub(super) output: StatsOutputArgs,
}

#[derive(Debug, Args)]
#[command(after_help = RECALL_AFTER_HELP)]
pub(super) struct RecallArgs {
    /// Optional query describing the clipboard item to recall.
    pub(super) query: Option<String>,

    /// Search mode to use when a query is present.
    #[arg(long, value_parser = parse_search_mode, default_value = "auto")]
    pub(super) mode: SearchMode,

    /// Maximum number of ranked candidates to consider.
    #[arg(long, default_value_t = 5, value_parser = parse_bounded_limit)]
    pub(super) limit: usize,

    /// Expand the best candidate text instead of returning the compact form.
    #[arg(long, default_value_t = false)]
    pub(super) full: bool,

    /// Force quoted best-text output when text is available.
    #[arg(long, default_value_t = false)]
    pub(super) quote: bool,

    /// Minimum normalized match score before search is treated as strong enough on its own.
    #[arg(long, value_parser = parse_normalized_score)]
    pub(super) min_score: Option<f64>,

    /// Bias ranking toward recency.
    #[arg(long, default_value_t = false)]
    pub(super) prefer_recent: bool,

    /// Bias ranking toward clipboard events from the matching app or bundle id.
    #[arg(long, value_parser = parse_preferred_app)]
    pub(super) prefer_app: Option<String>,

    #[command(flatten)]
    pub(super) filters: RetrievalFilterArgs,

    #[command(flatten)]
    pub(super) output: RecallOutputArgs,
}

impl RecallArgs {
    pub(super) fn search_mode(&self) -> SearchMode {
        self.mode
    }
}

#[derive(Debug, Args)]
#[command(after_help = GET_AFTER_HELP)]
pub(super) struct GetArgs {
    /// Snapshot identifier.
    pub(super) snapshot_id: i64,

    /// Number of recent events to include.
    #[arg(long, default_value_t = 10, value_parser = parse_bounded_limit)]
    pub(super) events: usize,

    #[command(flatten)]
    pub(super) filters: RetrievalFilterArgs,

    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
#[command(after_help = EXPORT_AFTER_HELP)]
pub(super) struct ExportArgs {
    /// Snapshot identifier.
    pub(super) snapshot_id: i64,

    /// Item index inside the stored snapshot.
    #[arg(long, value_parser = parse_item_index)]
    pub(super) item: usize,

    /// Representation UTI to export.
    #[arg(long, value_parser = parse_representation_uti)]
    pub(super) uti: String,

    /// Destination path for the raw bytes.
    #[arg(long)]
    pub(super) out: PathBuf,

    /// Replace an existing regular file at the destination path.
    #[arg(long, default_value_t = false)]
    pub(super) force: bool,

    #[command(flatten)]
    pub(super) filters: RetrievalFilterArgs,

    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
#[command(after_help = RESTORE_AFTER_HELP)]
pub(super) struct RestoreArgs {
    /// Snapshot identifier.
    pub(super) snapshot_id: i64,

    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
#[command(after_help = FORGET_AFTER_HELP)]
pub(super) struct ForgetArgs {
    /// Snapshot identifier.
    pub(super) snapshot_id: i64,

    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
#[command(after_help = PURGE_AFTER_HELP)]
pub(super) struct PurgeArgs {
    /// Delete snapshots whose last observation is older than this duration (`Nd`, `Nh`, `Nm`).
    #[arg(long, value_parser = parse_duration_value)]
    pub(super) older_than: DurationValue,

    /// Report what would be deleted without deleting anything.
    #[arg(long, default_value_t = false)]
    pub(super) dry_run: bool,

    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
#[command(after_help = STORAGE_AFTER_HELP)]
pub(super) struct StorageArgs {
    #[command(subcommand)]
    pub(super) command: StorageCommand,
}

#[derive(Debug, Subcommand)]
pub(super) enum StorageCommand {
    /// Reclaim SQLite database and WAL disk space.
    Compact(StorageCompactArgs),
    /// List image rows eligible for optimization without rewriting bytes.
    ImageCandidates(StorageImageCandidatesArgs),
    /// Convert eligible archived images to lossless WebP.
    OptimizeImages(StorageOptimizeImagesArgs),
}

#[derive(Debug, Args)]
pub(super) struct StorageCompactArgs {
    /// Report database size and freelist state without running VACUUM.
    #[arg(long, default_value_t = false)]
    pub(super) dry_run: bool,

    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct StorageImageCandidatesArgs {
    /// Maximum number of eligible image rows to list.
    #[arg(long, default_value_t = 25, value_parser = parse_bounded_limit)]
    pub(super) limit: usize,

    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct StorageOptimizeImagesArgs {
    /// Report eligible rows and estimated savings without changing image bytes.
    #[arg(long, default_value_t = false)]
    pub(super) dry_run: bool,

    /// Do not compact SQLite storage after optimizing images.
    #[arg(long, default_value_t = false)]
    pub(super) no_compact: bool,

    /// Maximum number of unprocessed image rows to scan.
    #[arg(long, default_value_t = 25, value_parser = parse_bounded_limit)]
    pub(super) limit: usize,

    /// Stream progress events as newline-delimited JSON.
    #[arg(long, value_enum)]
    pub(super) progress: Option<ProgressFormat>,

    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
#[command(after_help = OCR_AFTER_HELP)]
pub(super) struct OcrArgs {
    #[command(subcommand)]
    pub(super) command: OcrCommand,
}

#[derive(Debug, Subcommand)]
pub(super) enum OcrCommand {
    /// Report OCR queue and result counts.
    Status(OcrStatusArgs),
    /// List pending OCR candidates without running OCR.
    Candidates(OcrCandidatesArgs),
    /// Show one OCR result by raw SHA-256 hash.
    Get(OcrGetArgs),
    /// Clear one OCR result by raw SHA-256 hash.
    Clear(OcrClearArgs),
    /// Process pending OCR candidates.
    Run(OcrRunArgs),
}

#[derive(Debug, Args)]
pub(super) struct OcrStatusArgs {
    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct OcrCandidatesArgs {
    /// Maximum number of pending OCR hashes to list.
    #[arg(long, default_value_t = 25, value_parser = parse_bounded_limit)]
    pub(super) limit: usize,

    /// Restrict candidates to one snapshot id.
    #[arg(long)]
    pub(super) snapshot: Option<i64>,

    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct OcrGetArgs {
    /// Raw representation SHA-256 hash.
    #[arg(value_parser = parse_sha256_hash)]
    pub(super) raw_sha256: String,

    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct OcrClearArgs {
    /// Raw representation SHA-256 hash.
    #[arg(value_parser = parse_sha256_hash)]
    pub(super) raw_sha256: String,

    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct OcrRunArgs {
    /// Maximum number of pending image hashes to process.
    #[arg(long, default_value_t = 25, value_parser = parse_bounded_limit)]
    pub(super) limit: usize,

    /// Restrict processing to one snapshot id.
    #[arg(long)]
    pub(super) snapshot: Option<i64>,

    /// Requeue failed OCR hashes before processing.
    #[arg(long, default_value_t = false)]
    pub(super) retry_failed: bool,

    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
#[command(after_help = SETTINGS_AFTER_HELP)]
pub(super) struct SettingsArgs {
    #[command(subcommand)]
    pub(super) command: SettingsCommand,
}

#[derive(Debug, Subcommand)]
pub(super) enum SettingsCommand {
    /// Show the current capture policy.
    Show(SettingsShowArgs),
    /// Persistently pause or resume capture.
    Pause(SettingsPauseArgs),
    /// Enable or disable API-key-like clipboard filtering.
    ApiKeyFilter(SettingsApiKeyFilterArgs),
    /// Enable or disable automatic OCR for copied images.
    Ocr(SettingsOcrArgs),
    /// Set retention to a duration or `forever`.
    Retention(SettingsRetentionArgs),
    /// Reset capture policy and ignored apps to defaults.
    Reset(SettingsResetArgs),
    /// Manage ignored bundle identifiers.
    Ignore(SettingsIgnoreArgs),
}

#[derive(Debug, Args)]
pub(super) struct SettingsShowArgs {
    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct SettingsPauseArgs {
    /// `on` pauses capture, `off` resumes it.
    pub(super) state: ToggleState,

    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct SettingsApiKeyFilterArgs {
    /// `on` skips clipboard snapshots that look like API keys, `off` stores them normally.
    pub(super) state: ToggleState,

    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct SettingsOcrArgs {
    /// `on` enables automatic OCR for image captures, `off` disables it.
    pub(super) state: ToggleState,

    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct SettingsRetentionArgs {
    /// Retain snapshots for this duration, or `forever` to disable automatic pruning.
    #[arg(value_parser = parse_retention_value)]
    pub(super) value: RetentionValue,

    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct SettingsResetArgs {
    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct SettingsIgnoreArgs {
    #[command(subcommand)]
    pub(super) command: SettingsIgnoreCommand,
}

#[derive(Debug, Subcommand)]
pub(super) enum SettingsIgnoreCommand {
    /// Add a bundle identifier to the ignore list.
    Add(SettingsIgnoreBundleArgs),
    /// Remove a bundle identifier from the ignore list.
    Remove(SettingsIgnoreBundleArgs),
    /// List ignored bundle identifiers.
    List(SettingsIgnoreListArgs),
}

#[derive(Debug, Args)]
pub(super) struct SettingsIgnoreBundleArgs {
    /// Bundle identifier to add or remove.
    #[arg(value_parser = parse_bundle_id)]
    pub(super) bundle_id: String,

    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct SettingsIgnoreListArgs {
    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
#[command(after_help = DOCTOR_AFTER_HELP)]
pub(super) struct DoctorArgs {
    /// Emit diagnostics as JSON.
    #[arg(long, default_value_t = false)]
    pub(super) json: bool,

    /// Emit diagnostics as polished terminal output.
    #[arg(long, default_value_t = false)]
    pub(super) human: bool,
}

#[derive(Debug, Args)]
pub(super) struct AgentsArgs {
    #[command(subcommand)]
    pub(super) command: AgentsCommand,
}

#[derive(Debug, Subcommand)]
pub(super) enum AgentsCommand {
    /// Print a compact context bundle for agents.
    #[command(after_help = AGENTS_CONTEXT_AFTER_HELP)]
    Context(AgentsContextArgs),
    /// Manage OpenClaw skill integration.
    Openclaw(OpenClawArgs),
    /// Manage Hermes Agent skill integration.
    Hermes(HermesArgs),
}

#[derive(Debug, Args)]
pub(super) struct AgentsContextArgs {
    #[command(flatten)]
    pub(super) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct OpenClawArgs {
    #[command(subcommand)]
    pub(super) command: OpenClawCommand,
}

#[derive(Debug, Subcommand)]
pub(super) enum OpenClawCommand {
    /// Install the packaged clipboard-memory skill into OpenClaw.
    InstallSkill(OpenClawInstallSkillArgs),
    /// Remove an installed OpenClaw clipboard-memory skill.
    UninstallSkill(OpenClawUninstallSkillArgs),
    /// Print the packaged OpenClaw skill content.
    #[command(after_help = OPENCLAW_PRINT_AFTER_HELP)]
    PrintSkill,
    /// Check host PATH, installed skill state, metadata, and sandbox guidance.
    Doctor(OpenClawDoctorArgs),
}

#[derive(Debug, Args)]
#[command(after_help = OPENCLAW_INSTALL_AFTER_HELP)]
pub(super) struct OpenClawInstallSkillArgs {
    /// Install into the shared OpenClaw skill directory instead of the active workspace.
    #[arg(long, default_value_t = false)]
    pub(super) shared: bool,

    /// Write the skill into this exact destination directory.
    #[arg(long)]
    pub(super) dest: Option<PathBuf>,

    /// Replace an existing skill directory if one is already present.
    #[arg(long, default_value_t = false)]
    pub(super) force: bool,
}

#[derive(Debug, Args)]
#[command(after_help = OPENCLAW_UNINSTALL_AFTER_HELP)]
pub(super) struct OpenClawUninstallSkillArgs {
    /// Remove from the shared OpenClaw skill directory instead of the active workspace.
    #[arg(long, default_value_t = false)]
    pub(super) shared: bool,

    /// Remove the skill from this exact destination directory.
    #[arg(long)]
    pub(super) dest: Option<PathBuf>,
}

#[derive(Debug, Args)]
#[command(after_help = OPENCLAW_DOCTOR_AFTER_HELP)]
pub(super) struct OpenClawDoctorArgs {
    /// Check the shared OpenClaw skill directory instead of the active workspace.
    #[arg(long, default_value_t = false)]
    pub(super) shared: bool,

    /// Check this exact destination directory instead of resolving the default target.
    #[arg(long)]
    pub(super) dest: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(super) struct HermesArgs {
    #[command(subcommand)]
    pub(super) command: HermesCommand,
}

#[derive(Debug, Subcommand)]
pub(super) enum HermesCommand {
    /// Install the packaged clipboard-memory skill into Hermes Agent.
    InstallSkill(HermesInstallSkillArgs),
    /// Remove an installed Hermes Agent clipboard-memory skill.
    UninstallSkill(HermesUninstallSkillArgs),
    /// Print the packaged Hermes Agent skill content.
    #[command(after_help = HERMES_PRINT_AFTER_HELP)]
    PrintSkill,
    /// Check host PATH, installed skill state, metadata, and Hermes discovery.
    Doctor(HermesDoctorArgs),
}

#[derive(Debug, Args)]
#[command(after_help = HERMES_INSTALL_AFTER_HELP)]
pub(super) struct HermesInstallSkillArgs {
    /// Write the skill into this exact destination directory.
    #[arg(long)]
    pub(super) dest: Option<PathBuf>,

    /// Replace an existing skill directory if one is already present.
    #[arg(long, default_value_t = false)]
    pub(super) force: bool,
}

#[derive(Debug, Args)]
#[command(after_help = HERMES_UNINSTALL_AFTER_HELP)]
pub(super) struct HermesUninstallSkillArgs {
    /// Remove the skill from this exact destination directory.
    #[arg(long)]
    pub(super) dest: Option<PathBuf>,
}

#[derive(Debug, Args)]
#[command(after_help = HERMES_DOCTOR_AFTER_HELP)]
pub(super) struct HermesDoctorArgs {
    /// Check this exact destination directory instead of resolving the default target.
    #[arg(long)]
    pub(super) dest: Option<PathBuf>,
}

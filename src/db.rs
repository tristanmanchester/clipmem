mod core;
mod impls;
mod read;
mod schema;
mod sqlite_helpers;
mod store;
mod types;

#[cfg(test)]
mod tests;

#[cfg(test)]
use self::core::{
    configure_connection, harden_existing_sqlite_sidecar_permissions, sidecar_path,
    storage_file_sizes,
};
#[cfg(test)]
use self::schema::{explain_query_plan, CURRENT_SCHEMA_VERSION, SCHEMA};
#[cfg(test)]
use self::sqlite_helpers::collect_rows;
pub use self::types::{
    ArchiveRevision, Database, RecentResults, RetrievalFilters, RetrievalKind, SearchMode,
    SearchResults, StatsReport, StatsSnapshotLeaderboardEntry, StatsTimeBucketEntry,
    TimelineResults, TimelineSort,
};
pub(crate) use self::types::{
    CapturePolicy, CaptureSettings, CaptureSkipReason, CaptureStoreOutcome,
    ImageOptimizationCandidateSummary, ImageOptimizationReport, OcrCandidateSummary,
    OcrResultRecord, OcrRunReport, OcrStatusReport, PurgeReport, RecentCursorState,
    SearchCursorState, SnapshotDeletionReport, StorageCompactReport, TimelineCursorState,
};

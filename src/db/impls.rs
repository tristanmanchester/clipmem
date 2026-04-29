use crate::model::SearchHit;

use super::types::{
    ArchiveChangeKind, ArchiveRevision, CapturePolicy, CaptureSettings, CaptureSkipReason,
    ImageOptimizationCandidateSummary, OcrCandidate, OcrCandidateSummary, OcrResultRecord,
    OcrRunReport, OcrStatusReport, Page, PurgeReport, RecentCursorState, RecentResults,
    RetrievalFilters, RetrievalKind, SearchCursorState, SearchMode, SearchResults,
    SnapshotDeletionReport, TimelineCursorState, TimelineResults, TimelineSort,
};

impl ArchiveChangeKind {
    #[must_use]
    pub(in crate::db) const fn as_str(self) -> &'static str {
        match self {
            Self::ArchiveContent => "archive_content",
            Self::Settings => "settings",
            Self::Ocr => "ocr",
            Self::Storage => "storage",
            Self::Service => "service",
            Self::AppPreferences => "app_preferences",
        }
    }

    #[must_use]
    pub(in crate::db) const fn revision_column(self) -> &'static str {
        match self {
            Self::ArchiveContent => "archive_content_revision",
            Self::Settings => "settings_revision",
            Self::Ocr => "ocr_revision",
            Self::Storage => "storage_revision",
            Self::Service => "service_revision",
            Self::AppPreferences => "app_preferences_revision",
        }
    }
}

#[allow(dead_code)]
impl ArchiveRevision {
    #[must_use]
    pub(crate) fn new(
        revision: u64,
        archive_content_revision: u64,
        settings_revision: u64,
        ocr_revision: u64,
        storage_revision: u64,
        service_revision: u64,
        app_preferences_revision: u64,
        last_change_kind: String,
        updated_at: String,
    ) -> Self {
        Self {
            revision,
            archive_content_revision,
            settings_revision,
            ocr_revision,
            storage_revision,
            service_revision,
            app_preferences_revision,
            last_change_kind,
            updated_at,
        }
    }

    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub fn archive_content_revision(&self) -> u64 {
        self.archive_content_revision
    }

    #[must_use]
    pub fn settings_revision(&self) -> u64 {
        self.settings_revision
    }

    #[must_use]
    pub fn ocr_revision(&self) -> u64 {
        self.ocr_revision
    }

    #[must_use]
    pub fn storage_revision(&self) -> u64 {
        self.storage_revision
    }

    #[must_use]
    pub fn service_revision(&self) -> u64 {
        self.service_revision
    }

    #[must_use]
    pub fn app_preferences_revision(&self) -> u64 {
        self.app_preferences_revision
    }

    #[must_use]
    pub fn last_change_kind(&self) -> &str {
        &self.last_change_kind
    }

    #[must_use]
    pub fn updated_at(&self) -> &str {
        &self.updated_at
    }
}

impl SearchResults {
    #[must_use]
    pub(crate) fn new(mode_used: SearchMode, hits: Vec<SearchHit>, has_more: bool) -> Self {
        Self {
            mode_used,
            hits,
            has_more,
        }
    }

    #[must_use]
    pub fn mode_used(&self) -> SearchMode {
        self.mode_used
    }

    #[must_use]
    pub fn hits(&self) -> &[SearchHit] {
        &self.hits
    }

    #[must_use]
    pub fn has_more(&self) -> bool {
        self.has_more
    }
}

impl RecentResults {
    #[must_use]
    pub(crate) fn new(hits: Vec<SearchHit>, has_more: bool) -> Self {
        Self { hits, has_more }
    }

    #[must_use]
    pub fn hits(&self) -> &[SearchHit] {
        &self.hits
    }

    #[must_use]
    pub fn has_more(&self) -> bool {
        self.has_more
    }

    #[must_use]
    pub fn into_hits(self) -> Vec<SearchHit> {
        self.hits
    }
}

impl TimelineResults {
    #[must_use]
    pub(crate) fn new(events: Vec<crate::model::TimelineEvent>, has_more: bool) -> Self {
        Self { events, has_more }
    }

    #[must_use]
    pub fn events(&self) -> &[crate::model::TimelineEvent] {
        &self.events
    }

    #[must_use]
    pub fn has_more(&self) -> bool {
        self.has_more
    }

    #[must_use]
    pub fn into_events(self) -> Vec<crate::model::TimelineEvent> {
        self.events
    }
}

impl<T> Page<T> {
    #[must_use]
    pub(crate) fn new(items: Vec<T>, has_more: bool) -> Self {
        Self { items, has_more }
    }

    #[must_use]
    pub(crate) fn items(&self) -> &[T] {
        &self.items
    }

    #[must_use]
    pub(crate) fn into_items(self) -> Vec<T> {
        self.items
    }

    #[must_use]
    pub(crate) fn has_more(&self) -> bool {
        self.has_more
    }
}

impl RecentCursorState {
    #[must_use]
    pub(crate) fn new(last_seen_at: String, snapshot_id: i64) -> Self {
        Self {
            last_seen_at,
            snapshot_id,
        }
    }

    #[must_use]
    pub(crate) fn last_seen_at(&self) -> &str {
        &self.last_seen_at
    }

    #[must_use]
    pub(crate) fn snapshot_id(&self) -> i64 {
        self.snapshot_id
    }
}

impl SearchCursorState {
    #[must_use]
    pub(crate) fn new(
        mode_used: SearchMode,
        score: Option<f64>,
        last_seen_at: String,
        snapshot_id: i64,
    ) -> Self {
        Self {
            mode_used,
            score,
            last_seen_at,
            snapshot_id,
        }
    }

    #[must_use]
    pub(crate) fn mode_used(&self) -> SearchMode {
        self.mode_used
    }

    #[must_use]
    pub(crate) fn score(&self) -> Option<f64> {
        self.score
    }

    #[must_use]
    pub(crate) fn last_seen_at(&self) -> &str {
        &self.last_seen_at
    }

    #[must_use]
    pub(crate) fn snapshot_id(&self) -> i64 {
        self.snapshot_id
    }
}

impl SearchMode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Fts => "fts",
            Self::Literal => "literal",
        }
    }
}

impl TimelineSort {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        }
    }
}

impl RetrievalKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Html => "html",
            Self::Rtf => "rtf",
            Self::Url => "url",
            Self::File => "file",
            Self::Image => "image",
            Self::Pdf => "pdf",
            Self::Binary => "binary",
            Self::Other => "other",
        }
    }
}

impl RetrievalFilters {
    #[must_use]
    pub fn with_since(mut self, since: Option<String>) -> Self {
        self.since = since;
        self
    }

    #[must_use]
    pub fn with_until(mut self, until: Option<String>) -> Self {
        self.until = until;
        self
    }

    #[must_use]
    pub fn with_hours(mut self, hours: Option<u32>) -> Self {
        self.hours = hours;
        self
    }

    #[must_use]
    pub fn with_app(mut self, app: Option<String>) -> Self {
        self.app = app;
        self
    }

    #[must_use]
    pub fn with_bundle_id(mut self, bundle_id: Option<String>) -> Self {
        self.bundle_id = bundle_id;
        self
    }

    #[must_use]
    pub fn with_kind(mut self, kind: Option<RetrievalKind>) -> Self {
        self.kind = kind;
        self
    }

    #[must_use]
    pub fn requiring_text(mut self) -> Self {
        self.has_text = true;
        self
    }

    #[must_use]
    pub fn requiring_url(mut self) -> Self {
        self.has_url = true;
        self
    }

    #[must_use]
    pub fn requiring_file_url(mut self) -> Self {
        self.has_file_url = true;
        self
    }

    #[must_use]
    pub fn requiring_image(mut self) -> Self {
        self.has_image = true;
        self
    }

    #[must_use]
    pub fn requiring_pdf(mut self) -> Self {
        self.has_pdf = true;
        self
    }

    #[must_use]
    pub fn with_min_bytes(mut self, min_bytes: Option<usize>) -> Self {
        self.min_bytes = min_bytes;
        self
    }

    #[must_use]
    pub fn with_max_bytes(mut self, max_bytes: Option<usize>) -> Self {
        self.max_bytes = max_bytes;
        self
    }

    #[must_use]
    pub fn since(&self) -> Option<&str> {
        self.since.as_deref()
    }

    #[must_use]
    pub fn until(&self) -> Option<&str> {
        self.until.as_deref()
    }

    #[must_use]
    pub fn hours(&self) -> Option<u32> {
        self.hours
    }

    #[must_use]
    pub fn app(&self) -> Option<&str> {
        self.app.as_deref()
    }

    #[must_use]
    pub fn bundle_id(&self) -> Option<&str> {
        self.bundle_id.as_deref()
    }

    #[must_use]
    pub fn kind(&self) -> Option<RetrievalKind> {
        self.kind
    }

    #[must_use]
    pub fn has_text(&self) -> bool {
        self.has_text
    }

    #[must_use]
    pub fn has_url(&self) -> bool {
        self.has_url
    }

    #[must_use]
    pub fn has_file_url(&self) -> bool {
        self.has_file_url
    }

    #[must_use]
    pub fn has_image(&self) -> bool {
        self.has_image
    }

    #[must_use]
    pub fn has_pdf(&self) -> bool {
        self.has_pdf
    }

    #[must_use]
    pub fn min_bytes(&self) -> Option<usize> {
        self.min_bytes
    }

    #[must_use]
    pub fn max_bytes(&self) -> Option<usize> {
        self.max_bytes
    }
}

impl CaptureSettings {
    #[must_use]
    pub(crate) fn new(
        paused: bool,
        retention_seconds: Option<u64>,
        api_key_filter_enabled: bool,
        ocr_enabled: bool,
    ) -> Self {
        Self {
            paused,
            retention_seconds,
            api_key_filter_enabled,
            ocr_enabled,
        }
    }

    #[must_use]
    pub(crate) fn paused(&self) -> bool {
        self.paused
    }

    #[must_use]
    pub(crate) fn retention_seconds(&self) -> Option<u64> {
        self.retention_seconds
    }

    #[must_use]
    pub(crate) fn api_key_filter_enabled(&self) -> bool {
        self.api_key_filter_enabled
    }

    #[must_use]
    pub(crate) fn ocr_enabled(&self) -> bool {
        self.ocr_enabled
    }
}

impl CapturePolicy {
    #[must_use]
    pub(crate) fn new(settings: CaptureSettings, ignored_bundle_ids: Vec<String>) -> Self {
        Self {
            settings,
            ignored_bundle_ids,
        }
    }

    #[must_use]
    pub(crate) fn settings(&self) -> &CaptureSettings {
        &self.settings
    }

    #[must_use]
    pub(crate) fn ignored_bundle_ids(&self) -> &[String] {
        &self.ignored_bundle_ids
    }

    #[must_use]
    pub(crate) fn ignored_bundle_id_count(&self) -> usize {
        self.ignored_bundle_ids.len()
    }
}

impl CaptureSkipReason {
    #[must_use]
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::ApiKeyFilter => "api_key_filter",
            Self::RestoredSnapshot => "restored_snapshot",
        }
    }
}

impl SnapshotDeletionReport {
    #[must_use]
    pub(crate) fn new(
        snapshot_id: i64,
        item_count: usize,
        representation_count: usize,
        capture_event_count: usize,
        total_bytes: usize,
    ) -> Self {
        Self {
            snapshot_id,
            item_count,
            representation_count,
            capture_event_count,
            total_bytes,
        }
    }

    #[must_use]
    pub(crate) fn snapshot_id(&self) -> i64 {
        self.snapshot_id
    }

    #[must_use]
    pub(crate) fn item_count(&self) -> usize {
        self.item_count
    }

    #[must_use]
    pub(crate) fn representation_count(&self) -> usize {
        self.representation_count
    }

    #[must_use]
    pub(crate) fn capture_event_count(&self) -> usize {
        self.capture_event_count
    }

    #[must_use]
    pub(crate) fn total_bytes(&self) -> usize {
        self.total_bytes
    }
}

impl PurgeReport {
    #[must_use]
    pub(crate) fn new(
        older_than_seconds: u64,
        dry_run: bool,
        snapshot_count: usize,
        item_count: usize,
        representation_count: usize,
        capture_event_count: usize,
        total_bytes: usize,
    ) -> Self {
        Self {
            older_than_seconds,
            dry_run,
            snapshot_count,
            item_count,
            representation_count,
            capture_event_count,
            total_bytes,
        }
    }

    #[must_use]
    pub(crate) fn older_than_seconds(&self) -> u64 {
        self.older_than_seconds
    }

    #[must_use]
    pub(crate) fn dry_run(&self) -> bool {
        self.dry_run
    }

    #[must_use]
    pub(crate) fn snapshot_count(&self) -> usize {
        self.snapshot_count
    }

    #[must_use]
    pub(crate) fn item_count(&self) -> usize {
        self.item_count
    }

    #[must_use]
    pub(crate) fn representation_count(&self) -> usize {
        self.representation_count
    }

    #[must_use]
    pub(crate) fn capture_event_count(&self) -> usize {
        self.capture_event_count
    }

    #[must_use]
    pub(crate) fn total_bytes(&self) -> usize {
        self.total_bytes
    }
}

impl OcrCandidate {
    #[must_use]
    pub(crate) fn new(raw_sha256: String, blob_value: Vec<u8>, snapshot_count: usize) -> Self {
        Self {
            raw_sha256,
            blob_value,
            snapshot_count,
        }
    }

    #[must_use]
    pub(crate) fn raw_sha256(&self) -> &str {
        &self.raw_sha256
    }

    #[must_use]
    pub(crate) fn blob_value(&self) -> &[u8] {
        &self.blob_value
    }
}

impl OcrStatusReport {
    #[must_use]
    pub(crate) fn new(
        pending: usize,
        ready: usize,
        failed: usize,
        skipped: usize,
        snapshots_with_ocr_text: usize,
    ) -> Self {
        Self {
            pending,
            ready,
            failed,
            skipped,
            snapshots_with_ocr_text,
        }
    }

    #[must_use]
    pub(crate) fn pending(&self) -> usize {
        self.pending
    }

    #[must_use]
    pub(crate) fn ready(&self) -> usize {
        self.ready
    }

    #[must_use]
    pub(crate) fn failed(&self) -> usize {
        self.failed
    }

    #[must_use]
    pub(crate) fn skipped(&self) -> usize {
        self.skipped
    }

    #[must_use]
    pub(crate) fn snapshots_with_ocr_text(&self) -> usize {
        self.snapshots_with_ocr_text
    }
}

impl OcrCandidateSummary {
    #[must_use]
    pub(crate) fn new(
        raw_sha256: String,
        byte_len: usize,
        snapshot_count: usize,
        updated_at: String,
    ) -> Self {
        Self {
            raw_sha256,
            byte_len,
            snapshot_count,
            updated_at,
        }
    }

    #[must_use]
    pub(crate) fn raw_sha256(&self) -> &str {
        &self.raw_sha256
    }

    #[must_use]
    pub(crate) fn byte_len(&self) -> usize {
        self.byte_len
    }

    #[must_use]
    pub(crate) fn snapshot_count(&self) -> usize {
        self.snapshot_count
    }
}

impl OcrResultRecord {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub(crate) fn new(
        raw_sha256: String,
        status: String,
        engine: Option<String>,
        recognition_level: Option<String>,
        text_value: Option<String>,
        error: Option<String>,
        attempt_count: usize,
        updated_at: String,
        snapshot_count: usize,
    ) -> Self {
        Self {
            raw_sha256,
            status,
            engine,
            recognition_level,
            text_value,
            error,
            attempt_count,
            updated_at,
            snapshot_count,
        }
    }

    #[must_use]
    pub(crate) fn raw_sha256(&self) -> &str {
        &self.raw_sha256
    }

    #[must_use]
    pub(crate) fn status(&self) -> &str {
        &self.status
    }

    #[must_use]
    pub(crate) fn snapshot_count(&self) -> usize {
        self.snapshot_count
    }
}

impl ImageOptimizationCandidateSummary {
    #[must_use]
    pub(crate) fn new(
        snapshot_id: i64,
        item_index: i64,
        uti: String,
        byte_len: usize,
        raw_sha256: String,
    ) -> Self {
        Self {
            snapshot_id,
            item_index,
            uti,
            byte_len,
            raw_sha256,
        }
    }

    #[must_use]
    pub(crate) fn snapshot_id(&self) -> i64 {
        self.snapshot_id
    }

    #[must_use]
    pub(crate) fn item_index(&self) -> i64 {
        self.item_index
    }

    #[must_use]
    pub(crate) fn uti(&self) -> &str {
        &self.uti
    }

    #[must_use]
    pub(crate) fn byte_len(&self) -> usize {
        self.byte_len
    }
}

impl OcrRunReport {
    #[must_use]
    pub(crate) fn new(
        processed: usize,
        ready: usize,
        failed: usize,
        skipped: usize,
        remaining_pending: usize,
    ) -> Self {
        Self {
            processed,
            ready,
            failed,
            skipped,
            remaining_pending,
        }
    }

    #[must_use]
    pub(crate) fn processed(&self) -> usize {
        self.processed
    }

    #[must_use]
    pub(crate) fn ready(&self) -> usize {
        self.ready
    }

    #[must_use]
    pub(crate) fn failed(&self) -> usize {
        self.failed
    }

    #[must_use]
    pub(crate) fn skipped(&self) -> usize {
        self.skipped
    }

    #[must_use]
    pub(crate) fn remaining_pending(&self) -> usize {
        self.remaining_pending
    }
}

impl TimelineCursorState {
    #[must_use]
    pub(crate) fn new(observed_at: String, event_id: i64) -> Self {
        Self {
            observed_at,
            event_id,
        }
    }

    #[must_use]
    pub(crate) fn observed_at(&self) -> &str {
        &self.observed_at
    }

    #[must_use]
    pub(crate) fn event_id(&self) -> i64 {
        self.event_id
    }
}

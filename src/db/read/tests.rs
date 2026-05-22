use anyhow::Result;

use crate::db::{Database, RetrievalFilters, SearchMode, SearchResults};
use crate::model::{build_item, build_representation, build_snapshot, CaptureContext};

use super::queries::fts_query;
use super::query_analysis::{
    analyze_query, invalid_fts_message, is_simple_fts_query, literal_fts_match_query,
};
use crate::file_url::normalize_file_path;

pub(in crate::db) fn fake_snapshot(
    change_count: i64,
    text: &str,
) -> crate::model::ClipboardSnapshot {
    build_snapshot(
        CaptureContext::new(change_count)
            .with_frontmost_app_name("Terminal")
            .with_frontmost_app_bundle_id("com.apple.Terminal"),
        vec![build_item(
            0,
            vec![build_representation(
                "public.utf8-plain-text".to_string(),
                None,
                text.as_bytes().to_vec(),
            )],
        )],
    )
}

pub(in crate::db) fn fake_file_snapshot(
    change_count: i64,
    file_url: &str,
) -> crate::model::ClipboardSnapshot {
    build_snapshot(
        CaptureContext::new(change_count)
            .with_frontmost_app_name("Finder")
            .with_frontmost_app_bundle_id("com.apple.finder"),
        vec![build_item(
            0,
            vec![build_representation(
                "public.file-url".to_string(),
                Some(file_url.to_string()),
                file_url.as_bytes().to_vec(),
            )],
        )],
    )
}

pub(in crate::db) fn fake_image_snapshot(
    change_count: i64,
    bytes: Vec<u8>,
) -> crate::model::ClipboardSnapshot {
    build_snapshot(
        CaptureContext::new(change_count)
            .with_frontmost_app_name("Preview")
            .with_frontmost_app_bundle_id("com.apple.Preview"),
        vec![build_item(
            0,
            vec![build_representation("public.png".to_string(), None, bytes)],
        )],
    )
}

pub(in crate::db) fn unfiltered() -> RetrievalFilters {
    RetrievalFilters::default()
}

#[test]
pub(in crate::db) fn like_special_characters_skip_fts_mode() {
    assert!(analyze_query("50%").literal_preferred);
    assert!(analyze_query("config_test").literal_preferred);
    assert!(analyze_query(r"logs\2024").literal_preferred);
    assert!(analyze_query("https://example.com").literal_preferred);
    assert!(analyze_query("com.apple.Terminal").literal_preferred);
    assert!(analyze_query("~/path/with/slashes").literal_preferred);
    assert!(analyze_query("foo:bar").literal_preferred);
    assert!(analyze_query("git commit -m").literal_preferred);
    assert!(!analyze_query("git clone").literal_preferred);
}

#[test]
pub(in crate::db) fn query_analysis_preserves_exact_phrase_without_forcing_literal() {
    let analysis = analyze_query("\"launchctl bootstrap\"");
    assert_eq!(
        analysis.exact_phrase.as_deref(),
        Some("launchctl bootstrap")
    );
    assert!(!analysis.literal_preferred);
}

#[test]
pub(in crate::db) fn simple_fts_query_skips_per_row_phrase_scoring() {
    assert!(is_simple_fts_query(&analyze_query("git")));
    assert!(!is_simple_fts_query(&analyze_query("\"git clone\"")));
    assert!(!is_simple_fts_query(&analyze_query("git clone")));
    assert!(!is_simple_fts_query(&analyze_query("com.apple.Terminal")));

    let simple_sql = fts_query(false, false, true);
    let phrase_sql = fts_query(false, false, false);

    assert!(!simple_sql.contains("Phrase match in best text"));
    assert!(!simple_sql.contains("THEN 0.08"));
    assert!(phrase_sql.contains("Phrase match in best text"));
    assert!(phrase_sql.contains("THEN 0.08"));
}

#[test]
pub(in crate::db) fn invalid_fts_messages_are_narrowly_classified() {
    assert!(invalid_fts_message("fts5: syntax error near \"%\""));
    assert!(invalid_fts_message("malformed MATCH expression"));
    assert!(invalid_fts_message("no such column: https"));
    assert!(!invalid_fts_message("database disk image is malformed"));
}

#[test]
pub(in crate::db) fn literal_fts_match_query_skips_like_escaped_inputs() {
    assert!(literal_fts_match_query(&analyze_query("50%")).is_none());
    assert!(literal_fts_match_query(&analyze_query("config_test")).is_none());
    assert!(literal_fts_match_query(&analyze_query(r"logs\2024")).is_none());
    assert_eq!(
        literal_fts_match_query(&analyze_query("/tmp/repo/42/Cargo.toml")).as_deref(),
        Some("\"/tmp/repo/42/cargo.toml\"")
    );
    assert_eq!(
        analyze_query("~/path/with/slashes")
            .path_fragment
            .as_deref(),
        Some("path/with/slashes")
    );
    assert_eq!(
        analyze_query("file:///tmp/repo/42/Cargo.toml")
            .path_fragment
            .as_deref(),
        Some("/tmp/repo/42/cargo.toml")
    );
}

#[test]
pub(in crate::db) fn file_urls_decode_to_paths() {
    assert_eq!(
        normalize_file_path("file:///Users/test/Documents/hello%20world.txt"),
        "/Users/test/Documents/hello world.txt"
    );
    assert_eq!(
        normalize_file_path("file://LOCALHOST/Users/test/Documents/hello%20world.txt"),
        "/Users/test/Documents/hello world.txt"
    );
    assert_eq!(
        normalize_file_path("/Users/test/Documents/hello world.txt"),
        "/Users/test/Documents/hello world.txt"
    );
}

#[test]
pub(in crate::db) fn search_auto_uses_literal_matching_for_wildcard_queries() -> Result<()> {
    let mut db = Database::open_in_memory()?;

    db.store_capture(&fake_snapshot(1, "Discount: 50 percent off"))?;
    db.store_capture(&fake_snapshot(2, "Discount: 50%"))?;

    let results = db.search_auto("50%", 10, &unfiltered())?;
    assert_eq!(results.mode_used(), SearchMode::Literal);
    assert_eq!(results.hits().len(), 1);
    assert_eq!(results.hits()[0].preview_text(), "Discount: 50%");
    Ok(())
}

#[test]
pub(in crate::db) fn file_path_search_matches_paths_with_spaces() -> Result<()> {
    let mut db = Database::open_in_memory()?;

    db.store_capture(&fake_file_snapshot(
        1,
        "file:///Users/test/work/foo-bar.txt",
    ))?;
    let spaced = db.store_capture(&fake_file_snapshot(
        2,
        "file:///Users/test/work/foo%20bar.txt",
    ))?;

    let results = db.search_auto("/Users/test/work/foo bar.txt", 10, &unfiltered())?;

    assert_eq!(results.mode_used(), SearchMode::Literal);
    assert_eq!(results.hits().len(), 1);
    assert_eq!(results.hits()[0].snapshot_id(), spaced.snapshot_id());
    assert_eq!(
        results.hits()[0].file_paths(),
        &["/Users/test/work/foo bar.txt".to_string()]
    );
    Ok(())
}

#[test]
pub(in crate::db) fn file_path_literal_fast_path_includes_ocr_results() -> Result<()> {
    let mut db = Database::open_in_memory()?;

    let file_stored = db.store_capture(&fake_file_snapshot(
        1,
        "file:///Users/test/work/cargo%20build.txt",
    ))?;
    let image_stored = db.store_capture(&fake_image_snapshot(2, b"image-bytes".to_vec()))?;
    db.enqueue_ocr_for_snapshot(image_stored.snapshot_id())?;
    let candidates = db.next_ocr_candidates(25, None, false)?;
    assert_eq!(candidates.len(), 1);
    db.store_ocr_text(
        candidates[0].raw_sha256(),
        "fake",
        "fast",
        "Screenshot of /Users/test/work/cargo build.txt",
    )?;

    let results = db.search_auto("/Users/test/work/cargo build.txt", 10, &unfiltered())?;

    assert_eq!(results.mode_used(), SearchMode::Literal);
    let snapshot_ids: Vec<i64> = results.hits().iter().map(|hit| hit.snapshot_id()).collect();
    assert!(snapshot_ids.contains(&file_stored.snapshot_id()));
    assert!(snapshot_ids.contains(&image_stored.snapshot_id()));
    assert_eq!(results.hits().len(), 2);
    assert!(results.hits().iter().any(|hit| {
        hit.snapshot_id() == image_stored.snapshot_id()
            && hit.matched_fields() == ["ocr_text".to_string()]
    }));
    Ok(())
}

#[test]
pub(in crate::db) fn file_path_search_matches_percent_encoded_path_characters() -> Result<()> {
    let mut db = Database::open_in_memory()?;

    let stored = db.store_capture(&fake_file_snapshot(
        1,
        "file:///Users/test/work/release%231.txt",
    ))?;

    let results = db.search_auto("/Users/test/work/release#1.txt", 10, &unfiltered())?;

    assert_eq!(results.mode_used(), SearchMode::Literal);
    assert_eq!(results.hits().len(), 1);
    assert_eq!(results.hits()[0].snapshot_id(), stored.snapshot_id());
    assert_eq!(
        results.hits()[0].file_paths(),
        &["/Users/test/work/release#1.txt".to_string()]
    );
    Ok(())
}

#[test]
pub(in crate::db) fn file_path_search_handles_case_insensitive_localhost_authority() -> Result<()> {
    let mut db = Database::open_in_memory()?;

    let stored = db.store_capture(&fake_file_snapshot(
        1,
        "file://LOCALHOST/Users/test/work/report%20draft.txt",
    ))?;

    let results = db.search_auto("/Users/test/work/report draft.txt", 10, &unfiltered())?;

    assert_eq!(results.mode_used(), SearchMode::Literal);
    assert_eq!(results.hits().len(), 1);
    assert_eq!(results.hits()[0].snapshot_id(), stored.snapshot_id());
    assert_eq!(
        results.hits()[0].file_paths(),
        &["/Users/test/work/report draft.txt".to_string()]
    );
    Ok(())
}

#[test]
pub(in crate::db) fn find_snapshot_hydrates_nested_items_and_recent_events() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    let snapshot = build_snapshot(
        CaptureContext::new(1)
            .with_frontmost_app_name("Terminal")
            .with_frontmost_app_bundle_id("com.apple.Terminal"),
        vec![build_item(
            0,
            vec![build_representation(
                "public.utf8-plain-text".to_string(),
                None,
                b"git status".to_vec(),
            )],
        )],
    );
    let stored = db.store_capture(&snapshot)?;
    db.store_capture(&snapshot)?;

    let details = db
        .find_snapshot(stored.snapshot_id(), 10)?
        .expect("snapshot exists");
    assert_eq!(details.items().len(), 1);
    assert_eq!(details.recent_events().len(), 2);
    Ok(())
}

#[test]
pub(in crate::db) fn empty_fts_results_fall_back_to_literal_matches() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    db.store_capture(&fake_snapshot(1, "https://example.com/repo"))?;

    let results: SearchResults = db.search_auto("https://example.com/repo", 10, &unfiltered())?;
    assert_eq!(results.mode_used(), SearchMode::Literal);
    assert_eq!(results.hits().len(), 1);
    Ok(())
}

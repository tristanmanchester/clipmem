use std::path::Path;

use anyhow::{anyhow, Result};
use serde_json::json;

use crate::model::{SearchHit, TimelineEvent};

use crate::cli::errors::not_found_error;
use crate::cli::output::{
    GetEnvelope, ListEnvelope, ListRow, StatsEnvelope, OUTPUT_SCHEMA_VERSION,
};
use crate::cli::presentation::{
    emit_get_output, emit_list_output, emit_stats_output, generated_at_now,
    require_get_output_format,
};
use crate::cli::schema::{GetArgs, RecentArgs, SearchArgs, StatsArgs, TimelineArgs};

use super::retrieval_support::{
    load_snapshot_projections, merge_applied_filters, normalize_retrieval_filters,
    query_search_results,
};
use super::runtime::open_existing_db;

mod cursor;
mod recall;

pub(in crate::cli) use self::cursor::*;
pub(in crate::cli) use self::recall::recall;

pub(in crate::cli) fn search(db_path: &Path, args: &SearchArgs) -> Result<()> {
    let format = args.output.resolved()?;
    let filters = normalize_retrieval_filters(&args.filters)?;
    let search_mode = args.search_mode();
    let cursor = args
        .cursor
        .as_deref()
        .map(|value| parse_search_cursor(value, &args.query, search_mode, &filters))
        .transpose()?;
    let db = open_existing_db(db_path)?;
    let results = anyhow::Context::with_context(
        query_search_results(&db, args, &filters, cursor.as_ref()),
        || format!("search failed for query '{}'", args.query),
    )?;

    let projections =
        load_snapshot_projections(&db, results.hits().iter().map(SearchHit::snapshot_id))?;

    let generated_at = generated_at_now()?;
    let next_cursor = if results.has_more() {
        results
            .hits()
            .last()
            .map(|hit| {
                encode_search_cursor(&args.query, search_mode, &filters, results.mode_used(), hit)
            })
            .transpose()?
    } else {
        None
    };
    let envelope = ListEnvelope {
        schema_version: OUTPUT_SCHEMA_VERSION,
        command: "search",
        generated_at,
        applied_filters: merge_applied_filters(
            &filters,
            json!({
                "query": args.query,
                "requested_mode": search_mode.as_str(),
                "mode_used": results.mode_used().as_str(),
                "limit": args.limit,
                "cursor": args.cursor,
            }),
        ),
        truncated: results.has_more(),
        next_cursor,
        results: results
            .hits()
            .iter()
            .map(|hit| {
                let projection = projections
                    .get(&hit.snapshot_id())
                    .cloned()
                    .unwrap_or_default();
                ListRow::from_hit(hit, true, &projection)
            })
            .collect(),
    };
    emit_list_output(format, &envelope)
}

pub(in crate::cli) fn recent(db_path: &Path, args: &RecentArgs) -> Result<()> {
    let format = args.output.resolved()?;
    let filters = normalize_retrieval_filters(&args.filters)?;
    let cursor = args
        .cursor
        .as_deref()
        .map(|value| parse_recent_cursor(value, &filters))
        .transpose()?;
    let db = open_existing_db(db_path)?;
    let hits = anyhow::Context::with_context(
        db.recent_page(args.limit, &filters, cursor.as_ref()),
        || "recent query failed".to_string(),
    )?;

    let projections =
        load_snapshot_projections(&db, hits.items().iter().map(SearchHit::snapshot_id))?;

    let generated_at = generated_at_now()?;
    let next_cursor = if hits.has_more() {
        hits.items()
            .last()
            .map(|hit| encode_recent_cursor(&filters, hit))
            .transpose()?
    } else {
        None
    };
    let envelope = ListEnvelope {
        schema_version: OUTPUT_SCHEMA_VERSION,
        command: "recent",
        generated_at,
        applied_filters: merge_applied_filters(
            &filters,
            json!({
                "limit": args.limit,
                "cursor": args.cursor,
            }),
        ),
        truncated: hits.has_more(),
        next_cursor,
        results: hits
            .items()
            .iter()
            .map(|hit| {
                let projection = projections
                    .get(&hit.snapshot_id())
                    .cloned()
                    .unwrap_or_default();
                ListRow::from_hit(hit, false, &projection)
            })
            .collect(),
    };
    emit_list_output(format, &envelope)
}

pub(in crate::cli) fn timeline(db_path: &Path, args: &TimelineArgs) -> Result<()> {
    let format = args.output.resolved()?;
    let filters = normalize_retrieval_filters(&args.filters)?;
    let timeline_sort = args.timeline_sort();
    let cursor = args
        .cursor
        .as_deref()
        .map(|value| parse_timeline_cursor(value, &filters, timeline_sort))
        .transpose()?;
    let db = open_existing_db(db_path)?;
    let events = anyhow::Context::with_context(
        db.timeline_page(args.limit, &filters, timeline_sort, cursor.as_ref()),
        || "timeline query failed".to_string(),
    )?;

    let projections =
        load_snapshot_projections(&db, events.items().iter().map(TimelineEvent::snapshot_id))?;

    let next_cursor = if events.has_more() {
        events
            .items()
            .last()
            .map(|event| encode_timeline_cursor(&filters, timeline_sort, event))
            .transpose()?
    } else {
        None
    };
    let envelope = ListEnvelope {
        schema_version: OUTPUT_SCHEMA_VERSION,
        command: "timeline",
        generated_at: generated_at_now()?,
        applied_filters: merge_applied_filters(
            &filters,
            json!({
                "limit": args.limit,
                "sort": timeline_sort.as_str(),
                "cursor": args.cursor,
            }),
        ),
        truncated: events.has_more(),
        next_cursor,
        results: events
            .items()
            .iter()
            .map(|event| {
                let projection = projections
                    .get(&event.snapshot_id())
                    .cloned()
                    .unwrap_or_default();
                ListRow::from_timeline_event(event, &projection)
            })
            .collect(),
    };
    emit_list_output(format, &envelope)
}

pub(in crate::cli) fn stats(db_path: &Path, args: &StatsArgs) -> Result<()> {
    let format = args.output.resolved()?;
    let filters = normalize_retrieval_filters(&args.filters)?;
    let db = open_existing_db(db_path)?;
    let report = anyhow::Context::context(db.stats(&filters), "stats query failed")?;

    let envelope = StatsEnvelope {
        schema_version: OUTPUT_SCHEMA_VERSION,
        command: "stats",
        generated_at: generated_at_now()?,
        applied_filters: serde_json::to_value(&filters)?,
        stats: report,
    };
    emit_stats_output(format, &envelope)
}

pub(in crate::cli) fn show_snapshot(db_path: &Path, args: &GetArgs) -> Result<()> {
    let format = require_get_output_format(args.output.resolved()?)?;
    let filters = normalize_retrieval_filters(&args.filters)?;
    let db = open_existing_db(db_path)?;
    let snapshot =
        anyhow::Context::with_context(db.find_snapshot(args.snapshot_id, args.events), || {
            format!("get failed for snapshot {}", args.snapshot_id)
        })?
        .ok_or_else(|| not_found_error(format!("snapshot {} was not found", args.snapshot_id)))?;
    if !db.snapshot_matches_filters(args.snapshot_id, &filters)? {
        return Err(anyhow!(
            "snapshot {} does not satisfy the active filters",
            args.snapshot_id
        ));
    }

    let envelope = GetEnvelope {
        schema_version: OUTPUT_SCHEMA_VERSION,
        command: "get",
        generated_at: generated_at_now()?,
        applied_filters: serde_json::to_value(&filters)?,
        snapshot,
    };
    emit_get_output(format, &envelope)
}

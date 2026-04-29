use std::path::Path;

use anyhow::Result;

use crate::cli::formats::OutputFormat;
use crate::cli::formats::ProgressFormat;
use crate::cli::output::print_json_line;
use crate::cli::presentation::emit_json_or_text;
use crate::cli::presentation::{emit_image_optimization_output, emit_storage_compact_output};
use crate::cli::schema::{
    StorageArgs, StorageCommand, StorageCompactArgs, StorageImageCandidatesArgs,
    StorageOptimizeImagesArgs,
};
use crate::db::ImageOptimizationCandidateSummary;

use super::mutation_support::require_text_or_json;
use super::runtime::open_existing_db;

pub(in crate::cli) fn storage(db_path: &Path, args: &StorageArgs) -> Result<()> {
    match &args.command {
        StorageCommand::Compact(args) => storage_compact(db_path, args),
        StorageCommand::ImageCandidates(args) => storage_image_candidates(db_path, args),
        StorageCommand::OptimizeImages(args) => storage_optimize_images(db_path, args),
    }
}

fn storage_compact(db_path: &Path, args: &StorageCompactArgs) -> Result<()> {
    let format = require_text_or_json(args.output.resolved()?, "storage compact")?;
    let mut db = open_existing_db(db_path)?;
    let report = db.compact_storage(args.dry_run)?;
    emit_storage_compact_output(format, &report)
}

fn storage_optimize_images(db_path: &Path, args: &StorageOptimizeImagesArgs) -> Result<()> {
    if matches!(args.progress, Some(ProgressFormat::Jsonl)) {
        let mut db = open_existing_db(db_path)?;
        db.optimize_images_with_progress(args.dry_run, args.limit, !args.no_compact, |event| {
            print_json_line(&event)
        })?;
        return Ok(());
    }

    let format = require_text_or_json(args.output.resolved()?, "storage optimize-images")?;
    let mut db = open_existing_db(db_path)?;
    let report = db.optimize_images(args.dry_run, args.limit, !args.no_compact)?;
    emit_image_optimization_output(format, &report)
}

fn storage_image_candidates(db_path: &Path, args: &StorageImageCandidatesArgs) -> Result<()> {
    let format = require_text_or_json(args.output.resolved()?, "storage image-candidates")?;
    let db = open_existing_db(db_path)?;
    let candidates = db.image_optimization_candidate_summaries(args.limit)?;
    match format {
        OutputFormat::Json => {
            emit_json_or_text(true, &candidates, |rows| render_image_candidates_text(rows))
        }
        OutputFormat::Human | OutputFormat::Text => {
            print!("{}", render_image_candidates_text(&candidates));
            Ok(())
        }
        _ => unreachable!("unsupported storage image-candidates format should be rejected earlier"),
    }
}

fn render_image_candidates_text(candidates: &[ImageOptimizationCandidateSummary]) -> String {
    if candidates.is_empty() {
        return "image candidates=0\n".to_string();
    }
    let mut out = format!("image candidates={}\n", candidates.len());
    for candidate in candidates {
        out.push_str(&format!(
            "snapshot={} item={} uti={} bytes={}\n",
            candidate.snapshot_id(),
            candidate.item_index(),
            candidate.uti(),
            candidate.byte_len()
        ));
    }
    out
}

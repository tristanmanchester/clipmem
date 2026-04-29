use std::path::Path;

use anyhow::Result;

use crate::db::{OcrCandidateSummary, OcrResultRecord, OcrRunReport, OcrStatusReport};

use crate::cli::errors::not_found_error;
use crate::cli::formats::OutputFormat;
use crate::cli::human::{render_ocr_run_human, render_ocr_status_human};
use crate::cli::presentation::emit_json_or_text;
use crate::cli::schema::{
    OcrArgs, OcrCandidatesArgs, OcrClearArgs, OcrCommand, OcrGetArgs, OcrRunArgs, OcrStatusArgs,
};

use super::mutation_support::require_text_or_json;
use super::runtime::open_or_init_db;

pub(in crate::cli) fn ocr(db_path: &Path, args: &OcrArgs) -> Result<()> {
    match &args.command {
        OcrCommand::Status(args) => ocr_status(db_path, args),
        OcrCommand::Candidates(args) => ocr_candidates(db_path, args),
        OcrCommand::Get(args) => ocr_get(db_path, args),
        OcrCommand::Clear(args) => ocr_clear(db_path, args),
        OcrCommand::Run(args) => ocr_run(db_path, args),
    }
}

fn ocr_status(db_path: &Path, args: &OcrStatusArgs) -> Result<()> {
    let format = require_text_or_json(args.output.resolved()?, "ocr status")?;
    let db = open_or_init_db(db_path)?;
    let report = db.ocr_status_report()?;
    match format {
        OutputFormat::Json => emit_json_or_text(true, &report, render_ocr_status_text)?,
        OutputFormat::Human => print!("{}", render_ocr_status_human(&report)),
        OutputFormat::Text => print!("{}", render_ocr_status_text(&report)),
        _ => unreachable!("unsupported ocr status format should be rejected earlier"),
    }
    Ok(())
}

fn ocr_run(db_path: &Path, args: &OcrRunArgs) -> Result<()> {
    let format = require_text_or_json(args.output.resolved()?, "ocr run")?;
    let mut db = open_or_init_db(db_path)?;
    let engine = crate::ocr::default_engine();
    let report = crate::ocr::run_ocr_jobs(
        &mut db,
        &engine,
        args.limit,
        args.snapshot,
        args.retry_failed,
    )?;
    match format {
        OutputFormat::Json => emit_json_or_text(true, &report, render_ocr_run_text)?,
        OutputFormat::Human => print!("{}", render_ocr_run_human(&report)),
        OutputFormat::Text => print!("{}", render_ocr_run_text(&report)),
        _ => unreachable!("unsupported ocr run format should be rejected earlier"),
    }
    Ok(())
}

fn ocr_candidates(db_path: &Path, args: &OcrCandidatesArgs) -> Result<()> {
    let format = require_text_or_json(args.output.resolved()?, "ocr candidates")?;
    let db = open_or_init_db(db_path)?;
    let candidates = db.ocr_candidate_summaries(args.limit, args.snapshot)?;
    match format {
        OutputFormat::Json => {
            emit_json_or_text(true, &candidates, |rows| render_ocr_candidates_text(rows))?
        }
        OutputFormat::Human | OutputFormat::Text => {
            print!("{}", render_ocr_candidates_text(&candidates))
        }
        _ => unreachable!("unsupported ocr candidates format should be rejected earlier"),
    }
    Ok(())
}

fn ocr_get(db_path: &Path, args: &OcrGetArgs) -> Result<()> {
    let format = require_text_or_json(args.output.resolved()?, "ocr get")?;
    let db = open_or_init_db(db_path)?;
    let result = db.ocr_result(&args.raw_sha256)?.ok_or_else(|| {
        not_found_error(format!("no OCR result for raw hash {}", args.raw_sha256))
    })?;
    match format {
        OutputFormat::Json => emit_json_or_text(true, &result, render_ocr_result_text)?,
        OutputFormat::Human | OutputFormat::Text => print!("{}", render_ocr_result_text(&result)),
        _ => unreachable!("unsupported ocr get format should be rejected earlier"),
    }
    Ok(())
}

fn ocr_clear(db_path: &Path, args: &OcrClearArgs) -> Result<()> {
    let format = require_text_or_json(args.output.resolved()?, "ocr clear")?;
    let mut db = open_or_init_db(db_path)?;
    if !db.clear_ocr_result(&args.raw_sha256)? {
        return Err(not_found_error(format!(
            "no OCR result for raw hash {}",
            args.raw_sha256
        )));
    }
    let report = db.ocr_status_report()?;
    match format {
        OutputFormat::Json => emit_json_or_text(true, &report, render_ocr_status_text)?,
        OutputFormat::Human => print!("{}", render_ocr_status_human(&report)),
        OutputFormat::Text => print!("{}", render_ocr_status_text(&report)),
        _ => unreachable!("unsupported ocr clear format should be rejected earlier"),
    }
    Ok(())
}

fn render_ocr_status_text(report: &OcrStatusReport) -> String {
    format!(
        "ocr pending={} ready={} failed={} skipped={} snapshots_with_text={}\n",
        report.pending(),
        report.ready(),
        report.failed(),
        report.skipped(),
        report.snapshots_with_ocr_text()
    )
}

fn render_ocr_run_text(report: &OcrRunReport) -> String {
    format!(
        "ocr processed={} ready={} failed={} skipped={} remaining_pending={}\n",
        report.processed(),
        report.ready(),
        report.failed(),
        report.skipped(),
        report.remaining_pending()
    )
}

fn render_ocr_candidates_text(candidates: &[OcrCandidateSummary]) -> String {
    if candidates.is_empty() {
        return "ocr candidates=0\n".to_string();
    }
    let mut out = format!("ocr candidates={}\n", candidates.len());
    for candidate in candidates {
        out.push_str(&format!(
            "{} bytes={} snapshots={}\n",
            candidate.raw_sha256(),
            candidate.byte_len(),
            candidate.snapshot_count()
        ));
    }
    out
}

fn render_ocr_result_text(result: &OcrResultRecord) -> String {
    format!(
        "ocr raw_sha256={} status={} snapshots={}\n",
        result.raw_sha256(),
        result.status(),
        result.snapshot_count()
    )
}

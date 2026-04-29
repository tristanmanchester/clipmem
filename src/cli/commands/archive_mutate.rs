use std::fs::{File, OpenOptions};
use std::path::Path;

use anyhow::{anyhow, Context, Result};

use crate::db::{PurgeReport, SnapshotDeletionReport};
use crate::platform::restore_items;

use crate::cli::display::format_duration_compact;
use crate::cli::errors::{invalid_args_error, not_found_error};
use crate::cli::formats::OutputFormat;
use crate::cli::human::{
    render_export_human, render_forget_human, render_purge_human, render_restore_human,
};
use crate::cli::output::{ExportOutput, RestoreOutput};
use crate::cli::presentation::emit_json_or_text;
use crate::cli::schema::{ExportArgs, ForgetArgs, PurgeArgs, RestoreArgs};

use super::mutation_support::require_text_or_json;
use super::notify::notify_app_refresh;
use super::retrieval_support::normalize_retrieval_filters;
use super::runtime::open_existing_db;

pub(in crate::cli) fn export_snapshot_bytes(db_path: &Path, args: &ExportArgs) -> Result<()> {
    let format = require_text_or_json(args.output.resolved()?, "export")?;
    let filters = normalize_retrieval_filters(&args.filters)?;
    let db = open_existing_db(db_path)?;
    anyhow::Context::with_context(db.find_snapshot(args.snapshot_id, 1), || {
        format!("export failed for snapshot {}", args.snapshot_id)
    })?
    .ok_or_else(|| not_found_error(format!("snapshot {} was not found", args.snapshot_id)))?;
    if !db.snapshot_matches_filters(args.snapshot_id, &filters)? {
        return Err(anyhow!(
            "snapshot {} does not satisfy the active filters",
            args.snapshot_id
        ));
    }
    let representation = db
        .find_representation_bytes(args.snapshot_id, args.item, &args.uti)?
        .ok_or_else(|| {
            not_found_error(format!(
                "representation not found for snapshot {} item {} uti {}",
                args.snapshot_id, args.item, args.uti
            ))
        })?;

    if let Some(parent) = args.out.parent() {
        anyhow::Context::with_context(std::fs::create_dir_all(parent), || {
            format!("failed to create {}", parent.display())
        })?;
    }

    let mut output = create_export_destination(&args.out, args.force)?;
    use std::io::Write as _;
    anyhow::Context::with_context(output.write_all(representation.raw_bytes()), || {
        format!("failed to write export file {}", args.out.display())
    })?;

    let output = ExportOutput {
        snapshot_id: args.snapshot_id,
        item_index: args.item,
        uti: args.uti.clone(),
        byte_count: representation.byte_len(),
        raw_sha256: representation.raw_sha256().to_string(),
        out: args.out.display().to_string(),
    };
    match format {
        OutputFormat::Json => emit_json_or_text(true, &output, render_export_text)?,
        OutputFormat::Human => print!("{}", render_export_human(&output)),
        OutputFormat::Text => print!("{}", render_export_text(&output)),
        _ => unreachable!("unsupported export format should be rejected earlier"),
    }

    Ok(())
}

pub(in crate::cli) fn restore_snapshot(db_path: &Path, args: &RestoreArgs) -> Result<()> {
    let format = require_text_or_json(args.output.resolved()?, "restore")?;
    let db = open_existing_db(db_path)?;
    let snapshot = anyhow::Context::with_context(db.find_snapshot(args.snapshot_id, 1), || {
        format!("restore failed for snapshot {}", args.snapshot_id)
    })?
    .ok_or_else(|| not_found_error(format!("snapshot {} was not found", args.snapshot_id)))?;
    db.register_pending_restore(snapshot.sha256())
        .context("register pending restore")?;
    let report = match restore_items(snapshot.items()) {
        Ok(report) => report,
        Err(err) => {
            if let Err(clear_err) = db.clear_pending_restore(snapshot.sha256()) {
                eprintln!("pending restore cleanup failed: {clear_err:#}");
            }
            return Err(err).context("restore failed");
        }
    };
    let output = RestoreOutput {
        snapshot_id: args.snapshot_id,
        item_count: report.item_count(),
        representation_count: report.representation_count(),
        total_bytes: report.total_bytes(),
    };
    match format {
        OutputFormat::Json => emit_json_or_text(true, &output, render_restore_text)?,
        OutputFormat::Human => print!("{}", render_restore_human(&output)),
        OutputFormat::Text => print!("{}", render_restore_text(&output)),
        _ => unreachable!("unsupported restore format should be rejected earlier"),
    }
    notify_app_refresh();

    Ok(())
}

pub(in crate::cli) fn forget_snapshot(db_path: &Path, args: &ForgetArgs) -> Result<()> {
    let format = require_text_or_json(args.output.resolved()?, "forget")?;
    let mut db = open_existing_db(db_path)?;
    let report = anyhow::Context::with_context(db.forget_snapshot(args.snapshot_id), || {
        format!("forget failed for snapshot {}", args.snapshot_id)
    })?
    .ok_or_else(|| not_found_error(format!("snapshot {} was not found", args.snapshot_id)))?;
    match format {
        OutputFormat::Json => emit_json_or_text(true, &report, render_forget_text)?,
        OutputFormat::Human => print!("{}", render_forget_human(&report)),
        OutputFormat::Text => print!("{}", render_forget_text(&report)),
        _ => unreachable!("unsupported forget format should be rejected earlier"),
    }
    notify_app_refresh();

    Ok(())
}

pub(in crate::cli) fn purge_snapshots(db_path: &Path, args: &PurgeArgs) -> Result<()> {
    let format = require_text_or_json(args.output.resolved()?, "purge")?;
    let mut db = open_existing_db(db_path)?;
    let report = anyhow::Context::with_context(
        db.purge_snapshots_older_than(args.older_than.seconds(), args.dry_run),
        || format!("purge failed for duration {}", args.older_than.raw()),
    )?;
    match format {
        OutputFormat::Json => emit_json_or_text(true, &report, render_purge_text)?,
        OutputFormat::Human => print!("{}", render_purge_human(&report)),
        OutputFormat::Text => print!("{}", render_purge_text(&report)),
        _ => unreachable!("unsupported purge format should be rejected earlier"),
    }
    if !args.dry_run && report.snapshot_count() > 0 {
        notify_app_refresh();
    }

    Ok(())
}

fn create_export_destination(path: &Path, force: bool) -> Result<File> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            let file_type = metadata.file_type();
            if file_type.is_symlink() {
                return Err(invalid_args_error(format!(
                    "export destination {} is a symbolic link; choose a regular file path instead",
                    path.display()
                )));
            }
            if !metadata.is_file() {
                return Err(invalid_args_error(format!(
                    "export destination {} is not a regular file",
                    path.display()
                )));
            }
            if !force {
                return Err(invalid_args_error(format!(
                    "export destination {} already exists (pass --force to replace it)",
                    path.display()
                )));
            }

            std::fs::remove_file(path)
                .with_context(|| format!("failed to replace {}", path.display()))?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error).with_context(|| format!("failed to inspect {}", path.display()));
        }
    }

    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("failed to create export file {}", path.display()))
}

fn render_restore_text(output: &RestoreOutput) -> String {
    format!(
        "restored snapshot={} items={} representations={} bytes={}\n",
        output.snapshot_id, output.item_count, output.representation_count, output.total_bytes
    )
}

fn render_export_text(output: &ExportOutput) -> String {
    format!(
        "exported snapshot={} item={} uti={} bytes={} sha256={} out={}\n",
        output.snapshot_id,
        output.item_index,
        output.uti,
        output.byte_count,
        output.raw_sha256,
        output.out
    )
}

fn render_forget_text(report: &SnapshotDeletionReport) -> String {
    format!(
        "forgot snapshot={} items={} representations={} capture_events={} bytes={}\n",
        report.snapshot_id(),
        report.item_count(),
        report.representation_count(),
        report.capture_event_count(),
        report.total_bytes()
    )
}

fn render_purge_text(report: &PurgeReport) -> String {
    let action = if report.dry_run() {
        "purge dry-run"
    } else {
        "purged"
    };
    format!(
        "{} older_than={} snapshots={} items={} representations={} capture_events={} bytes={}\n",
        action,
        format_duration_compact(report.older_than_seconds()),
        report.snapshot_count(),
        report.item_count(),
        report.representation_count(),
        report.capture_event_count(),
        report.total_bytes()
    )
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::create_export_destination;

    #[test]
    fn create_export_destination_rejects_existing_file_without_force() {
        let path = temp_path("existing-export.bin");
        std::fs::write(&path, b"old").unwrap();

        let error = create_export_destination(&path, false).unwrap_err();

        assert!(error.to_string().contains("already exists"));
        assert_eq!(std::fs::read(&path).unwrap(), b"old");

        cleanup_path(&path);
    }

    #[test]
    fn create_export_destination_replaces_existing_file_with_force() {
        let path = temp_path("forced-export.bin");
        std::fs::write(&path, b"old").unwrap();

        let mut file = create_export_destination(&path, true).unwrap();
        file.write_all(b"new").unwrap();
        drop(file);

        assert_eq!(std::fs::read(&path).unwrap(), b"new");

        cleanup_path(&path);
    }

    #[test]
    fn create_export_destination_rejects_directory() {
        let path = temp_path("export-directory");
        std::fs::create_dir_all(&path).unwrap();

        let error = create_export_destination(&path, true).unwrap_err();

        assert!(error.to_string().contains("not a regular file"));

        cleanup_path(&path);
    }

    #[cfg(unix)]
    #[test]
    fn create_export_destination_rejects_symlink() {
        let target = temp_path("export-target.bin");
        let link = temp_path("export-link.bin");
        std::fs::write(&target, b"target").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let error = create_export_destination(&link, true).unwrap_err();

        assert!(error.to_string().contains("symbolic link"));
        assert_eq!(std::fs::read(&target).unwrap(), b"target");

        cleanup_path(&link);
        cleanup_path(&target);
    }

    fn temp_path(name: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();

        let dir = std::env::temp_dir()
            .join("clipmem-archive-mutate-tests")
            .join(format!("{}-{timestamp}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    fn cleanup_path(path: &std::path::Path) {
        if let Ok(metadata) = std::fs::symlink_metadata(path) {
            if metadata.is_dir() && !metadata.file_type().is_symlink() {
                let _ = std::fs::remove_dir_all(path);
            } else {
                let _ = std::fs::remove_file(path);
            }
        }
        if let Some(parent) = path.parent() {
            let _ = std::fs::remove_dir_all(parent);
        }
    }
}

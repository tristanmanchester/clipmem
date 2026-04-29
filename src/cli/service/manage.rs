use std::env;
use std::fs;
use std::path::Path;
use std::process::Command as ProcessCommand;

use anyhow::{anyhow, bail, Context, Result};

use crate::db::{CaptureStoreOutcome, Database};
use crate::platform::capture_snapshot;

use super::context::{build_context, conflict_message, ensure_supported, select_provider};
use super::launchctl::{
    ensure_parent_dir, launchctl_bootout, launchctl_bootstrap, launchctl_disable, launchctl_enable,
    launchctl_kickstart, run_command_checked, touch_log_file, write_direct_plist,
};
use super::model::*;
use super::status::status_report;

pub(crate) fn setup(db_path: &Path) -> Result<SetupReport> {
    ensure_supported()?;
    let context = build_context(db_path)?;
    let status = status_report(db_path)?;
    let selection = select_provider(&context);
    ensure_no_conflict(&status)?;
    let seed_capture = seed_capture(db_path)?;
    let action = start_with_provider(&context, &selection)?;
    bump_service_revision(db_path)?;
    Ok(SetupReport {
        seed_capture,
        action,
    })
}

pub(crate) fn start(db_path: &Path) -> Result<ServiceActionReport> {
    ensure_supported()?;
    let context = build_context(db_path)?;
    let status = status_report(db_path)?;
    let selection = select_provider(&context);
    ensure_no_conflict(&status)?;
    let report = start_with_provider(&context, &selection)?;
    bump_service_revision(db_path)?;
    Ok(report)
}

pub(crate) fn stop(db_path: &Path) -> Result<ServiceActionReport> {
    ensure_supported()?;
    let context = build_context(db_path)?;
    let status = status_report(db_path)?;
    if status.conflict {
        bail!(conflict_message());
    }

    let report = if status.homebrew.installed || status.homebrew.loaded || status.homebrew.running {
        stop_homebrew_provider(&context)?
    } else {
        stop_direct_provider(&context)?
    };
    bump_service_revision(db_path)?;
    Ok(report)
}

pub(crate) fn uninstall(db_path: &Path) -> Result<ServiceActionReport> {
    ensure_supported()?;
    let context = build_context(db_path)?;
    let status = status_report(db_path)?;
    if status.conflict {
        bail!(conflict_message());
    }

    let report = if status.homebrew.installed || status.homebrew.loaded || status.homebrew.running {
        uninstall_homebrew_provider(&context)?
    } else {
        uninstall_direct_provider(&context)?
    };
    bump_service_revision(db_path)?;
    Ok(report)
}

fn ensure_no_conflict(status: &ServiceStatusReport) -> Result<()> {
    if status.conflict {
        bail!(conflict_message());
    }
    Ok(())
}

fn start_with_provider(
    context: &ServiceContext,
    selection: &ProviderSelection,
) -> Result<ServiceActionReport> {
    match selection.provider {
        ServiceProvider::Homebrew => start_homebrew_provider(context, &selection.notes),
        ServiceProvider::Launchagent => start_direct_provider(context, &selection.notes),
    }
}

fn start_homebrew_provider(
    context: &ServiceContext,
    notes: &[String],
) -> Result<ServiceActionReport> {
    let brew = context
        .brew_path
        .as_ref()
        .ok_or_else(|| anyhow!("`brew` is not available on PATH"))?;
    let output = ProcessCommand::new(brew)
        .args(["services", "start", "clipmem"])
        .output()
        .context("run `brew services start clipmem`")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if brew_services_formula_unavailable(&stderr) {
            let mut fallback_notes = notes.to_vec();
            fallback_notes.push(
                "`brew services` is installed, but the clipmem formula does not expose a service file; using a direct LaunchAgent with the Homebrew binary.".to_string(),
            );
            return start_direct_provider(context, &fallback_notes);
        }

        run_command_checked(Ok(output), "brew services start clipmem")?;
    }
    Ok(ServiceActionReport {
        action: "start",
        provider: ServiceProvider::Homebrew,
        binary_path: context.binary_path.clone(),
        db_path: context.default_db_path.clone(),
        label: HOMEBREW_LABEL,
        notes: notes.to_vec(),
    })
}

fn brew_services_formula_unavailable(stderr: &str) -> bool {
    stderr.contains("has not implemented #plist")
        || stderr.contains("has not implemented #service")
        || stderr.contains("provided a locatable service file")
}

fn stop_homebrew_provider(context: &ServiceContext) -> Result<ServiceActionReport> {
    let brew = context
        .brew_path
        .as_ref()
        .ok_or_else(|| anyhow!("`brew` is not available on PATH"))?;
    run_command_checked(
        ProcessCommand::new(brew)
            .args(["services", "stop", "clipmem"])
            .output(),
        "brew services stop clipmem",
    )?;
    Ok(ServiceActionReport {
        action: "stop",
        provider: ServiceProvider::Homebrew,
        binary_path: context.binary_path.clone(),
        db_path: context.default_db_path.clone(),
        label: HOMEBREW_LABEL,
        notes: vec!["The Homebrew formula remains installed.".to_string()],
    })
}

fn uninstall_homebrew_provider(context: &ServiceContext) -> Result<ServiceActionReport> {
    let mut report = stop_homebrew_provider(context)?;
    report.action = "uninstall";
    report.notes.push(
        "Only the Homebrew-managed service was removed; the formula stays installed.".to_string(),
    );
    Ok(report)
}

fn start_direct_provider(
    context: &ServiceContext,
    notes: &[String],
) -> Result<ServiceActionReport> {
    ensure_parent_dir(&context.direct_plist_path)?;
    ensure_parent_dir(&context.direct_stdout_path)?;
    touch_log_file(&context.direct_stdout_path)?;
    touch_log_file(&context.direct_stderr_path)?;
    write_direct_plist(context)?;
    launchctl_bootout(DIRECT_LABEL)?;
    launchctl_enable(DIRECT_LABEL)?;
    launchctl_bootstrap(&context.direct_plist_path)?;
    launchctl_kickstart(DIRECT_LABEL)?;
    Ok(ServiceActionReport {
        action: "start",
        provider: ServiceProvider::Launchagent,
        binary_path: context.binary_path.clone(),
        db_path: context.db_path.clone(),
        label: DIRECT_LABEL,
        notes: notes.to_vec(),
    })
}

fn stop_direct_provider(context: &ServiceContext) -> Result<ServiceActionReport> {
    launchctl_bootout(DIRECT_LABEL)?;
    launchctl_disable(DIRECT_LABEL)?;
    Ok(ServiceActionReport {
        action: "stop",
        provider: ServiceProvider::Launchagent,
        binary_path: context.binary_path.clone(),
        db_path: context.db_path.clone(),
        label: DIRECT_LABEL,
        notes: vec!["The LaunchAgent plist remains installed.".to_string()],
    })
}

fn uninstall_direct_provider(context: &ServiceContext) -> Result<ServiceActionReport> {
    launchctl_bootout(DIRECT_LABEL)?;
    launchctl_disable(DIRECT_LABEL)?;
    let mut notes = Vec::new();
    match fs::remove_file(&context.direct_plist_path) {
        Ok(()) => notes.push("Removed the direct LaunchAgent plist.".to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            notes.push("The direct LaunchAgent plist was already absent.".to_string());
        }
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "remove direct LaunchAgent plist {}",
                    context.direct_plist_path.display()
                )
            });
        }
    }
    Ok(ServiceActionReport {
        action: "uninstall",
        provider: ServiceProvider::Launchagent,
        binary_path: context.binary_path.clone(),
        db_path: context.db_path.clone(),
        label: DIRECT_LABEL,
        notes,
    })
}

fn seed_capture(db_path: &Path) -> Result<SeedCaptureOutcome> {
    if env::var_os("CLIPMEM_TEST_SKIP_SETUP_CAPTURE_ONCE").as_deref() == Some("1".as_ref()) {
        return Ok(SeedCaptureOutcome::NotAttempted);
    }

    let mut db = Database::open_or_init(db_path)?;
    let snapshot = capture_snapshot().context("setup clipboard read failed")?;
    match db
        .store_capture_if_allowed(&snapshot)
        .context("setup database write failed")?
    {
        CaptureStoreOutcome::Stored(_) => Ok(SeedCaptureOutcome::Stored),
        CaptureStoreOutcome::Skipped(reason) => Ok(SeedCaptureOutcome::Skipped(reason)),
    }
}

fn bump_service_revision(db_path: &Path) -> Result<()> {
    Database::open_or_init(db_path)?
        .bump_service_revision()
        .map(|_| ())
        .context("record service revision")
}

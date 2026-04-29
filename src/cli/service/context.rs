use std::path::Path;
use std::process::Command as ProcessCommand;

use anyhow::{anyhow, bail, Result};

use crate::cli::db_path::default_db_path;

use super::launchctl::{
    brew_services_available, find_executable, home_dir, homebrew_prefix_for_binary,
    trusted_binary_path,
};
use super::model::{
    ProviderSelection, ServiceContext, ServiceProvider, DIRECT_LABEL, HOMEBREW_LABEL,
};

pub(super) fn ensure_supported() -> Result<()> {
    if cfg!(target_os = "macos") {
        Ok(())
    } else {
        bail!("clipmem setup and service commands are only supported on macOS");
    }
}

pub(super) fn build_context(db_path: &Path) -> Result<ServiceContext> {
    let home = home_dir()?;
    let binary_path = trusted_binary_path()?;
    let direct_plist_path = home
        .join("Library/LaunchAgents")
        .join(format!("{DIRECT_LABEL}.plist"));
    let homebrew_plist_path = home
        .join("Library/LaunchAgents")
        .join(format!("{HOMEBREW_LABEL}.plist"));
    let app_support_dir = db_path.parent().ok_or_else(|| {
        anyhow!(
            "database path {} has no parent directory",
            db_path.display()
        )
    })?;
    let log_dir = app_support_dir.join("logs");
    let brew_path = find_executable("brew");
    let homebrew_prefix = homebrew_prefix_for_binary(&binary_path);

    Ok(ServiceContext {
        binary_path,
        db_path: db_path.to_path_buf(),
        default_db_path: default_db_path(),
        direct_plist_path,
        homebrew_plist_path,
        direct_stdout_path: log_dir.join("clipmem.stdout.log"),
        direct_stderr_path: log_dir.join("clipmem.stderr.log"),
        brew_path,
        homebrew_prefix,
    })
}

pub(super) fn select_provider(context: &ServiceContext) -> ProviderSelection {
    let mut notes = Vec::new();
    if context.db_path != context.default_db_path {
        notes.push(format!(
            "Direct LaunchAgent provider will use the explicit database path {}.",
            context.db_path.display()
        ));
        return ProviderSelection {
            provider: ServiceProvider::Launchagent,
            reason: "explicit database paths require the direct per-user LaunchAgent".to_string(),
            notes,
        };
    }

    if let Some(brew_path) = &context.brew_path {
        if context.homebrew_prefix.is_some() {
            if brew_services_available(brew_path) && brew_services_can_manage_clipmem(brew_path) {
                return ProviderSelection {
                    provider: ServiceProvider::Homebrew,
                    reason:
                        "current binary is the Homebrew formula and `brew services` is available"
                            .to_string(),
                    notes,
                };
            }
            notes.push(
                "Homebrew install detected, but `brew services` cannot manage the clipmem formula; falling back to a direct LaunchAgent.".to_string(),
            );
        }
    }

    ProviderSelection {
        provider: ServiceProvider::Launchagent,
        reason: "using the direct per-user LaunchAgent managed by clipmem".to_string(),
        notes,
    }
}

fn brew_services_can_manage_clipmem(brew: &Path) -> bool {
    let Ok(output) = ProcessCommand::new(brew)
        .args(["services", "info", "clipmem", "--json"])
        .output()
    else {
        return false;
    };
    if !output.status.success() {
        return false;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.contains("\"schedulable\":true") || stdout.contains("\"schedulable\": true")
}

pub(super) fn conflict_message() -> String {
    "Both the Homebrew service and the direct LaunchAgent are installed. Remove one first with `brew services stop clipmem` or `clipmem service uninstall`.".to_string()
}

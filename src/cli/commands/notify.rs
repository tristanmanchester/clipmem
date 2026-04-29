#[cfg(target_os = "macos")]
pub(in crate::cli) const APP_REFRESH_NOTIFICATION: &str = "io.openclaw.clipmem.revision.changed";

pub(in crate::cli) fn notify_app_refresh() {
    #[cfg(target_os = "macos")]
    {
        if std::env::var_os("CLIPMEM_DISABLE_APP_REFRESH_NOTIFY").is_some() {
            return;
        }
        let _ = std::process::Command::new("/usr/bin/notifyutil")
            .args(["-p", APP_REFRESH_NOTIFICATION])
            .spawn();
    }
}

use std::path::{Path, PathBuf};

use crate::archive::Database;

use super::launchctl::homebrew_prefix_for_binary;
use super::manage::bump_service_revision;
use super::model::{ServiceProvider, ServiceProviderStatus, ServiceState};
use super::status::{
    command_binary_path, configured_binary_path_from_plist, configured_binary_path_from_plist_xml,
    watcher_binary_mismatch_note,
};

#[test]
fn homebrew_prefix_matches_opt_homebrew_wrapper_path() {
    assert_eq!(
        homebrew_prefix_for_binary(Path::new("/opt/homebrew/bin/clipmem")),
        Some(PathBuf::from("/opt/homebrew"))
    );
}

#[test]
fn homebrew_prefix_matches_usr_local_wrapper_path() {
    assert_eq!(
        homebrew_prefix_for_binary(Path::new("/usr/local/bin/clipmem")),
        Some(PathBuf::from("/usr/local"))
    );
}

#[test]
fn homebrew_prefix_matches_opt_homebrew_cellar_path() {
    assert_eq!(
        homebrew_prefix_for_binary(Path::new("/opt/homebrew/Cellar/clipmem/1.2.3/bin/clipmem")),
        Some(PathBuf::from("/opt/homebrew"))
    );
}

#[test]
fn homebrew_prefix_matches_usr_local_cellar_path() {
    assert_eq!(
        homebrew_prefix_for_binary(Path::new("/usr/local/Cellar/clipmem/1.2.3/bin/clipmem")),
        Some(PathBuf::from("/usr/local"))
    );
}

#[test]
fn homebrew_prefix_rejects_non_homebrew_paths() {
    assert_eq!(
        homebrew_prefix_for_binary(Path::new("/Users/tristan/.cargo/bin/clipmem")),
        None
    );
}

#[test]
fn service_revision_recording_is_best_effort_for_missing_archives() {
    let path = std::env::temp_dir().join(format!(
        "clipmem-service-test-{}-missing.sqlite3",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);

    bump_service_revision(&path);

    assert!(!path.exists());
}

#[test]
fn service_revision_recording_updates_existing_archives() {
    let dir = std::env::temp_dir().join(format!(
        "clipmem-service-test-{}-existing",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("test database directory should initialize");
    let path = dir.join("clipmem.sqlite3");
    let _ = std::fs::remove_file(&path);
    let db = Database::open_or_init(&path).expect("test database should initialize");

    bump_service_revision(&path);

    assert_eq!(
        db.archive_revision()
            .expect("archive revision should load")
            .service_revision(),
        1
    );

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn command_binary_path_reads_first_program_argument() {
    assert_eq!(
        command_binary_path("/Users/test/clipmem/target/debug/clipmem watch --skip-initial"),
        Some("/Users/test/clipmem/target/debug/clipmem".to_string())
    );
    assert_eq!(
        command_binary_path("\"/Users/test/Clip Mem/clipmem\" watch"),
        Some("/Users/test/Clip Mem/clipmem".to_string())
    );
}

#[test]
fn launchagent_plist_reports_configured_binary_path() {
    let path = std::env::temp_dir().join(format!(
        "clipmem-service-test-{}-{}.plist",
        std::process::id(),
        "configured-binary"
    ));
    std::fs::write(
            &path,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
  <dict>
    <key>ProgramArguments</key>
    <array>
      <string>/tmp/clipmem-debug</string>
      <string>watch</string>
    </array>
  </dict>
</plist>
"#,
        )
        .expect("write test plist");

    assert_eq!(
        configured_binary_path_from_plist(&path),
        Some("/tmp/clipmem-debug".to_string())
    );

    let _ = std::fs::remove_file(path);
}

#[test]
fn launchagent_plist_xml_fallback_reports_configured_binary_path() {
    let path = std::env::temp_dir().join(format!(
        "clipmem-service-test-{}-{}.plist",
        std::process::id(),
        "configured-binary-xml"
    ));
    std::fs::write(
        &path,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
  <dict>
    <key>ProgramArguments</key>
    <array>
      <string>/tmp/clipmem&amp;debug</string>
      <string>watch</string>
    </array>
  </dict>
</plist>
"#,
    )
    .expect("write test plist");

    assert_eq!(
        configured_binary_path_from_plist_xml(&path),
        Some("/tmp/clipmem&debug".to_string())
    );

    let _ = std::fs::remove_file(path);
}

#[test]
fn watcher_binary_mismatch_reports_configured_path() {
    let provider = ServiceProviderStatus {
        provider: ServiceProvider::Launchagent,
        label: "io.openclaw.clipmem.watch".to_string(),
        state: ServiceState::Running,
        installed: true,
        loaded: true,
        running: true,
        pid: Some(123),
        plist_path: None,
        configured_binary_path: Some("/opt/homebrew/bin/clipmem".to_string()),
        running_command: None,
        running_binary_path: None,
        stdout_log_path: None,
        stderr_log_path: None,
    };

    let note = watcher_binary_mismatch_note(
        Path::new("/Users/test/clipmem/target/debug/clipmem"),
        [&provider],
    )
    .expect("different watcher binary should be reported");

    assert!(note.contains("launchagent watcher uses /opt/homebrew/bin/clipmem"));
    assert!(note.contains("/Users/test/clipmem/target/debug/clipmem"));
}

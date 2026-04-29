pub(crate) use std::fs;
pub(crate) use std::io::Cursor;
pub(crate) use std::path::{Path, PathBuf};
pub(crate) use std::process;
pub(crate) use std::process::Command;
pub(crate) use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) use anyhow::Result;
pub(crate) use clipmem::archive::Database;
pub(crate) use clipmem::capture::{
    build_item, build_representation, build_snapshot, CaptureContext, ClipboardSnapshot,
};
pub(crate) use image::{ImageFormat, Rgba, RgbaImage};
pub(crate) use rusqlite::Connection;
pub(crate) use serde_json::Value;

pub(crate) fn temp_db_path(test_name: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();

    let dir = std::env::temp_dir()
        .join("clipmem-cli-tests")
        .join(format!("{test_name}-{}-{timestamp}", process::id()));
    fs::create_dir_all(&dir).expect("temporary test directory should be created");
    dir.join("database.sqlite3")
}

pub(crate) fn temp_artifact_path(test_name: &str, suffix: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();

    let dir = std::env::temp_dir()
        .join("clipmem-cli-tests")
        .join(format!("{test_name}-{}-{timestamp}", process::id()));
    fs::create_dir_all(&dir).expect("temporary test directory should be created");
    dir.join(format!("artifact{suffix}"))
}

pub(crate) fn cleanup_db(path: &Path) {
    for suffix in ["", "-shm", "-wal"] {
        let candidate = if suffix.is_empty() {
            path.to_path_buf()
        } else {
            PathBuf::from(format!("{}{suffix}", path.display()))
        };
        let _ = fs::remove_file(candidate);
    }

    if let Some(parent) = path.parent() {
        let _ = fs::remove_dir(parent);
    }
}

pub(crate) fn cleanup_temp_artifact(path: &Path) {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            let _ = fs::remove_dir_all(path);
        } else {
            let _ = fs::remove_file(path);
        }
    }

    if let Some(parent) = path.parent() {
        let _ = fs::remove_dir(parent);
    }
}

pub(crate) fn text_snapshot(change_count: i64, text: &str) -> ClipboardSnapshot {
    app_text_snapshot(change_count, "Terminal", "com.apple.Terminal", text)
}

pub(crate) fn app_text_snapshot(
    change_count: i64,
    app_name: &str,
    app_bundle_id: &str,
    text: &str,
) -> ClipboardSnapshot {
    let item = build_item(
        0,
        vec![build_representation(
            "public.utf8-plain-text".to_string(),
            None,
            text.as_bytes().to_vec(),
        )],
    );

    build_snapshot(
        CaptureContext::new(change_count)
            .with_frontmost_app_name(app_name)
            .with_frontmost_app_bundle_id(app_bundle_id),
        vec![item],
    )
}

pub(crate) fn rich_snapshot(
    change_count: i64,
    text: &str,
    url: &str,
    file_url: &str,
) -> ClipboardSnapshot {
    let item = build_item(
        0,
        vec![
            build_representation(
                "public.utf8-plain-text".to_string(),
                Some(text.to_string()),
                text.as_bytes().to_vec(),
            ),
            build_representation(
                "public.url".to_string(),
                Some(url.to_string()),
                url.as_bytes().to_vec(),
            ),
            build_representation(
                "public.file-url".to_string(),
                Some(file_url.to_string()),
                file_url.as_bytes().to_vec(),
            ),
        ],
    );

    build_snapshot(
        CaptureContext::new(change_count)
            .with_frontmost_app_name("Terminal")
            .with_frontmost_app_bundle_id("com.apple.Terminal"),
        vec![item],
    )
}

pub(crate) fn app_rich_snapshot(
    change_count: i64,
    app_name: &str,
    app_bundle_id: &str,
    text: &str,
    url: &str,
    file_url: &str,
) -> ClipboardSnapshot {
    let item = build_item(
        0,
        vec![
            build_representation(
                "public.utf8-plain-text".to_string(),
                Some(text.to_string()),
                text.as_bytes().to_vec(),
            ),
            build_representation(
                "public.url".to_string(),
                Some(url.to_string()),
                url.as_bytes().to_vec(),
            ),
            build_representation(
                "public.file-url".to_string(),
                Some(file_url.to_string()),
                file_url.as_bytes().to_vec(),
            ),
        ],
    );

    build_snapshot(
        CaptureContext::new(change_count)
            .with_frontmost_app_name(app_name)
            .with_frontmost_app_bundle_id(app_bundle_id),
        vec![item],
    )
}

pub(crate) fn html_snapshot(change_count: i64, html: &str) -> ClipboardSnapshot {
    let item = build_item(
        0,
        vec![build_representation(
            "public.html".to_string(),
            Some(html.to_string()),
            html.as_bytes().to_vec(),
        )],
    );

    build_snapshot(
        CaptureContext::new(change_count)
            .with_frontmost_app_name("Safari")
            .with_frontmost_app_bundle_id("com.apple.Safari"),
        vec![item],
    )
}

pub(crate) fn rtf_snapshot(change_count: i64, rtf: &str) -> ClipboardSnapshot {
    let item = build_item(
        0,
        vec![build_representation(
            "public.rtf".to_string(),
            Some(rtf.to_string()),
            rtf.as_bytes().to_vec(),
        )],
    );

    build_snapshot(
        CaptureContext::new(change_count)
            .with_frontmost_app_name("TextEdit")
            .with_frontmost_app_bundle_id("com.apple.TextEdit"),
        vec![item],
    )
}

pub(crate) fn image_snapshot(change_count: i64, bytes: &[u8]) -> ClipboardSnapshot {
    let item = build_item(
        0,
        vec![build_representation(
            "public.png".to_string(),
            None,
            bytes.to_vec(),
        )],
    );

    build_snapshot(
        CaptureContext::new(change_count)
            .with_frontmost_app_name("Preview")
            .with_frontmost_app_bundle_id("com.apple.Preview"),
        vec![item],
    )
}

pub(crate) fn lossless_test_tiff() -> Result<Vec<u8>> {
    let mut image = RgbaImage::new(256, 256);
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        let alpha = if (x + y) % 3 == 0 { 96 } else { 255 };
        *pixel = Rgba([(x % 16) as u8, (y % 16) as u8, ((x + y) % 16) as u8, alpha]);
    }

    let mut out = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image).write_to(&mut out, ImageFormat::Tiff)?;
    Ok(out.into_inner())
}

pub(crate) fn seed_database(path: &Path, snapshots: &[ClipboardSnapshot]) -> Result<Vec<i64>> {
    let mut db = Database::open_or_init(path)?;
    let mut ids = Vec::new();

    for snapshot in snapshots {
        let stored = db.store_capture(snapshot)?;
        ids.push(stored.snapshot_id());
    }

    Ok(ids)
}

pub(crate) fn seed_events(path: &Path, snapshots: &[ClipboardSnapshot]) -> Result<Vec<(i64, i64)>> {
    let mut db = Database::open_or_init(path)?;
    let mut ids = Vec::new();

    for snapshot in snapshots {
        let stored = db.store_capture(snapshot)?;
        ids.push((stored.snapshot_id(), stored.event_id()));
    }

    Ok(ids)
}

pub(crate) fn set_event_observed_at(path: &Path, event_id: i64, observed_at: &str) -> Result<()> {
    let conn = Connection::open(path)?;
    conn.execute(
        "UPDATE capture_events SET observed_at = ?1 WHERE id = ?2",
        (observed_at, event_id),
    )?;
    Ok(())
}

pub(crate) fn run_cli(args: &[&str]) -> process::Output {
    Command::new(env!("CARGO_BIN_EXE_clipmem"))
        .args(args)
        .output()
        .expect("clipmem binary should execute")
}

pub(crate) fn run_cli_with_env(args: &[&str], envs: &[(&str, &str)]) -> process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_clipmem"));
    command.args(args);
    for (key, value) in envs {
        command.env(key, value);
    }
    command
        .output()
        .expect("clipmem binary should execute with env")
}

pub(crate) fn run_cli_with_owned_env(args: &[&str], envs: &[(String, String)]) -> process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_clipmem"));
    command.args(args);
    for (key, value) in envs {
        command.env(key, value);
    }
    command
        .output()
        .expect("clipmem binary should execute with owned env")
}

pub(crate) fn run_command_with_env(
    command_path: &Path,
    args: &[&str],
    envs: &[(&str, &str)],
) -> process::Output {
    let mut command = Command::new(command_path);
    command.args(args);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().unwrap_or_else(|error| {
        panic!(
            "{} should execute with env: {}",
            command_path.display(),
            error
        )
    })
}

pub(crate) fn stdout_text(output: &process::Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be UTF-8")
}

pub(crate) fn stderr_text(output: &process::Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be UTF-8")
}

pub(crate) fn status_code(output: &process::Output) -> i32 {
    output
        .status
        .code()
        .expect("process should exit with an explicit status code")
}

pub(crate) fn assert_no_ansi(text: &str) {
    assert!(
        !text.contains("\u{1b}["),
        "captured human output should not contain ANSI escape codes:\n{text}"
    );
}

pub(crate) fn assert_human_output(text: &str, title: &str) {
    assert!(text.contains(title), "missing title {title:?}\n{text}");
    assert!(text.contains('═'), "missing heavy separator\n{text}");
    assert!(
        !text.contains("\"schema_version\""),
        "human output should not contain JSON envelope markers\n{text}"
    );
    assert_no_ansi(text);
}

pub(crate) fn temp_test_dir(test_name: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();

    std::env::temp_dir()
        .join("clipmem-agent-tests")
        .join(format!("{test_name}-{}-{timestamp}", process::id()))
}

#[cfg(unix)]
pub(crate) fn write_executable(path: &Path, content: &str) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::write(path, content)?;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
pub(crate) fn write_executable(path: &Path, content: &str) -> Result<()> {
    fs::write(path, content)?;
    Ok(())
}

#[cfg(unix)]
pub(crate) fn set_mode(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(mode);
    fs::set_permissions(path, perms)?;
    Ok(())
}

pub(crate) fn write_openclaw_skill_package(skill_dir: &Path) -> Result<()> {
    fs::create_dir_all(skill_dir.join("references"))?;
    fs::create_dir_all(skill_dir.join("scripts"))?;
    fs::write(
        skill_dir.join("SKILL.md"),
        include_str!("../../extras/openclaw/clipboard-memory/SKILL.md"),
    )?;
    fs::write(
        skill_dir.join("references/commands.md"),
        include_str!("../../extras/openclaw/clipboard-memory/references/commands.md"),
    )?;
    fs::write(
        skill_dir.join("references/troubleshooting.md"),
        include_str!("../../extras/openclaw/clipboard-memory/references/troubleshooting.md"),
    )?;
    fs::write(
        skill_dir.join("references/json-schema.md"),
        include_str!("../../extras/openclaw/clipboard-memory/references/json-schema.md"),
    )?;
    fs::write(
        skill_dir.join("references/examples.md"),
        include_str!("../../extras/openclaw/clipboard-memory/references/examples.md"),
    )?;
    fs::write(
        skill_dir.join("references/setup-check.md"),
        include_str!("../../extras/openclaw/clipboard-memory/references/setup-check.md"),
    )?;
    write_executable(
        &skill_dir.join("scripts/check-setup.sh"),
        include_str!("../../extras/openclaw/clipboard-memory/scripts/check-setup.sh"),
    )?;
    Ok(())
}

pub(crate) fn write_hermes_skill_package(skill_dir: &Path) -> Result<()> {
    fs::create_dir_all(skill_dir.join("references"))?;
    fs::create_dir_all(skill_dir.join("scripts"))?;
    fs::write(
        skill_dir.join("SKILL.md"),
        include_str!("../../extras/hermes/clipboard-memory/SKILL.md"),
    )?;
    fs::write(
        skill_dir.join("references/commands.md"),
        include_str!("../../extras/hermes/clipboard-memory/references/commands.md"),
    )?;
    fs::write(
        skill_dir.join("references/troubleshooting.md"),
        include_str!("../../extras/hermes/clipboard-memory/references/troubleshooting.md"),
    )?;
    fs::write(
        skill_dir.join("references/json-schema.md"),
        include_str!("../../extras/hermes/clipboard-memory/references/json-schema.md"),
    )?;
    fs::write(
        skill_dir.join("references/examples.md"),
        include_str!("../../extras/hermes/clipboard-memory/references/examples.md"),
    )?;
    fs::write(
        skill_dir.join("references/setup-check.md"),
        include_str!("../../extras/hermes/clipboard-memory/references/setup-check.md"),
    )?;
    write_executable(
        &skill_dir.join("scripts/check-setup.sh"),
        include_str!("../../extras/hermes/clipboard-memory/scripts/check-setup.sh"),
    )?;
    Ok(())
}

#[cfg(target_os = "macos")]
pub(crate) fn write_stateful_launchctl_stub(bin_dir: &Path, state_dir: &Path) -> Result<()> {
    let state_dir = state_dir.display().to_string();
    let script = format!(
        "#!/bin/sh
STATE_DIR='{state_dir}'
DIRECT_STATE=\"$STATE_DIR/direct.state\"
DIRECT_DISABLED=\"$STATE_DIR/direct.disabled\"
BOOTOUT_FAIL=\"$STATE_DIR/bootout.fail\"
DISABLE_FAIL=\"$STATE_DIR/disable.fail\"
HOMEBREW_STATE=\"$STATE_DIR/homebrew.state\"
mkdir -p \"$STATE_DIR\"
case \"$1\" in
  list)
    [ -f \"$HOMEBREW_STATE\" ] && cat \"$HOMEBREW_STATE\"
    [ -f \"$DIRECT_STATE\" ] && cat \"$DIRECT_STATE\"
    ;;
  bootstrap)
    if [ -f \"$DIRECT_DISABLED\" ]; then
      printf 'Bootstrap failed: 5: Input/output error\\n' >&2
      exit 5
    fi
    printf '123 0 io.openclaw.clipmem.watch\\n' > \"$DIRECT_STATE\"
    ;;
  bootout)
    if [ -f \"$BOOTOUT_FAIL\" ]; then
      printf 'forced bootout failure\\n' >&2
      exit 42
    fi
    case \"$2\" in
      *homebrew.mxcl.clipmem) rm -f \"$HOMEBREW_STATE\" ;;
      *io.openclaw.clipmem.watch)
        if [ -f \"$DIRECT_STATE\" ]; then
          rm -f \"$DIRECT_STATE\"
        else
          printf 'No such process\\n' >&2
          exit 3
        fi
        ;;
    esac
    ;;
  enable)
    case \"$2\" in
      *io.openclaw.clipmem.watch) rm -f \"$DIRECT_DISABLED\" ;;
    esac
    ;;
  disable)
    if [ -f \"$DISABLE_FAIL\" ]; then
      printf 'forced disable failure\\n' >&2
      exit 43
    fi
    case \"$2\" in
      *io.openclaw.clipmem.watch) touch \"$DIRECT_DISABLED\" ;;
    esac
    ;;
  kickstart)
    ;;
  *)
    ;;
esac
exit 0
"
    );
    write_executable(&bin_dir.join("launchctl"), &script)
}

#[cfg(target_os = "macos")]
pub(crate) fn write_stateful_brew_stub(
    bin_dir: &Path,
    state_dir: &Path,
    services_available: bool,
) -> Result<()> {
    let state_dir = state_dir.display().to_string();
    let availability = if services_available { 0 } else { 1 };
    let script = format!(
        "#!/bin/sh
STATE_DIR='{state_dir}'
HOMEBREW_STATE=\"$STATE_DIR/homebrew.state\"
HOMEBREW_LOG=\"$STATE_DIR/brew.log\"
mkdir -p \"$STATE_DIR\"
printf '%s\\n' \"$*\" >> \"$HOMEBREW_LOG\"
if [ \"$1\" = \"services\" ] && [ \"$2\" = \"list\" ]; then
  exit {availability}
fi
if [ \"$1\" = \"services\" ] && [ \"$2\" = \"info\" ] && [ \"$3\" = \"clipmem\" ]; then
  [ {availability} -eq 0 ] || exit 1
  printf '[{{\"name\":\"clipmem\",\"schedulable\":true}}]\\n'
  exit 0
fi
if [ \"$1\" = \"services\" ] && [ \"$2\" = \"start\" ] && [ \"$3\" = \"clipmem\" ]; then
  printf '456 0 homebrew.mxcl.clipmem\\n' > \"$HOMEBREW_STATE\"
  exit 0
fi
if [ \"$1\" = \"services\" ] && [ \"$2\" = \"stop\" ] && [ \"$3\" = \"clipmem\" ]; then
  rm -f \"$HOMEBREW_STATE\"
  exit 0
fi
exit 0
"
    );
    write_executable(&bin_dir.join("brew"), &script)
}

#[cfg(target_os = "macos")]
pub(crate) fn write_brew_stub_without_clipmem_service(
    bin_dir: &Path,
    state_dir: &Path,
) -> Result<()> {
    let state_dir = state_dir.display().to_string();
    let script = format!(
        "#!/bin/sh
STATE_DIR='{state_dir}'
HOMEBREW_LOG=\"$STATE_DIR/brew.log\"
mkdir -p \"$STATE_DIR\"
printf '%s\\n' \"$*\" >> \"$HOMEBREW_LOG\"
if [ \"$1\" = \"services\" ] && [ \"$2\" = \"list\" ]; then
  exit 0
fi
if [ \"$1\" = \"services\" ] && [ \"$2\" = \"info\" ] && [ \"$3\" = \"clipmem\" ]; then
  printf '[{{\"name\":\"clipmem\",\"schedulable\":null}}]\\n'
  exit 0
fi
if [ \"$1\" = \"services\" ] && [ \"$2\" = \"start\" ] && [ \"$3\" = \"clipmem\" ]; then
  printf 'Error: Invalid usage: Formula `clipmem` has not implemented #plist, #service or provided a locatable service file.\\n' >&2
  exit 1
fi
exit 0
"
    );
    write_executable(&bin_dir.join("brew"), &script)
}

#[cfg(unix)]
pub(crate) fn run_setup_check_script_with_launchctl(
    script_path: &Path,
    launchctl_row: Option<&str>,
) -> Result<process::Output> {
    let test_dir = temp_test_dir("setup-check-script");
    let bin_dir = test_dir.join("bin");
    fs::create_dir_all(&bin_dir)?;

    write_executable(
        &bin_dir.join("clipmem"),
        match launchctl_row {
            Some("homebrew") => "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  printf 'clipmem test\\n'\n  exit 0\nfi\nif [ \"$1\" = \"doctor\" ] && [ \"$2\" = \"--json\" ]; then\n  printf '{\"fts5_create_virtual_table_ok\":true}\\n'\n  exit 0\nfi\nif [ \"$1\" = \"service\" ] && [ \"$2\" = \"status\" ] && [ \"$3\" = \"--json\" ]; then\n  printf '{\"homebrew\":{\"running\":true,\"loaded\":true},\"launchagent\":{\"running\":false,\"loaded\":false},\"stale\":false,\"recent_capture_within_last_hour\":1,\"conflict\":false}\\n'\n  exit 0\nfi\nif [ \"$1\" = \"agents\" ] && [ \"$2\" = \"openclaw\" ] && [ \"$3\" = \"--help\" ]; then\n  exit 0\nfi\nif [ \"$1\" = \"agents\" ] && [ \"$2\" = \"openclaw\" ] && [ \"$3\" = \"doctor\" ]; then\n  exit 0\nfi\nprintf 'unexpected clipmem args: %s\\n' \"$*\" >&2\nexit 99\n",
            Some("- 0 io.openclaw.clipmem.watch") => "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  printf 'clipmem test\\n'\n  exit 0\nfi\nif [ \"$1\" = \"doctor\" ] && [ \"$2\" = \"--json\" ]; then\n  printf '{\"fts5_create_virtual_table_ok\":true}\\n'\n  exit 0\nfi\nif [ \"$1\" = \"service\" ] && [ \"$2\" = \"status\" ] && [ \"$3\" = \"--json\" ]; then\n  printf '{\"homebrew\":{\"running\":false,\"loaded\":false},\"launchagent\":{\"running\":false,\"loaded\":true},\"stale\":true,\"recent_capture_within_last_hour\":0,\"conflict\":false}\\n'\n  exit 0\nfi\nif [ \"$1\" = \"agents\" ] && [ \"$2\" = \"openclaw\" ] && [ \"$3\" = \"--help\" ]; then\n  exit 0\nfi\nif [ \"$1\" = \"agents\" ] && [ \"$2\" = \"openclaw\" ] && [ \"$3\" = \"doctor\" ]; then\n  exit 0\nfi\nprintf 'unexpected clipmem args: %s\\n' \"$*\" >&2\nexit 99\n",
            Some("123 0 io.openclaw.clipmem.watch") => "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  printf 'clipmem test\\n'\n  exit 0\nfi\nif [ \"$1\" = \"doctor\" ] && [ \"$2\" = \"--json\" ]; then\n  printf '{\"fts5_create_virtual_table_ok\":true}\\n'\n  exit 0\nfi\nif [ \"$1\" = \"service\" ] && [ \"$2\" = \"status\" ] && [ \"$3\" = \"--json\" ]; then\n  printf '{\"homebrew\":{\"running\":false,\"loaded\":false},\"launchagent\":{\"running\":true,\"loaded\":true},\"stale\":false,\"recent_capture_within_last_hour\":0,\"conflict\":false}\\n'\n  exit 0\nfi\nif [ \"$1\" = \"agents\" ] && [ \"$2\" = \"openclaw\" ] && [ \"$3\" = \"--help\" ]; then\n  exit 0\nfi\nif [ \"$1\" = \"agents\" ] && [ \"$2\" = \"openclaw\" ] && [ \"$3\" = \"doctor\" ]; then\n  exit 0\nfi\nprintf 'unexpected clipmem args: %s\\n' \"$*\" >&2\nexit 99\n",
            _ => "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  printf 'clipmem test\\n'\n  exit 0\nfi\nif [ \"$1\" = \"doctor\" ] && [ \"$2\" = \"--json\" ]; then\n  printf '{\"fts5_create_virtual_table_ok\":true}\\n'\n  exit 0\nfi\nif [ \"$1\" = \"service\" ] && [ \"$2\" = \"status\" ] && [ \"$3\" = \"--json\" ]; then\n  printf '{\"homebrew\":{\"running\":false,\"loaded\":false},\"launchagent\":{\"running\":false,\"loaded\":false},\"stale\":true,\"recent_capture_within_last_hour\":0,\"conflict\":false}\\n'\n  exit 0\nfi\nif [ \"$1\" = \"agents\" ] && [ \"$2\" = \"openclaw\" ] && [ \"$3\" = \"--help\" ]; then\n  exit 0\nfi\nif [ \"$1\" = \"agents\" ] && [ \"$2\" = \"openclaw\" ] && [ \"$3\" = \"doctor\" ]; then\n  exit 0\nfi\nprintf 'unexpected clipmem args: %s\\n' \"$*\" >&2\nexit 99\n",
        },
    )?;

    let path_value = format!("{}:/usr/bin:/bin", bin_dir.display());
    let output = run_command_with_env(script_path, &[], &[("PATH", &path_value)]);

    let _ = fs::remove_dir_all(&test_dir);
    Ok(output)
}

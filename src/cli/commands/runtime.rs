use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};

use crate::app::{
    format_watch_capture_line, mark_change_handled, should_skip_initial_change, WatchState,
};
use crate::db::{CaptureSkipReason, CaptureStoreOutcome, Database};
use crate::model::ClipboardSnapshot;
use crate::platform::{capture_snapshot, current_change_count};

use crate::cli::commands::notify::notify_app_refresh;
use crate::cli::errors::{db_error, platform_error};
use crate::cli::human::render_capture_once_human;
use crate::cli::output::{
    render_capture_once_text, CaptureOnceOutput, CaptureOnceSkippedOutput, CaptureOnceStoredOutput,
};
use crate::cli::presentation::emit_json_or_text;
use crate::cli::schema::{CaptureOnceArgs, WatchArgs};

static OCR_WORKERS: LazyLock<Mutex<HashSet<PathBuf>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

fn capture_skip_reason_label(reason: CaptureSkipReason) -> &'static str {
    reason.as_str()
}

pub(in crate::cli) fn watch(db_path: &Path, args: &WatchArgs) -> Result<()> {
    let mut db = open_or_init_db(db_path)?;
    let interval_ms = args.interval_ms.max(50);
    let mut state = WatchState::new();

    loop {
        if let Err(err) = run_watch_iteration(&mut db, args, &mut state) {
            eprintln!("{err:#}");
        }

        thread::sleep(Duration::from_millis(interval_ms));
    }
}

pub(in crate::cli) fn run_watch_iteration(
    db: &mut Database,
    args: &WatchArgs,
    state: &mut WatchState,
) -> Result<()> {
    run_watch_iteration_with_capture(db, args, state, current_change_count, capture_snapshot)
}

pub(in crate::cli) fn run_watch_iteration_with_capture<CountFn, CaptureFn>(
    db: &mut Database,
    args: &WatchArgs,
    state: &mut WatchState,
    current_change_count_fn: CountFn,
    capture_snapshot_fn: CaptureFn,
) -> Result<()>
where
    CountFn: FnOnce() -> Result<i64>,
    CaptureFn: FnOnce() -> Result<ClipboardSnapshot>,
{
    let change_count = current_change_count_fn()
        .map_err(|error| platform_error(format!("read clipboard change count failed: {error}")))?;

    if should_skip_initial_change(args.skip_initial, state) {
        mark_change_handled(change_count, state);
        return Ok(());
    }

    if !crate::app::should_capture_change(change_count, state) {
        return Ok(());
    }

    let snapshot = capture_snapshot_fn()
        .map_err(|error| platform_error(format!("capture failed: {error}")))?;
    let settings = db
        .capture_settings()
        .context("load capture settings failed")?;
    if settings.paused() {
        mark_change_handled(snapshot.change_count(), state);
        return Ok(());
    }
    if db
        .bundle_id_is_ignored(snapshot.frontmost_app_bundle_id())
        .context("load ignored bundle ids failed")?
    {
        mark_change_handled(snapshot.change_count(), state);
        return Ok(());
    }
    let outcome = anyhow::Context::context(
        db.store_watched_capture_if_allowed(&snapshot),
        "database write failed",
    )?;
    match outcome {
        CaptureStoreOutcome::Stored(result) => {
            if settings.ocr_enabled() {
                let snapshot_id = result.snapshot_id();
                if let Err(err) = db.enqueue_ocr_for_snapshot(snapshot_id) {
                    eprintln!("ocr enqueue failed for snapshot {snapshot_id}: {err:#}");
                } else {
                    start_ocr_worker(db.path().to_path_buf());
                }
            }
            mark_change_handled(snapshot.change_count(), state);
            db.apply_retention_policy()
                .context("apply retention policy failed")?;
            notify_app_refresh();
            if !args.quiet {
                println!("{}", format_watch_capture_line(&snapshot, &result));
            }
        }
        CaptureStoreOutcome::Skipped(reason) => {
            mark_change_handled(snapshot.change_count(), state);
            if !args.quiet {
                println!(
                    "skipped capture reason={} kind={} bytes={} source={}",
                    capture_skip_reason_label(reason),
                    snapshot.snapshot_kind(),
                    snapshot.total_bytes(),
                    snapshot
                        .frontmost_app_name()
                        .or(snapshot.frontmost_app_bundle_id())
                        .unwrap_or("unknown app")
                );
            }
        }
    }

    Ok(())
}

pub(in crate::cli) fn start_ocr_worker(db_path: PathBuf) {
    let Some(worker_registration) = claim_ocr_worker(&db_path) else {
        return;
    };

    thread::spawn(move || {
        let _worker_registration = worker_registration;
        let run = || -> Result<()> {
            let mut worker_db = Database::open_or_init(&db_path)?;
            let engine = crate::ocr::default_engine();
            let mut processed = 0usize;
            loop {
                let report = crate::ocr::run_ocr_jobs(&mut worker_db, &engine, 1, None, false)?;
                if report.processed() == 0 {
                    break;
                }
                processed += report.processed();
            }
            if processed > 0 {
                notify_app_refresh();
            }
            Ok(())
        };
        if let Err(err) = run() {
            eprintln!("ocr failed: {err:#}");
        }
    });
}

fn claim_ocr_worker(db_path: &Path) -> Option<OcrWorkerRegistration> {
    let key = canonical_ocr_worker_key(db_path);
    let mut workers = ocr_workers();
    if !workers.insert(key.clone()) {
        return None;
    }
    Some(OcrWorkerRegistration { key })
}

fn ocr_workers() -> MutexGuard<'static, HashSet<PathBuf>> {
    match OCR_WORKERS.lock() {
        Ok(workers) => workers,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn canonical_ocr_worker_key(db_path: &Path) -> PathBuf {
    if let Ok(path) = db_path.canonicalize() {
        return path;
    }
    if db_path.is_absolute() {
        return db_path.to_path_buf();
    }
    std::env::current_dir()
        .map(|cwd| cwd.join(db_path))
        .unwrap_or_else(|_| db_path.to_path_buf())
}

struct OcrWorkerRegistration {
    key: PathBuf,
}

impl Drop for OcrWorkerRegistration {
    fn drop(&mut self) {
        ocr_workers().remove(&self.key);
    }
}

pub(in crate::cli) fn capture_once(db_path: &Path, args: &CaptureOnceArgs) -> Result<()> {
    let mut db = open_or_init_db(db_path)?;
    let snapshot = capture_snapshot()
        .map_err(|error| platform_error(format!("capture-once clipboard read failed: {error}")))?;
    let payload = match anyhow::Context::context(
        db.store_capture_if_allowed(&snapshot),
        "capture-once database write failed",
    )? {
        CaptureStoreOutcome::Stored(store) => {
            db.apply_retention_policy()
                .context("apply retention policy failed")?;
            notify_app_refresh();
            CaptureOnceOutput::Stored(CaptureOnceStoredOutput { store, snapshot })
        }
        CaptureStoreOutcome::Skipped(reason) => {
            CaptureOnceOutput::Skipped(CaptureOnceSkippedOutput {
                status: "skipped",
                reason,
                kind: snapshot.snapshot_kind().as_str().to_string(),
                total_bytes: snapshot.total_bytes(),
                frontmost_app_name: snapshot.frontmost_app_name().map(str::to_string),
                frontmost_app_bundle_id: snapshot.frontmost_app_bundle_id().map(str::to_string),
            })
        }
    };
    if args.human {
        print!("{}", render_capture_once_human(&payload));
    } else {
        emit_json_or_text(args.json, &payload, render_capture_once_text)?;
    }

    Ok(())
}

pub(in crate::cli) fn open_existing_db(path: &Path) -> Result<Database> {
    if !path.is_file() {
        return Err(db_error(format!(
            "database does not exist at {}. Run `clipmem setup` to initialize capture.",
            path.display()
        )));
    }
    match Database::open_existing(path) {
        Ok(db) => {
            if let Err(error) = db.ensure_supported_schema_shape() {
                if error
                    .chain()
                    .any(|cause| cause.to_string().contains("prerelease schema"))
                {
                    return Err(db_error(
                        "database operation failed; this may be an incompatible prerelease schema. Move the database aside and run `clipmem setup`."
                    ));
                }
                return Err(error);
            }
            Ok(db)
        }
        Err(error) => {
            if error
                .chain()
                .any(|cause| cause.to_string().contains("incompatible prerelease schema"))
            {
                Err(db_error(format!(
                    "incompatible prerelease schema detected at {}. Move the database aside and run `clipmem setup`.",
                    path.display()
                )))
            } else {
                Err(error).with_context(|| format!("failed to open database at {}", path.display()))
            }
        }
    }
}

pub(in crate::cli) fn open_or_init_db(path: &Path) -> Result<Database> {
    anyhow::Context::with_context(Database::open_or_init(path), || {
        format!("failed to open database at {}", path.display())
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::claim_ocr_worker;

    #[test]
    fn ocr_worker_guard_allows_distinct_database_paths_in_same_process() {
        let first = temp_db_path("ocr-worker-first");
        let second = temp_db_path("ocr-worker-second");
        create_empty_file(&first);
        create_empty_file(&second);

        let first_registration =
            claim_ocr_worker(&first).expect("first database should claim worker");
        assert!(
            claim_ocr_worker(&first).is_none(),
            "same database should not claim a second worker"
        );

        let second_registration =
            claim_ocr_worker(&second).expect("different database should claim its own worker");

        drop(first_registration);
        assert!(
            claim_ocr_worker(&first).is_some(),
            "dropped registrations should release their database key"
        );

        drop(second_registration);
        cleanup_db(&first);
        cleanup_db(&second);
    }

    fn temp_db_path(test_name: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir()
            .join("clipmem-ocr-worker-tests")
            .join(format!("{test_name}-{}-{timestamp}.sqlite3", process::id()))
    }

    fn create_empty_file(path: &Path) {
        fs::create_dir_all(path.parent().expect("test path should have parent"))
            .expect("test database parent should be created");
        fs::File::create(path).expect("test database file should be created");
    }

    fn cleanup_db(path: &Path) {
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
}

pub(super) const ROOT_AFTER_HELP: &str = "\
Examples:
  clipmem setup
  clipmem service status
  clipmem service providers --format json
  clipmem app settings show --format json
  clipmem recall \"what was that shell command?\"
  clipmem recent --hours 24
  clipmem stats
  clipmem timeline --hours 24 --format json
  clipmem get 42 --format json

Agent-first flow:
  1. Use `clipmem recall` for the best answer plus alternatives.
  2. Use `clipmem timeline` for chronological event history.
  3. Use `clipmem search` for direct lexical matching.
  4. Use `clipmem get` for nested snapshot detail.
  5. Use `clipmem app settings show|set|clear --format json` for app preferences.
  6. Run `clipmem agents context --format json` before multi-step work.
  7. See docs/action-parity.md for the full capability map.";

pub(super) const APP_AFTER_HELP: &str = "\
Examples:
  clipmem app settings show --format json
  clipmem app settings set default-recent-hours 12 --format json
  clipmem app settings clear binary-path-override --format json

Notes:
  - App commands manage menu bar app preferences, not capture policy.
  - Preference mutations bump `app_preferences_revision` for open app refresh.";

pub(super) const APP_SETTINGS_AFTER_HELP: &str = "\
Examples:
  clipmem app settings show --format json
  clipmem app settings set binary-path-override /usr/local/bin/clipmem --format json
  clipmem app settings set database-path-override /tmp/clipmem.sqlite3 --format json
  clipmem app settings set default-recent-hours 12 --format json
  clipmem app settings set default-query-mode recall --format json
  clipmem app settings set hotkey-enabled false --format json
  clipmem app settings clear binary-path-override --format json

Keys:
  binary-path-override, database-path-override, default-recent-hours,
  default-query-mode, hotkey-enabled";

pub(super) const WATCH_AFTER_HELP: &str = "\
Examples:
  clipmem watch
  clipmem watch --interval-ms 350
  clipmem watch --skip-initial

Notes:
  - Default interval is 400 ms.
  - Intervals below 50 ms are clamped to 50 ms at runtime.
  - Status lines stream on stdout; runtime diagnostics go to stderr.";

pub(super) const CAPTURE_ONCE_AFTER_HELP: &str = "\
Examples:
  clipmem capture-once
  clipmem capture-once --human
  clipmem capture-once --json";

pub(super) const SEARCH_AFTER_HELP: &str = "\
Examples:
  clipmem search \"git commit -m\"
  clipmem search \"git commit -m\" --human
  clipmem search \"https://example.com/repo\" --format json
  clipmem search \"launchctl bootstrap\" --limit 25 --cursor \"<next_cursor>\" --format json
  clipmem search --mode literal \"foo:bar\"

Notes:
  - `--limit` is the page size. Defaults to 10 and is bounded 1-250.
  - `--cursor` resumes from a prior `next_cursor`. Cursors are opaque and tied to the active query, mode, and filters.
  - Auto mode prefers literal matching for URLs, paths, bundle ids, dotted identifiers, and shell-like fragments when that is more reliable.";

pub(super) const RECENT_AFTER_HELP: &str = "\
Examples:
  clipmem recent
  clipmem recent --human
  clipmem recent --hours 24 --app safari
  clipmem recent --format json --limit 25 --cursor \"<next_cursor>\"

Notes:
  - `recent` is snapshot-centric and deduplicated by snapshot id.
  - `--limit` is the page size. Defaults to 10 and is bounded 1-250.
  - `--cursor` resumes from a prior `next_cursor`. Cursors are opaque and tied to the active filters.";

pub(super) const TIMELINE_AFTER_HELP: &str = "\
Examples:
  clipmem timeline --hours 24
  clipmem timeline --hours 24 --human
  clipmem timeline --since 2026-04-16T09:00:00Z --until 2026-04-16T18:00:00Z --sort asc --format json
  clipmem timeline --app safari --has-url --limit 25 --cursor \"<next_cursor>\" --format json

Notes:
  - `timeline` is event-centric and returns one row per real capture event.
  - `--limit` is the page size. Defaults to 10 and is bounded 1-250.
  - `--cursor` resumes from a prior `next_cursor`. Cursors are opaque and tied to the active filters and sort order.";

pub(super) const STATS_AFTER_HELP: &str = "\
Examples:
  clipmem stats
  clipmem stats --hours 24
  clipmem stats --human
  clipmem stats --app safari --format json

Notes:
  - `stats` reports aggregate archive metrics and leaderboards for the active filters.
  - Supports shared retrieval filters, including time, app, bundle id, kind, shape, and byte filters.
  - Output formats are intentionally limited to `text`, `json`, and `human`.";

pub(super) const RECALL_AFTER_HELP: &str = "\
Examples:
  clipmem recall \"what was that command I copied?\"
  clipmem recall \"what was that command I copied?\" --human
  clipmem recall \"find the URL I copied yesterday\" --format json
  clipmem recall --prefer-recent --hours 24
  clipmem recall \"give me the exact text\" --quote --full

Notes:
  - `recall` is the primary best-first retrieval command for agents.
  - `--limit` controls ranked candidates considered. Defaults to 5 and is bounded 1-250.
  - Use `--quote` or `--full` when the exact surfaced text matters more than the compact answer.";

pub(super) const GET_AFTER_HELP: &str = "\
Examples:
  clipmem get 42
  clipmem get 42 --human
  clipmem get 42 --format json
  clipmem get 42 --events 25 --format md

Notes:
  - `get` is for detailed nested snapshot inspection after you already know the snapshot id.
  - `--events` defaults to 10 and is bounded 1-250.
  - `--format toon` is not supported because `get` returns nested snapshot detail.";

pub(super) const EXPORT_AFTER_HELP: &str = "\
Examples:
  clipmem export 42 --item 0 --uti public.png --out ./clipboard.png
  clipmem export 42 --item 0 --uti public.utf8-plain-text --out ./clipboard.txt --app terminal
  clipmem export 42 --item 0 --uti public.png --out ./clipboard.png --force
  clipmem export 42 --item 0 --uti public.png --out ./clipboard.png --format json

Notes:
  - `export` creates a new file at `--out` by default and refuses to replace an existing path.
  - Pass `--force` to replace an existing regular file.
  - Symlink destinations are never allowed.
  - Success output is written to stdout; failures are written to stderr only.
  - JSON output reports the destination, representation identity, and written byte count.";

pub(super) const RESTORE_AFTER_HELP: &str = "\
Examples:
  clipmem restore 42
  clipmem restore 42 --format json

Notes:
  - Restores every stored item and representation for the snapshot back onto the macOS clipboard.
  - The active watcher suppresses the matching restored state so it doesn't create a duplicate capture event.";

pub(super) const FORGET_AFTER_HELP: &str = "\
Examples:
  clipmem forget 42
  clipmem forget 42 --format json

Notes:
  - Forget irreversibly deletes the snapshot content row, all child representations, and all capture events for that snapshot id.
  - This is a hard delete; there is no recycle bin.";

pub(super) const PURGE_AFTER_HELP: &str = "\
Examples:
  clipmem purge --older-than 30d
  clipmem purge --older-than 12h --dry-run
  clipmem purge --older-than 30d --dry-run --format json

Notes:
  - Purge ages snapshots by `last_observed_at`, not the original snapshot creation time.
  - Duration grammar is a single integer plus one unit: `Nd`, `Nh`, or `Nm`.";

pub(super) const STORAGE_AFTER_HELP: &str = "\
Examples:
  clipmem storage compact --format json
  clipmem storage image-candidates --format json
  clipmem storage compact --dry-run --format json
  clipmem storage optimize-images --format json
  clipmem storage optimize-images --dry-run --format json
  clipmem storage optimize-images --no-compact --format json
  clipmem storage optimize-images --limit 50 --format json
  clipmem storage optimize-images --progress jsonl

Notes:
  - `compact` reclaims SQLite and WAL disk space without changing clipboard content.
  - `optimize-images` converts eligible stored image bytes to lossless WebP, then compacts SQLite storage by default.
  - `optimize-images --progress jsonl` streams machine-readable progress events.
  - Use `--no-compact` when batching optimization runs and compacting once at the end.";

pub(super) const SETTINGS_AFTER_HELP: &str = "\
Examples:
  clipmem settings show
  clipmem settings pause on
  clipmem settings api-key-filter on
  clipmem settings ocr on
  clipmem settings retention 30d
  clipmem settings retention forever
  clipmem settings reset --format json
  clipmem settings ignore add com.apple.Passwords
  clipmem settings ignore list --format json";

pub(super) const OCR_AFTER_HELP: &str = "\
Examples:
  clipmem ocr status
  clipmem ocr candidates --format json
  clipmem ocr get <raw-sha256> --format json
  clipmem ocr clear <raw-sha256> --format json
  clipmem ocr run
  clipmem ocr run --limit 50
  clipmem ocr run --snapshot 42 --retry-failed --format json

Notes:
  - OCR is opt-in for background capture; use `clipmem settings ocr on`.
  - `ocr run` is the backfill path for existing image snapshots.
  - Phase 1 stores OCR text only; original image bytes remain unchanged.";

pub(super) const DOCTOR_AFTER_HELP: &str = "\
Examples:
  clipmem doctor
  clipmem doctor --human
  clipmem doctor --json

Notes:
  - Doctor output is requested human or structured output and always goes to stdout.
  - A nonzero exit code means required checks or diagnostics failed.";

pub(super) const SETUP_AFTER_HELP: &str = "\
Examples:
  clipmem setup
  clipmem setup --db /tmp/clipmem.sqlite3

Notes:
  - `setup` performs one foreground capture, then starts background capture using Homebrew services or a direct LaunchAgent.
  - Re-running `setup` is safe and updates the managed service definition as needed.";

pub(super) const SERVICE_AFTER_HELP: &str = "\
Examples:
  clipmem service start
  clipmem service status
  clipmem service status --json
  clipmem service stop
  clipmem service uninstall

Notes:
  - Homebrew installs prefer `brew services`; Cargo and manual installs use a direct LaunchAgent.
  - `status` is informational and reports freshness, provider state, and any setup conflicts.";

pub(super) const SERVICE_STATUS_AFTER_HELP: &str = "\
Examples:
  clipmem service status
  clipmem service status --human
  clipmem service status --json

Notes:
  - Text output is intended for humans.
  - Human output is polished for interactive terminals.
  - `--json` is the stable machine-readable form used by packaged skill health checks.";

pub(super) const OPENCLAW_INSTALL_AFTER_HELP: &str = "\
Examples:
  clipmem agents openclaw install-skill
  clipmem agents openclaw install-skill --shared
  clipmem agents openclaw install-skill --dest /tmp/clipboard-memory --force

Notes:
  - Default install target is the current OpenClaw workspace skills directory.
  - `--shared` installs into ~/.openclaw/skills/clipboard-memory instead.";

pub(super) const OPENCLAW_UNINSTALL_AFTER_HELP: &str = "\
Examples:
  clipmem agents openclaw uninstall-skill
  clipmem agents openclaw uninstall-skill --shared
  clipmem agents openclaw uninstall-skill --dest /tmp/clipboard-memory";

pub(super) const OPENCLAW_PRINT_AFTER_HELP: &str = "\
Examples:
  clipmem agents openclaw print-skill

Notes:
  - Prints the packaged OpenClaw SKILL.md to stdout for inspection or templating.";

pub(super) const OPENCLAW_DOCTOR_AFTER_HELP: &str = "\
Examples:
  clipmem agents openclaw doctor
  clipmem agents openclaw doctor --shared
  clipmem agents openclaw doctor --dest /tmp/clipboard-memory

Notes:
  - Reports are written to stdout.
  - A nonzero exit code means required OpenClaw integration checks failed.";

pub(super) const HERMES_INSTALL_AFTER_HELP: &str = "\
Examples:
  clipmem agents hermes install-skill
  clipmem agents hermes install-skill --dest /tmp/clipboard-memory --force

Notes:
  - Default install target is ~/.hermes/skills/productivity/clipboard-memory.
  - `--dest` installs into the exact directory you provide.";

pub(super) const HERMES_UNINSTALL_AFTER_HELP: &str = "\
Examples:
  clipmem agents hermes uninstall-skill
  clipmem agents hermes uninstall-skill --dest /tmp/clipboard-memory";

pub(super) const HERMES_PRINT_AFTER_HELP: &str = "\
Examples:
  clipmem agents hermes print-skill

Notes:
  - Prints the packaged Hermes Agent SKILL.md to stdout for inspection or templating.";

pub(super) const HERMES_DOCTOR_AFTER_HELP: &str = "\
Examples:
  clipmem agents hermes doctor
  clipmem agents hermes doctor --dest /tmp/clipboard-memory

Notes:
  - Reports are written to stdout.
  - A nonzero exit code means required Hermes Agent integration checks failed.
  - Missing `hermes` is reported as a warning so package validation can still pass.";

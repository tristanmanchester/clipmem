# Command reference

Exhaustive flag-level reference for every `clipmem` command and
subcommand. For conceptual explanations and usage guidance, see
[searching and filtering](searching-and-filtering.md),
[output formats](output-formats.md), and
[managing your archive](managing-your-archive.md).

## Subcommand overview

| Command | Default format | Supports `toon`? | Purpose |
|---|---|---|---|
| `recall [QUERY]` | `md` | yes | Ranked best-first answer with alternatives |
| `search <QUERY>` | `text` | yes | Lexical / FTS match over the archive |
| `recent` | `text` | yes | Recent unique snapshots (deduplicated) |
| `timeline` | `text` | yes | Chronological capture events (not deduplicated) |
| `stats` | `text` | no | Archive aggregates and leaderboards |
| `get <ID>` | `text` | no | Nested detail for one snapshot |
| `restore <ID>` | `text` | — | Restore a stored snapshot to the clipboard |
| `export <ID>` | raw bytes | — | Write one representation to disk |
| `forget <ID>` | `text` | — | Hard-delete one snapshot and its history |
| `purge` | `text` | — | Delete old snapshots by age |
| `storage compact` | `text` | — | Reclaim SQLite and WAL disk space |
| `storage image-candidates` | `text` | — | List image rows eligible for optimization |
| `storage optimize-images` | `text` | — | Convert eligible images to lossless WebP; stream progress with `--progress jsonl` |
| `settings show` | `text` | no | Show capture policy |
| `settings pause` | `text` | — | Pause or resume capture |
| `settings api-key-filter` | `text` | — | Enable or disable API key filtering |
| `settings ocr` | `text` | — | Enable or disable local OCR for new captures |
| `settings retention` | `text` | — | Set retention duration |
| `settings reset` | `text` | — | Reset capture policy and ignored apps |
| `settings ignore` | `text` | no | Manage ignored bundle IDs |
| `app settings` | `text` | — | Show or mutate menu bar app preferences |
| `app launch-at-login` | `text` | — | Show or mutate app launch-at-login preference |
| `app update-check` | `text` | — | Show, run, or clear app update-check state |
| `app quit` | `text` | — | Request the menu bar app to quit |
| `ocr status` | `text` | — | Show OCR queue and result counts |
| `ocr candidates` | `text` | — | List pending OCR candidates |
| `ocr get` | `text` | — | Inspect one OCR result |
| `ocr clear` | `text` | — | Clear one OCR result |
| `ocr run` | `text` | — | Backfill OCR for stored image snapshots |
| `setup` | — | — | Initialize and start background capture |
| `service start` | — | — | Start background capture |
| `service stop` | — | — | Stop background capture |
| `service status` | `text` | — | Report watcher state and freshness |
| `service providers` | `text` | — | Inspect provider availability without mutation |
| `service revision` | `text` | — | Print archive revision counters without probing service providers |
| `service uninstall` | — | — | Remove managed service definition |
| `watch` | — | — | Continuous foreground polling |
| `capture-once` | — | — | Single clipboard capture |
| `doctor` | `text` | — | SQLite and FTS5 diagnostics |
| `agents context` | `text` | — | Agent context bundle and capability map |
| `agents openclaw install-skill` | — | — | Install packaged skill |
| `agents openclaw uninstall-skill` | — | — | Remove installed skill |
| `agents openclaw print-skill` | — | — | Print SKILL.md to stdout |
| `agents openclaw doctor` | `text` | — | Integration health check |
| `agents hermes install-skill` | — | — | Install packaged Hermes skill |
| `agents hermes uninstall-skill` | — | — | Remove installed Hermes skill |
| `agents hermes print-skill` | — | — | Print Hermes SKILL.md to stdout |
| `agents hermes doctor` | `text` | — | Hermes integration health check |

## Global flags

- `--db <path>` — override the SQLite database path. Default:
  `~/Library/Application Support/clipmem/clipmem.sqlite3`.

## Shared retrieval filters

These flags are accepted by `search`, `recent`, `timeline`, `stats`,
`recall`, `get`, and `export`:

| Flag | Type | Description |
|---|---|---|
| `--since <RFC3339>` | timestamp | Captures at or after this time |
| `--until <RFC3339>` | timestamp | Captures at or before this time |
| `--hours <N>` | integer | Last N hours (ignored if `--since` set) |
| `--app <name>` | string | Case-insensitive substring match on app name |
| `--bundle-id <id>` | string | Case-insensitive exact match on bundle ID |
| `--kind <type>` | enum | `text`, `html`, `rtf`, `url`, `file`, `image`, `pdf`, `binary`, `other` |
| `--has-text` | flag | Require non-empty text representation |
| `--has-url` | flag | Require URL representation |
| `--has-file-url` | flag | Require file URL representation |
| `--has-image` | flag | Require image representation |
| `--has-pdf` | flag | Require PDF representation |
| `--min-bytes <N>` | integer | Minimum snapshot byte count |
| `--max-bytes <N>` | integer | Maximum snapshot byte count |

---

## Retrieval commands

### `clipmem recall [QUERY]`

Best-first ranked answer with alternatives.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--format` | `md\|json\|toon\|human` | `md` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |
| `--mode` | `auto\|fts\|literal` | `auto` | Search mode |
| `--limit` | 1-250 | 5 | Ranked candidates to consider |
| `--full` | flag | — | Expand best candidate text |
| `--quote` | flag | — | Force quoted best-text output |
| `--min-score` | 0.0-1.0 | — | Minimum match score threshold |
| `--prefer-recent` | flag | — | Bias ranking toward recency |
| `--prefer-app` | string | — | Bias toward matching app |
| + shared filters | | | |

```bash
clipmem recall "what was that command I copied?"
clipmem recall "find the URL" --format json
clipmem recall "find the URL" --human
clipmem recall --prefer-recent --hours 24
clipmem recall "exact text" --quote --full
```

### `clipmem search <QUERY>`

Direct lexical matching over stored text.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--format` | `text\|json\|jsonl\|md\|toon\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |
| `--mode` | `auto\|fts\|literal` | `auto` | Search mode |
| `--limit` | 1-250 | 10 | Page size |
| `--cursor` | string | — | Resume from prior `next_cursor` |
| + shared filters | | | |

```bash
clipmem search "git commit -m"
clipmem search "https://example.com" --format json
clipmem search "git commit -m" --human
clipmem search --mode literal "foo:bar"
clipmem search --mode fts "\"launchctl\" AND bootstrap"
```

### `clipmem recent`

Recent unique clipboard states, deduplicated by snapshot.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--format` | `text\|json\|jsonl\|md\|toon\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |
| `--limit` | 1-250 | 10 | Page size |
| `--cursor` | string | — | Resume from prior `next_cursor` |
| + shared filters | | | |

```bash
clipmem recent
clipmem recent --hours 24 --app safari
clipmem recent --human
clipmem recent --format json --limit 25 --cursor "<next_cursor>"
```

### `clipmem timeline`

Chronological capture events, one row per observation.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--format` | `text\|json\|jsonl\|md\|toon\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |
| `--sort` | `asc\|desc` | `desc` | Chronological sort order |
| `--limit` | 1-250 | 10 | Page size |
| `--cursor` | string | — | Resume from prior `next_cursor` |
| + shared filters | | | |

```bash
clipmem timeline --hours 24
clipmem timeline --hours 24 --human
clipmem timeline --sort asc --format json
clipmem timeline --app safari --has-url --limit 25 --format json
```

### `clipmem stats`

Archive aggregates and leaderboards for matching captures and
snapshots.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--format` | `text\|json\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |
| + shared filters | | | |

```bash
clipmem stats
clipmem stats --hours 24
clipmem stats --human
clipmem stats --app safari --format json
```

### `clipmem get <SNAPSHOT_ID>`

Nested item/representation detail for a single snapshot.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--format` | `text\|json\|jsonl\|md\|human` | `text` | Output format (no `toon`) |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |
| `--events` | 1-250 | 10 | Recent capture events to include |
| + shared filters | | | |

```bash
clipmem get 42
clipmem get 42 --format json
clipmem get 42 --human
clipmem get 42 --events 25 --format md
```

---

## Action commands

### `clipmem restore <SNAPSHOT_ID>`

Restore the full stored snapshot back onto the macOS clipboard.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--format` | `text\|json\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |

```bash
clipmem restore 42
clipmem restore 42 --human
clipmem restore 42 --format json
```

The active watcher suppresses the matching restored state, so restoring
an archived snapshot doesn't create a duplicate capture event.

### `clipmem export <SNAPSHOT_ID>`

Write one stored representation as raw bytes to a file.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--item` | integer | required | 0-based item index |
| `--uti` | string | required | Representation UTI |
| `--out` | path | required | Destination file path |
| `--force` | flag | — | Replace existing regular file |
| `--format` | `text\|json\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |
| + shared filters | | | |

```bash
clipmem export 42 --item 0 --uti public.png --out ./clipboard.png
clipmem export 42 --item 0 --uti public.png --out ./clipboard.png \
  --force
clipmem export 42 --item 0 --uti public.png --out ./clipboard.png \
  --human
clipmem export 42 --item 0 --uti public.png --out ./clipboard.png \
  --format json
```

### `clipmem forget <SNAPSHOT_ID>`

Irreversibly delete one snapshot and its capture history.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--format` | `text\|json\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |

```bash
clipmem forget 42
clipmem forget 42 --human
clipmem forget 42 --format json
```

### `clipmem purge`

Delete snapshots older than a given duration.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--older-than` | duration | required | Age threshold (`Nd`, `Nh`, `Nm`) |
| `--dry-run` | flag | — | Preview without deleting |
| `--format` | `text\|json\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |

```bash
clipmem purge --older-than 30d
clipmem purge --older-than 12h --dry-run
clipmem purge --older-than 30d --dry-run --human
clipmem purge --older-than 30d --dry-run --format json
```

### `clipmem storage compact`

Reclaim SQLite database and WAL disk space without changing clipboard
content.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--dry-run` | flag | — | Report current size/page/WAL state without running `VACUUM` |
| `--format` | `text\|json\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |

```bash
clipmem storage compact
clipmem storage compact --dry-run
clipmem storage compact --human
clipmem storage compact --format json
```

JSON output includes the database path, DB/WAL/SHM sizes before and
after, total bytes before and after, reclaimed bytes, estimated
reclaimable bytes, page count, freelist count, WAL checkpoint fields,
and whether compaction completed.

### `clipmem storage image-candidates`

List image representation rows eligible for optimization without rewriting
stored bytes.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--limit` | 1-250 | 25 | Maximum candidate rows to inspect |
| `--format` | `text\|json\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |

```bash
clipmem storage image-candidates
clipmem storage image-candidates --limit 50 --format json
```

### `clipmem storage optimize-images`

Convert eligible stored image representations to lossless WebP. The
optimizer replaces the original encoded image bytes only when WebP
saves at least 10% and at least 64 KiB. It preserves exact decoded
pixels, including alpha, but not original PNG/TIFF/JPEG container
metadata.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--dry-run` | flag | — | Estimate work and savings without changing rows |
| `--no-compact` | flag | — | Skip the automatic SQLite compaction step |
| `--limit` | 1-250 | 25 | Maximum unprocessed image rows to scan |
| `--progress` | `jsonl` | — | Stream progress events as newline-delimited JSON |
| `--format` | `text\|json\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |

```bash
clipmem storage optimize-images
clipmem storage optimize-images --human
clipmem storage optimize-images --dry-run --format json
clipmem storage optimize-images --no-compact --limit 50 --format json
clipmem storage optimize-images --limit 50 --format json
clipmem storage optimize-images --progress jsonl
```

Rows already marked `compressed` or `skipped` are not retried by
normal runs. JSON output reports scanned rows, compressed rows,
skipped rows, conflicts, original bytes, optimized bytes, logical
bytes saved, whether compaction ran, filesystem bytes reclaimed, and
whether database compaction is still recommended.

`--progress jsonl` streams newline-delimited JSON records to stdout and
cannot be combined with `--format`, `--json`, or `--human`. Progress
records include `started`, `scanning`, `compacting`, and `complete`
events. The `scanning` percentage is based on scanned image candidates,
while the final `complete.report` contains the same fields as
`--format json`.

---

## Management commands

### `clipmem setup`

Initialize the database, seed one capture, and start background
capture.

```bash
clipmem setup
clipmem setup --db /tmp/clipmem.sqlite3
```

Re-running `setup` is safe and updates the managed service definition.

### `clipmem service start`

Start background capture using the preferred service provider.

### `clipmem service stop`

Stop background capture without uninstalling the service definition.

### `clipmem service status`

Report provider state, freshness, service wiring, and watcher binary
mismatches.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--json` | flag | — | Emit status as JSON |
| `--human` | flag | — | Emit polished terminal output |

```bash
clipmem service status
clipmem service status --human
clipmem service status --json
```

### `clipmem service providers`

Inspect background service provider availability and selected provider without
starting, stopping, or reinstalling capture.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--format` | `text\|json\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |

```bash
clipmem service providers
clipmem service providers --format json
```

### `clipmem service revision`

Print the archive revision ledger from the active database without probing
Homebrew, LaunchAgent, or other service providers. This is the lightweight
change-detection command used by app polling when it only needs revision
counters.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--format` | `text\|json` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |

```bash
clipmem service revision
clipmem service revision --format json
clipmem service revision --json
```

### `clipmem service uninstall`

Remove the managed service definition.

### `clipmem watch`

Continuously poll the clipboard and archive observed changes.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--interval-ms` | integer | 400 | Poll interval in milliseconds |
| `--quiet` | flag | — | Suppress per-capture status lines |
| `--skip-initial` | flag | — | Skip the already-present clipboard state |

```bash
clipmem watch
clipmem watch --interval-ms 350
clipmem watch --skip-initial --quiet
```

Intervals below 50ms are clamped to 50ms at runtime.

### `clipmem capture-once`

Capture the current clipboard state once.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--json` | flag | — | Emit captured snapshot as JSON |
| `--human` | flag | — | Emit polished terminal output |

```bash
clipmem capture-once
clipmem capture-once --human
clipmem capture-once --json
```

### `clipmem ocr status`

Show OCR queue counts and how many snapshots have ready OCR text.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--format` | `text\|json\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |

```bash
clipmem ocr status
clipmem ocr status --human
clipmem ocr status --format json
```

### `clipmem ocr candidates`

List pending OCR hashes and affected snapshot counts without invoking Apple
Vision.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--limit` | 1-250 | 25 | Maximum candidate hashes |
| `--snapshot` | integer | — | Restrict candidates to one snapshot ID |
| `--format` | `text\|json\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |

```bash
clipmem ocr candidates --format json
clipmem ocr candidates --snapshot 42 --limit 10
```

### `clipmem ocr get <RAW_SHA256>`

Inspect one OCR result by raw representation hash.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--format` | `text\|json\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |

### `clipmem ocr clear <RAW_SHA256>`

Delete one OCR result and rebuild affected OCR cache rows.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--format` | `text\|json\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |

### `clipmem ocr run`

Process pending OCR work for existing image snapshots. This is the
backfill path for images copied before OCR was enabled.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--limit` | 1-250 | 25 | Maximum image hashes to process |
| `--snapshot` | integer | — | Restrict processing to one snapshot ID |
| `--retry-failed` | flag | — | Requeue failed OCR hashes before running |
| `--format` | `text\|json\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |

```bash
clipmem ocr run
clipmem ocr run --limit 50
clipmem ocr run --human
clipmem ocr run --snapshot 42 --retry-failed --format json
```

---

## Settings commands

### `clipmem settings show`

Show the current capture policy.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--format` | `text\|json\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |

### `clipmem settings pause <on|off>`

Persistently pause (`on`) or resume (`off`) capture.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--format` | `text\|json\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |

### `clipmem settings api-key-filter <on|off>`

Enable (`on`) or disable (`off`) API key filtering.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--format` | `text\|json\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |

### `clipmem settings ocr <on|off>`

Enable (`on`) or disable (`off`) automatic local OCR for new image
captures. OCR is off by default. Turning it on doesn't rewrite stored
image bytes or compress images.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--format` | `text\|json\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |

### `clipmem settings retention <duration|forever>`

Set retention to a duration (`30d`, `12h`, `15m`) or `forever` to
disable automatic pruning.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--format` | `text\|json\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |

### `clipmem settings reset`

Reset capture policy and ignored apps to defaults.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--format` | `text\|json\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |

### `clipmem settings ignore add <BUNDLE_ID>`

Add a bundle ID to the ignore list.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--format` | `text\|json\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |

### `clipmem settings ignore remove <BUNDLE_ID>`

Remove a bundle ID from the ignore list.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--format` | `text\|json\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |

### `clipmem settings ignore list`

List ignored bundle IDs.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--format` | `text\|json\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |

---

## Menu bar app commands

### `clipmem app settings show`

Show menu bar app preferences such as binary/database overrides, default recent
hours, default query mode, and hotkey enablement.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--format` | `text\|json\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |

### `clipmem app settings set <KEY> <VALUE>`

Set one app-local preference. Supported keys are
`binary-path-override`, `database-path-override`, `default-recent-hours`,
`default-query-mode`, and `hotkey-enabled`.

### `clipmem app settings clear <KEY>`

Clear one app-local preference and return it to the app default.

### `clipmem app launch-at-login show|set|clear`

Inspect or mutate the app-owned launch-at-login preference bridge. The menu bar
app applies the preference through `SMAppService` when it refreshes.

```bash
clipmem app launch-at-login show --format json
clipmem app launch-at-login set on --format json
clipmem app launch-at-login clear --format json
```

### `clipmem app update-check show|run|clear`

Inspect cached update-check state, run the GitHub latest stable release lookup,
or clear cached update-check state.

```bash
clipmem app update-check show --format json
clipmem app update-check run --format json
clipmem app update-check clear --format json
```

### `clipmem app quit`

Request the menu bar app to quit.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--format` | `text\|json\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |

```bash
clipmem app quit --format json
```

---

## Diagnostics

### `clipmem doctor`

Print SQLite and FTS5 diagnostics.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--json` | flag | — | Emit diagnostics as JSON |
| `--human` | flag | — | Emit polished terminal output |

```bash
clipmem doctor
clipmem doctor --human
clipmem doctor --json
```

A nonzero exit code means required checks failed.

---

## Agent integration

### `clipmem agents context`

Emit the metadata-first agent context bundle: `clipmem_version`, `generated_at`,
database path, service health, capture policy, archive revision, statistics,
menu bar app state, recent activity metadata, privacy guidance, and capability
map. Raw clipboard text and representation bytes are intentionally excluded,
but operational metadata such as app names, timestamps, counts, paths, and app
preference state is included.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--format` | `text\|json\|human` | `text` | Output format |
| `--json` | flag | — | Alias for `--format json` |
| `--human` | flag | — | Alias for `--format human` |

```bash
clipmem agents context --format json
```

### `clipmem agents openclaw install-skill`

Install the packaged clipboard-memory skill into OpenClaw.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--shared` | flag | — | Install to shared directory (`~/.openclaw/skills/`) |
| `--dest` | path | — | Install to a specific directory |
| `--force` | flag | — | Replace an existing skill directory |

```bash
clipmem agents openclaw install-skill
clipmem agents openclaw install-skill --shared
clipmem agents openclaw install-skill --dest /path/to/skill --force
```

### `clipmem agents openclaw uninstall-skill`

Remove an installed skill directory.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--shared` | flag | — | Remove from shared directory |
| `--dest` | path | — | Remove from a specific directory |

### `clipmem agents openclaw print-skill`

Print the packaged SKILL.md to stdout for inspection.

### `clipmem agents openclaw doctor`

Check host PATH, installed skill state, and sandbox guidance.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--shared` | flag | — | Check shared directory |
| `--dest` | path | — | Check a specific directory |

```bash
clipmem agents openclaw doctor
clipmem agents openclaw doctor --shared
```

A nonzero exit code means required integration checks failed.

### `clipmem agents hermes install-skill`

Install the packaged clipboard-memory skill into Hermes Agent.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--dest` | path | — | Install to a specific directory |
| `--force` | flag | — | Replace an existing skill directory |

```bash
clipmem agents hermes install-skill
clipmem agents hermes install-skill --dest /path/to/clipboard-memory --force
```

Without `--dest`, the command installs to
`~/.hermes/skills/productivity/clipboard-memory`.

### `clipmem agents hermes uninstall-skill`

Remove an installed Hermes Agent skill directory.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--dest` | path | — | Remove from a specific directory |

### `clipmem agents hermes print-skill`

Print the packaged Hermes Agent `SKILL.md` to stdout for inspection.

### `clipmem agents hermes doctor`

Check host PATH, installed skill state, Hermes metadata, and live
Hermes skill discovery when the `hermes` binary is available.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--dest` | path | — | Check a specific directory |

```bash
clipmem agents hermes doctor
clipmem agents hermes doctor --dest /path/to/clipboard-memory
```

A missing `hermes` binary is reported as a warning so package
validation can still pass. A nonzero exit code means required
integration checks failed.

---

## Environment variables

- `CLIPMEM_OPENCLAW_WORKSPACE` — overrides the OpenClaw workspace root
  used by `agents openclaw install-skill` and `agents openclaw doctor`.
  Falls back to `openclaw config get agents.defaults.workspace`, then
  `~/.openclaw/workspace`.
- `HOME` — resolves `~/` in default paths.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success |
| `1` | Uncategorized runtime failure |
| `2` | Invalid arguments |
| `3` | Not found |
| `4` | Unsupported format for this subcommand |
| `5` | Database error |
| `6` | Platform error (macOS API or filesystem) |

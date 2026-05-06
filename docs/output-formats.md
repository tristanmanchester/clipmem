# Output formats

clipmem supports six output formats. Every retrieval command defaults
to a human-readable format and can switch to structured output for
scripts and agents.

## Choosing the right format

- **`text`** — human-readable terminal output. Don't parse this.
- **`json`** — single structured object with a stable envelope. Parse
  this for scripts and agents.
- **`jsonl`** — newline-delimited JSON rows. Use when streaming many
  results through a pipe.
- **`md`** — compact markdown. Human-oriented, default for `recall`.
- **`toon`** — flat, compact skim view with scalar columns only. Use
  for quick scans when you don't need nested fields.
- **`human`** — polished terminal display with aligned summaries,
  compact tables, and visual bars. Don't parse this.

## Format support by command

| Command | Default format | Supported formats |
|---|---|---|
| `recall` | `md` | `md`, `json`, `toon`, `human` |
| `search` | `text` | `text`, `json`, `jsonl`, `md`, `toon`, `human` |
| `recent` | `text` | `text`, `json`, `jsonl`, `md`, `toon`, `human` |
| `timeline` | `text` | `text`, `json`, `jsonl`, `md`, `toon`, `human` |
| `stats` | `text` | `text`, `json`, `human` |
| `get` | `text` | `text`, `json`, `jsonl`, `md`, `human` |
| `restore` | `text` | `text`, `json`, `human` |
| `forget` | `text` | `text`, `json`, `human` |
| `purge` | `text` | `text`, `json`, `human` |
| `storage compact` | `text` | `text`, `json`, `human` |
| `storage optimize-images` | `text` | `text`, `json`, `human`; progress stream via `--progress jsonl` |
| `settings show` | `text` | `text`, `json`, `human` |
| `settings ignore list` | `text` | `text`, `json`, `human` |
| `ocr status` | `text` | `text`, `json`, `human` |
| `ocr run` | `text` | `text`, `json`, `human` |
| `service status` | `text` | `text`, `json` (via `--json`), `human` |
| `capture-once` | — | `json` (via `--json`), `human` |
| `doctor` | `text` | `text`, `json` (via `--json`), `human` |
| `export` | `text` | `text`, `json`, `human` |

`--json` is a compatibility alias for `--format json` on `recall`,
`search`, `recent`, `timeline`, `stats`, `get`, `restore`, `forget`,
`purge`, `storage compact`, `storage optimize-images`, `export`,
`ocr status`, `ocr run`, `capture-once`, and `doctor`.

`--human` is a compatibility alias for `--format human` where format
selection is available. For commands that use a JSON-only boolean flag,
such as `service status`, `capture-once`, and `doctor`, `--human`
selects the polished terminal view directly. `--human` can't be
combined with `--json` or a non-`human` `--format` value.

## Human terminal output

`--human` is for interactive terminal reading. It uses aligned key
metrics, compact tables, separators, and proportional bars for quick
scanning. When stdout is a terminal, clipmem may add color to IDs,
scores, and warnings. Captured output, redirected output, and
environments with `NO_COLOR` set stay uncolored.

Don't parse `--human` output. Use `--format json`, `--format jsonl`,
or `--format toon` for scripts and agent workflows that need stable
fields.

## JSON envelope

`search`, `recent`, and `timeline` return a stable paginated list
envelope:

```json
{
  "schema_version": 2,
  "command": "search",
  "generated_at": "2026-04-17T12:34:56Z",
  "applied_filters": { "hours": 24, "app": "safari" },
  "truncated": false,
  "next_cursor": null,
  "results": []
}
```

- `schema_version` — integer. Breaking changes bump this value.
  Additive changes (new optional keys) are allowed within the same
  version.
- `command` — echoes the subcommand name.
- `generated_at` — RFC 3339 timestamp when the response was produced.
- `applied_filters` — echoes the filters actually applied.
- `truncated` — `true` when more rows exist beyond `--limit`.
- `next_cursor` — opaque string to pass back as `--cursor` when
  `truncated` is `true`. `null` when there are no more rows.
- `results` — array of flattened snapshot rows.

`recall` returns a ranked-answer envelope instead of a paginated list:

```json
{
  "schema_version": 2,
  "command": "recall",
  "generated_at": "2026-04-17T12:34:56Z",
  "applied_filters": { "hours": 24, "app": "safari" },
  "query": "launchctl bootstrap",
  "best_candidate": {},
  "alternatives": [],
  "best_match_confidence": "high",
  "best_match_score": 0.92,
  "why_selected": "strong literal match",
  "quoted_text": null
}
```

- `query` — the optional recall query, or `null` when recall ranked
  recent items without a query
- `best_candidate` — the top-ranked row
- `alternatives` — lower-ranked candidate rows
- `why_selected` — short explanation of why this candidate was picked
- `best_match_confidence` — `"high"`, `"medium"`, or `"low"`
- `best_match_score` — float in `[0.0, 1.0]`
- `quoted_text` — present only when `--quote` is set and usable text
  exists

`stats` returns the same stable envelope shape without pagination
fields:

```json
{
  "schema_version": 2,
  "command": "stats",
  "generated_at": "2026-04-17T12:34:56Z",
  "applied_filters": { "hours": 24, "app": "safari" },
  "stats": {
    "snapshot_count": 12,
    "capture_event_count": 18,
    "dedupe_ratio": 0.33333333333333337
  }
}
```

The `stats` object includes archive totals, first and last observed
timestamps, content-kind breakdowns, top apps, full UTC hour and
weekday distributions, and fixed-length snapshot leaderboards.

## Flattened text fields

`get --format json` and retrieval rows surface flat text fields so
you don't have to walk the nested representation tree:

- `best_text`, `best_text_uti` — the best available text and its UTI
- `text_fragments` — array of `{ representation, text }` objects
- `urls` — extracted URLs
- `file_paths` — extracted file paths
- `html_text`, `rtf_text` — extracted HTML and RTF text (if present)
- `ocr_text`, `ocr_status` — OCR text and status when image OCR has
  been attempted
- `text_summary` — concise summary for agents

`search` and `recall` additionally include:

- `why_matched` — short explanation of the match
- `matched_fields` — which indexed fields matched

The full nested `items[].representations[]` structure is still
available on `get` for deep inspection and export workflows.

## TOON skim output

`--format toon` is a compact skim view designed for quick scanning.
It keeps scalar columns that are easy for agents and humans to read;
it intentionally omits heavyweight fields.

- `search` / `recent` rows keep scalar identifiers, timestamps, app
  info, `display_text`, counts, bytes, score, and `why_matched`.
- `timeline` rows keep scalar event metadata plus `display_text`.
- `recall` keeps scalar best-match metadata and candidate rows.

Use `--format json` when you need `urls`, `file_paths`,
`text_fragments`, `best_text_uti`, `html_text`, `rtf_text`,
`matched_fields`, or any nested representation detail.

`--format toon` isn't supported on `get` because `get` returns nested
snapshot detail rather than flat list output.

## Action JSON output

Action commands return structured output when you request JSON:

- `restore --format json` — whether the snapshot was restored, the
  restored item count, representation count, and snapshot ID
- `forget --format json` — the deleted snapshot ID, event count, item
  count, representation count, and byte count
- `purge --format json` — age cutoff, dry-run state, and
  deleted-or-would-delete counts
- `storage compact --format json` — DB/WAL/SHM file sizes,
  reclaimed bytes, estimated reclaimable bytes, page/freelist counts,
  and checkpoint state
- `storage optimize-images --format json` — scanned, compressed,
  skipped, conflict, byte-savings, auto-compaction, and filesystem
  savings fields
- `storage optimize-images --progress jsonl` — progress records with
  `started`, `scanning`, `compacting`, and `complete` events, where the
  final event wraps the same report fields as `--format json`
- `export --format json` — destination path, byte count, UTI, item
  index, snapshot ID, and representation hash
- `settings pause|api-key-filter|ocr|retention --format json` — the
  updated capture policy
- `settings ignore add|remove --format json` — the updated ignored
  bundle ID list

## Script-friendly guarantees

clipmem is designed for script and agent consumption:

- stdout contains only the requested command output
- stderr contains diagnostics only
- No interactive prompts anywhere in the CLI
- List commands use bounded defaults and opaque cursor pagination
- Retrieval `--format json` envelopes are stable within `schema_version: 2`; management and inspection commands may use command-specific JSON shapes and document their stable keys separately.

### Exit codes

| Code | Meaning |
|---|---|
| `0` | Success |
| `1` | Uncategorized runtime failure |
| `2` | Invalid arguments |
| `3` | Not found (snapshot ID, representation, or no results) |
| `4` | Unsupported format for this subcommand |
| `5` | Database error |
| `6` | Platform error (macOS API or filesystem) |

Scripts can use these to distinguish "no such snapshot" (try a
different ID) from "database locked" (retry with backoff) from
"wrong format" (fix the `--format` flag).

## Next steps

- [Command reference](command-reference.md) — exhaustive flag-level
  reference for every command
- [Agent integration](agent-integration.md) — use clipmem with
  OpenClaw, Hermes Agent, and other agent runtimes

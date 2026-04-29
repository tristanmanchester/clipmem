---
name: clipboard-memory
description: Recall what the user copied on this Mac via the local clipmem archive: exact text, commands, SQL, URLs, file paths, HTML, images, and PDFs. Trigger on requests like "what was that command I copied?", "paste back that SQL", "the URL I copied from Safari", "show me what I copied from Xcode today", "find the snippet/path/link from before I restarted", and indirect paraphrases about clipboard history or recovering copied content. Prefer this over web, repo, or filesystem search only when the target was likely copied.
version: "1.3.1"
license: MIT
platforms: [macos]
metadata:
  hermes:
    category: productivity
    tags: [clipboard, memory, macos, local-first, cli, retrieval]
---

Use this skill to recover clipboard history from the local `clipmem` archive in
Hermes Agent. This package is installed by
`clipmem agents hermes install-skill`. The canonical cross-agent source lives
under `skills/clipboard-memory/`.

# Clipboard Memory

Use this skill when the target is likely something the user copied on this Mac,
even if they never say "clipboard". Prefer it over web, repo, or filesystem
search only when the item likely lived on the clipboard.

## Use this skill when

The user asks things like:

- "what was that command I copied?"
- "show me the URL I copied from Safari earlier"
- "find that snippet, path, note, or link I copied yesterday"
- "give me the exact text I copied, not a summary"
- "what did I copy before I restarted?"
- "paste me back that SQL I was looking at"
- "get the PDF I copied last week"
- "show me everything I copied from Xcode today"

Also trigger when the user asks indirectly to recover or paste back something
from earlier and the clipboard is the most plausible source, even if they never
say "clipboard", "copy", or "paste".

## Do not use this skill for

- web search or current-events lookups
- searching the repository, local files, or browser history when the user never
  copied the target
- content the user typed but never copied to the clipboard
- anything on a non-macOS machine (`clipmem` only captures `NSPasteboard`)

## Fast decision workflow

1. Confirm the target likely came from the clipboard.
2. If empty results would be surprising, run
   [`scripts/check-setup.sh`](scripts/check-setup.sh) first. Prefer
   `scripts/check-setup.sh --json` when you want structured diagnostics. The
   prose fallback is [references/setup-check.md](references/setup-check.md).
3. Pick the narrowest starting command:
   - vague recollection -> `recall`
   - chronological history / "today" / "yesterday" / "in order" -> `timeline`
   - recent unique things -> `recent`
   - exact phrase, punctuation-heavy string, URL fragment, or file suffix ->
     `search --mode literal`
   - snapshot id already known -> `get`
   - binary / image / PDF recovery -> `get` then `export`
4. Add only the filters the user actually implied: `--hours`, `--app`,
   `--kind`, `--has-url`, `--has-file`, `--prefer-recent`.
5. If the first pass is weak, empty, or low-confidence, broaden once before
   giving up: widen `--hours`, drop source filters, inspect `alternatives`,
   then switch between `recall`, `search`, `recent`, and `timeline`.
6. Return the recovered content plus provenance such as `observed_at`,
   `app_name`, and `snapshot_id` when useful.

## Capability map

The repo-side agent-native action parity contract lives in
`docs/action-parity.md`. Use it when you need the maintained map from
user-visible outcomes to agent-accessible commands, entity CRUD expectations,
and derived-cache boundaries.

## Critical behavior rules

- Before answering from a stale, empty, or ambiguous archive, run `clipmem agents context --format json` and use the health, settings, revision, and stats summary to decide whether to broaden search or diagnose setup.
- Always use `--format json` when you will parse the response. `--format toon`
  is for token-efficient enumeration only. `--format jsonl` is for streaming
  many rows into a pipeline. Never parse `md` or `text`.
- Treat `recall` as a convenience ranking helper, not an authority. For uncertain cases, compose primitive commands in this order: `search`, `recent`, `timeline`, `get`, then OS follow-through such as `pbcopy`, `open`, or `open -R`.
- Never claim "nothing found" until you have broadened the search once and
  checked `truncated` / `next_cursor`.
- When `best_match_confidence` is `"low"` or there are several plausible hits,
  present the top candidates instead of pretending certainty.
- For exact-text requests, quote `best_text` verbatim. Do not paraphrase
  commands, SQL, code, URLs, or file paths unless the user asked for a summary.
- For binary-only snapshots where `best_text` is empty, say so plainly and
  recover the raw bytes with `export`; do not invent text.
- Keep the query and filters stable while paginating with `--cursor`.
- Prefer `recent` over `timeline` when the user wants unique things rather
  than every copy event.

## Health check and setup

Before querying, confirm the setup is healthy. Empty results can mean the
watcher is stale, not that the archive has no match.

1. Background capture must be running. `clipmem setup` is the canonical fix;
   Homebrew users can also use `brew services start clipmem`.
2. The binary `clipmem` must be on PATH with access to
   `~/Library/Application Support/clipmem/clipmem.sqlite3`.
3. Run [`scripts/check-setup.sh`](scripts/check-setup.sh) when results look
   wrong. It exits `0` on a healthy host, `1` if the watcher is stale, `2` if
   the binary is missing, and `3` if `clipmem doctor` or
   `clipmem service status` fails. `scripts/check-setup.sh --json` emits
   structured status for Hermes.
4. If Hermes cannot see this skill, run `clipmem agents hermes doctor` and
   follow its remediation lines.

## Command ladder

Always pick the narrowest command that answers the question.

1. **`clipmem recall`** - best-first ranked answer with alternatives. Start
   here for most "what was that thing I copied" requests.
2. **`clipmem timeline`** - chronological capture events, including repeated
   copies of the same content. Use for "today", "yesterday",
   "before I restarted", "in order", or "every time".
3. **`clipmem recent`** - recent unique snapshots, deduplicated by snapshot.
4. **`clipmem search`** - direct lexical / FTS matching. Use `--mode literal`
   for exact substrings or punctuation-heavy content.
5. **`clipmem get SNAPSHOT_ID`** - nested item / representation detail for a
   single snapshot already in hand.
6. **`clipmem export SNAPSHOT_ID --item N --uti UTI --out PATH`** - raw bytes
   for binary, image, or PDF payloads.
7. **`clipmem ocr candidates`, `clipmem ocr get`, `clipmem ocr clear`, and
   `clipmem storage image-candidates`** - inspect queued OCR or image
   optimization work before running batch workflows, or clear one stale OCR
   result.
8. **`clipmem settings reset --format json`** - reset capture policy and ignored
   apps when the user explicitly asks to restore defaults.
9. **`clipmem service providers --format json`** - inspect service provider
   state without starting or stopping capture.
10. **`clipmem app settings`, `clipmem app launch-at-login`, or `clipmem app update-check`
   with `--format json`** - inspect or change menu bar app preferences and
   app-owned state when the user asks about app defaults.
11. **`clipmem agents context --format json`** - compact health, settings,
   revision, stats, and capability context before multi-step work.

The full flag reference, JSON envelope, and kind values live in
[references/commands.md](references/commands.md),
[references/json-schema.md](references/json-schema.md), and
[references/examples.md](references/examples.md).

## Common recovery playbooks

- **Command, SQL, or code snippet** - start with
  `clipmem recall "..." --format json --limit 5`; if punctuation matters,
  follow with `clipmem search "..." --mode literal --format json`.
- **URL, path, or filename fragment** - add `--has-url` or `--has-file`; scope
  by `--app safari` or another app only if the user implied it.
- **"Everything I copied today / from Xcode"** - use
  `clipmem timeline --hours 24 --app xcode --format json` (or `--hours 48` for
  "yesterday", `--hours 168` for "last week").
- **Recent unique clipboard items** - use
  `clipmem recent --hours N --format json` or `--format toon` when you only
  need a compact list.
- **Image or PDF recovery** - locate the snapshot with `recall` or `search`,
  inspect it with `get`, then recover bytes with `export`.
- **Before a restart or long ago** - start broader than you think
  (`--hours 72`, `168`, or more) and prefer `timeline` if ordering matters.

## Output format rule

- `--format json` - single structured object. Use whenever you will parse the
  response. Stable within `schema_version: 2`.
- `--format toon` - flat, token-efficient list. Prefer for high-cardinality
  enumeration (`timeline`, `search`, `recent`, `recall`) when you only need the
  top fields. Note: `get` does **not** support `toon`.
- `--format jsonl` - newline-delimited records. Use when streaming many rows
  into a pipeline.
- `--format md` / `--format text` - human-readable previews only; never parse
  these.

`--json` is an alias for `--format json` on `search`, `recent`, `timeline`,
`get`, `capture-once`, and `doctor`.

## Reading the response

Read these JSON fields first; walk nested `items[].representations[]` only
after a `get` call:

- `best_candidate.best_text` - the flattened primary text.
- `best_candidate.urls` - URL array (empty when none).
- `best_candidate.file_paths` - file-URL array.
- `best_match_confidence`, `why_selected`, and `alternatives` - confidence and
  fallback options.
- `observed_at`, `app_name`, and `kind` - provenance and content shape.
- `next_cursor`, `truncated` - pagination state.
- `schema_version` - pin to `2` for stability.

Full schema in [references/json-schema.md](references/json-schema.md).

## Quick examples

```bash
# best-first answer
clipmem recall "that command I copied" --format json --limit 5

# Safari today, chronological
clipmem timeline --app safari --hours 24 --format json --limit 25

# recent unique items, token-efficient
clipmem recent --hours 72 --format toon --limit 20

# exact URL or punctuation-heavy string
clipmem search "https://example.com/path?q=1" --mode literal --format json

# recover an image
clipmem get 42 --format json
clipmem export 42 --item 0 --uti public.png --out ./clipboard.png
```

## Troubleshooting

If `recall` looks empty or weak, widen `--hours`, drop source filters, or
switch to `timeline` / `search`. For setup issues, Hermes skill discovery, or
binary-only snapshots, see
[references/troubleshooting.md](references/troubleshooting.md).

## Exit codes

`0` success, `1` uncategorized runtime, `2` invalid args, `3` not found, `4`
unsupported format, `5` database error, `6` platform error.

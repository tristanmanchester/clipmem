# Architecture

clipmem captures, deduplicates, and indexes every clipboard state into
a local SQLite database. This document explains the capture model,
storage design, search strategy, and known limitations.

## Capture model

For each observed clipboard state, clipmem stores:

- The whole clipboard snapshot
- Each pasteboard item inside that snapshot
- Every representation type the item exposes, identified by UTI
  (Uniform Type Identifier, for example `public.png`, `public.url`)
- Raw bytes for every representation
- Decoded text when the representation is text-like, plus strict
  UTF-8/UTF-16 recovery for some byte-only payloads
- A searchable text projection that powers default retrieval
- Optional OCR text for image representations when local OCR is
  enabled or backfilled
- The frontmost application at capture time as a best-effort source
  hint

## Deduplication

Identical clipboard snapshots are deduplicated by SHA-256 fingerprint.
Copying the same thing ten times doesn't create ten full blob copies —
it creates one `snapshots` row and ten `capture_events` rows. This
keeps the database compact without losing timing information.

## Search strategy

Default search uses SQLite's built-in full-text search (FTS5) when
the query fits, and falls back to literal substring matching for
wildcard-like input, invalid FTS syntax, or zero FTS hits.

Five FTS5 indexes power different query patterns:

- `snapshots_fts` — `unicode61` tokenizer over `search_text` and
  `preview_text` for standard full-text queries
- `snapshots_literal_fts` — `trigram` tokenizer over a combined
  haystack for fuzzy literal matching
- `snapshot_file_url_fts` — `trigram` tokenizer over extracted file
  URLs
- `snapshot_ocr_fts` — `unicode61` tokenizer over completed OCR text
- `snapshot_ocr_literal_fts` — `trigram` tokenizer over completed OCR
  text for literal matching

## Database schema

The core tables:

- `snapshots` — deduplicated clipboard states, keyed by SHA-256
  fingerprint
- `snapshot_items` — items inside a snapshot (a clipboard can contain
  multiple items)
- `item_representations` — one row per item/UTI pair, with raw blob
  storage
- `capture_events` — each time a snapshot was observed, with timestamp
  and frontmost app
- `snapshot_stats` — denormalized stats maintained by triggers
  (capture count, first/last observed, last frontmost app)

Supporting tables:

- `clipmem_settings` — persistent capture policy (pause, retention,
  API key filter, OCR)
- `ignored_bundle_ids` — excluded app bundle IDs
- `snapshot_projection_cache` — extracted URLs and file URLs
- `snapshot_event_filter_cache` — lowercase app names and bundle IDs
  for fast filtering
- `snapshot_literal_cache` — combined haystack for literal substring
  search
- `ocr_results` — OCR status and text keyed by representation
  `raw_sha256`
- `snapshot_ocr_cache` — aggregated ready OCR text per snapshot

The agent-facing entity and CRUD contract is documented in
[action parity](action-parity.md). In that contract, derived tables
such as `snapshot_stats`, projection caches, OCR caches, and FTS
virtual tables are not independent user entities; they are maintained
from source archive rows and verified through source-entity behavior.

Cache maintenance is intentionally hybrid. Triggers maintain the
steady-state derived tables: snapshot stats, event-filter cache,
literal and FTS search indexes, OCR cache rows, and representation
projection updates after ordinary representation changes. During
`store_capture`, new snapshot representation inserts temporarily defer
representation-derived trigger work. The capture transaction inserts
all representations, rebuilds `snapshot_projection_cache` once for the
new snapshot, then inserts the `capture_events` row so downstream
event/search refresh work sees the complete representation projection.

## What gets indexed

Text, HTML, URLs, file URLs, RTF, JSON, and XML are indexed when a
reasonable text form is available. These formats are searchable through
`recall`, `search`, `recent`, and `timeline`.

Images are stored as blobs. If OCR is enabled or backfilled, clipmem
also stores OCR text and status for image representations. Completed
OCR text is searchable through `recall`, `search`, `recent`, and
`timeline`. If an image-only snapshot has ready OCR text and no better
native text, that OCR text becomes `best_text` with
`best_text_uti = "com.clipmem.ocr.text"`.

PDFs and opaque binary types are stored as blobs but are not OCR'd.
They appear in results with metadata (kind, size, app, time), but
`best_text` is empty when no text projection exists. Use
`clipmem export` to recover the raw binary payload.

Phase 1 OCR doesn't store bounding boxes, confidence scores,
thumbnails, compressed images, or language preferences. Image
compression is not part of this phase.

## Polling behavior

The watcher polls `NSPasteboard.changeCount` on a short interval
(400ms by default). This is best-effort — if the clipboard changes
multiple times within a single poll window, intermediate states can be
missed.

This trade-off is intentional. Clipboard polling is the only reliable
approach on macOS (there is no push notification API for clipboard
changes), and 400ms catches nearly all real-world copies.

## Frontmost app tracking

The frontmost application is recorded as a practical hint, not a
guaranteed pasteboard owner. macOS doesn't expose which process
actually wrote to the pasteboard, so clipmem records the app that was
in the foreground at the time of capture. This is usually correct but
can be wrong for background processes that write to the clipboard.

## Search semantics

Search is lexical and rule-based, not semantic. `--mode auto` is the
default and intelligently picks between FTS5 and literal matching
based on query characteristics. It works well for commands, code,
URLs, notes, logs, and copied prose.

For strict boolean queries, use `--mode fts`. For exact substring
matching of punctuation-heavy content, use `--mode literal`.

## Platform constraints

- Clipboard capture uses `NSPasteboard` via `objc2` / `objc2-app-kit`
  and only runs on macOS.
- Image OCR uses Apple Vision through `objc2-vision` on macOS.
  Non-macOS builds compile with an unsupported OCR engine that returns
  a clear error.
- `clipmem restore` is macOS-only and writes the stored snapshot back
  to the live system clipboard.
- The database and search layers compile on other platforms for
  development, but capture and restore won't work.

## `get` and raw bytes

`clipmem get --format json` omits raw blob bytes. Flattened text
fields are included for convenience. Use `clipmem export` to recover
binary payloads (images, PDFs, and other non-text representations).

## TOON format constraints

`--format toon` is only supported for skim output (`search`, `recent`,
`timeline`, `recall`), not for nested `get` detail. It intentionally
omits heavyweight fields instead of embedding them as JSON blobs.

## Next steps

- [Output formats](output-formats.md) — JSON envelope, flattened
  fields, and TOON skim output
- [Command reference](command-reference.md) — exhaustive flag-level
  reference
- [Action parity](action-parity.md) — agent-native capability and CRUD
  contract
- [Contributing](contributing.md) — project layout and development
  workflow

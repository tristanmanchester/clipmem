# Changelog

All notable changes to `clipmem` are documented in this file. This file is the
source of truth for GitHub release notes, so every user-facing change belongs
under `Unreleased` before the next version is tagged.

The format is based on Keep a Changelog, and this project uses semantic
versioning where practical.

## Unreleased

### Added

- Added an agent-native action parity contract that maps user-visible clipboard
  outcomes to CLI and skill surfaces, and linked it from the CLI help, packaged
  skills, and architecture docs.
- Added a durable archive revision ledger that records archive, settings, OCR,
  storage, and service changes so external CLI and agent mutations can be
  observed by app and integration clients.
- Added revision-aware menu bar refresh handling so the macOS app detects
  external archive, settings, OCR, storage, and service changes made by agents
  or other CLI clients.
- Added `clipmem agents context`, a compact JSON/text context bundle for agents
  that reports service health, capture policy, archive revision, stats, and the
  maintained capability map.
- Added `clipmem app settings` commands for reading, setting, and clearing menu
  bar app preferences such as binary/database overrides, default recent hours,
  default query mode, and hotkey enablement.
- Added `clipmem app launch-at-login` and `clipmem app update-check` commands
  for agent-readable menu bar app state, including the app-owned launch
  preference bridge and cached update-check state.
- Added primitive `clipmem ocr candidates` and `clipmem storage image-candidates`
  inspection commands so agents can list queued OCR work and image optimization
  candidates without running batch workflows.
- Added `clipmem service providers --format json` for read-only service provider
  discovery without starting, stopping, or reinstalling capture.
- Added `clipmem service revision --format json` for lightweight archive
  revision polling without probing service providers.
- Added `clipmem settings reset` plus `clipmem ocr get` and `clipmem ocr clear`
  so capture settings and per-hash OCR results have explicit agent-accessible
  reset/read/delete operations.
- Strengthened packaged agent skill policy around context preflights, primitive
  command composition, low-confidence handling, exact-text quoting, and OS
  follow-through actions.
- Added menu bar Diagnostics discovery actions for copying agent context and
  skill install commands, and documented the agent context path in getting
  started and menu bar app docs.
- Added `clipmem app update-check run` and `clipmem app quit` so agents can
  run the menu bar app's live update check and request app quit through CLI
  parity commands.
- Expanded `clipmem agents context` with generation time, safe menu bar app
  state, bounded recent activity metadata, privacy guidance, and a fuller
  capability map.
- Added best-effort macOS app refresh notifications after CLI mutations so an
  open menu bar app can react faster while still using archive revisions as
  the durable source of truth.
- Added richer menu bar Diagnostics agent discovery actions for OpenClaw and
  Hermes doctor commands, packaged skill inspection, and the maintained
  capability map.

### Changed

- Refreshed the README menu bar popover screenshot to show Markdown rendering,
  link badges, and updated row icons.
- Updated the ClawHub `clipboard-memory` skill package to 1.3.4 for the
  agent-native command and JSON contract updates.
- Updated CI file-length limits for the larger agent-native command and
  menu bar refresh implementation files.

### Fixed

- Fixed `clipmem agents context --format json` on non-macOS test and
  integration environments so it still returns database, settings, revision,
  and capability metadata when macOS service probes are unavailable.
- Fixed app preference reads on non-macOS hosts so read-only app state commands
  report defaults instead of failing on a missing `defaults` command.
- Fixed `ocr clear` so it targets existing archives instead of creating a
  missing database path while clearing OCR state.
- Fixed the `clipmem agents context` capability map so it advertises only the
  output formats that the context command accepts.
- Fixed service revision recording so service start/stop/setup/uninstall does
  not create missing archive databases or fail completed service actions while
  recording best-effort app refresh metadata.
- Fixed best-effort macOS app refresh notifications so detached `notifyutil`
  calls are reaped after signaling the menu bar app.
- Fixed open menu bar app preference rehydration after external `clipmem app`
  mutations so database/binary overrides, launch-at-login, hotkey state, and
  cached update-check state refresh without restarting the app.
- Fixed external agent and CLI mutations so open History and Quick Recall
  windows refresh search, recall, recent, timeline, OCR, storage, and app
  default-mode/default-hours state from the durable revision ledger.
- Fixed `clipmem app` preference mutations to update both the invocation
  database and the configured app database override, so an already-open app can
  observe changes even when it uses a different archive path.
- Fixed `clipmem app update-check run --format json` to store integer
  timestamps so macOS defaults writes remain compatible with the app reader.
- Fixed read-only OCR inspection commands so `ocr candidates` and `ocr get` fail
  on missing archives instead of creating an empty database.
- Fixed `storage image-candidates` to inspect optimization metadata without
  loading stored image blobs.
- Fixed app refresh notifications so CLI mutations do not block on
  `notifyutil`.
- Fixed menu bar Markdown rendering for multiline text so headings and body text
  keep their line breaks in recent rows and History instead of being flattened
  into one line before rendering.
- Fixed shared retrieval filters so `--hours 0` is rejected instead of
  applying an effectively empty "since now" result window.
- Updated the crates.io trusted publishing action to its Node.js 24 release so
  release workflows no longer emit the GitHub Actions Node.js 20 deprecation
  warning.

## 0.4.3 - 2026-04-28

### Added

- Added basic Markdown rendering to the macOS menu bar app so recent rows and
  History detail text visually show bold, italics, headings, and non-clickable
  styled links while preserving the original clipboard content.
- Added Command-click activation for rendered Markdown links in the macOS menu
  bar app, including browser opening for web links, Finder reveal for file
  links, and link-aware row badges such as `url`, `file`, and `directory`.
- Added pointing-hand cursor feedback when hovering rendered Markdown links
  while holding Command in the macOS menu bar app.
- Added distinct colours to recent/history row metadata badges and matching
  row icons for plain text, URLs, files, directories, and mixed links.

### Changed

- Changed the macOS menu bar Markdown renderer to skip Command-click link
  monitoring for rows and detail views without Markdown links.
- Replaced wide internal archive/search construction paths with named-field
  parts and filter builder methods so query mapping and CLI filter
  normalization are harder to assemble incorrectly.
- Standardized stats archive DTOs on the same accessor-method API used by
  search hits, timeline events, and snapshot details while preserving their
  serialized output shape.
- Documented the failure modes for public recent and stats database queries so
  API consumers can rely on consistent error contracts across retrieval
  methods.
- Changed the public `Database::recent` API to return `RecentResults`,
  preserving whether more recent items are available while keeping a
  `recent_hits` convenience helper for callers that only need the hit list.
- Added a public `Database::timeline` API with `TimelineResults` so exported
  timeline DTOs have a supported archive retrieval path.
- Grouped retrieval CLI command helpers under a dedicated command submodule,
  reducing the flat command directory and making search, recall, cursor, and
  filter code easier to navigate together.
- Narrowed macOS `objc2` framework crate features to the APIs `clipmem` uses,
  reducing unnecessary default framework bindings in builds.
- Routed retrieval text output through the shared output envelope model used by
  structured formats while preserving the existing text output shape.

### Fixed

- Fixed macOS menu bar row rendering so metadata badge fallback computation no
  longer performs synchronous directory probes while rows are being drawn.
- Fixed macOS menu bar row badge updates so reused rows show the current
  content type after recent/search/history content changes.
- Fixed Markdown directory links in the macOS menu bar app so the resolved row
  badge can show `directory` instead of always falling back to `file`.
- Fixed rendered Markdown link event handling so Command-clicks from another
  Clipmem window are ignored by row-local link monitors.
- Fixed rendered Markdown link contrast in selected macOS menu bar rows so
  links stay readable on highlighted backgrounds.
- Fixed rendered Markdown link cursor tracking so one row being removed no
  longer disables Command-hover link feedback for other visible rows.
- Fixed rendered Markdown link event handling so link-bearing rows share a
  single window-level monitor instead of installing one monitor per row.
- Fixed rendered Markdown link cursor restoration so leaving a Command-hovered
  link restores the previous cursor instead of forcing the arrow cursor.
- Fixed rendered Markdown link hit-testing in the macOS menu bar app so
  Command-clicking trailing empty row space no longer activates the nearest
  link.
- Fixed Markdown heading rendering so code-like lines indented by four or more
  spaces preserve their leading `#` text instead of being treated as headings.
- Fixed a macOS menu bar crash when Command-clicking rendered Markdown links
  from the recent-items popover.
- Fixed literal-mode exact phrase searches so quoted punctuation-heavy queries
  such as `"config_test"` search for the unquoted phrase and still treat `%`
  and `_` as literal characters.
- Removed the narrow `colored` runtime dependency from human CLI output while preserving the existing color/no-color behavior.
- Fixed the output-format documentation so the recall JSON contract describes
  `best_candidate` and `alternatives` instead of list pagination fields.
- Added diagnostic notes to `clipmem service status` when launchctl, plist, or
  process probes fail so missing service details are easier to troubleshoot.
- Fixed direct LaunchAgent stop and uninstall commands so real `launchctl` and
  plist removal failures are reported instead of being silently ignored.
- Fixed background OCR worker startup so watchers for different archive
  databases no longer suppress each other's OCR processing.
- Fixed file URL path normalization so `file://localhost/...` clipboard
  entries decode and search correctly even when the `localhost` authority or
  URL scheme uses uppercase characters.
- Fixed the macOS menu bar JSON models so OCR text and status emitted by the
  CLI are decoded and available in history/detail views.
- Tightened macOS menu bar decoding for clipboard kinds, service state, and
  recall confidence so the app follows the CLI JSON contract directly.
- Corrected the `should_capture_change` Rustdoc so it describes the boolean
  watch-state contract instead of a nonexistent persistence error path.

### Security

- Hardened existing SQLite WAL and SHM sidecar file permissions alongside the
  main archive database when opening an archive.

## 0.4.2 - 2026-04-26

### Added

- Added live progress for menu bar image compression and a public
  `clipmem storage optimize-images --progress jsonl` stream for image
  optimization progress events.
- Added a documented solution for the file URL capture storage performance
  fix, including the trigger deferral pattern and benchmark results.

### Changed

- Moved settings action feedback to the bottom of the window and kept storage
  maintenance success messages visible longer so results are easier to read.
- Updated the ClawHub clipboard-memory skill package to 1.3.3 so the revised
  command and setup-check references can publish cleanly.

### Fixed

- Fixed menu bar popover scrolling so the recent-items scrollbar keeps a
  stable size and the top edge no longer rubber-bands with a flicker.
- Fixed file path search so displayed local paths containing percent-encoded
  characters such as `#` match their stored `file://` clipboard entries.
- Fixed file path search so local paths containing spaces match percent-encoded
  `file://` clipboard entries.
- Fixed `clipmem recall --limit` so weak-search fallback output counts the
  best candidate toward the requested limit instead of returning one extra
  alternative.

### Performance

- Improved OpenClaw and Hermes skill validation by deduplicating referenced
  Markdown files with borrowed-path tracking instead of allocating a path for
  every repeated link. In a 25,000-reference agent skill benchmark, median
  reference extraction time dropped from 5.483 ms to 3.875 ms.
- Improved text clipboard capture normalization by trusting an already
  searchable pasteboard string before decoding the same raw UTF-8 bytes and by
  avoiding an extra string copy in representation construction. In an 80-item,
  256 KiB text representation benchmark, median construction time dropped from
  64.894 ms to 60.672 ms.
- Improved human-readable CLI rendering for large clipboard previews by
  truncating table cells with bounded scanning and replacing newlines without
  copying the full source text. In a 10,000-cell long-preview benchmark,
  median truncation time dropped from 2.636 ms to 1.057 ms.
- Improved search and timeline row mapping for file-heavy clipboard entries by
  normalizing aggregated file paths in one pass and skipping percent-decoder
  work for unescaped `file://` URLs. In a 20,000-path row-mapping benchmark,
  median normalization time dropped from 3.574 ms to 1.185 ms.
- Improved snapshot detail hydration by loading all item representations with
  one grouped query instead of issuing one representation query per clipboard
  item. In a 1,000-item, 2,000-representation snapshot hydration benchmark,
  median item hydration time dropped from 1.298 ms to 911 us.
- Improved `recall` candidate ranking by avoiding per-candidate lowercase
  allocations during literal scoring and preferred-app matching. In a
  20,000-candidate literal scoring benchmark, median scoring time dropped from
  4.026 ms to 1.518 ms; in the preferred-app matching benchmark, median time
  dropped from 1.066 ms to 148 us.
- Improved API key filtering for large clipboard text by scanning contextual
  token lines with a rolling lookahead instead of collecting every line and
  allocating lowercase cue copies. In a 25,000-line contextual-token benchmark,
  median scan time dropped from 47.261 ms to 19.064 ms; in the no-match
  benchmark, median scan time dropped from 53.462 ms to 24.458 ms.
- Improved rich text clipboard normalization by parsing RTF directly from the
  input stream instead of first copying it into a character vector and
  allocating control words. In a 20,000-token RTF extraction benchmark, median
  extraction time dropped from 4.400 ms to 2.100 ms.
- Improved shared whitespace normalization used by text, HTML, RTF, and
  projection cleanup by appending normalized segments directly instead of
  collecting them before joining. In a 20,000-token whitespace benchmark,
  median normalization time dropped from 891.083 us to 857.833 us.
- Improved large TOON CLI output rendering by streaming row fields directly
  into the output buffer instead of allocating encoded fields before joining
  them. In a 10,000-row TOON rendering benchmark, median render time dropped
  from 19.614 ms to 8.554 ms.
- Improved large JSON and JSONL CLI output by writing serialized values
  directly to stdout instead of first materializing each line as a `String`.
  In a 10,000-row pretty JSON serialization benchmark, median serialization
  time dropped from 9.527 ms to 8.462 ms.
- Improved menu bar row rendering by reusing timestamp formatters instead of
  constructing new date formatters for each row. In a 10,000-row timestamp
  formatting benchmark, median formatting time dropped from 5.701 s to
  1.394 s.
- Improved unfiltered `stats` snapshot leaderboards by indexing the largest
  snapshot and most-captured ordering paths. In a 50,000-snapshot leaderboard
  benchmark, largest snapshot selection dropped from 5.300 ms to 6.416 us and
  most-captured selection dropped from 3.004 ms to 13.667 us.
- Improved pending OCR candidate discovery by indexing the queue in retry
  order. In a 50,000-row pending OCR benchmark, median candidate selection
  dropped from 3.664 ms to 8.666 us.
- Improved `optimize-images` candidate discovery on image-heavy archives by
  indexing the uncompressed image queue in the same order the optimizer scans
  it. In a 50,000-image candidate benchmark, median discovery time dropped
  from 5.603 ms to 19.583 us.
- Improved storage of new snapshots with many file URL representations by
  deferring representation-derived cache rebuilds until the snapshot rows are
  inserted. In a 1,000-file-url capture benchmark, median store time dropped
  from 3.565 s to 52.203 ms.

## 0.4.1 - 2026-04-23

### Fixed

- Fixed menu bar app version drift by syncing the checked-in Xcode bundle
  version to `Cargo.toml` and adding CI/release checks that fail when they
  differ.
- Fixed crate packaging by removing a local agent skill symlink from the
  published source set.

## 0.4.0 - 2026-04-23

### Added

- Added native Hermes Agent support with a packaged `clipboard-memory`
  skill, `clipmem agents hermes` install, uninstall, print, and doctor
  commands, and documentation for the default Hermes skill path.
- Added a file-length lint with CI enforcement so oversized Rust and Swift
  source files fail fast, while existing large files are pinned behind
  explicit per-file limits that can be ratcheted down over time.
- Added stricter Rust lint gates with `cargo fmt --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` in CI and release
  verification, while keeping the lightweight Python file-length lint as a
  separate check.
- Added a pinned Rust 1.88.0 toolchain file and macOS Rust CI coverage so
  Clippy and tests also run against macOS-gated source.

### Fixed

- Fixed literal search so app-name-only matches are included even when another
  clipboard item's text also matches the query.
- Fixed the CI workflow installer guard so it uses runner-provided tools
  instead of skipping checks when `rg` is unavailable.
- Fixed the file-length lint so it checks tracked source files only, rejects
  stale ratchet overrides, and ignores untracked local scratch or generated
  files.
- Fixed release publishing workflow permissions for crates.io Trusted
  Publishing and removed an unused reusable-workflow input.
- Fixed `clipmem stats --format` parsing so unsupported formats are rejected by
  the parser instead of being advertised as valid stats formats.
- Fixed `clipmem service status` so database inspection failures are reported
  in the status payload instead of aborting the whole command.
- Fixed HTML clipboard text normalization so numeric entities like `&#39;` and
  `&#x2F;` remain searchable as their original characters instead of spaces.
- Fixed file URL projection so `file://localhost/...` clipboard entries are
  shown as absolute local file paths instead of paths prefixed with
  `localhost`.

### Changed

- Raised the pinned Rust toolchain and minimum supported Rust version to 1.88
  so CI accepts the currently resolved dependency set.
- Updated GitHub Actions Rust toolchain setup to use Rust 1.88 in CI,
  ClawHub skill publishing, and crate publishing verification.
- Updated Rust 1.88 Clippy compatibility by removing Linux-only restore-plan
  dead code and adopting inline format arguments in newly linted paths.
- Split the oversized CLI, database, and CLI integration test sources into
  real Rust modules so the file-length lint no longer needs ratchet
  overrides for those areas.
- Updated the README and menu bar app documentation screenshot to show the
  redesigned recent-clips popover.
- Moved menu bar app compression and cleanup actions into a first-class
  Settings > Storage tab, and moved Diagnostics out of History into Settings.
- Updated the ClawHub clipboard-memory skill package metadata for a corrected
  1.3.1 publish with the current schema version 2, `recent`, and export
  overwrite guidance.
- Added GitHub Actions automation that checks the packaged
  `clipboard-memory` ClawHub skill for registry drift and publishes it when
  the repo declares a newer skill version.
- Updated release documentation to include the new local lint checks, current
  macOS runner, and version-neutral release artifact guidance.

## 0.3.4 - 2026-04-21

### Fixed

- Fixed LaunchAgent status tests and crate publishing on Linux by falling back
  to direct plist XML parsing when `plutil` is unavailable.

## 0.3.3 - 2026-04-21

### Changed

- Redesigned the menu bar popover to show clipboard items first. The service
  status grid, Setup/Start/Stop buttons, and update banner are removed from
  the popover; a compact HealthBanner now appears only when something is broken.
- Single-click on a popover item restores it to the clipboard and dismisses
  the popover.
- Merged the Recall and Search modes into a single Search mode with a
  Smart/Exact toggle. Smart uses semantic recall; Exact uses literal search.
  The mode picker in Quick Recall and History now shows three items (Search,
  Recent, Timeline) instead of four.
- Relocated service controls (Setup, Start, Stop, Uninstall) to the Settings
  General tab and Diagnostics view. Added an Open Logs Folder button to
  Diagnostics.
- Replaced the verbose update banner with a compact blue-tinted row that
  matches the HealthBanner style.
- Simplified the Quick Recall footer to Restore, Open in History, and Forget.
  Keyboard shortcuts for Focus Search and Refresh still work.
- Reduced the menu bar popover frame from 420x620 to 380x500.
- Updated ResultRowView spacing to use consistent design tokens and increased
  metadata font weight for better legibility.

### Fixed

- Fixed restore actions so restored clipboard states are suppressed as known
  duplicates instead of briefly appearing as fresh recent captures.
- Fixed restore recents moving or flashing when a stale watcher binary is
  still running by suppressing restore-induced duplicate events at the
  database layer and surfacing watcher binary mismatches in diagnostics.

### Performance

- Improved `clipmem stats` performance for whole-archive and snapshot-filtered
  reports by avoiding unnecessary temporary event tables and reusing maintained
  snapshot aggregates instead of recomputing per-snapshot capture totals. On a
  5,000-snapshot, 100,000-event archive benchmark, median unfiltered stats
  dropped from 799.8 ms to 37.5 ms, app-filtered stats dropped from 607.0 ms
  to 224.6 ms, and kind-filtered stats dropped from 1,075.2 ms to 559.8 ms.
- Added SQLite indexes for stats app and time-bucket aggregation so archive
  reports remain faster on larger capture histories.
- Improved JSON output performance for `search`, `recent`, `timeline`, and
  `recall` by avoiding redundant snapshot summary hydration while building
  list projections. In an internal projection hydration benchmark over 5,000
  snapshots and 500,000 capture events, median projection time dropped from
  245.313 ms to 90.780 ms.
- Improved repeated duplicate capture storage by skipping redundant app-filter
  and literal-search cache rewrites when a new event does not change those
  cached values. In warm repeated duplicate-capture runs, same-app duplicate
  storage averaged 0.470 s before the change and 0.143 s after it.
- Improved OCR candidate discovery on image-heavy archives by indexing image
  representation rows and raw image hashes used by OCR queue lookups. In an
  image-heavy candidate-discovery benchmark, average lookup time dropped from
  11,759.497 ms to 26.652 ms.
- Improved purge dry-run and deletion planning on large archives by using the
  maintained snapshot observation-time index for expiration candidate scans. A
  noisy 5,000-snapshot, 100,000-event dry-run benchmark showed the six-sample
  median moving from 8.452 ms to 8.035 ms, with the fastest sample improving
  from 6.631 ms to 4.262 ms.
- Improved snapshot detail, export, and restore hydration for heavily recopied
  items by reusing maintained capture summary data instead of aggregating the
  full event history on every lookup. In the heavy-event snapshot lookup
  benchmark, lookup time dropped from 30.347 ms to 0.406 ms.
- Improved OCR status reporting on large OCR queues by counting status buckets
  and snapshots with recognized text through dedicated SQLite indexes. The OCR
  status benchmark dropped from 32.861 ms to 5.274 ms on the immediate
  re-benchmark, with a later validation run at 13.560 ms.
- Improved `recent --hours` queries by using maintained snapshot observation
  timestamps instead of scanning matching capture events for since-only
  filters. The large retrieval benchmark's `recent_24h` query dropped from
  77.748 ms to 1.015 ms.
- Improved simple full-text search queries by skipping per-row phrase scoring
  checks that are only needed for quoted or multi-token searches. In the large
  retrieval benchmark, simple FTS search dropped from 20.700 ms to 13.714 ms,
  and app-filtered simple FTS dropped from 19.909 ms to 15.816 ms.

## 0.3.2 - 2026-04-20

### Changed

- Updated the README logo and macOS menu bar app icon assets to use the
  refreshed `clipmem` logo.

## 0.3.1 - 2026-04-20

### Fixed

- Fixed the local menu bar build script so it quits any existing menu bar app
  instance before launching the debug build and verifies the debug app is
  running.
- Fixed the menu bar icon asset so it uses the bundled transparent logo SVG as
  a compiled menu bar image, and added the app icon asset used by Spotlight and
  LaunchServices.

## 0.3.0 - 2026-04-20

### Added

- Added `--human` CLI output for polished terminal summaries, tables, and
  visual bars across retrieval, stats, detail, status, settings, OCR, storage,
  and archive action commands.
- Added a menu bar app manual purge flow that previews archive deletion counts
  before purging snapshots older than a chosen duration.
- Added the project logo to the README and bundled a black transparent SVG
  version for the macOS menu bar icon.

### Changed

- Added pull request CI coverage that builds and tests the macOS menu bar app
  with unsigned `xcodebuild` Debug jobs.

### Fixed

- Fixed Quick Recall's Open and Space actions so History opens focused on the
  selected snapshot instead of a generic History window.
- Fixed the macOS menu bar logo so it stays plain when healthy and shows an
  attention badge for stale, setup, error, and conflict states.
- Switched the README logo to a non-transparent PNG so it remains visible in
  GitHub dark mode.
- Fixed Homebrew formula repair for release artifacts that use a multiline
  Apple Silicon install guard.

## 0.2.13 - 2026-04-20

### Added

- Added `clipmem storage compact` for SQLite/WAL compaction and
  `clipmem storage optimize-images` for opt-in lossless WebP image
  optimization, with menu bar actions and JSON reports. Image
  optimization now compacts SQLite storage by default so freed pages
  are returned to the filesystem.

### Fixed

- Hardened Homebrew formula publishing so nested macOS and architecture guards
  are removed before tap audit runs, preventing release commits with no active
  formula URL on Linux or Intel macOS.
- Fixed menu-bar maintenance confirmations so Compact Database, Optimize
  Images, and Uninstall Service register the first button click instead of
  requiring the dropdown to be reopened.
- Fixed the menu-bar status item fallback icons so stale/setup/error states
  remain visible on macOS versions without the previous badge symbols.
- Clarified menu-bar capture status so stopped or missing watchers are shown as
  actionable service states instead of as stale clipboard activity.

## 0.2.12 - 2026-04-20

### Added

- Added database file size to `clipmem service status` text and JSON output,
  and surfaced it in the menu bar dropdown.
- Added inline search to the menu bar dropdown's recent clipboard list, with a
  shortcut into full History search when the loaded recents don't match.

### Changed

- Expanded the menu bar dropdown's recent preview and compacted its status
  summary so more clipboard history fits in the panel.
- Display clipboard capture times in the menu bar app using the Mac's local
  time zone instead of raw UTC database timestamps.

### Fixed

- Fixed generated Homebrew formula and cask files so the tap audit can validate
  release commits across Homebrew's supported OS and architecture matrix.

## 0.2.11 - 2026-04-19

### Fixed

- Fixed macOS text clipboard captures whose `public.utf16-plain-text`
  representation contained embedded NUL bytes, which could make stored text
  appear truncated or fail search even though the full text representation was
  captured.
- Added a schema repair that rebuilds stored snapshot text projections from
  captured item representations so affected existing captures become
  searchable after upgrade.

## 0.2.10 - 2026-04-19

### Changed

- Updated README and GitHub repository description to mention opt-in local OCR
  for copied images.

### Fixed

- Added the missing menu bar app Settings toggle for enabling or disabling
  local OCR for copied images.

## 0.2.9 - 2026-04-19

### Added

- Added opt-in local OCR for copied image snapshots on macOS using Apple
  Vision, including background OCR for new captures, backfill with
  `clipmem ocr run`, and queue reporting with `clipmem ocr status`.
- Added OCR settings with `clipmem settings ocr on|off`; OCR is disabled by
  default.
- Added OCR text/status fields to flattened JSON output and indexed completed
  OCR text for `search`, `recall`, `recent`, `timeline`, and `get`.

### Changed

- Bumped the JSON output schema version to `2` because flattened retrieval rows
  now include OCR fields.

## 0.2.8 - 2026-04-19

### Added

- Added `clipmem stats` with text and JSON output for archive aggregates,
  app/activity leaderboards, content mix, dedupe ratio, and shared retrieval
  filters.

## 0.2.7 - 2026-04-19

### Added

- Added a full documentation set under `docs/`, including installation,
  getting started, command reference, agent integration, archive management,
  output formats, privacy, architecture, menu bar app, and troubleshooting
  guides.
- Added menu bar app update checks against the latest stable GitHub release.
- Added update availability UI in the menu bar panel and settings window, with
  actions to copy the Homebrew upgrade command or open the release page.

### Changed

- Reduced `README.md` to a concise project overview that points readers to the
  deeper documentation pages.

## 0.2.6 - 2026-04-19

### Fixed

- Fixed `clipmem setup` recovery when a previously disabled direct LaunchAgent
  caused `launchctl bootstrap` to fail with status 5.

## 0.2.5 - 2026-04-19

### Added

- Added menu bar app screenshots to the README and release documentation.

### Fixed

- Fixed release app signing for notarization by disabling injected base
  entitlements and adding timestamped signing flags.

### Changed

- Updated GitHub Actions checkout steps to `actions/checkout@v6`.

## 0.2.4 - 2026-04-19

### Changed

- Added `Sendable` conformance to menu bar app model, client, and request types
  used across Swift concurrency boundaries.
- Bumped the menu bar app marketing version to `0.2.4`.

## 0.2.3 - 2026-04-19

### Changed

- Moved menu bar app release jobs to the `macos-15` GitHub Actions runner.
- Bumped the menu bar app marketing version to `0.2.3`.

## 0.2.2 - 2026-04-19

### Added

- Added a native SwiftUI macOS menu bar app with history browsing, quick recall,
  diagnostics, settings, launch-at-login support, and an Option-Shift-V quick
  recall hotkey.
- Added the Homebrew cask release path for installing the CLI and menu bar app
  together.
- Added clipboard restore, forget, purge, and persistent settings commands.
- Added pause, retention, ignored app, ignored bundle ID, and API-key filtering
  controls for capture policy.
- Added menu bar app tests, fixtures, command construction checks, and decoding
  coverage.
- Added clipboard-memory skill eval fixtures and improved setup checks.

### Changed

- Hardened release automation with a local `cargo-dist` installer, audited
  installer setup, trusted crate publishing, and hand-maintained workflow
  updates.
- Updated the service setup flow so Homebrew installs use direct LaunchAgent
  management unless a Homebrew service stanza is available.
- Expanded README and release documentation for the menu bar app, policy
  controls, and the split Homebrew formula and cask install surfaces.

### Fixed

- Hardened export destination handling, including explicit overwrite behavior.
- Fixed service binary path handling to avoid PATH poisoning.
- Fixed menu bar app setup feedback, window activation, command construction,
  error handling, filter handling, and review findings.

## 0.2.1 - 2026-04-17

### Changed

- Tightened TOON skim output for agent-facing retrieval flows.
- Reduced retrieval latency across `recent`, `search`, `recall`, `timeline`,
  and startup by moving read hot paths onto maintained snapshot-level caches
  instead of rebuilding archive-wide aggregates at query time.
- Accelerated filtered FTS searches by avoiding global event materialization
  and using cheaper snapshot-level filtering on common app and bundle-id paths.
- Accelerated literal search with trigram-backed candidate narrowing and
  dedicated fast paths for punctuation-heavy text and file-path lookups.
- Reduced healthy-open startup cost by skipping cache rebuilds when an existing
  database is already at the current schema version.

### Performance

- Improved `recent` on the large retrieval harness from roughly `98ms` at
  `v0.2.0` to single-digit milliseconds.
- Improved app-filtered FTS search from roughly `940ms` at its original hot
  spot to about `15ms`.
- Improved literal path lookups from tens of milliseconds to low single-digit
  milliseconds.
- Improved opening an existing healthy archive from roughly `118ms` to about
  `2-3ms`.

### Notes

- Existing databases continue to migrate forward automatically on open.
- Release automation is driven by a `v0.2.1` tag and validates that the tag
  matches `Cargo.toml`.

## 0.2.0 - 2026-04-17

### Added

- Added `clipmem setup` as the canonical onboarding command for seeding the
  archive and starting background capture.
- Added service management commands for status, start, stop, and uninstall.
- Added LaunchAgent status reporting and setup diagnostics for agent skill
  check scripts.

### Changed

- Productized the background capture flow so Homebrew, Cargo, and source
  installs share the same setup behavior.
- Updated LaunchAgent install and uninstall scripts to delegate to the CLI
  service workflow.
- Refreshed agent skill guidance for the new setup and service commands.

## 0.1.2 - 2026-04-17

### Added

- Added `skills/clipboard-memory/` as the canonical cross-agent skill package.

### Changed

- Renamed the OpenClaw skill package path from `clipboard_memory` to
  `clipboard-memory`.
- Updated OpenClaw install, uninstall, doctor, README, and tests to use the
  hyphenated skill package name.

## 0.1.1 - 2026-04-17

### Added

- Added `clipmem recall` for ranked, agent-facing clipboard retrieval.
- Added `clipmem timeline` for chronological capture-event retrieval.
- Added flattened text projections across retrieval output so agents can read
  clipboard content without walking raw representation data.
- Added portable and OpenClaw-native clipboard-memory skill packages with
  command reference, examples, JSON schema, setup checks, and troubleshooting
  docs.
- Added parity tests for packaged skill content.

### Changed

- Improved clipboard query ranking and OpenClaw skill packaging.
- Polished CLI help, exit codes, and stderr handling.
- Rewrote the README around the current CLI and agent workflow.

## 0.1.0 - 2026-04-17

### Added

- Added the initial macOS clipboard memory CLI backed by SQLite.
- Added clipboard capture from `NSPasteboard`, snapshot deduplication, capture
  events, raw representation storage, and frontmost-app source hints.
- Added searchable text projections with SQLite FTS5 and literal search
  fallback.
- Added commands to capture once, watch the clipboard, search history, list
  recent snapshots, inspect snapshots, export raw representations, and run
  database diagnostics.
- Added LaunchAgent install and uninstall scripts for background capture.
- Added the first OpenClaw skill package and installer script.
- Added Homebrew, crates.io, and GitHub release automation.

### Fixed

- Hardened search fallback escaping and UTF-16 decoding.
- Hardened archive storage, model boundaries, CLI rendering, watcher setup, and
  installer flows before the first public release.
- Hardened crates.io publishing preflight checks.

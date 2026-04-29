# Menu bar app

clipmem includes a native SwiftUI menu bar app at
`macos/ClipmemMenuBar/`. It's a Mac app, not Electron, Tauri, or a
webview. The app keeps the Rust CLI as the source of truth by shelling
out to `clipmem` asynchronously and decoding JSON responses from the
same commands used by scripts and agents.

![clipmem menu bar popover showing a filter field and recent clipboard entries](clipmem-menu-bar-panel.png)

## Install with Homebrew

Install the signed app with Homebrew:

```bash
brew install --cask tristanmanchester/tap/clipmem-app
open -a ClipmemMenuBar
```

The cask depends on the `clipmem` formula, runs `clipmem setup` after
installation, and enables launch at login by default. You can disable
launch at login in the app's **Settings** window.

## First launch

When the app launches for the first time, it:

1. Locates the `clipmem` binary (see binary discovery order below).
2. Self-excludes its own bundle ID (`io.openclaw.clipmem.menubar`)
   from the ignore list so it doesn't capture its own clipboard
   operations.
3. Connects to the existing SQLite database and settings layer — there
   is no separate Swift-side config database to keep in sync.

## App scenes

The app uses explicit SwiftUI scenes:

- **MenuBarExtra** — recent items, quick recall, settings, and quit.
- **History window** — full browsing surface with sidebar modes,
  filters, result list, detail view, and inspector actions.
- **Quick Recall window** — keyboard-first recall, search, recent,
  and timeline access.
- **Settings** — service controls, binary path override, database path,
  defaults, hotkey, ignored bundle IDs, retention, pause, API-key
  filtering, privacy controls, storage maintenance (compress images,
  compact database, purge old history), diagnostics, and agent
  integration command discovery.

Text snippets that use basic Markdown render bold, italics, headings, and
styled links in recent rows and History detail. Command-click a rendered link
to open `http` and `https` targets in the default browser or reveal `file`
targets in Finder. Link-bearing rows can show more specific badges such as
`url`, `file`, or `directory`; these badges are presentation-only and do not
change the stored clipboard kind.

## Binary discovery order

The app looks for the `clipmem` binary in this order:

1. `CLIPMEM_BINARY_PATH` environment variable
2. User preference override in the app's **Settings**
3. Repo-local `target/debug/clipmem`
4. Repo-local `target/release/clipmem`
5. `/opt/homebrew/bin/clipmem`
6. `/usr/local/bin/clipmem`
7. `~/.cargo/bin/clipmem`
8. `~/.local/bin/clipmem`

The app passes `--db <path>` to the backend when the database override
setting is set. Otherwise it uses the CLI default database path.

## Settings and policy

Ignore-list, pause, API-key filtering, and retention are the existing
SQLite-backed `clipmem settings` policy. Changes made in the app's
**Settings** window apply to the same database that the CLI and agents
use.

The Diagnostics tab includes agent integration actions for copying the
context preflight command, OpenClaw and Hermes doctor commands, packaged skill
install and print commands, and the maintained action-parity capability map.
These are copy-only helpers; the CLI remains the source of truth for actual
agent integration state.

App-local preferences such as the binary path override, database path override,
default recall/search mode, default recent window, hotkey enablement,
launch-at-login preference, and cached update-check state are also exposed
through `clipmem app ... --format json`. Mutations from the CLI bump the
archive revision ledger and publish a best-effort macOS notification so an
already-open menu bar app can rehydrate settings promptly. Missed notifications
are harmless because the app continues polling the durable revision signal with
`clipmem service revision --format json`, which avoids the heavier service
provider checks used by full status reports.

## Build from source

Build and launch the development app from the repo root:

```bash
scripts/build_and_run_menubar.sh
```

This script runs `cargo build`, builds the app with `xcodebuild`, sets
`CLIPMEM_BINARY_PATH` for the launched app, starts a matching debug
watcher from `target/debug/clipmem`, and opens the built `.app` from
`macos/ClipmemMenuBar/DerivedData`. The watcher pid and logs are stored
under `macos/ClipmemMenuBar/DerivedData`.

Use `--app-only` when you need to launch only the app and leave the
current watcher state untouched:

```bash
scripts/build_and_run_menubar.sh --app-only
```

You can also build or test the app directly:

```bash
xcodebuild -project macos/ClipmemMenuBar/ClipmemMenuBar.xcodeproj \
  -scheme ClipmemMenuBar -configuration Debug \
  -derivedDataPath macos/ClipmemMenuBar/DerivedData build

xcodebuild -project macos/ClipmemMenuBar/ClipmemMenuBar.xcodeproj \
  -scheme ClipmemMenuBar -configuration Debug \
  -derivedDataPath macos/ClipmemMenuBar/DerivedData test
```

## Known limitations

- Global hotkey support is Option-Shift-V with a reset-to-default
  preference. Arbitrary hotkey recording isn't implemented yet.
- Image representations can be exported for preview and action
  workflows, but PDFs and opaque binaries are shown through metadata
  and export actions rather than inline rendering.
- The app restores clipboard snapshots but doesn't auto-paste into
  the previous application.
- Search is lexical and rule-based. The UI intentionally doesn't
  describe it as semantic or AI search.

## Next steps

- [Installation](installation.md) — install the CLI that the menu
  bar app depends on
- [Getting started](getting-started.md) — set up background capture
- [Privacy and security](privacy-and-security.md) — understand how
  your data is stored

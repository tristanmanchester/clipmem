<p align="center">
  <img src="docs/clipmem-logo.png" alt="clipmem logo" width="128">
</p>

# clipmem

[![Crates.io](https://img.shields.io/crates/v/clipmem.svg)](https://crates.io/crates/clipmem)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust 1.88+](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org)

A private clipboard memory for your Mac.

`clipmem` watches the macOS clipboard, saves every observed state in a local
SQLite archive, and lets you search, recall, inspect, restore, and export what
you copied later. It works as a terminal tool, a native menu bar app, and an
agent-friendly memory layer for OpenClaw, Hermes Agent, and other local
automation.

Use it when you want to:

- recover the command, URL, snippet, file path, or message you copied earlier
- ask a coding agent "what did I copy?" without dumping your whole clipboard
  archive into the prompt
- restore an older clipboard item exactly, including rich clipboard formats
- filter history by time, app, content type, size, URL, image, PDF, or file URL
- make copied screenshots searchable with opt-in Apple Vision OCR, fully
  on-device

<table>
  <tr>
    <td align="center" width="50%">
      <img src="docs/clipmem-screenshot.png" alt="WhatsApp conversation asking OpenClaw what was copied, with clipmem recall output" width="500"><br>
      <sub>Ask OpenClaw what you copied.</sub>
    </td>
    <td align="center" width="50%">
      <img src="docs/clipmem-menu-bar-panel.png" alt="clipmem menu bar popover showing a filter field and recent clipboard entries" width="340"><br>
      <sub>Browse recent clips from the menu bar.</sub>
    </td>
  </tr>
</table>

## Requirements

- **macOS** — clipboard capture uses `NSPasteboard` via `objc2`
- **Apple Silicon** for the prebuilt Homebrew package
- **Rust 1.88+** for building from source

## Install

Choose one path:

CLI only (Apple Silicon):

```bash
brew install tristanmanchester/tap/clipmem
clipmem setup
```

CLI + native menu bar app (Apple Silicon):

```bash
brew install --cask tristanmanchester/tap/clipmem-app
open -a ClipmemMenuBar
```

The cask installs the CLI, runs `clipmem setup`, and enables launch at login.
See [docs/installation.md](docs/installation.md) for Cargo, source builds,
upgrades, and Intel Mac support.

## Start using it

```bash
clipmem setup
clipmem service status
```

Copy something new, then ask for it:

```bash
printf "clipmem can find this copied sentence later" | pbcopy
sleep 1
clipmem recall "copied sentence"
clipmem recent --hours 1 --human
```

Use the snapshot id from `recent` to inspect or restore a previous clipboard
state:

```bash
clipmem get 42 --human
clipmem restore 42
```

See [docs/getting-started.md](docs/getting-started.md) for a full walkthrough.

## Everyday commands

| Task | Command |
| --- | --- |
| Best-first recall | `clipmem recall "what was that shell command?"` |
| Exact text for pasting | `clipmem recall "the copied API URL" --quote --full` |
| Recent unique clipboard states | `clipmem recent --hours 24 --human` |
| Direct lexical search | `clipmem search "invoice" --hours 168` |
| Chronological event history | `clipmem timeline --hours 24 --format json` |
| Detailed snapshot view | `clipmem get <snapshot-id> --human` |
| Restore an old clipboard state | `clipmem restore <snapshot-id>` |
| Archive health check | `clipmem doctor` |
| Agent context preflight | `clipmem agents context --format json` |

For screenshots and image-only clipboard items:

```bash
clipmem settings ocr on
clipmem ocr run --limit 25
clipmem recall "text from that screenshot"
```

## Privacy model

- Clipboard data stays on this Mac in
  `~/Library/Application Support/clipmem/clipmem.sqlite3`.
- OCR uses Apple Vision locally; image data is not sent to a hosted service.
- The database directory is created with `0700` permissions and the database
  file with `0600` permissions.
- The archive is not encrypted by `clipmem`; use FileVault or equivalent disk
  encryption for at-rest protection.
- Use `clipmem forget`, `clipmem purge`, `clipmem settings pause`, and ignored
  app rules to control what is retained.

## Documentation

- [Installation](docs/installation.md) — all install methods, requirements, and upgrades
- [Getting started](docs/getting-started.md) — setup, background capture, and first queries
- [Searching and filtering](docs/searching-and-filtering.md) — recall, search, timeline, OCR text, filters, and pagination
- [Output formats](docs/output-formats.md) — JSON envelope, TOON, exit codes, and script-friendly guarantees
- [Managing your archive](docs/managing-your-archive.md) — restore, delete, export, and capture policy
- [Command reference](docs/command-reference.md) — exhaustive flag-level reference for every command
- [Agent integration](docs/agent-integration.md) — OpenClaw, Hermes Agent, and portable skill usage
- [Menu bar app](docs/menu-bar-app.md) — native SwiftUI menu bar frontend
- [Privacy and security](docs/privacy-and-security.md) — local-only guarantees, file permissions, and uninstall
- [Architecture](docs/architecture.md) — capture model, deduplication, search strategy, and limitations
- [Troubleshooting](docs/troubleshooting.md) — diagnose empty results, watcher issues, and database problems
- [Contributing](docs/contributing.md) — project layout, build, test, and release workflow

## License

`clipmem` is released under the MIT License. See [LICENSE](LICENSE).

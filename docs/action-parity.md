# Agent-native action parity

This document is the audit contract for keeping clipmem agent-native.
It maps user-visible outcomes to the agent-accessible surfaces that
produce the same result. The goal is not a one-to-one mapping of every
button to a tool; the goal is outcome parity: anything a user can
achieve through the CLI or menu bar app should be achievable by an
agent through documented commands, packaged skill guidance, or a
documented app-owned command bridge.

## Scorecard

| Principle | Target | Evidence |
|---|---:|---|
| Action parity | 100% | Every user-facing outcome has an agent path in this document. |
| Tools as primitives | 100% | Primitive commands are documented separately from convenience workflows. |
| Context injection | 100% | Agents have a documented context preflight command, refresh policy, app state, recent activity metadata, and privacy guidance. |
| Shared workspace | 100% | CLI, app, watcher, and agents converge on the same archive path unless explicitly overridden. |
| CRUD completeness | 100% | Product entities have supported operations or are documented as derived/internal non-entities. |
| UI integration | 100% | External CLI mutations have an app-observable revision path. |
| Capability discovery | 100% | Capabilities are discoverable through CLI help, docs, app surfaces, and packaged skills. |
| Prompt-native features | 100% | Agent judgment is described in editable skill/docs policy; workflow commands are convenience helpers. |

## Current source of truth

- Archive data lives in the SQLite database described in
  [architecture](architecture.md).
- The CLI is the durable automation surface. Prefer `--format json`
  when an agent parses output.
- The menu bar app shells out to the CLI for archive operations.
- Packaged skills under `skills/clipboard-memory/`,
  `extras/openclaw/clipboard-memory/`,
  `extras/hermes/clipboard-memory/`, and
  `extras/agent-skills/clipboard-memory/` teach agent runtimes how to
  use the CLI.

## User action map

| User-visible outcome | Agent path | Status |
|---|---|---|
| Search copied content by text | `clipmem search ... --format json` | Covered |
| Recall likely copied content | `clipmem recall ... --format json`, or compose `search`/`recent`/`timeline`/`get` | Covered |
| Browse recent unique items | `clipmem recent --format json` | Covered |
| Browse chronological capture events | `clipmem timeline --format json` | Covered |
| Inspect one snapshot | `clipmem get <snapshot-id> --format json` | Covered |
| Export raw bytes | `clipmem export <snapshot-id> --item <n> --uti <uti> --out <path> --format json` | Covered |
| Restore a snapshot to the clipboard | `clipmem restore <snapshot-id> --format json` | Covered |
| Delete one snapshot | `clipmem forget <snapshot-id> --format json` | Covered |
| Delete old snapshots | `clipmem purge --older-than <duration> --format json` | Covered |
| Preview old-snapshot deletion | `clipmem purge --older-than <duration> --dry-run --format json` | Covered |
| Compact storage | `clipmem storage compact --format json` | Covered |
| Optimize stored images | `clipmem storage optimize-images --format json` or `--progress jsonl` | Covered |
| Read capture policy | `clipmem settings show --format json` | Covered |
| Pause or resume capture | `clipmem settings pause on|off` | Covered |
| Toggle API-key filtering | `clipmem settings api-key-filter on|off` | Covered |
| Toggle OCR | `clipmem settings ocr on|off` | Covered |
| Change retention | `clipmem settings retention <duration|forever>` | Covered |
| Manage ignored bundle IDs | `clipmem settings ignore add|remove|list` | Covered |
| Inspect OCR queue | `clipmem ocr status --format json` | Covered |
| Inspect OCR candidates and results | `clipmem ocr candidates|get --format json` | Covered |
| Clear one OCR result | `clipmem ocr clear <raw-sha256> --format json` | Covered |
| Backfill OCR | `clipmem ocr run --format json` | Covered |
| Initialize capture | `clipmem setup` | Covered |
| Manage watcher service | `clipmem service start|stop|status|uninstall` | Covered |
| Inspect service providers | `clipmem service providers --format json` | Covered |
| Run diagnostics | `clipmem doctor --json` | Covered |
| Install or inspect agent skills | `clipmem agents openclaw ...`, `clipmem agents hermes ...` | Covered |
| Copy recovered text only | `clipmem get ... --format json` followed by `pbcopy` | Covered by CLI + shell |
| Open recovered URL | `clipmem get/search ... --format json` followed by `open <url>` | Covered by CLI + shell |
| Reveal recovered file path | `clipmem get/search ... --format json` followed by `open -R <path>` | Covered by CLI + shell |
| Read current agent context | `clipmem agents context --format json` | Covered |
| Read/write menu bar app preferences | `clipmem app settings show|set|clear --format json` | Covered |
| Read/write launch-at-login state | `clipmem app launch-at-login show|set|clear --format json` bridge | Covered |
| Check app update state | `clipmem app update-check show|run|clear --format json` | Covered |
| Quit the menu bar app | `clipmem app quit --format json` | Covered |
| Observe external agent mutations in open UI | `clipmem service status --json` revision signal, app polling, best-effort Darwin notifications, and `clipmem agents context --format json` | Covered |

## Primitive command taxonomy

Primitive commands expose one bounded read or mutation that agents can
compose without relying on product-specific ranking or batch workflow
judgment.

| Area | Read primitives | Mutation primitives |
|---|---|---|
| Archive retrieval | `search`, `recent`, `timeline`, `get`, `stats`, `purge --dry-run` | `restore`, `forget` |
| Archive bytes | `get`, `export` metadata from prior `get` | `export --out <path>` |
| Capture policy | `settings show`, `settings ignore list` | `settings pause`, `settings api-key-filter`, `settings ocr`, `settings retention`, `settings reset`, `settings ignore add|remove` |
| OCR | `ocr status`, `ocr candidates`, `ocr get` | `ocr clear`, `ocr run` |
| Storage | `storage image-candidates`, `storage compact --dry-run`, `storage optimize-images --dry-run` | `storage compact`, `storage optimize-images` |
| Service | `service status`, `service providers`, `doctor` | `setup`, `capture-once`, `service start|stop|uninstall` |
| Menu bar app | `app settings show`, `app launch-at-login show`, `app update-check show`, `app update-check run`, `app quit` | `app settings set|clear`, `app launch-at-login set|clear`, `app update-check clear` |
| Agent integration | `agents context`, `agents openclaw|hermes doctor`, `agents openclaw|hermes print-skill` | `agents openclaw|hermes install-skill|uninstall-skill` |

Convenience workflows remain supported for human ergonomics and agent
efficiency, but they are not the proof of primitive coverage:

- `recall` ranks likely answers and packages alternatives. Agents should
  treat it as a convenience ranking helper, then verify uncertain results
  with `search`, `recent`, `timeline`, or `get`.
- `setup` composes database initialization, one capture, and service
  startup. Agents can still inspect the resulting state with `service
  status`, `settings show`, and `agents context`.
- `purge`, `ocr run`, and `storage optimize-images` are bounded batch
  workflows with dry-run or candidate inspection commands where a preview
  is meaningful.

## Agent context contract

`clipmem agents context --format json` is the preflight bundle for agent
sessions. It intentionally avoids raw clipboard content and includes:

- `generated_at` and CLI `clipmem_version` for freshness.
- Database path and whether an archive snapshot was available.
- Service health, provider state, and archive revision counters.
- Capture settings and archive statistics.
- Safe menu bar app state from `app settings`, `app launch-at-login`,
  and `app update-check`.
- Bounded recent activity metadata for the last 24 hours and 7 days.
- Capability discovery grouped by retrieval, mutation, destructive,
  app, service, OCR, storage, and skill integration commands.
- Privacy guidance that the bundle excludes raw clipboard content while
  still including operational metadata such as app names, timestamps,
  counts, paths, and app preference state.
- Guidance that content retrieval remains explicit through
  `recall`, `search`, `recent`, `timeline`, `get`, or `export`.

## Entity CRUD contract

Product entities are the durable concepts users and agents reason
about. Derived caches and indexes are intentionally excluded because
their lifecycle follows their source entity.

| Entity | Create | Read | Update | Delete | Notes |
|---|---|---|---|---|---|
| Snapshot aggregate | `capture-once` / `watch` | `recall` / `search` / `recent` / `timeline` / `get` | Explicitly immutable; create a new capture or delete/re-capture instead | `forget` / `purge` | Captured representation bytes are archive records, not editable documents. |
| Capture event | Capture path | `timeline` / `get --events` | Explicitly immutable | `forget` / `purge` through owning snapshot | Events are history; deleting an event independently would falsify snapshot observation history. |
| Capture settings | Database initialization | `settings show` | `settings pause` / `api-key-filter` / `ocr` / `retention` | `settings reset` | Singleton row uses reset semantics instead of row deletion. |
| Ignored bundle ID | `settings ignore add` | `settings ignore list` | Remove plus add | `settings ignore remove` | Exact bundle IDs are normalized; rename has no extra semantics. |
| OCR result | OCR enqueue/backfill | `ocr get <raw-sha256>` | OCR processing / retry | `ocr clear <raw-sha256>` | OCR text is derived from stored image bytes. |
| App preference | App default values / first `clipmem app settings set` | `clipmem app settings show` | `clipmem app settings set <key> <value>` | `clipmem app settings clear <key>` | App-local, not archive policy. |
| App update-check cache | App update check | `clipmem app update-check show` | `clipmem app update-check run` | `clipmem app update-check clear` | App-local state mirrors the menu bar update checker. |
| Launch-at-login bridge | App default values / first `clipmem app launch-at-login set` | `clipmem app launch-at-login show` | `clipmem app launch-at-login set on|off` | `clipmem app launch-at-login clear` | CLI stores desired state; the menu bar app applies it through `SMAppService`. |
| Agent skill installation | `agents openclaw|hermes install-skill` | `agents openclaw|hermes doctor`, `print-skill` | `install-skill --force` | `agents openclaw|hermes uninstall-skill` | Skill package files live in the target agent runtime. |

Non-entities for CRUD scoring:

- `snapshot_stats`
- `snapshot_projection_cache`
- `snapshot_event_filter_cache`
- `snapshot_literal_cache`
- `snapshot_ocr_cache`
- FTS virtual tables
- `pending_restores`
- `archive_revisions`
- App `didConfigureLaunchAtLogin` and `didInstallSelfIgnore` flags

These stores are internal, derived, or temporary control state. Their
correctness is tested through source entity behavior and cache
rebuild/maintenance tests, not direct user CRUD.

## Agent behavior contract

Agents should:

- Prefer JSON for parsing and avoid parsing text/Markdown output.
- Load current context before broad clipboard recovery sessions or
  when results appear stale.
- Treat `recall` as a convenience ranking helper, not as the only
  possible source of truth.
- Inspect alternatives and confidence before claiming certainty.
- Use `get` before destructive, restore, export, or OS follow-through
  actions when a snapshot ID came from an uncertain search.
- Use dry-run commands before broad destructive operations.
- Report provenance such as snapshot ID, observed time, app, and
  confidence when helpful.

## Maintenance rules

- Update this file whenever a user-visible CLI, app, skill, storage, or
  workflow capability is added.
- Update packaged skill references and `tests/skill_parity.rs` when
  agent instructions change.
- Update `CHANGELOG.md` under `Unreleased` for user-facing changes.

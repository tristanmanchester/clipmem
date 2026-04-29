# Agent integration

clipmem is designed as a JSON-first CLI that agents such as OpenClaw
and Hermes Agent can call directly. You can also install packaged
skills that give agents structured knowledge of clipmem's commands,
filters, and output schemas.

The maintained agent-native capability map lives in
[action parity](action-parity.md). It records which user-visible
outcomes are available to agents, which commands provide them, which
entity CRUD operations are supported, and which internal stores are
excluded from CRUD scoring because they are derived or temporary state.

Before multi-step work, run `clipmem agents context --format json` to collect a
compact bundle with CLI `clipmem_version`, `generated_at`, database path, service
health, capture policy, archive revision state, archive statistics, app
preferences, recent activity metadata, privacy guidance, and a capability
summary. The context bundle is metadata-first: it does not include raw
clipboard text or representation bytes, but it does include operational
metadata such as app names, timestamps, counts, paths, and app preference state.

Agents can inspect and change menu bar app preferences with
`clipmem app settings show|set|clear --format json`, inspect or request
launch-at-login changes with `clipmem app launch-at-login show|set|clear
--format json`, run the same stable-release update lookup used by the app with
`clipmem app update-check run --format json`, and quit the menu bar app with
`clipmem app quit --format json`. App-state mutations bump the app-preferences
archive revision and publish a best-effort macOS refresh notification so an
open menu bar app can refresh from external CLI changes quickly while still
using the revision ledger as the durable source of truth.

## Install the OpenClaw skill

Install the packaged skill using the CLI rather than copying files
manually:

```bash
clipmem agents openclaw install-skill
```

By default this installs into the active OpenClaw workspace:

```
~/.openclaw/workspace/skills/clipboard-memory
```

The workspace root is resolved from
`openclaw config get agents.defaults.workspace`, falling back to
`~/.openclaw/workspace`.

To install into the shared OpenClaw skills directory instead:

```bash
clipmem agents openclaw install-skill --shared
# writes to ~/.openclaw/skills/clipboard-memory
```

Or to a specific destination:

```bash
clipmem agents openclaw install-skill --dest /path/to/skill --force
```

## Install the Hermes Agent skill

Install the packaged Hermes Agent skill using the CLI rather than
copying files manually:

```bash
clipmem agents hermes install-skill
```

By default this installs into the Hermes local skills tree:

```
~/.hermes/skills/productivity/clipboard-memory
```

To install into a specific destination, pass the exact skill directory:

```bash
clipmem agents hermes install-skill --dest /path/to/clipboard-memory --force
```

Hermes can also scan shared skill directories when they are configured
under `skills.external_dirs` in `~/.hermes/config.yaml`. Use `--dest`
when you want clipmem to write directly into one of those directories.

## What the skill ships

The OpenClaw, Hermes Agent, and portable packages ship the same
command references and setup checker. Runtime-specific package metadata
and troubleshooting guidance live in each package's `SKILL.md` and
`references/troubleshooting.md`.

```
SKILL.md
references/commands.md
references/json-schema.md
references/examples.md
references/setup-check.md
references/troubleshooting.md
scripts/check-setup.sh
```

The packaged skills point agents at the JSON-first retrieval commands:

- `clipmem recall "<query>" --format json` - start here
- `clipmem recall --prefer-recent --hours 24 --format json` - for
  recent items without a specific query
- `clipmem timeline ... --format json` - when chronology matters
- `clipmem search ... --format json` - when direct lexical matching
  matters
- `clipmem get <snapshot-id> --format json` - when deeper nested
  detail is needed

Agents that need to act on a recovered result can compose those
commands with normal macOS shell primitives: pipe exact text to
`pbcopy`, open recovered web URLs with `open`, reveal recovered file
paths with `open -R`, or use `clipmem restore` when the whole stored
pasteboard state should return to the clipboard. See the action parity
map for the full outcome-to-command table.

## Primitive commands vs workflows

The skills and parity map distinguish primitive commands from
convenience workflows:

- Primitive reads include `search`, `recent`, `timeline`, `get`,
  `stats`, `settings show`, `service status`, `service providers`, `service revision`,
  `ocr candidates`, `ocr get`, `storage image-candidates`, `app
  settings show`, `app launch-at-login show`, `app update-check show`,
  `app update-check run`, and `agents context`.
- Primitive mutations include `restore`, `forget`,
  `settings pause|api-key-filter|ocr|retention|reset`, `settings
  ignore add|remove`, `ocr clear`, `storage compact`, app preference
  setters, launch-at-login setters, `app update-check clear`, and
  `app quit`.
- Convenience workflows such as `recall`, `setup`, `purge`, `ocr run`,
  and `storage optimize-images` remain useful, but agents should verify
  uncertain results or preview broad mutations with the corresponding
  primitive read or dry-run commands.

## Manage the skill

Check OpenClaw integration health:

```bash
clipmem agents openclaw doctor
```

Check Hermes Agent integration health:

```bash
clipmem agents hermes doctor
```

Print the packaged OpenClaw SKILL.md for inspection:

```bash
clipmem agents openclaw print-skill
```

Print the packaged Hermes Agent SKILL.md for inspection:

```bash
clipmem agents hermes print-skill
```

Uninstall the OpenClaw skill:

```bash
clipmem agents openclaw uninstall-skill
```

Uninstall the Hermes Agent skill:

```bash
clipmem agents hermes uninstall-skill
```

## Troubleshooting

OpenClaw must be able to run `clipmem` from its own environment, not
just from your interactive shell.

If you installed `clipmem` into `~/.local/bin`, make sure that
directory is on the PATH seen by OpenClaw. If sandboxing is active,
the binary may also need to be available inside the sandbox image or
container.

Start with:

```bash
clipmem agents openclaw doctor
openclaw sandbox explain   # when available
```

Hermes must also be able to see both the skill directory and the
`clipmem` binary from its own runtime environment. Start with:

```bash
clipmem agents hermes doctor
hermes skills list          # when Hermes is installed
```

For empty results, sandbox visibility, or binary-only snapshots where
exact text isn't available, see
[troubleshooting](troubleshooting.md).

## Example agent prompts

Once the skill is installed, agents can handle prompts like:

- "Find that ffmpeg command I copied yesterday."
- "Search my clipboard history for the SQL migration with WAL mode."
- "What was the URL I copied from Safari about objc2 `NSPasteboard`?"
- "Show me the full clipboard entry for snapshot 128."

## Skill packaging

The skill exists in four locations within the repository:

- `skills/clipboard-memory/` - the canonical cross-agent skill source
  that repo-based installers discover from the root.
- `extras/openclaw/clipboard-memory/` - the OpenClaw-native package
  installed by `clipmem agents openclaw install-skill`.
- `extras/hermes/clipboard-memory/` - the Hermes Agent-native package
  installed by `clipmem agents hermes install-skill`.
- `extras/agent-skills/clipboard-memory/` - a portable packaging mirror
  for generic agent-skill runtimes beyond native targets.

For compatibility, `./scripts/install_openclaw_skill.sh` still exists
as a thin wrapper around
`clipmem agents openclaw install-skill --shared --force`.

## Next steps

- [Output formats](output-formats.md) — understand the JSON envelope,
  flattened text fields, and TOON skim output
- [Command reference](command-reference.md) — exhaustive flag-level
  reference for every command
- [Action parity](action-parity.md) — map user-visible outcomes to
  agent-accessible commands and documented entity operations
- [Troubleshooting](troubleshooting.md) — diagnose empty results,
  PATH issues, and database problems

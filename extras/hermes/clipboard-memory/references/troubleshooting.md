# Troubleshooting clipboard memory in Hermes

This guide covers the most common failures when Hermes Agent uses the
`clipboard-memory` skill. Start with the setup checker because it verifies the
`clipmem` binary, database health, and watcher freshness in one command.

## Quick health check

Run the packaged setup checker from the installed skill directory:

```bash
scripts/check-setup.sh --json
```

The JSON output is intended for agents. It reports whether `clipmem` is present,
whether `clipmem doctor --json` passed, whether a background service is loaded,
and whether the watcher looks stale.

For Hermes-specific installation checks, run:

```bash
clipmem agents hermes doctor
```

This command verifies the default Hermes target at
`~/.hermes/skills/productivity/clipboard-memory`, validates packaged skill
files, checks `clipmem` on PATH, and warns if the `hermes` binary is not
available for live discovery checks.

## Hermes cannot find the skill

Symptom: `clipboard-memory` does not appear in Hermes after installation.

1. Confirm the package exists:

   ```bash
   clipmem agents hermes doctor
   ```

2. If you installed to a custom path, run doctor against that exact path:

   ```bash
   clipmem agents hermes doctor --dest /path/to/clipboard-memory
   ```

3. Restart Hermes or open a fresh Hermes session so it rescans skills.
4. If you want Hermes to scan a shared skill directory instead of the default
   local directory, add that directory to `skills.external_dirs` in
   `~/.hermes/config.yaml`.

Hermes local skills normally live under `~/.hermes/skills/`. This package uses
the `productivity/clipboard-memory` category path by default.

## Hermes can see the skill but cannot run clipmem

Symptom: Hermes selects the skill, but shell calls fail with "clipmem: command
not found" or similar.

Run:

```bash
command -v clipmem
clipmem --version
clipmem doctor --json
```

If `clipmem` is installed under `~/.local/bin`, Homebrew, or a Cargo bin path,
make sure the PATH seen by Hermes includes that directory. GUI-launched
processes on macOS often receive a smaller PATH than interactive shells.

## Empty or stale results

Symptom: Commands work, but `recall`, `recent`, or `timeline` return too little.

Run:

```bash
clipmem service status --json
```

Expect `stale: false` and either the Homebrew service or direct LaunchAgent to
be running. If the watcher is stale, run:

```bash
clipmem setup
```

Homebrew users can also run:

```bash
brew services start clipmem
```

Then copy a new piece of text and retry the Hermes query.

## Search quality issues

If results are weak:

- Widen `--hours`.
- Drop restrictive `--app`, `--kind`, `--has-url`, or `--has-file-url` filters.
- Use `clipmem search --mode literal` for exact punctuation-heavy strings.
- Use `clipmem timeline` for chronology.
- Inspect `alternatives` when `best_match_confidence` is `"low"`.

Do not report "nothing found" until you have broadened once and checked
`truncated` / `next_cursor`.

## Binary, image, and PDF snapshots

Some snapshots have little or no text. When `best_text` is empty, inspect the
snapshot:

```bash
clipmem get SNAPSHOT_ID --format json
```

Then export the stored bytes:

```bash
clipmem export SNAPSHOT_ID --item 0 --uti public.png --out ./clipboard.png
```

Use the UTI from `items[].representations[]` rather than guessing when possible.

## Common fixes

| Symptom | Likely cause | Fix |
|---|---|---|
| Skill missing in Hermes | Installed outside scanned skills directory | Run `clipmem agents hermes install-skill --force` or configure `skills.external_dirs` |
| `clipmem` not found | Hermes PATH differs from shell PATH | Add the install directory to Hermes' environment |
| Empty results | Watcher is stale or stopped | Run `clipmem setup` and copy a fresh item |
| FTS search fails | SQLite FTS5 unavailable | Use `--mode literal` |
| Binary item has no text | Snapshot stores raw bytes | Use `get` then `export` |

# Getting started

clipmem archives your clipboard in the background and lets you search,
recall, and restore anything you've copied. This guide walks you through
setup, background capture, and your first queries.

## Initialize and start capture

Run the setup command to create the database, capture the current
clipboard state, and start the background watcher:

```bash
clipmem setup
```

### What setup does

1. Creates the database directory at
   `~/Library/Application Support/clipmem/` with `0700` permissions.
2. Initializes the SQLite database and FTS5 indexes.
3. Captures the current clipboard state as the first archive entry.
4. Writes or updates a per-user LaunchAgent plist
   (`io.openclaw.clipmem.watch`).
5. Bootstraps the LaunchAgent so the watcher runs in the background.

Re-running `clipmem setup` is always safe. It updates the service
definition without losing existing data.

## Verify capture is working

1. Copy something to your clipboard (for example, select text in a
   browser and press Cmd+C).
2. Wait a moment for the watcher to pick it up (it polls every 400ms
   by default).
3. Confirm the item appears in your archive:

```bash
clipmem recent --hours 1
```

You should see the item you just copied.

## Check service status

Confirm the background watcher is running:

```bash
clipmem service status
```

If the watcher isn't running, start it:

```bash
clipmem service start
```

For agent sessions, collect the bounded machine-readable context bundle:

```bash
clipmem agents context --format json
```

This reports the database path, service health, capture policy, archive
revision, archive statistics, and capability summary without dumping raw
clipboard content.

## Run your first queries

Recall something you copied:

```bash
clipmem recall "what was that shell command?"
```

List recent clipboard items from the last 24 hours:

```bash
clipmem recent --hours 24
```

Check the health of your database:

```bash
clipmem doctor
```

## Background capture

`clipmem setup` is the standard way to start background capture. The
watcher polls `NSPasteboard.changeCount` on a short interval (400ms by
default) and archives every new clipboard state it observes.

The current Homebrew formula doesn't include a Homebrew service stanza,
so `clipmem setup` and `clipmem service start` manage a direct per-user
LaunchAgent (`io.openclaw.clipmem.watch`). If a future formula adds a
service stanza, the same commands will prefer `brew services`
automatically.

### Service commands

```bash
clipmem setup               # initialize and start capture
clipmem service status       # check watcher state and freshness
clipmem service stop         # stop without uninstalling
clipmem service uninstall    # remove the managed service definition
```

### Foreground debugging

Run the watcher in the foreground to see captures in real time:

```bash
clipmem watch --interval-ms 350
```

Use `--skip-initial` to avoid capturing the clipboard state that already
exists when the watcher starts. Use `--quiet` to suppress per-capture
status lines.

## Next steps

- [Searching and filtering](searching-and-filtering.md) — find
  clipboard items with recall, search, timeline, and filters
- [Managing your archive](managing-your-archive.md) — restore, delete,
  and configure capture policy
- [Output formats](output-formats.md) — choose the right format for
  scripts, agents, and terminal use
- [Agent integration](agent-integration.md) — install packaged skills
  and use the agent context/capability commands

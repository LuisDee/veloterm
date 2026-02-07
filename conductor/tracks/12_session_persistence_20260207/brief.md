<!-- ARCHITECT CONTEXT | Track: 12_session_persistence | Wave: 5 | CC: v1 -->

## Cross-Cutting Constraints
- Graceful Shutdown: session save is part of the shutdown sequence
- Configuration Management: session file location configurable
- Testing: TDD, serialization round-trip tests

## Interfaces

### Owns
- Session serialization (layout → file)
- Session deserialization (file → layout)
- Auto-save on exit, manual save command

### Consumes
- `PaneTree` (Track 04) — pane layout to serialize
- `TabManager` (Track 06) — tab state to serialize
- `Config` (Track 03) — session file path, auto-restore setting

## Dependencies
- Track 04_pane_layout: PaneTree structure
- Track 06_tabs: TabManager structure

<!-- END ARCHITECT CONTEXT -->

# Track 12: Session Persistence

## What This Track Delivers

Save and restore the terminal session layout (tabs, panes, split ratios, CWD per pane) across application restarts. On graceful shutdown, the session state is serialized to a file. On startup, if a previous session exists, the user is offered to restore it — recreating the tab and pane layout with shells started in the saved working directories.

## Scope

### IN
- Session state serialization: tabs, pane tree structure, split ratios, CWD per pane
- Session file format (JSON or TOML)
- Auto-save on graceful exit
- Restore prompt on startup (or auto-restore if configured)
- Shell restart in saved CWD per pane
- Handle stale sessions (CWD no longer exists → fallback to $HOME)

### OUT
- Scrollback content persistence (too large, ephemeral by nature)
- Shell history persistence (shell handles this)
- Running process state (processes don't survive restart)
- Remote session persistence (SSH reconnection)

## Key Design Decisions

1. **Session file format**: JSON vs TOML vs binary?
   Trade-off: JSON is easy to serialize with serde; TOML matches config format; binary is compact but not human-debuggable

2. **Restore behavior**: Auto-restore silently vs prompt user vs configurable?
   Trade-off: auto-restore is seamless; prompt gives control; configurable satisfies both

3. **What to persist**: Layout only vs layout + CWD vs layout + CWD + environment variables?
   Trade-off: layout-only is simplest; CWD is most useful; env vars add complexity and potential security concerns

4. **Multi-session**: One session file (overwrite) vs named sessions vs session per window?
   Trade-off: single session is simplest; named sessions allow workspace switching; per-window is most flexible

## Architectural Notes

- The PaneTree and TabManager must expose serializable state — design the serialization boundary cleanly (don't serialize Terminal or PtySession directly)
- CWD per pane depends on shell integration (Track 10) for accurate CWD tracking — without it, fall back to the shell's initial CWD
- Session restore spawns new shell processes — it reconstructs layout, not running process state
- The session file should be human-readable for debugging — prefer JSON with serde_json

## Complexity: M
## Estimated Phases: ~3

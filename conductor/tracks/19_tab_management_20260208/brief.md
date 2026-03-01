<!-- ARCHITECT CONTEXT | Track: 19_tab_management | Wave: 7 | CC: v2 -->

## Cross-Cutting Constraints
- Platform Abstraction: Cmd+W/T/N on macOS, Ctrl+Shift+W/T/N on Linux
- Testing: TDD for tab close logic, reorder logic, keyboard shortcut routing
- UI Reference Compliance: tab styling must match the reference mockup aesthetic

## Interfaces

### Owns
- Tab close button (x) rendering and click handling
- Cmd+W to close current tab
- Cmd+T for new tab, Cmd+N for new window
- Cmd+1/2/3...9 to switch tabs
- Drag-to-reorder tabs
- Tab title showing running process or CWD

### Consumes
- `TabManager` (Track 06) — existing tab add/remove/switch infrastructure
- `TabBar` (Track 06) — existing tab bar rendering
- `Config` (Track 03) — keybinding configuration
- `ShellIntegration` (Track 10) — CWD tracking for tab titles

## Dependencies
- Track 06_tabs: TabManager and TabBar infrastructure
- Track 10_shell_integration: CWD tracking for tab titles

<!-- END ARCHITECT CONTEXT -->

# Track 19: Tab Management Polish

## UI Reference

The visual aesthetic MUST match the reference mockup:
- **Reference Cargo.toml:** `/Users/luisdeburnay/Downloads/Cargo.toml`
- **Reference main.rs:** `/Users/luisdeburnay/Downloads/src/main.rs`

Key reference details: tab bar at top with numbered tabs, accent color (#D97757) for active tab, "+" button for new tab, 28px tab bar height.

## What This Track Delivers

Polishes the tab system to production quality with close buttons on each tab, standard keyboard shortcuts (Cmd+W/T/N, Cmd+1-9), drag-to-reorder tabs, and intelligent tab titles that show the running process name or current working directory. Brings tab management to parity with professional terminals like iTerm2 and Terminal.app.

## Scope

### IN
- Close button (x) on each tab — visible on hover or always for active tab
- Cmd+W to close current tab (with confirmation if it's the last tab)
- Cmd+T to open new tab
- Cmd+N to open new window (spawns new VeloTerm process)
- Cmd+1 through Cmd+9 to switch to tab by number
- Cmd+Shift+[ and Cmd+Shift+] to switch to previous/next tab
- Drag-to-reorder tabs within the tab bar
- Tab title: show shell process name (e.g., "vim", "python") or CWD basename (e.g., "~/projects")
- Tab title updates when process or CWD changes

### OUT
- Tab pinning
- Tab groups or tab stacking
- Tab context menu (Track 20)
- Tab detach to new window
- Tab color customization

## Key Design Decisions

1. **Close button visibility**: Always visible vs visible on hover vs visible only on active tab?
   Trade-off: always visible is clearest; hover-only is cleaner but less discoverable; active-only saves space

2. **Last tab behavior**: Close last tab closes window vs close last tab opens new blank tab vs prevent closing last tab?
   Trade-off: close window is iTerm2 behavior; new blank is Terminal.app behavior; prevent closing is safest

3. **Tab drag implementation**: GPU-rendered drag preview vs swap on drag-over (no preview) vs native drag-and-drop?
   Trade-off: rendered preview is most polished; swap-on-hover is simpler; native DnD has platform-specific quirks

4. **Tab title source**: Shell integration CWD (accurate but requires Track 10) vs PTY process name (works without shell integration) vs user-configurable per tab?
   Trade-off: CWD is most useful; process name is reliable; user-configurable is most flexible

## Architectural Notes

- The existing `TabBar` in `src/tab/bar.rs` renders numbered tabs with a "+" button — adding close buttons requires modifying the per-tab rendering
- Cmd+1-9 is currently Ctrl+Shift+1-9 — need to verify whether Cmd+number is intercepted by macOS or available for the app
- Tab drag-to-reorder requires tracking mouse state (drag started, drag position, drop target) — similar state machine to selection drag
- Cmd+N for new window requires spawning a new process (`std::process::Command`) — verify the binary path and argument handling
- Tab title from CWD depends on shell integration (Track 10) providing `OSC 7` CWD updates — verify this works or fall back to process name

## Complexity: M
## Estimated Phases: ~3

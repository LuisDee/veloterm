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

# Track 19: Tab Management Polish — Specification

## Overview

Polishes the existing tab system to production quality. Adds close buttons on tabs, standard platform keyboard shortcuts (Cmd+W/T/N on macOS, Ctrl+Shift equivalents on Linux), Cmd+1-9 tab switching, drag-to-reorder via swap-on-hover, and intelligent tab titles showing the foreground process name (e.g., "vim", "claude") with CWD basename fallback when the process is just the default shell.

## Design Decisions

1. **Close button visibility**: Visible on hover for any tab, always visible on active tab. Matches Chrome/iTerm2 behavior.
2. **Last tab behavior**: Closing the last tab closes the window (iTerm2 behavior).
3. **Tab drag implementation**: Swap on drag-over — tabs swap positions as the user drags over them. No floating preview.
4. **Tab title source**: Foreground process name first (e.g., "vim", "claude", "python"). Fall back to CWD basename when the foreground process is just the default shell (zsh/bash).

## Existing Infrastructure

- `TabManager` in `src/tab/mod.rs` — manages ordered list of `Tab` objects with `new_tab()`, `close_tab()`, `select_tab()`, `next_tab()`, `prev_tab()`, `move_tab()`, `set_title()`
- `close_tab(index)` returns `None` if it's the last tab (refuses to close) — needs change for DD2
- `generate_tab_bar_quads()` in `src/tab/bar.rs` — renders tab backgrounds, separators, notification badges, "+" button
- `generate_tab_label_text_cells()` — renders numbered tab labels ("1", "2", "3") — needs change for tab titles
- `hit_test_tab_bar()` — returns `TabBarAction::SelectTab(idx)` or `NewTab` — needs close button hit testing
- `match_tab_command()` in `src/input/mod.rs` — uses `Ctrl+Shift` only, no `Cmd` support — needs platform-aware shortcuts
- `TabCommand` enum: `NewTab`, `NextTab`, `PrevTab`, `SelectTab(usize)`, `MoveTabLeft`, `MoveTabRight` — needs `CloseTab`
- `AppCommand` enum: font size only — needs `CloseWindow`, `NewWindow`
- `process_shell_updates()` in window.rs already sets tab titles from CWD — needs process name logic
- Tab bar constants: `TAB_BAR_HEIGHT=28.0`, `MAX_TAB_WIDTH=200.0`, `MIN_TAB_WIDTH=60.0`, `NEW_TAB_BUTTON_WIDTH=28.0`

## Functional Requirements

### FR-1: Tab Close Button
- Render an "x" close button on each tab
- Active tab: "x" always visible, right-aligned within the tab
- Inactive tabs: "x" visible only on hover
- Click on "x" closes that specific tab
- Close button area: ~16x16px hit target at right edge of tab
- "x" rendered as a text cell (character "x" or "×") in the tab label generation

### FR-2: Close Tab Command (Cmd+W / Ctrl+Shift+W)
- Add `CloseTab` variant to `TabCommand`
- Keybinding: Cmd+W on macOS, Ctrl+Shift+W on Linux
- Closes the currently active tab
- If last tab: close the entire window (exit the event loop)
- Modify `TabManager::close_tab()` to allow closing the last tab (return pane IDs for cleanup)

### FR-3: New Tab Shortcut (Cmd+T)
- Existing `Ctrl+Shift+T` maps to `NewTab`
- Add Cmd+T on macOS as an additional binding
- Behavior unchanged: creates new tab after the active tab

### FR-4: New Window (Cmd+N)
- Add `NewWindow` variant to `AppCommand`
- Keybinding: Cmd+N on macOS, Ctrl+Shift+N on Linux
- Spawns a new VeloTerm process using `std::process::Command`
- Uses `std::env::current_exe()` to find the binary path

### FR-5: Tab Number Switching (Cmd+1-9)
- Add Cmd+1-9 on macOS as additional bindings (alongside existing Ctrl+Shift+1-9)
- Cmd+1 selects tab 1, Cmd+9 selects tab 9 (or last tab)
- Behavior unchanged from existing `SelectTab`

### FR-6: Tab Navigation (Cmd+Shift+[ and ])
- Add Cmd+Shift+[ for previous tab, Cmd+Shift+] for next tab on macOS
- Maps to existing `PrevTab` and `NextTab` commands
- Supplements existing Ctrl+Shift+PageUp/PageDown

### FR-7: Drag-to-Reorder Tabs
- Track mouse drag state in tab bar: `is_dragging_tab: bool`, `drag_tab_index: usize`, `drag_x: f32`
- On mouse press on a tab (not on close button): begin drag
- On mouse move while dragging: if cursor crosses into adjacent tab's area, swap tabs via `TabManager::move_tab()`
- On mouse release: end drag
- Visual feedback: none beyond the immediate swap (no floating preview)

### FR-8: Intelligent Tab Titles
- Tab title priority: foreground process name > CWD basename > tab number
- Detect foreground process: query PTY for the foreground process group's name
- If process is the default shell (zsh, bash, fish, sh): fall back to CWD basename from shell integration (OSC 7)
- If neither available: show "Tab N" (current behavior with number)
- Title updates on each frame's PTY drain (when process or CWD changes)
- Truncate long titles to fit within tab width (ellipsis at end)

### FR-9: Platform-Aware Keybinding Refactor
- Refactor `match_tab_command()` to support both Cmd (macOS) and Ctrl+Shift (Linux) modifiers
- Use existing `is_primary_modifier()` pattern from `match_app_command()`
- Maintain backward compatibility with Ctrl+Shift bindings on all platforms

## Non-Functional Requirements

- Tab close must clean up PTY resources (kill child process, close file descriptors)
- Drag-to-reorder must be responsive (<16ms per swap)
- Process name detection must not block the render loop
- Tab title updates should not cause unnecessary redraws

## Acceptance Criteria

- [ ] Close button visible on active tab, appears on hover for inactive tabs
- [ ] Clicking close button closes the correct tab
- [ ] Cmd+W closes current tab; closing last tab closes window
- [ ] Cmd+T opens new tab
- [ ] Cmd+N opens new VeloTerm window
- [ ] Cmd+1-9 switches to numbered tab
- [ ] Cmd+Shift+[ and ] navigate between tabs
- [ ] Dragging a tab over another swaps their positions
- [ ] Tab titles show process name (e.g., "vim") or CWD basename
- [ ] All shortcuts work with Ctrl+Shift on Linux

## Out of Scope

- Tab pinning
- Tab groups or tab stacking
- Tab context menu (Track 20)
- Tab detach to new window
- Tab color customization
- Floating drag preview (GPU-rendered)

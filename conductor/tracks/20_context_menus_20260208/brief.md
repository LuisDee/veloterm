<!-- ARCHITECT CONTEXT | Track: 20_context_menus | Wave: 7 | CC: v2 -->

## Cross-Cutting Constraints
- Testing: TDD for menu item dispatch, menu positioning, keyboard navigation within menus
- Platform Abstraction: right-click on all platforms; Ctrl+click as alternative on macOS
- UI Reference Compliance: menu styling must match warm dark theme aesthetic

## Interfaces

### Owns
- Context menu overlay rendering
- Tab bar context menu: "New Tab", "Close Tab", "Close Other Tabs"
- Terminal area context menu: "Copy", "Paste", "Select All", "Clear", "New Tab", "New Window", "Split Pane Horizontally", "Split Pane Vertically"
- Menu keyboard navigation (arrow keys, Enter to select, Escape to dismiss)

### Consumes
- `TabManager` (Track 06) — tab operations (new, close, close others)
- `PaneTree` (Track 04) — pane split operations
- `Clipboard` (Track 02) — copy/paste operations
- `Terminal` (Track 02) — clear, select all
- `Config` (Track 03) — keybinding hints for menu items

## Dependencies
- Track 06_tabs: tab operations
- Track 04_pane_layout: pane split operations
- Track 17_selection_clipboard: basic context menu pattern established

<!-- END ARCHITECT CONTEXT -->

# Track 20: Context Menus

## UI Reference

The visual aesthetic MUST match the reference mockup:
- **Reference Cargo.toml:** `/Users/luisdeburnay/Downloads/Cargo.toml`
- **Reference main.rs:** `/Users/luisdeburnay/Downloads/src/main.rs`

Context menus should use the elevated surface color (`#282724`), with warm text and subtle borders matching the Anthropic dark theme.

## What This Track Delivers

Full right-click context menu system for both the tab bar and the terminal content area. Tab bar context menu provides tab management operations (new, close, close others). Terminal area context menu provides clipboard operations, clearing, and pane/tab management. Menus display keyboard shortcut hints and support keyboard navigation.

## Scope

### IN
- Right-click context menu infrastructure (reusable menu component)
- Tab bar right-click menu: "New Tab" (Cmd+T), "Close Tab" (Cmd+W), "Close Other Tabs"
- Terminal area right-click menu: "Copy" (Cmd+C), "Paste" (Cmd+V), "Select All" (Cmd+A), separator, "Clear" (Cmd+K), separator, "New Tab" (Cmd+T), "New Window" (Cmd+N), separator, "Split Pane Right", "Split Pane Down"
- Menu items show keyboard shortcut hints on the right
- Menu items conditionally enabled/disabled (e.g., "Copy" disabled when nothing selected)
- Menu dismissed on click outside, Escape, or item selection
- Menu positioned near cursor, clamped to window bounds

### OUT
- Nested submenus (keep menus flat for now)
- Menu bar at top of window (macOS native menu bar)
- Customizable menu items via config
- Menu animations/transitions

## Key Design Decisions

1. **Menu rendering**: egui popup overlay vs custom GPU-rendered menu vs native OS context menu (NSMenu on macOS)?
   Trade-off: egui handles text input and layout; custom matches theme exactly; native feels platform-correct but doesn't match theme

2. **Menu item organization**: Flat list with separators vs grouped sections with headers?
   Trade-off: flat with separators is standard and simpler; grouped adds visual hierarchy but more complexity

3. **Context sensitivity**: Same menu everywhere vs different menus per context (tab bar vs terminal vs selection)?
   Trade-off: different menus per context is more useful; same menu is simpler to maintain

## Architectural Notes

- Track 17 establishes the basic context menu pattern (Copy/Paste/Select All) — this track extends it with full menus
- The menu overlay must capture all input while visible (modal behavior) — same pattern as search overlay and vi-mode
- Menu items dispatch actions: some are direct (copy, paste) and some require confirmation (close other tabs)
- If egui is used for menus, it's already in the dependency list as a Phase 2+ dependency — verify egui-wgpu integration
- Menu position must account for terminal padding, tab bar height, and window boundaries

## Complexity: M
## Estimated Phases: ~3

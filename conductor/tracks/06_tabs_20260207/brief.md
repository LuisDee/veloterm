<!-- ARCHITECT CONTEXT | Track: 06_tabs | Wave: 3 | CC: v1 -->

## Cross-Cutting Constraints
- Graceful Shutdown: close all tabs and their pane trees cleanly
- Platform Abstraction: Cmd+T on macOS, Ctrl+Shift+T on Linux
- Testing: TDD, visual validation via screenshots

## Interfaces

### Owns
- `TabManager` — tab lifecycle (new, close, switch, reorder)
- Tab bar rendering (UI chrome at top of window)
- Tab title derivation (from shell CWD or running command)

### Consumes
- `PaneTree` (Track 04) — each tab owns one pane tree
- `Config` (Track 03) — tab keybindings, tab bar theme

## Dependencies
- Track 04_pane_layout: PaneTree for per-tab pane management
- Track 03_config: keybindings and tab bar styling

<!-- END ARCHITECT CONTEXT -->

# Track 06: Tab System

## What This Track Delivers

A tab system where each tab contains an independent pane tree (which may itself have splits). Users can create, close, switch, and reorder tabs via keyboard shortcuts and mouse clicks. The tab bar renders at the top of the window showing tab titles derived from the active pane's CWD or running command.

## Scope

### IN
- Tab data structure: ordered list of tabs, each owning a PaneTree
- Tab bar rendering at the top of the window
- New tab creation (with fresh shell in new pane)
- Close tab (with confirmation if multiple panes)
- Switch tabs via click, keyboard shortcut, or Ctrl+number
- Tab title: derived from focused pane's shell CWD or running command name
- Tab reordering via drag or keyboard

### OUT
- Split pane logic within tabs (Track 04 — pane_layout)
- Tab-specific configuration overrides (not planned)
- Detach tab to new window (not planned for MVP)

## Key Design Decisions

1. **Tab bar rendering**: egui widget vs custom GPU-rendered tab bar vs native platform tab bar?
   Trade-off: egui is fastest to implement; custom GPU matches terminal aesthetic; native feels platform-appropriate but diverges per OS

2. **Tab title source**: Shell CWD only vs running command name vs user-configurable title?
   Trade-off: CWD is reliable; command name requires shell integration; user title adds flexibility

3. **Close confirmation**: Confirm on close if pane has running process vs always close immediately vs configurable?
   Trade-off: confirmation prevents accidental loss; immediate close is faster; configurable adds complexity

4. **Tab limit**: Unlimited tabs vs configurable maximum vs scroll when tab bar overflows?
   Trade-off: unlimited is flexible but tab bar overflows; scrolling handles overflow; limit is opinionated

5. **Tab-to-window relationship**: One tab bar per window vs tabs-as-windows (each tab is a separate OS window)?
   Trade-off: single window with tab bar is standard; separate windows leverage OS window management

## Architectural Notes

- The tab bar consumes vertical space — the renderer must account for tab bar height when calculating the grid area
- Each tab switch means swapping which PaneTree is active — the renderer renders only the active tab's panes
- Tab titles that show CWD will benefit from Track 10 (shell integration) for accurate CWD tracking — use a fallback (shell process name) until then
- If egui is chosen for tab bar rendering, this decision also applies to Track 05 (pane UI) and Track 09 (search overlay)

## Complexity: M
## Estimated Phases: ~3

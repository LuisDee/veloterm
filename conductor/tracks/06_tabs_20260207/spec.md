# Track 06: Tab System — Specification

## Overview

Add a tab system where each tab owns an independent PaneTree. Users can create, close, switch, and reorder tabs via keyboard shortcuts and mouse clicks. A GPU-rendered tab bar at the top of the window shows tab titles derived from the active pane's shell name (with CWD fallback when shell integration lands in Track 10).

## Design Decisions

| # | Decision | Choice | Rationale |
|---|----------|--------|-----------|
| 1 | Tab bar rendering | Custom GPU overlay | Uses existing OverlayQuad pipeline + glyph atlas. No new dependencies. Consistent with divider rendering. |
| 2 | Tab title source | Shell CWD with fallback | Show shell process name initially (e.g. "zsh"). CWD tracking via OSC 7 will be added in Track 10. |
| 3 | Close confirmation | No confirmation | Always close immediately for fast workflow. |
| 4 | Tab overflow | Shrink tab widths | Tabs shrink proportionally as count increases, with minimum width (~60px). All tabs always visible. |
| 5 | Tab-window relationship | One tab bar per window | Standard terminal emulator pattern. Single window, multiple tabs. |

## Data Model

### TabId

Unique identifier for each tab, using the same atomic counter pattern as `PaneId`.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TabId(pub u64);
```

### Tab

```rust
pub struct Tab {
    pub id: TabId,
    pub title: String,       // Display title (shell name or CWD)
    pub pane_tree: PaneTree,  // Independent pane layout per tab
}
```

### TabManager

```rust
pub struct TabManager {
    tabs: Vec<Tab>,
    active_index: usize,
}
```

**Key methods:**
- `new() -> Self` — creates manager with one default tab
- `new_tab() -> TabId` — appends tab, returns its ID
- `close_tab(index: usize) -> Option<Vec<PaneId>>` — removes tab, returns pane IDs for cleanup. If last tab, returns None (prevents closing last tab).
- `active_tab() -> &Tab` / `active_tab_mut() -> &mut Tab`
- `select_tab(index: usize)` — switches active tab (clamps to valid range)
- `next_tab()` / `prev_tab()` — cycle through tabs
- `move_tab(from: usize, to: usize)` — reorder tabs
- `tab_count() -> usize`
- `tabs() -> &[Tab]` — for rendering tab bar
- `set_title(tab_index: usize, title: &str)` — update tab title

## Tab Bar Rendering

### Layout
- Tab bar height: **28px** (physical pixels), rendered at top of window
- Content area starts at y=28px — all pane layouts offset by tab bar height
- Each tab: background quad + title text (rendered via glyph atlas)
- Active tab: `theme.accent` background, `theme.text_primary` text
- Inactive tabs: `theme.pane_background` background, `theme.text_muted` text
- Tab separator: 1px vertical line using `theme.border`

### Tab Width Calculation
- Max tab width: 200px
- Min tab width: 60px
- Available width = window_width - 28px (new-tab button area)
- Tab width = clamp(available_width / tab_count, 60, 200)

### New-Tab Button
- "+" rendered at the right end of the tab bar
- Click creates new tab
- 28x28px hit area

### Tab Bar Interaction
- Click on tab → switch to that tab
- Click on "+" → new tab
- Middle-click on tab → close tab (future enhancement, not MVP)

## Keyboard Shortcuts

All use Ctrl+Shift prefix (consistent with existing pane commands):

| Shortcut | Action |
|----------|--------|
| Ctrl+Shift+T | New tab |
| Ctrl+Shift+W | Close tab (if single pane) / close pane (if multiple panes) |
| Ctrl+Shift+Tab | Next tab |
| Ctrl+Shift+1..9 | Select tab by number |
| Ctrl+Shift+PageUp | Previous tab |
| Ctrl+Shift+PageDown | Next tab |
| Ctrl+Shift+{ | Move tab left |
| Ctrl+Shift+} | Move tab right |

**Note on Ctrl+Shift+W:** Currently closes the focused pane. With tabs, behavior changes: if the active tab has multiple panes, close the focused pane. If the active tab has only one pane, close the tab entirely.

## Integration with App

### Window Layout Change
- `calculate_layout()` must account for tab bar height (28px)
- Pane rects are computed within bounds `Rect::new(0.0, TAB_BAR_HEIGHT, width, height - TAB_BAR_HEIGHT)`
- Overlay quads (dividers, unfocused dimming) also offset by tab bar height

### PaneState Migration
- `App.pane_states: HashMap<PaneId, PaneState>` remains flat across all tabs
- Each tab's PaneTree references PaneIds that index into this shared map
- When a tab is closed, its PaneIds are removed from `pane_states`

### Rendering Pipeline Change
1. Generate tab bar overlay quads (backgrounds + separators)
2. Render tab titles via glyph atlas (reuse existing text rendering)
3. Render active tab's panes (existing pipeline, offset by tab bar height)
4. Render pane overlays (dividers, unfocused dimming) for active tab only

### Event Routing
- Mouse events in tab bar area (y < 28px) → tab interaction
- Mouse events below tab bar → existing pane interaction (with y-offset)
- Keyboard events → check tab commands first, then pane commands, then PTY

## Files to Create/Modify

### New Files
- `src/tab/mod.rs` — TabId, Tab, TabManager with tests
- `src/tab/bar.rs` — Tab bar quad generation, hit testing, tab title text

### Modified Files
- `src/window.rs` — Replace `pane_tree` with `TabManager`, add tab bar rendering, update event routing
- `src/input/mod.rs` — Add `TabCommand` enum and `match_tab_command()`
- `src/lib.rs` — Add `pub mod tab;`
- `src/pane/interaction.rs` — Offset mouse coordinates by tab bar height
- `src/renderer/mod.rs` — May need text rendering helpers for tab titles

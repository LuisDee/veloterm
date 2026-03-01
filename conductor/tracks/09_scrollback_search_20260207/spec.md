<!-- ARCHITECT CONTEXT | Track: 09_scrollback_search | Wave: 4 | CC: v1 -->

## Cross-Cutting Constraints
- Performance Budget: search must not block the render loop
- Testing: TDD, regex matching correctness tests

## Interfaces

### Owns
- Search engine (regex match in scrollback buffer)
- Search UI overlay (input field + match count + navigation)
- Match highlighting in terminal view

### Consumes
- `Config` (Track 03) — search keybindings, highlight colors
- Terminal scrollback content (existing alacritty_terminal)

## Dependencies
- Track 03_config: keybindings and UI theme colors

<!-- END ARCHITECT CONTEXT -->

# Track 09: Scrollback Search — Specification

## Overview

Add a regex-capable search feature to VeloTerm that searches visible content and scrollback buffer, displays results via a custom GPU-rendered search bar overlay, and highlights visible matches in real-time as the user types. The search operates per-pane and integrates with the existing wgpu dual-pipeline rendering architecture.

## Functional Requirements

### FR-1: Search Engine
- Regex search using the `regex` crate over visible + scrollback content
- Search scope: all content in the active pane (visible rows + scrollback history)
- Access terminal content via `alacritty_terminal`'s grid (rows from `-history_size` to `screen_rows`)
- Return match results as a list of `(row, start_col, end_col)` positions
- Case-insensitive search by default (regex flag `(?i)`)
- Handle invalid regex gracefully — display error indicator, don't crash
- Count all matches in full buffer; only resolve positions for visible matches + small buffer

### FR-2: Incremental Search
- Search executes on every keystroke (search-as-you-type)
- Results update live as the user modifies the query
- Empty query clears all results and highlighting
- Performance: search must complete without blocking the render loop; run search on the main thread but limit work per frame if needed

### FR-3: Search UI Overlay
- Custom GPU-rendered search bar using the existing overlay pipeline (`overlay.wgsl` shader)
- Bar appears at top-right of the active pane (similar to browser find-in-page)
- UI elements:
  - Text input field showing current query
  - Match count indicator: "N of M" (current match index / total matches)
  - Up/Down navigation arrows (prev/next match)
  - Close button (X)
  - Error indicator when regex is invalid
- Bar renders above terminal content (on top of grid, no displacement of terminal rows)
- Text in the search bar rendered via glyph atlas (same font rendering path as terminal)

### FR-4: Match Highlighting
- Only highlight matches visible on screen (plus a small buffer of ~5 rows above/below viewport)
- All visible matches: distinct background color (theme `search_match` color)
- Current/active match: different background color (theme `search_match_active` color)
- Highlighting rendered via the grid pipeline by setting special background colors on matched cells
- Re-highlight on scroll (when viewport changes, recalculate which matches are visible)
- Highlighting clears immediately when search is closed

### FR-5: Match Navigation
- Next match (`Enter` or `Ctrl+Shift+N` / down arrow in search bar): advance to next match
- Previous match (`Shift+Enter` or `Ctrl+Shift+P` / up arrow in search bar): go to previous match
- Navigation wraps around (after last match → first match, before first → last)
- Scroll-to-match: terminal viewport scrolls to show the current match row
- Current match index updates in the "N of M" display

### FR-6: Modal Input State
- When search is active, keyboard input goes to the search bar, NOT the PTY
- New modal state: `InputMode::Search` added to input handling
- Escape closes the search overlay and returns to normal terminal input
- Mouse clicks outside the search bar close it
- The terminal remains visible and interactive via mouse while search is open

### FR-7: Keyboard Shortcuts
- Open search: `Ctrl+Shift+F` (configurable via Config keybindings)
- Close search: `Escape`
- Next match: `Enter` or down arrow (when search bar focused)
- Previous match: `Shift+Enter` or up arrow (when search bar focused)
- All shortcuts integrate with existing keybinding system in `Config`

## Non-Functional Requirements

### NFR-1: Performance
- Search must not block rendering (target: <16ms per frame maintained)
- Match counting on large scrollback (10,000+ lines) must complete within 100ms
- Only visible match positions resolved for highlighting (avoid O(n) highlight generation)
- Damage tracking: search highlight changes mark affected rows as dirty

### NFR-2: Testing
- Unit tests for regex search engine (match positions, edge cases, invalid regex)
- Unit tests for match navigation (wrapping, scroll-to-match)
- Unit tests for search state machine (open/close, mode transitions)
- Unit tests for overlay quad generation (search bar positioning, layout)
- Target: >80% coverage for new modules

### NFR-3: Consistency
- Search bar visual style consistent with existing overlay UI (tab bar, dividers)
- Colors from theme configuration (extend theme with search-specific colors)
- Font rendering uses existing glyph atlas

## Acceptance Criteria

1. `Ctrl+Shift+F` opens a search bar overlay at top-right of active pane
2. Typing a query shows matches highlighted in the terminal in real-time
3. Match count displays "N of M" and updates as query changes
4. Enter/Shift+Enter navigate between matches, scrolling viewport as needed
5. Navigation wraps around from last→first and first→last
6. Escape closes search, clears all highlighting, returns input to terminal
7. Invalid regex shows error indicator without crashing
8. Search works across visible content and full scrollback buffer
9. Only visible matches are highlighted (performance optimization)
10. All new code has >80% test coverage

## Out of Scope

- Search across multiple panes or tabs (search is per-pane)
- Search-and-replace functionality
- Saved search patterns or search history
- Case sensitivity toggle in UI (regex `(?i)` flag handles this)
- Search result persistence across pane changes

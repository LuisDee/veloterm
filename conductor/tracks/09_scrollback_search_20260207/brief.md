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

# Track 09: Scrollback Search

## What This Track Delivers

A search feature that finds regex matches in the terminal's scrollback buffer and highlights them in the terminal view. A search overlay UI (input field + match count + prev/next navigation) appears when activated via keyboard shortcut. Matches are highlighted in the terminal with a distinct background color, and the view scrolls to show the current match.

## Scope

### IN
- Regex search engine using `regex` crate on scrollback content
- Search UI overlay: text input, match count display, next/prev match navigation
- Match highlighting: distinct background color on all matches, current match highlighted differently
- Incremental search: results update as user types
- Search wrapping: next/prev cycles through matches
- Scroll-to-match: terminal view scrolls to show current match
- Keyboard shortcuts: open search, next match, prev match, close search

### OUT
- Search across multiple panes or tabs (search is per-pane)
- Search-and-replace
- Saved search patterns or search history

## Key Design Decisions

1. **Search UI rendering**: egui overlay widget vs custom GPU-rendered search bar vs minimal text-only overlay?
   Trade-off: egui handles text input natively; custom matches terminal aesthetic; text-only is simplest

2. **Search scope**: Visible content + scrollback vs scrollback only vs configurable?
   Trade-off: visible + scrollback is most useful; scrollback-only matches less grep convention

3. **Incremental vs committed search**: Search as you type vs search on Enter?
   Trade-off: incremental is more responsive but may be slow on large scrollback; committed is simpler

4. **Match limit**: Highlight all matches vs limit to N matches with "N+ more" indicator?
   Trade-off: all matches can be expensive to render; limit keeps performance predictable

## Architectural Notes

- The `regex` crate is already planned in tech-stack.md Phase 2+ dependencies
- alacritty_terminal has a scrollback buffer accessible via `term.grid()` — search operates on this content
- The search overlay needs to capture keyboard input before it reaches the PTY — add a modal input state
- If egui is chosen here, this decision should be consistent with Track 05 (pane UI) and Track 06 (tabs)

## Complexity: M
## Estimated Phases: ~3

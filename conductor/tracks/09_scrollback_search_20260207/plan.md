# Track 09: Scrollback Search — Implementation Plan

## Phase 1: Search Engine Core [checkpoint: e608dc2]

### 1.1 Write tests for SearchEngine types and regex matching
- [x] 1.1.1 Create `src/search/mod.rs` module with `SearchMatch` type (row, start_col, end_col) and `SearchEngine` struct
- [x] 1.1.2 Write tests: basic literal search returns correct match positions
- [x] 1.1.3 Write tests: regex search (e.g., `\d+`, `https?://`) returns correct positions
- [x] 1.1.4 Write tests: case-insensitive search by default
- [x] 1.1.5 Write tests: invalid regex returns error (not panic)
- [x] 1.1.6 Write tests: empty query returns no matches
- [x] 1.1.7 Write tests: multi-line content with matches across rows
- [x] 1.1.8 Write tests: match count across full buffer

### 1.2 Implement SearchEngine
- [x] 1.2.1 Implement `SearchEngine::new()` and `SearchEngine::search(query, lines) -> SearchResult`
- [x] 1.2.2 Use `regex` crate with `(?i)` flag for case-insensitive matching
- [x] 1.2.3 Return `SearchResult { matches: Vec<SearchMatch>, total_count: usize, error: Option<String> }`
- [x] 1.2.4 Handle invalid regex gracefully — return error string, empty matches <!-- adbb08e -->

### 1.3 Write tests for SearchState and match navigation
- [x] 1.3.1 Write tests: SearchState tracks query string, current match index, total count
- [x] 1.3.2 Write tests: `next_match()` advances index, wraps from last→0
- [x] 1.3.3 Write tests: `prev_match()` decrements index, wraps from 0→last
- [x] 1.3.4 Write tests: `set_query()` resets current index to 0
- [x] 1.3.5 Write tests: `current_match()` returns the active SearchMatch
- [x] 1.3.6 Write tests: visible match filtering (given viewport range, return only visible matches)

### 1.4 Implement SearchState
- [x] 1.4.1 Implement `SearchState` struct with query, matches, current_index, is_active fields
- [x] 1.4.2 Implement navigation methods (next_match, prev_match, current_match)
- [x] 1.4.3 Implement `visible_matches(viewport_start, viewport_end, buffer)` — returns matches in visible range ± 5 rows
- [x] 1.4.4 Implement `scroll_target()` — returns row of current match for scroll-to-match <!-- ed3bd5a -->

### Phase 1 Completion — Verification and Checkpointing

---

## Phase 2: Search UI Overlay & Input Mode [checkpoint: 00164ec]

### 2.1 Write tests for search overlay quad generation
- [x] 2.1.1 Write tests: search bar generates overlay quads at top-right of pane rect <!-- 834cdfe -->
- [x] 2.1.2 Write tests: search bar dimensions (width, height based on cell size) <!-- 834cdfe -->
- [x] 2.1.3 Write tests: search bar contains background quad + text area <!-- 834cdfe -->
- [x] 2.1.4 Write tests: match count text positioning within bar <!-- 834cdfe -->

### 2.2 Implement search overlay rendering
- [x] 2.2.1 Create `src/search/overlay.rs` — `generate_search_bar_quads(pane_rect, cell_size, search_state, theme)` returns `Vec<OverlayQuad>` <!-- 834cdfe -->
- [x] 2.2.2 Generate background quad (theme search bar color, rounded feel via sizing) <!-- 834cdfe -->
- [x] 2.2.3 Generate text cell instances for query text, match count ("N of M"), and navigation indicators <!-- 834cdfe -->
- [x] 2.2.4 Integrate into `Renderer::update_overlays()` — append search bar quads when search is active <!-- 834cdfe -->

### 2.3 Write tests for modal input handling
- [x] 2.3.1 Write tests: `InputMode::Search` captures printable characters into query <!-- 834cdfe -->
- [x] 2.3.2 Write tests: Backspace removes last character from query <!-- 834cdfe -->
- [x] 2.3.3 Write tests: Escape exits search mode, returns `InputMode::Normal` <!-- 834cdfe -->
- [x] 2.3.4 Write tests: Enter triggers next_match <!-- 834cdfe -->
- [x] 2.3.5 Write tests: Shift+Enter triggers prev_match <!-- 834cdfe -->
- [x] 2.3.6 Write tests: Ctrl+Shift+F toggles search mode on/off <!-- 834cdfe -->
- [x] 2.3.7 Write tests: arrow up/down in search mode trigger prev/next match <!-- 834cdfe -->

### 2.4 Implement modal input handling
- [x] 2.4.1 Add `InputMode` enum (`Normal`, `Search`) to input module <!-- 834cdfe -->
- [x] 2.4.2 Add `match_search_command()` — handle key events when in search mode <!-- 834cdfe -->
- [x] 2.4.3 Integrate into `window.rs` input dispatch — check InputMode before routing to PTY <!-- 834cdfe -->
- [x] 2.4.4 Ctrl+Shift+F opens search (sets InputMode::Search, activates SearchState) <!-- 834cdfe -->
- [x] 2.4.5 Escape closes search (sets InputMode::Normal, deactivates SearchState, clears highlights) <!-- 834cdfe -->

### Phase 2 Completion — Verification and Checkpointing

---

## Phase 3: Match Highlighting & App Integration [checkpoint: 9a0611f]

### 3.1 Write tests for match highlighting in grid
- [x] 3.1.1 Write tests: `apply_search_highlights()` modifies GridCell background for matched cells <!-- 644aa9e -->
- [x] 3.1.2 Write tests: current match uses `search_match_active` color, others use `search_match` color <!-- 644aa9e -->
- [x] 3.1.3 Write tests: only cells in visible viewport (± buffer) get highlight applied <!-- 644aa9e -->
- [x] 3.1.4 Write tests: clearing search removes all highlight overrides <!-- 644aa9e -->

### 3.2 Implement match highlighting
- [x] 3.2.1 Extend theme config with `search_match` and `search_match_active` colors (with defaults) <!-- 644aa9e -->
- [x] 3.2.2 Implement `apply_search_highlights(cells, visible_matches, current_index, theme)` — override bg color on matched cells <!-- 644aa9e -->
- [x] 3.2.3 Integrate into `grid_bridge` or renderer cell generation — apply highlights after extracting grid cells <!-- 644aa9e -->
- [x] 3.2.4 Mark affected rows as dirty in damage tracking when search state changes <!-- 644aa9e -->

### 3.3 Write tests for scroll-to-match
- [x] 3.3.1 Write tests: navigating to a match outside viewport triggers scroll <!-- 644aa9e -->
- [x] 3.3.2 Write tests: navigating to a match inside viewport does NOT scroll <!-- 644aa9e -->
- [x] 3.3.3 Write tests: wrapping navigation scrolls correctly <!-- 644aa9e -->

### 3.4 Implement scroll-to-match and full integration
- [x] 3.4.1 Implement scroll-to-match in `window.rs` — on match navigation, adjust terminal `display_offset` to show current match row <!-- 644aa9e -->
- [x] 3.4.2 Wire incremental search: on each keystroke in search mode, re-run search engine, update SearchState, request redraw <!-- 644aa9e -->
- [x] 3.4.3 Wire overlay rendering: when search active, generate search bar quads in `generate_overlay_quads()` <!-- 644aa9e -->
- [x] 3.4.4 Wire highlight clearing: on search close, clear highlights, snap to bottom (optional), mark full damage <!-- 644aa9e -->
- [x] 3.4.5 Add `Ctrl+Shift+F` to default keybindings in Config <!-- 644aa9e -->

### Phase 3 Completion — Verification and Checkpointing

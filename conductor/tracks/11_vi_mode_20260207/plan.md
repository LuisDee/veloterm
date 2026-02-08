# Track 11: Vi-Mode Selection — Implementation Plan

## Phase 1: Vi-Mode State Machine & Basic Motions [checkpoint: ea3bbb5]

### 1.1 [x] Create vi-mode module with state machine and mode types <!-- 0db2024 -->
- Create `src/vi_mode/mod.rs` with `ViMode` enum (Normal, Visual, VisualLine, VisualBlock), `ViState` struct (mode, cursor position, anchor, count prefix)
- Define `ViAction` enum for all actions (motions, mode transitions, yank, search)
- Define `ViCommand` for parsed input → action mapping
- **Tests:** State machine transitions (Normal→Visual→Normal, Normal→VisualLine→Normal, Normal→VisualBlock→Normal, Escape behavior)

### 1.2 [x] Implement count prefix parsing and basic motion commands <!-- 67e9e2c -->
- Implement count accumulation (digits 1-9, with 0 as LineStart unless mid-count)
- Implement character motions: h, l, j, k with count support
- Implement line motions: 0, $, ^
- Clamp cursor to valid bounds (row: 0..total_lines, col: 0..line_length)
- **Tests:** Count parsing, each motion with and without count, boundary clamping

### 1.3 [x] Implement word and buffer motions <!-- 67e9e2c -->
- Implement word motions: w, b, e using existing `find_word_boundaries` logic
- Implement buffer motions: gg, G
- Implement viewport motions: H, M, L
- Implement scroll motions: Ctrl+U, Ctrl+D
- **Tests:** Word motion across word boundaries, buffer top/bottom, viewport-relative motions, half-page scroll

### 1.4 [x] Add vi-mode config and entry keybinding <!-- 952e91b -->
- Extend `Config` with `vi_mode` section: `enabled` (bool), `entry_key` (String)
- Extend `InputMode` enum with `Vi` variant
- Add `should_toggle_vi_mode()` to input matching
- Wire vi-mode toggle in the focused pane's input path
- **Tests:** Config parsing with vi_mode section, default values, keybinding match

### 1.5 [x] Phase 1 Completion — Verification and Checkpointing <!-- ea3bbb5 -->

---

## Phase 2: Visual Selection & Yank [checkpoint: 7a8f78c]

### 2.1 [x] Implement Visual (character-wise) selection <!-- a524f1a -->
- On entering Visual mode, record anchor = current cursor position
- Selection spans from anchor to cursor (inclusive), normalized in reading order
- Extend `SelectionType` with `VisualBlock` variant
- Convert vi selection to `Selection` struct for rendering via `apply_selection_flags`
- **Tests:** Selection range for various anchor/cursor combos, selection across lines, normalize order

### 2.2 [x] Implement Visual-Line selection <!-- a524f1a -->
- On entering Visual-Line mode, record anchor row
- Selection covers full rows from min(anchor_row, cursor_row) to max(anchor_row, cursor_row)
- All columns in selected rows are marked
- **Tests:** Single-line selection, multi-line up/down, motion within visual-line mode

### 2.3 [x] Implement Visual-Block (rectangular) selection <!-- a524f1a -->
- On entering Visual-Block mode, record anchor (row, col)
- Selection is a rectangle: rows min..max, cols min..max of anchor and cursor
- Implement `selected_text_block()` for rectangular text extraction
- **Tests:** Rectangle selection, column alignment, block text extraction

### 2.4 [x] Implement yank to clipboard <!-- ff3876e -->
- `y` in any visual mode extracts selected text and copies to system clipboard via `arboard`
- Visual: contiguous text, Visual-Line: full lines with newlines, Visual-Block: per-row with newlines
- After yank, transition to Normal mode and clear selection
- **Tests:** Yank text extraction for each mode, mode transition after yank

### 2.5 [x] Phase 2 Completion — Verification and Checkpointing <!-- 7a8f78c -->

---

## Phase 3: Search, Cursor Rendering & Status Bar

### 3.1 [ ] Implement vi-mode search (/ and ?)
- `/` and `?` enter a search sub-mode with text input in status bar area
- Reuse `SearchEngine` from `src/search/mod.rs` for query execution
- `Enter` confirms and moves vi cursor to first match, `Escape` cancels
- `n` jumps to next match, `N` to previous (direction-aware: / = forward, ? = backward)
- **Tests:** Search forward/backward, n/N navigation, wrapping, cancel behavior

### 3.2 [ ] Implement vi cursor rendering and CELL_FLAG_VI_CURSOR
- Add `CELL_FLAG_VI_CURSOR` constant to grid_renderer
- In vi-mode, mark the cell at vi cursor position with this flag
- Renderer draws vi cursor as a distinct block overlay (e.g., inverted colors)
- Vi cursor is independent of terminal cursor
- **Tests:** Flag applied at correct position, flag cleared on mode exit, no interference with selection flags

### 3.3 [ ] Implement mode indicator in status bar
- Render mode text at the bottom of the pane: "-- NORMAL --", "-- VISUAL --", "-- VISUAL LINE --", "-- VISUAL BLOCK --"
- During search, show "/ <query>" or "? <query>"
- Status bar only visible when vi-mode is active
- **Tests:** Correct text for each mode, search prompt display, hidden when not in vi-mode

### 3.4 [ ] Integration: wire vi-mode into main event loop
- In the main event loop, when a pane is in vi-mode, route keyboard input to the vi handler
- Ensure PTY receives no input while vi-mode is active
- Ensure per-pane independence (multiple panes, only focused pane affected)
- Viewport scrolling follows vi cursor
- **Tests:** Input routing, PTY isolation, multi-pane independence

### 3.5 [ ] Phase 3 Completion — Verification and Checkpointing

# Track 17: Text Selection & Clipboard — Implementation Plan

## Phase 1: Mouse Selection & Click Detection

### Task 1.1: Click state machine and single-click selection start [x] <!-- 3c816f8 -->
- Add `MouseSelectionState` to window.rs (or new `src/input/mouse.rs`):
  - `click_count: u8` (1=single, 2=double, 3=triple)
  - `last_click_time: Instant`
  - `last_click_pos: (f32, f32)`
  - `active_selection: Option<Selection>`
  - `is_dragging: bool`
  - `drag_origin: (usize, usize)` (cell coordinates)
- Store `MouseSelectionState` in `PaneState` (per-pane selection)
- On left mouse press in content area:
  - Calculate cell position via `pixel_to_cell()` with padding offset
  - Increment click count if within 300ms and ~5px of last click, else reset to 1
  - For click_count=1: set `drag_origin`, clear previous selection, set `is_dragging = true`
- On mouse move while dragging: update `active_selection.end` to current cell
- On mouse release: set `is_dragging = false`
- Selection cleared on any left click that starts a new selection
- Write tests: click count detection (single, double, triple, timeout reset), pixel_to_cell with padding offset, selection start/update/finalize

### Task 1.2: Double-click word and triple-click line selection [x] <!-- 3c816f8 -->
- On click_count=2 (double-click):
  - Call `find_word_boundaries()` to get (start_col, end_col)
  - Set selection with `SelectionType::Word`, start=(row, start_col), end=(row, end_col)
  - Store word-boundary origin for drag-extends-by-word
- On click_count=3 (triple-click):
  - Set selection with `SelectionType::Line`, start=(row, 0), end=(row, cols-1)
  - Store line origin for drag-extends-by-line
- During drag after double-click: extend selection to word boundaries of current cell
- During drag after triple-click: extend selection to full lines
- Write tests: double-click word selection, triple-click line selection, drag-extend-by-word, drag-extend-by-line

### Task 1.3: Shift+click selection extend [x] <!-- 3c816f8 -->
- On Shift+left-click:
  - If active selection exists: extend `selection.end` to clicked cell
  - If no selection: create new Range selection from current cursor position to click
- Preserve existing selection type for extend behavior
- Write tests: extend existing selection, create from cursor, shift+click after word selection

### Phase 1 Checkpoint

## Phase 2: Selection Rendering & Clipboard

### Task 2.1: Selection rendering in shader [x] <!-- 1c27749 -->
- In grid.wgsl fragment shader: detect `CELL_FLAG_SELECTED` (bit 6)
  - Add `is_selected` to VertexOutput (extract bit 6 from flags)
  - When selected: swap fg and bg colors (invert selection rendering)
- In render pipeline (window.rs): before building PaneRenderDescriptor:
  - Call `apply_selection_flags()` on grid cells using pane's active selection
  - Clear CELL_FLAG_SELECTED from all cells first, then re-apply current selection
- Verify selection highlight renders correctly via screenshot
- Write tests: CELL_FLAG_SELECTED bit extraction in shader data, apply/clear flags cycle

### Task 2.2: Cmd+C copy and Cmd+V paste [x] <!-- 8727b97 -->
- In keyboard event handler (window.rs):
  - Check `is_copy_keybinding()` before PTY write
  - If copy: extract text via `selected_text()` (or `selected_text_block`/`selected_text_lines` based on type)
  - Write to `arboard::Clipboard`, clear selection
  - If no selection on copy: do nothing (don't send Ctrl+C to terminal)
- Check `is_paste_keybinding()`:
  - Read from `arboard::Clipboard`
  - Call `paste_bytes(text, bracketed_paste_enabled)` — check terminal's bracketed paste mode flag
  - Write bytes to active pane's PTY
- Write tests: copy extracts correct text by selection type, paste wraps with bracketed mode, empty selection copy is no-op

### Task 2.3: Cmd+A select all [x] <!-- 53de739 -->
- Add `is_select_all_keybinding()` to clipboard.rs:
  - macOS: Cmd+A (Super+A), Linux: Ctrl+Shift+A
- In keyboard handler: intercept before PTY write
  - Create Range selection spanning (0, 0) to (rows-1, cols-1) of visible terminal
  - Store as active selection on focused pane
  - Request redraw
- Write tests: keybinding detection, selection spans full grid

### Phase 2 Checkpoint

## Phase 3: Context Menu & Integration

### Task 3.1: Native OS context menu (macOS)
- Create `src/context_menu.rs` module with platform-specific context menu
- macOS implementation using objc2:
  - Build NSMenu with items: Copy, Paste, Select All, NSMenuItem.separatorItem, Split Vertical, Split Horizontal, Close Pane
  - Copy item enabled only when selection exists
  - Show menu at mouse position via `popUpMenuPositioningItem:atLocation:inView:`
  - Return selected `ContextMenuAction` enum variant
- `ContextMenuAction` enum: Copy, Paste, SelectAll, SplitVertical, SplitHorizontal, ClosePane
- Wire MouseButton::Right event in window.rs:
  - Build menu with current state (has_selection)
  - Show menu, handle returned action
  - Dispatch to existing handlers (clipboard ops, pane commands)
- Write tests: ContextMenuAction enum, action dispatch routing (unit test the dispatch, not the OS menu)

### Task 3.2: Final integration and visual validation
- Run full test suite — all existing + new tests pass
- Build and run application
- Verify: click-drag selection highlights cells
- Verify: double-click selects word, triple-click selects line
- Verify: Cmd+C copies selection to clipboard
- Verify: Cmd+V pastes into terminal
- Verify: Cmd+A selects all visible text
- Verify: right-click shows native context menu with correct items
- Screenshot validation with Playwright MCP

### Phase 3 Checkpoint

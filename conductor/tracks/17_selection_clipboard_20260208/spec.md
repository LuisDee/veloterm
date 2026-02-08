# Track 17: Text Selection & Clipboard — Specification

## Overview

Full production-grade text selection with click-and-drag highlighting, double-click word selection, triple-click line selection, clipboard integration (Cmd+C/V/A), and native OS right-click context menu. Wires existing selection infrastructure (`Selection`, `SelectionType`, `pixel_to_cell`, `find_word_boundaries`, `selected_text`, `apply_selection_flags`) to the window event handler and GPU rendering pipeline.

## Design Decisions

1. **Click count detection**: Time-based (300ms threshold). Clicks within 300ms of the previous click at approximately the same position increment the click count (1→single, 2→double, 3→triple). Click count resets after 300ms idle or significant mouse movement.

2. **Word boundary definition**: Match iTerm2/Terminal.app behavior. Words are delimited by whitespace and common programming punctuation. Alphanumeric + underscore = word characters (existing `is_word_char` in selection.rs). Path separators (`/`, `.`, `-`) treated as delimiters (not word characters), matching standard terminal behavior.

3. **Context menu**: Native OS context menu (NSMenu on macOS) with terminal-specific actions: Copy, Paste, Select All, separator, Split Vertical, Split Horizontal, Close Pane. Native menu provides platform-correct look/feel and accessibility.

4. **Selection scope**: Selection confined to active pane. Mouse events outside the focused pane do not extend selection.

## Functional Requirements

### FR-1: Click-and-drag character selection
- Left mouse press starts selection at cell under cursor
- Mouse drag updates selection end position in real-time
- Mouse release finalizes selection
- Selection type: `SelectionType::Range`
- Mouse coordinates account for terminal padding and tab bar offset
- Selection state stored in `PaneState` (per-pane)

### FR-2: Double-click word selection
- Two clicks within 300ms at same position → select word under cursor
- Word boundaries from existing `find_word_boundaries()`
- Selection type: `SelectionType::Word`
- Subsequent drag extends selection by whole words

### FR-3: Triple-click line selection
- Three clicks within 300ms at same position → select entire line
- Selection spans col 0 to last non-space column
- Selection type: `SelectionType::Line`
- Subsequent drag extends selection by whole lines

### FR-4: Shift+click selection extend
- Shift+left-click extends existing selection to clicked position
- If no selection exists, creates selection from cursor position to click
- Preserves original selection type semantics

### FR-5: Selection rendering in shader
- Shader reads `CELL_FLAG_SELECTED` (bit 6, value 0x40) from cell flags
- Selected cells render with inverted fg/bg colors (swap foreground and background)
- `apply_selection_flags()` called during cell preparation in render pipeline
- Selection flags cleared before each re-application

### FR-6: Cmd+C — Copy selected text
- Existing `is_copy_keybinding()` detects Cmd+C / Ctrl+Shift+C
- Extract text via `selected_text()` / `selected_text_block()` / `selected_text_lines()` based on SelectionType
- Write to system clipboard via `arboard::Clipboard`
- Clear selection after copy

### FR-7: Cmd+V — Paste from clipboard
- Existing `is_paste_keybinding()` detects Cmd+V / Ctrl+Shift+V
- Read from system clipboard via `arboard::Clipboard`
- Wrap with bracketed paste sequences via `paste_bytes()`
- Write to active pane's PTY

### FR-8: Cmd+A — Select all
- Detect `Cmd+A` / `Ctrl+Shift+A` keybinding
- Select all visible terminal content (row 0, col 0 to last row, last col)
- Uses `SelectionType::Range`

### FR-9: Right-click context menu
- Right-click (MouseButton::Right) opens native OS context menu
- Menu items: Copy (if selection), Paste, Select All, ---separator---, Split Vertical, Split Horizontal, Close Pane
- Copy: disabled/hidden when no selection
- Menu actions dispatch to existing handlers (clipboard, pane commands)
- macOS: NSMenu via objc2 runtime calls

## Existing Infrastructure (read-only reference)

| Component | File | Status |
|-----------|------|--------|
| `SelectionType` enum | `src/input/selection.rs` | Complete |
| `Selection` struct | `src/input/selection.rs` | Complete |
| `pixel_to_cell()` | `src/input/selection.rs` | Complete |
| `find_word_boundaries()` | `src/input/selection.rs` | Complete |
| `selected_text()` | `src/input/selection.rs` | Complete |
| `apply_selection_flags()` | `src/input/selection.rs` | Complete |
| `CELL_FLAG_SELECTED` (0x40) | `src/renderer/grid_renderer.rs` | Defined, not used in shader |
| `is_copy_keybinding()` | `src/input/clipboard.rs` | Complete |
| `is_paste_keybinding()` | `src/input/clipboard.rs` | Complete |
| `paste_bytes()` | `src/input/clipboard.rs` | Complete |
| `wrap_bracketed_paste()` | `src/input/clipboard.rs` | Complete |

## Non-Goals

- Rectangular/block selection (already in vi-mode, Track 11)
- Rich text / HTML copy (plain text only)
- Clipboard history
- Middle-click paste (X11 primary selection)
- Context menu items beyond the listed ones (Track 20 scope)

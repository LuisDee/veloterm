<!-- ARCHITECT CONTEXT | Track: 11_vi_mode | Wave: 4 | CC: v1 -->

## Cross-Cutting Constraints
- Testing: TDD, motion command tests, selection boundary tests
- Platform Abstraction: keybindings identical across platforms (vi is universal)

## Interfaces

### Owns
- Vi-mode state machine (normal, visual, visual-line, visual-block)
- Motion commands (h/j/k/l, w/b/e, 0/$, gg/G, etc.)
- Selection via visual mode

### Consumes
- `Config` (Track 03) — vi-mode enable/disable, entry keybinding
- Existing selection module (`src/input/selection.rs`)
- Terminal scrollback (existing)

## Dependencies
- Track 03_config: vi-mode toggle and keybinding

<!-- END ARCHITECT CONTEXT -->

# Track 11: Vi-Mode Selection — Specification

## Overview

Vi-mode adds a modal keyboard-driven navigation and selection system to VeloTerm's terminal panes. When activated via a configurable keybinding (default: `Ctrl+Shift+Space`), the focused pane enters vi-mode where keyboard input is intercepted by the vi-mode handler instead of being sent to the PTY. The user navigates the scrollback buffer with vi motions and selects text using visual, visual-line, and visual-block modes.

Vi-mode state is **per-pane** — each pane maintains independent vi-mode state. The current mode is displayed via **status bar text** following standard vi conventions (e.g., "-- NORMAL --", "-- VISUAL --").

## Functional Requirements

### FR-1: Vi-Mode Entry and Exit
- A new `InputMode::Vi(ViMode)` variant is added to the existing `InputMode` enum
- Entry: Configurable keybinding (default `Ctrl+Shift+Space`) toggles vi-mode on the focused pane
- Exit: `Escape` from normal mode exits vi-mode entirely, returning to `InputMode::Normal`
- `Escape` from visual/visual-line/visual-block returns to vi normal mode (not exit)
- On entry, the vi cursor is placed at the terminal cursor's current position
- While in vi-mode, all keyboard input is handled by the vi-mode handler; no bytes are sent to the PTY

### FR-2: Vi-Mode State Machine
Four modes with defined transitions:
- **Normal**: Default vi-mode state. Motions move the cursor. `v` enters Visual, `V` enters Visual-Line, `Ctrl+V` enters Visual-Block. `Escape` exits vi-mode.
- **Visual**: Character-wise selection. Anchor at mode-entry position, cursor moves to extend selection. `Escape` returns to Normal. `v` also returns to Normal.
- **Visual-Line**: Entire lines selected from anchor row to cursor row. `Escape` returns to Normal. `V` also returns to Normal.
- **Visual-Block**: Rectangular block from anchor (row, col) to cursor (row, col). `Escape` returns to Normal. `Ctrl+V` also returns to Normal.

### FR-3: Cursor and Motion Commands
All motions accept an optional count prefix (e.g., `5j` moves 5 lines down):

| Key | Motion | Description |
|-----|--------|-------------|
| `h` | CharLeft | Move cursor one column left |
| `l` | CharRight | Move cursor one column right |
| `j` | LineDown | Move cursor one line down |
| `k` | LineUp | Move cursor one line up |
| `w` | WordForward | Move to start of next word |
| `b` | WordBackward | Move to start of previous word |
| `e` | WordEnd | Move to end of current/next word |
| `0` | LineStart | Move to first column |
| `$` | LineEnd | Move to last non-blank column |
| `^` | FirstNonBlank | Move to first non-blank character |
| `gg` | BufferTop | Move to top of scrollback |
| `G` | BufferBottom | Move to bottom of scrollback |
| `H` | ViewportTop | Move to top of visible viewport |
| `M` | ViewportMiddle | Move to middle of visible viewport |
| `L` | ViewportBottom | Move to bottom of visible viewport |
| `Ctrl+U` | HalfPageUp | Scroll half page up |
| `Ctrl+D` | HalfPageDown | Scroll half page down |

Word boundaries use the same logic as the existing `find_word_boundaries` in `src/input/selection.rs`.

### FR-4: Visual Selection
- In Visual mode, a selection spans from the anchor position to the current cursor position (character-wise)
- In Visual-Line mode, entire rows are selected from anchor row to cursor row
- In Visual-Block mode, a true rectangle is selected: min(anchor_col, cursor_col)..max(anchor_col, cursor_col) across min(anchor_row, cursor_row)..max(anchor_row, cursor_row)
- Selected cells are marked with `CELL_FLAG_SELECTED` for rendering, reusing the existing `apply_selection_flags` mechanism
- The existing `SelectionType` enum is extended with `VisualBlock` variant for rectangular selection

### FR-5: Yank (Copy to Clipboard)
- `y` in any visual mode: copies the selected text to the system clipboard and exits to Normal mode
- In Visual mode: selected text as contiguous string
- In Visual-Line mode: full lines including newlines
- In Visual-Block mode: rectangular block with each row separated by newline

### FR-6: Search in Vi-Mode
- `/` opens a forward search prompt (displayed in the status bar area)
- `?` opens a backward search prompt
- Search uses the existing `SearchEngine` from `src/search/mod.rs` (Track 09)
- `n` jumps to next match, `N` jumps to previous match
- `Enter` confirms search and moves cursor to match position
- `Escape` cancels search and returns to Normal mode
- The vi cursor moves to match positions; the viewport scrolls to keep the cursor visible

### FR-7: Vi Cursor Rendering
- In vi Normal mode: block cursor at the vi-mode position (distinct from the terminal cursor)
- In visual modes: block cursor at current position, with selection highlighted
- The vi cursor is rendered as an overlay, independent of the terminal's cursor position
- A new `CELL_FLAG_VI_CURSOR` flag marks the cell at the vi cursor position

### FR-8: Mode Indicator
- Status bar text displays the current vi-mode state:
  - Normal: `-- NORMAL --`
  - Visual: `-- VISUAL --`
  - Visual-Line: `-- VISUAL LINE --`
  - Visual-Block: `-- VISUAL BLOCK --`
  - Search: `/ <query>` or `? <query>`
- The status bar indicator is rendered at the bottom of the pane

### FR-9: Configuration
- Add `vi_mode` section to `Config`:
  - `enabled: bool` (default: `true`) — whether vi-mode is available
  - `entry_key: String` (default: `"ctrl+shift+space"`) — keybinding to toggle vi-mode
- Vi-mode respects the `keys.bindings` system for customization

### FR-10: Count Prefix
- Numeric keys `1`-`9` begin a count prefix; `0` is `LineStart` unless preceded by other digits
- Count accumulates (e.g., pressing `1` then `2` gives count 12)
- Count applies to the next motion command
- Maximum count capped at 9999

## Non-Functional Requirements

- **NFR-1**: All vi motions must execute in under 1ms for buffers up to 100,000 lines
- **NFR-2**: Vi-mode adds no overhead when not active (zero-cost when in `InputMode::Normal`)
- **NFR-3**: All vi keybindings work identically on macOS and Linux
- **NFR-4**: Test coverage >80% for the vi-mode module

## Acceptance Criteria

1. User can enter vi-mode with `Ctrl+Shift+Space` and exit with `Escape`
2. All motion commands (h/j/k/l/w/b/e/0/$etc.) navigate correctly with optional count
3. Visual, Visual-Line, and Visual-Block selections highlight correctly
4. `y` copies the selected text to clipboard in all visual modes
5. `/` and `?` search works using Track 09 search engine, `n`/`N` navigate matches
6. Status bar shows current mode text
7. Vi cursor renders as a block at the correct position
8. Vi-mode is per-pane — one pane can be in vi-mode while others accept normal input
9. Config allows enabling/disabling vi-mode and customizing the entry keybinding

## Out of Scope

- Command mode (`:commands`) — this is a terminal, not a text editor
- Text modification commands (`d`, `c`, `x`) — terminal content is read-only
- Vi-mode within the shell input line (the shell handles its own vi-mode)
- Custom motion definitions or plugins
- Marks (`m` / `'`) — may be added in a future track
- Macros and registers — may be added in a future track

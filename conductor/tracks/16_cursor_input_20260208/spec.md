<!-- ARCHITECT CONTEXT | Track: 16_cursor_input | Wave: 6 | CC: v2 -->

## Cross-Cutting Constraints
- Testing: TDD for cursor position tracking, blink timing, key-to-PTY byte mapping
- Performance Budget: cursor rendering must not add measurable latency; blink timer must not wake GPU unnecessarily

## Interfaces

### Owns
- Cursor position accuracy verification
- Cursor blink rate configuration
- Key input verification (backspace, delete, arrows, history)

### Consumes
- `CursorRenderer` (Track 01) — existing cursor rendering infrastructure
- `Config` (Track 03) — cursor.style, cursor.blink, cursor.blink_rate
- `Terminal` (Track 02) — alacritty_terminal cursor state

## Dependencies
- Track 01_window_gpu_pipeline: CursorRenderer
- Track 02_core_terminal_emulation: Terminal state and PTY key mapping

<!-- END ARCHITECT CONTEXT -->

# Track 16: Cursor & Input Polish — Specification

## Overview

This track wires the existing CursorRenderer (which has 40 tests but is never called in the render loop) into the actual frame rendering pipeline, adds configurable blink rate, implements blink-pause-on-keystroke, and verifies keyboard input correctness across all fundamental key types.

The CursorState, CursorStyle, blink toggle logic, focus-aware hollow cursor, and cell instance generation all exist in `src/renderer/cursor.rs` — they just need to be connected to the window event loop and render pipeline.

## Functional Requirements

### FR-1: Wire Cursor Rendering into Frame Loop

- Extract cursor position from `alacritty_terminal`'s `Term::cursor()` after each PTY drain
- Update `CursorState` with current position each frame
- Call `CursorState::tick_blink()` during `RedrawRequested` to advance blink timer
- Generate cursor `CellInstance` via `CursorState::to_cell_instance()` and inject as overlay into the render pipeline
- Cursor renders on top of the cell at the cursor position with inverted colors (existing logic)
- Cursor style (block/beam/underline) determined by config `cursor.style`

### FR-2: Configurable Blink Rate

- Add `cursor.blink_rate` field to `CursorConfig` (u64 milliseconds, default 500)
- Validation: minimum 100ms, maximum 2000ms; 0 = disable blinking
- Pass blink rate to `CursorState` on creation and on config hot-reload
- Update the existing `BLINK_INTERVAL` constant to use the configured value instead
- When `cursor.blink` is false OR `blink_rate` is 0, cursor is always visible (no timer)
- Hot-reloadable via existing config watcher

### FR-3: Blink Pause on Keystroke

- On any keystroke that produces PTY output, reset the blink timer and force cursor visible
- Resume blinking after idle (no keystrokes for one full blink interval)
- Implementation: track `last_keypress_time` in cursor state, compare against blink interval in `tick_blink()`
- This matches standard terminal behavior (Alacritty, Kitty, WezTerm, Ghostty all do this)

### FR-4: Window Focus Cursor Behavior

- On window focus lost: render hollow block cursor (existing `CursorState::set_focused(false)` logic)
- On window focus gained: restore configured cursor style (existing `set_focused(true)` logic)
- Wire `WindowEvent::Focused(bool)` to `CursorState::set_focused()` in window.rs
- Trigger redraw on focus change

### FR-5: Keyboard Input Verification

Verify all fundamental keyboard inputs produce correct PTY byte sequences (most already implemented in `src/input/mod.rs`):

- **Backspace**: produces `0x7F` (DEL) — verify cursor moves left and character is deleted
- **Delete**: produces `\x1b[3~` — verify character under cursor is removed
- **Arrow keys**: Up `\x1b[A`, Down `\x1b[B`, Right `\x1b[C`, Left `\x1b[D` — verify cursor position updates
- **Home/End**: `\x1b[H` / `\x1b[F` — verify cursor jumps to line start/end
- **Ctrl+A / Ctrl+E**: `0x01` / `0x05` — pass-through to shell for beginning/end of line
- **Up/Down arrows**: verify shell command history navigation works (pass-through)
- Write verification tests for each key type confirming correct escape sequences

## Non-Functional Requirements

- Cursor rendering adds <1ms per frame
- Blink timer only requests redraws when cursor visibility state actually changes
- No unnecessary GPU wakeups when cursor is not blinking (blink disabled or window unfocused)

## Acceptance Criteria

1. Cursor is visible and renders at the correct position in the terminal
2. Cursor blinks at the configured rate (default 500ms) when idle
3. Cursor stops blinking and shows solid on keystroke, resumes after idle
4. Cursor renders as hollow block when window is unfocused
5. Cursor style (block/beam/underline) matches config and renders correctly
6. `cursor.blink_rate` is configurable via `veloterm.toml` and hot-reloadable
7. Backspace, delete, arrows, home, end, Ctrl+A, Ctrl+E all work correctly
8. All existing tests continue to pass

## Out of Scope

- New cursor styles beyond block/beam/underline
- Cursor color customization beyond theme defaults
- IME (Input Method Editor) support for CJK input
- Mouse cursor (pointer) customization
- Application cursor mode (DECCKM) — different arrow key sequences
- Alt modifier key support
- Smooth cursor animation between positions

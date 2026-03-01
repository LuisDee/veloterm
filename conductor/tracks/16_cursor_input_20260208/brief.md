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

# Track 16: Cursor & Input Polish

## UI Reference

The visual aesthetic MUST match the reference mockup:
- **Reference Cargo.toml:** `/Users/luisdeburnay/Downloads/Cargo.toml`
- **Reference main.rs:** `/Users/luisdeburnay/Downloads/src/main.rs`

## What This Track Delivers

Verifies and hardens the cursor and keyboard input experience to production quality. Ensures the cursor accurately tracks the input position in all scenarios (normal typing, after backspace, after arrow key movement, after command history navigation). Makes cursor blink rate configurable. Verifies that all fundamental keyboard inputs (backspace, delete, arrow keys, home, end) produce correct PTY byte sequences.

## Scope

### IN
- Cursor position accuracy: verify cursor tracks correctly after typing, backspace, delete, arrow movement
- Cursor blink rate: configurable interval (default 500ms) via config
- Cursor style verification: block, beam, underline all render correctly and are configurable
- Hollow cursor when window unfocused (verify existing)
- Keyboard input verification: backspace, delete, home, end, arrow keys produce correct terminal escape sequences
- Up/down arrow command history (verify shell pass-through works correctly)
- Ctrl+A (beginning of line), Ctrl+E (end of line) pass-through

### OUT
- New cursor styles beyond block/beam/underline
- Cursor color customization beyond theme defaults
- IME (Input Method Editor) support for CJK input
- Mouse cursor (pointer) customization

## Key Design Decisions

1. **Cursor blink implementation**: CSS-style animation timer vs GPU shader-based blink vs application-level timer toggling visibility?
   Trade-off: timer is simplest and already implemented; shader avoids CPU wakeups; both are valid

2. **Blink pause on keystroke**: pause blink and show solid cursor during typing (like most terminals) vs continuous blink regardless?
   Trade-off: pausing is more polished and standard behavior; continuous is simpler

3. **Cursor trail/animation**: instant position change vs smooth cursor movement animation between positions?
   Trade-off: instant is standard for terminals and fastest; smooth looks polished but may feel laggy

## Architectural Notes

- The existing `CursorRenderer` in `src/renderer/cursor.rs` already supports block/beam/underline/hollow — this track is primarily verification and polish
- Cursor position comes from `alacritty_terminal`'s `Term::cursor()` — verify this stays in sync after every input type
- The blink timer at 500ms is already implemented — making it configurable requires adding `cursor.blink_rate` to Config
- Arrow keys and backspace are handled by alacritty_terminal's key binding system — verify the byte sequences are correct for both normal and application cursor mode

## Complexity: S
## Estimated Phases: ~2

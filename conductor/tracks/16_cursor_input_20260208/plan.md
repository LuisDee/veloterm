# Track 16: Cursor & Input Polish — Implementation Plan

## Phase 1: Config & Cursor Integration

### Task 1.1: Add blink_rate config field [x] <!-- 6fdea6a -->
- Add `blink_rate` field to `CursorConfig` (u64, default 500ms)
- Add `RawCursorConfig` deserialization support
- Validation: min 100ms, max 2000ms, 0 = disable
- Add `cursor_changed` flag to `ConfigDelta` if not already present
- Update `Config::from_toml()` and `Config::diff()`
- Write tests: defaults, parsing, validation bounds, delta detection

### Task 1.2: Wire cursor position from terminal to CursorState [x] <!-- c2acf64 -->
- Add `CursorState` field to Renderer (or window state)
- After each PTY drain in window.rs, extract cursor position from terminal via `Term::cursor()`
- Update `CursorState` position each frame before rendering
- Map config `cursor.style` string to `CursorStyle` enum
- Wire `WindowEvent::Focused(bool)` to `CursorState::set_focused()`
- Write tests: cursor position updates after terminal state change, focus transitions

### Task 1.3: Render cursor as cell instance overlay [x] <!-- c2acf64 -->
- In render_frame(), call `CursorState::to_cell_instance()` to generate cursor overlay
- Inject cursor cell instance into the render pipeline (append to instance buffer)
- Apply padding offset to cursor position (content-area coordinates)
- Call `tick_blink()` during RedrawRequested to advance blink timer
- Only request redraw on blink state change (avoid unnecessary GPU wakeups)
- Write tests: cursor instance generation, blink state transitions trigger/skip redraws

### Phase 1 Checkpoint [x] [checkpoint: 5f67bc5]

## Phase 2: Blink Polish & Input Verification

### Task 2.1: Configurable blink rate and blink pause on keystroke [x] <!-- a827635 -->
- Replace hardcoded `BLINK_INTERVAL` with configurable value from `CursorConfig.blink_rate`
- Track `last_keypress_time` in CursorState
- On keystroke: reset blink timer, force cursor visible
- Resume blinking after idle (no keystrokes for one full blink interval)
- When blink disabled (config or blink_rate=0): cursor always visible, no timer
- Wire config hot-reload: cursor config change updates blink rate
- Write tests: blink rate changes, keystroke pause/resume, disabled blink, hot-reload

### Task 2.2: Keyboard input verification tests [x] <!-- a827635 -->
- Write comprehensive tests verifying PTY byte sequences for:
  - Backspace → 0x7F
  - Delete → \x1b[3~
  - Arrow keys → correct CSI sequences
  - Home/End → \x1b[H / \x1b[F
  - Ctrl+A → 0x01, Ctrl+E → 0x05
  - Enter → \r, Tab → \t, Escape → \x1b
- Verify no regressions in existing key translation
- Add any missing edge case tests

### Task 2.3: Final integration and visual validation [x] <!-- 9f81b81 -->
- Run full test suite — all existing + new tests pass
- Build and run application — verify cursor renders at correct position
- Verify cursor blinks at ~500ms by default
- Verify cursor shows solid during typing, resumes blink after idle
- Verify hollow block on window unfocus
- Verify Cmd+Plus/Minus still works (no input regression)
- Screenshot validation with Playwright MCP

### Phase 2 Checkpoint [x] [checkpoint: 5f67bc5]

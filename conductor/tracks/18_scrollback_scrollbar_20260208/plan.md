# Track 18: Scrollback & Scrollbar — Implementation Plan

## Phase 1: Scroll State Machine & Mouse Wheel Handling (FR-1, FR-2, FR-3, FR-8)

Core scroll logic: state tracking, mouse wheel event handling, smooth animation, clamping, and auto-scroll behavior.

### Task 1.1: Scroll state and offset math
- [x] Create `src/scroll.rs` with `ScrollState` struct: `target_offset: usize`, `current_offset: f32`, `last_scroll_time: Instant` <!-- abd48e6 -->
- [x] Implement `apply_line_delta(delta: f32, history_size: usize)` — updates target, clamps to 0..=history_size <!-- abd48e6 -->
- [x] Implement `apply_pixel_delta(delta_px: f32, cell_height: f32, history_size: usize)` — converts px to lines, applies immediately <!-- abd48e6 -->
- [x] Implement `tick(dt_secs: f32) -> bool` — ease-out interpolation, returns true if still animating <!-- abd48e6 -->
- [x] Implement `snap_to_bottom()` — resets target and current to 0 <!-- abd48e6 -->
- [x] Implement `current_line_offset() -> usize` — rounded current for display_offset <!-- abd48e6 -->
- [x] TDD: tests for clamping, delta conversion, ease-out convergence, snap-to-bottom (25 tests) <!-- abd48e6 -->

### Task 1.2: Wire MouseWheel events in window.rs
- [x] Handle `WindowEvent::MouseWheel` for focused pane <!-- bc3fc91 -->
- [x] `LineDelta`: call `scroll_state.apply_line_delta()`, request redraw <!-- bc3fc91 -->
- [x] `PixelDelta`: call `scroll_state.apply_pixel_delta()`, apply to terminal immediately <!-- bc3fc91 -->
- [x] Update `terminal.set_display_offset()` each frame based on `scroll_state.current_line_offset()` <!-- bc3fc91 -->
- [x] Request continuous redraws while scroll animation is in progress <!-- bc3fc91 -->

### Task 1.3: Auto-scroll and keyboard snap
- [x] On keyboard input to PTY: call `scroll_state.snap_to_bottom()`, apply to terminal <!-- bc3fc91 -->
- [x] Auto-scroll: when `display_offset == 0` and new output arrives, stay at bottom (default behavior) <!-- bc3fc91 -->
- [x] Scroll lock: when user scrolls up (`display_offset > 0`), don't auto-scroll <!-- bc3fc91 -->
- [x] TDD: tests for snap-to-bottom on keystroke, scroll lock preservation (covered by scroll.rs unit tests) <!-- abd48e6 -->

### Phase 1 Completion
- [x] Phase completion verification and checkpointing protocol

---

## Phase 2: Scrollbar Overlay & Auto-Hide (FR-4, FR-5)

Visual scrollbar rendering as an auto-hiding overlay quad.

### Task 2.1: Scrollbar geometry and rendering
- [x] Add `scrollbar_thumb_rect()` to scroll.rs <!-- 507bf2c -->
- [x] Thumb height: `max(20.0, (visible_rows / total_rows) * track_height)` <!-- 507bf2c -->
- [x] Thumb position: inverted (offset 0 = thumb at bottom, max = thumb at top) <!-- 507bf2c -->
- [x] Bar width: 6px, positioned at right edge of pane content area (inside padding) <!-- 507bf2c -->
- [x] Return None when history_size == 0 <!-- 507bf2c -->
- [x] TDD: tests for thumb size, position, edge cases (10 tests) <!-- 507bf2c -->

### Task 2.2: Wire scrollbar into overlay pipeline
- [x] Add scrollbar quads to `generate_overlay_quads()` in window.rs <!-- 507bf2c -->
- [x] For each visible pane with scrollback: generate scrollbar thumb quad <!-- 507bf2c -->
- [x] Color: `[1.0, 1.0, 1.0, alpha]` where alpha comes from auto-hide state <!-- 507bf2c -->
- [x] Scrollbar renders on top of terminal content but below search/tab overlays <!-- 507bf2c -->

### Task 2.3: Auto-hide with fade animation
- [x] Track `last_scroll_time: Option<Instant>` in ScrollState <!-- abd48e6 -->
- [x] `scrollbar_alpha(now: Instant) -> f32`: returns 0.3 if within 1.5s, lerp to 0.0 over next 0.3s <!-- abd48e6 -->
- [x] On scroll activity: update `last_scroll_time`, request redraw <!-- bc3fc91 -->
- [x] Request continuous redraws during fade-out animation <!-- bc3fc91 -->
- [x] TDD: tests for alpha timing (visible, fading, hidden) <!-- abd48e6 -->

### Phase 2 Completion
- [x] Phase completion verification and checkpointing protocol

---

## Phase 3: Scrollbar Interaction & Verification (FR-6, FR-7, FR-9)

Scrollbar mouse interaction (click-to-position, drag) and line wrapping verification.

### Task 3.1: Scrollbar hit testing and click-to-position
- [x] Add `scrollbar_hit_test()` enum: `None`, `Track(y)`, `Thumb(y)` <!-- 507bf2c -->
- [x] On left mouse click in scrollbar track region: compute target offset, apply via ease-out <!-- c1a3d41 -->
- [x] Show scrollbar on interaction (reset auto-hide timer) <!-- c1a3d41 -->
- [x] TDD: tests for hit test geometry, position-to-offset conversion (7 tests) <!-- 507bf2c -->

### Task 3.2: Scrollbar thumb drag
- [x] Track drag state: `is_dragging_scrollbar`, `drag_start_y`, `drag_start_offset` <!-- c1a3d41 -->
- [x] On mouse press on thumb: enter drag mode, capture start position <!-- c1a3d41 -->
- [x] On mouse drag: compute new offset from delta, apply immediately <!-- c1a3d41 -->
- [x] On mouse release: exit drag mode <!-- c1a3d41 -->
- [x] TDD: tests for drag offset calculation (6 tests) <!-- c1a3d41 -->

### Task 3.3: Line wrapping and reflow verification
- [x] Add tests confirming text wraps at terminal width boundary <!-- 4fefc23 -->
- [x] Add tests confirming content reflows on terminal resize <!-- 4fefc23 -->
- [x] These validate alacritty_terminal behavior (3 tests) <!-- 4fefc23 -->

### Task 3.4: Final integration and visual validation
- [x] Build and launch via `./take-screenshot.sh` — verified
- [x] Scrollbar hidden when no history (correct behavior at startup)
- [x] Terminal rendering, tab bar, cursor all working correctly

### Phase 3 Completion
- [x] Phase completion verification and checkpointing protocol

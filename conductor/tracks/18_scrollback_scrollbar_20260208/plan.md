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
- [ ] Add `scrollbar_thumb_rect(pane_rect, padding, visible_rows, total_rows, display_offset, max_offset) -> Option<OverlayQuad>` to scroll.rs
- [ ] Thumb height: `max(20.0, (visible_rows / total_rows) * track_height)`
- [ ] Thumb position: inverted (offset 0 = thumb at bottom, max = thumb at top)
- [ ] Bar width: 6px, positioned at right edge of pane content area (inside padding)
- [ ] Return None when history_size == 0
- [ ] TDD: tests for thumb size calculation, position mapping, edge cases (no history, full scroll)

### Task 2.2: Wire scrollbar into overlay pipeline
- [ ] Add scrollbar quads to `generate_overlay_quads()` in window.rs
- [ ] For each visible pane with scrollback: generate scrollbar thumb quad
- [ ] Color: `[1.0, 1.0, 1.0, alpha]` where alpha comes from auto-hide state
- [ ] Scrollbar renders on top of terminal content but below search/tab overlays

### Task 2.3: Auto-hide with fade animation
- [ ] Track `last_scroll_time: Option<Instant>` in ScrollState
- [ ] `scrollbar_alpha(now: Instant) -> f32`: returns 0.3 if within 1.5s, lerp to 0.0 over next 0.3s, then 0.0
- [ ] On scroll activity: update `last_scroll_time`, request redraw
- [ ] Request continuous redraws during fade-out animation
- [ ] TDD: tests for alpha timing (visible, fading, hidden)

### Phase 2 Completion
- [ ] Phase completion verification and checkpointing protocol

---

## Phase 3: Scrollbar Interaction & Verification (FR-6, FR-7, FR-9)

Scrollbar mouse interaction (click-to-position, drag) and line wrapping verification.

### Task 3.1: Scrollbar hit testing and click-to-position
- [ ] Add `scrollbar_hit_test(click_x, click_y, pane_rect, padding) -> ScrollbarHit` enum: `None`, `Track(y)`, `Thumb(y)`
- [ ] On left mouse click in scrollbar track region: compute target offset from proportional position, apply via ease-out
- [ ] Show scrollbar on interaction (reset auto-hide timer)
- [ ] TDD: tests for hit test geometry, position-to-offset conversion

### Task 3.2: Scrollbar thumb drag
- [ ] Track drag state in ScrollState: `is_dragging_scrollbar: bool`, `drag_start_y: f32`, `drag_start_offset: usize`
- [ ] On mouse press on thumb: enter drag mode, capture start position
- [ ] On mouse drag: compute new offset from delta, apply immediately (no ease-out)
- [ ] On mouse release: exit drag mode
- [ ] TDD: tests for drag offset calculation

### Task 3.3: Line wrapping and reflow verification
- [ ] Add tests confirming text wraps at terminal width boundary
- [ ] Add tests confirming content reflows on terminal resize
- [ ] These validate alacritty_terminal behavior — may pass immediately

### Task 3.4: Final integration and visual validation
- [ ] Build and launch via `./take-screenshot.sh`
- [ ] Verify scrollbar appears when scrolling through history
- [ ] Verify scrollbar fades out after inactivity
- [ ] Verify smooth scroll animation on mouse wheel

### Phase 3 Completion
- [ ] Phase completion verification and checkpointing protocol

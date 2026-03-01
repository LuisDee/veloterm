<!-- ARCHITECT CONTEXT | Track: 18_scrollback_scrollbar | Wave: 6 | CC: v2 -->

## Cross-Cutting Constraints
- Performance Budget: smooth scrolling must maintain 60fps; scrollbar rendering must not add measurable overhead
- Testing: TDD for scroll offset math, scrollbar position calculation, smooth interpolation
- UI Reference Compliance: scrollbar must be subtle, auto-hiding, consistent with warm dark theme

## Interfaces

### Owns
- Mouse wheel / trackpad scroll handling
- Smooth scroll animation (frame-interpolated)
- Scrollbar UI (auto-hiding, subtle styling)
- Scroll position tracking and clamping

### Consumes
- `Terminal` (Track 02) — scrollback buffer and display offset
- `Config` (Track 03) — scrollback.lines, scroll speed settings
- `GridRenderer` (Track 01) — viewport offset for rendering
- `DamageTracker` (Track 07) — damage on scroll

## Dependencies
- Track 02_core_terminal_emulation: Scrollback buffer management
- Track 07_perf_damage: Damage tracking for scroll events
- Track 15_font_padding: padding affects scrollbar positioning

<!-- END ARCHITECT CONTEXT -->

# Track 18: Scrollback & Scrollbar — Specification

## Overview

Complete scrollback experience with mouse wheel/trackpad scrolling through terminal history, smooth scroll animation, and a subtle auto-hiding scrollbar overlay. Scroll granularity is line-level (matching alacritty_terminal's integer display_offset), with smooth ease-out animation between line boundaries for discrete mouse wheel events and direct pixel-delta passthrough for trackpad gestures. The scrollbar is a thin (~6px) semi-transparent overlay that auto-hides after ~1.5s of inactivity.

## Design Decisions

1. **Smooth scroll algorithm**: Hybrid — ease-out for discrete `LineDelta` (mouse wheel), passthrough for `PixelDelta` (trackpad). No platform-specific code; input type determines behavior.
2. **Scroll granularity**: Line-by-line. Display offset is always an integer line count. Ease-out animation interpolates smoothly between line boundaries.
3. **Scrollbar style**: Thin overlay (~6px wide), semi-transparent, rendered on top of terminal content via OverlayQuad system. Does not consume terminal column space.
4. **Scrollbar auto-hide**: Timed fade-out after ~1.5s of no scroll activity. Scrollbar alpha interpolates from visible to transparent.

## Existing Infrastructure

- `Terminal::scroll_up/scroll_down/display_offset/history_size` — alacritty_terminal scroll API (`src/terminal/mod.rs:127-196`)
- `extract_grid_cells()` — already respects `display_offset` for viewport extraction (`src/terminal/grid_bridge.rs:148-199`)
- `OverlayQuad` system — rect + RGBA quads rendered on top of content (`src/pane/divider.rs:53-58`)
- `generate_overlay_quads()` — composition point in `src/window.rs:697-769`
- No `MouseWheel` handler exists yet in window.rs event loop

## Functional Requirements

### FR-1: Mouse Wheel Scrolling
- Handle `WindowEvent::MouseWheel` with both `LineDelta` and `PixelDelta` variants
- `LineDelta(_, y)`: convert to line count (typically y * 3 lines), set as scroll target for ease-out animation
- `PixelDelta(pos)`: convert `pos.y` pixels to lines using cell_height, apply immediately to display_offset
- Scroll direction: positive delta = scroll up (view history), negative = scroll down (toward bottom)
- Clamp scroll target to valid range: `0..=terminal.history_size()`

### FR-2: Smooth Scroll Animation (LineDelta only)
- Maintain per-pane `scroll_target: usize` and `scroll_current: f32`
- On LineDelta event: update `scroll_target` (clamped)
- Each frame: interpolate `scroll_current` toward `scroll_target` using ease-out (`t * (2 - t)`)
- When `scroll_current` crosses a line boundary, call `terminal.set_display_offset(rounded_value)`
- Animation duration: ~150ms for typical 3-line scroll
- Request redraw while animation is in progress

### FR-3: Auto-Scroll to Bottom
- When new terminal output arrives and `display_offset == 0` (already at bottom), stay at bottom
- When user has scrolled up (`display_offset > 0`), do NOT auto-scroll — preserve scroll lock
- Provide `snap_to_bottom()` on any keyboard input to PTY (user is typing, return to live view)

### FR-4: Scrollbar Overlay Rendering
- Render a thin vertical bar (~6px wide) on the right edge of the terminal pane content area
- Positioned inside the pane rect, accounting for terminal padding
- Thumb height: `(visible_rows / total_rows) * track_height`, minimum 20px
- Thumb position: `(display_offset / max_offset) * (track_height - thumb_height)`, inverted (0 = bottom)
- Color: semi-transparent white (~0.3 alpha), slightly brighter on hover (~0.5 alpha)
- Only visible when `history_size > 0` (there's content to scroll to)
- Rendered via OverlayQuad system (same pipeline as dividers/tab bar)

### FR-5: Scrollbar Auto-Hide
- Scrollbar becomes visible on any scroll event (wheel, trackpad, scrollbar interaction)
- After ~1.5s of no scroll activity, begin fade-out animation (~300ms)
- Alpha interpolates from visible (0.3) to transparent (0.0)
- Track `last_scroll_time` per pane to drive fade logic

### FR-6: Scrollbar Click-to-Position
- Clicking on the scrollbar track (not the thumb) scrolls to that proportional position
- Click position mapped to `(click_y / track_height) * max_offset`
- Triggers ease-out animation to the target position

### FR-7: Scrollbar Drag
- Clicking and dragging the scrollbar thumb scrubs through history
- Track drag state: `is_dragging_scrollbar`, `drag_start_y`, `drag_start_offset`
- During drag: `new_offset = drag_start_offset + (delta_y / track_height) * max_offset`
- Smooth, immediate response (no ease-out during drag)

### FR-8: Scroll Position Clamping
- Display offset always clamped to `0..=history_size`
- Scroll target always clamped before animation begins
- Handle edge case: history_size can shrink (lines fall off scrollback buffer) — clamp current offset if it exceeds new max

### FR-9: Line Wrapping & Reflow Verification
- Verify that text wraps correctly at terminal width boundaries
- Verify that content reflows properly on window resize
- These should already work via alacritty_terminal — add tests to confirm

## Non-Functional Requirements

- Scroll animation must not drop below 60fps
- Scrollbar rendering adds <0.1ms per frame (single overlay quad)
- No allocations in the scroll hot path (mouse wheel handler)
- Works on both macOS and Linux without platform-specific code

## Acceptance Criteria

- [ ] Mouse wheel scrolls through terminal history
- [ ] Trackpad smooth scroll works with pixel-level deltas
- [ ] Discrete scroll events animate smoothly between line positions
- [ ] Scrollbar appears on scroll activity, fades out after ~1.5s
- [ ] Scrollbar thumb size reflects visible vs total content
- [ ] Scrollbar click-to-position works
- [ ] Scrollbar drag-to-scrub works
- [ ] Auto-scroll to bottom on new output (when at bottom)
- [ ] Scroll lock when user has scrolled up
- [ ] Keyboard input snaps to bottom
- [ ] Scroll position clamped to valid range
- [ ] Line wrapping works correctly
- [ ] Content reflows on resize

## Out of Scope

- Horizontal scrolling (content always wraps)
- Infinite scrollback (uses configurable limit from config)
- Scrollback search (Track 09)
- Page up/down keyboard shortcuts (defer to vi-mode, Track 11)
- Scrollbar color theming (uses hardcoded semi-transparent white)

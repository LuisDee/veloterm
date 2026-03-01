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

# Track 18: Scrollback & Scrollbar

## UI Reference

The visual aesthetic MUST match the reference mockup:
- **Reference Cargo.toml:** `/Users/luisdeburnay/Downloads/Cargo.toml`
- **Reference main.rs:** `/Users/luisdeburnay/Downloads/src/main.rs`

The reference uses `scrollable()` with default scrollbar styling — match a subtle, thin scrollbar that auto-hides when not scrolling.

## What This Track Delivers

Complete scrollback experience with mouse wheel/trackpad scrolling through terminal history, smooth scroll animation (not instant jumps), and a subtle auto-hiding scrollbar on the right side. Verifies that line wrapping works correctly and that content reflows properly when the window is resized.

## Scope

### IN
- Mouse wheel scrolling through terminal history (scroll up to view past output)
- Trackpad smooth scroll gesture support (pixel-level scrolling, not line-level)
- Smooth scroll animation with easing (frame-interpolated offset transitions)
- Auto-hiding scrollbar on right side of terminal area
- Scrollbar thumb size reflects visible portion vs total scrollback
- Scrollbar click-to-scroll-to-position
- Scrollbar drag to scrub through history
- Scroll position clamped to valid range (0 to max scrollback)
- Auto-scroll to bottom on new output (when already at bottom)
- Scroll lock: if user has scrolled up, don't auto-scroll on new output
- Line wrapping verification: text wraps correctly at terminal width
- Reflow verification: content reflows on window resize

### OUT
- Horizontal scrolling (terminal content always wraps)
- Infinite scrollback (configurable limit via config)
- Scrollback search (already implemented in Track 09)
- Page up/Page down keyboard shortcuts (defer to vi-mode in Track 11)

## Key Design Decisions

1. **Smooth scroll algorithm**: Linear interpolation (lerp) vs ease-out curve vs spring physics vs match macOS native scroll feel?
   Trade-off: lerp is simplest; ease-out feels natural; spring physics is most realistic; native match requires platform-specific tuning

2. **Scroll granularity**: Line-by-line (traditional terminal) vs pixel-smooth (modern app feel) vs configurable?
   Trade-off: line-by-line is standard for terminals; pixel-smooth feels modern but may conflict with cell grid alignment; configurable adds complexity

3. **Scrollbar style**: Thin overlay (macOS-style, ~6px) vs traditional scrollbar (~12px) vs invisible with scroll indicators only?
   Trade-off: thin overlay is modern and matches reference; traditional wastes space; invisible hides scrollback existence

4. **Scrollbar auto-hide timing**: Hide after 1s of no scroll activity vs hide on mouse leave vs always visible when scrollback exists?
   Trade-off: timed hide is macOS standard; mouse leave is responsive; always visible is discoverable

## Architectural Notes

- alacritty_terminal manages the scrollback buffer internally — scroll offset is a `usize` representing lines above the visible viewport
- Current scrolling may already work via alacritty_terminal's `scroll()` method — verify and enhance with smooth animation
- Smooth scrolling requires interpolating between the current display offset and the target offset over multiple frames — this means the render loop must keep animating even when no terminal content has changed
- The scrollbar is the first non-content GPU overlay that needs to be rendered on top of the terminal grid — establish the z-ordering pattern
- Damage tracking must be aware of scroll events — when scrolling, the entire visible area changes (full damage)
- Scrollbar position must account for terminal padding (Track 15)

## Complexity: M
## Estimated Phases: ~3

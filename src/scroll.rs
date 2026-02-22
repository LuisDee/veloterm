// Scroll state machine: manages smooth scrolling with ease-out animation,
// pixel/line delta handling, auto-hide timing, and scrollbar geometry.

use std::time::Instant;

/// Per-pane scroll state tracking smooth animation and auto-hide.
pub struct ScrollState {
    /// Target display offset in lines (integer, what we're animating toward).
    target_offset: usize,
    /// Current interpolated offset (float, for smooth animation).
    current_offset: f32,
    /// Last time a scroll event occurred (for auto-hide).
    last_scroll_time: Option<Instant>,
    /// Accumulated fractional lines from pixel deltas (sub-line remainder).
    pixel_accumulator: f32,
    /// Whether the user is currently dragging the scrollbar thumb.
    pub is_dragging_scrollbar: bool,
    /// Y position where the scrollbar drag started.
    pub drag_start_y: f32,
    /// Scroll offset when the drag started.
    pub drag_start_offset: usize,
}

/// Animation speed: fraction of remaining distance covered per second.
/// Higher = faster convergence. At 12.0, a 3-line scroll completes in ~150ms.
const EASE_OUT_SPEED: f32 = 12.0;

/// Threshold below which we snap to the target (avoids endless micro-animation).
const SNAP_THRESHOLD: f32 = 0.01;

/// Lines per discrete mouse wheel notch.
const LINES_PER_NOTCH: f32 = 3.0;

impl ScrollState {
    pub fn new() -> Self {
        Self {
            target_offset: 0,
            current_offset: 0.0,
            last_scroll_time: None,
            pixel_accumulator: 0.0,
            is_dragging_scrollbar: false,
            drag_start_y: 0.0,
            drag_start_offset: 0,
        }
    }

    /// Apply a discrete line delta (mouse wheel). Positive = scroll up (view history).
    pub fn apply_line_delta(&mut self, delta: f32, history_size: usize) {
        let lines = (delta * LINES_PER_NOTCH).round() as isize;
        let new_target = (self.target_offset as isize + lines).max(0) as usize;
        self.target_offset = new_target.min(history_size);
        self.last_scroll_time = Some(Instant::now());
    }

    /// Apply a pixel delta (trackpad). Converts to lines, applies immediately.
    pub fn apply_pixel_delta(&mut self, delta_px: f32, cell_height: f32, history_size: usize) {
        if cell_height <= 0.0 {
            return;
        }
        self.pixel_accumulator += delta_px / cell_height;
        let whole_lines = self.pixel_accumulator.trunc() as isize;
        if whole_lines != 0 {
            self.pixel_accumulator -= whole_lines as f32;
            let new_offset = (self.target_offset as isize + whole_lines).max(0) as usize;
            self.target_offset = new_offset.min(history_size);
            self.current_offset = self.target_offset as f32;
        }
        self.last_scroll_time = Some(Instant::now());
    }

    /// Advance the animation by `dt` seconds. Returns true if still animating.
    pub fn tick(&mut self, dt_secs: f32) -> bool {
        let target = self.target_offset as f32;
        let diff = target - self.current_offset;
        if diff.abs() < SNAP_THRESHOLD {
            self.current_offset = target;
            return false;
        }
        // Exponential ease-out: move a fraction of the remaining distance each frame.
        self.current_offset += diff * (1.0 - (-EASE_OUT_SPEED * dt_secs).exp());
        // Check again after move
        if (self.target_offset as f32 - self.current_offset).abs() < SNAP_THRESHOLD {
            self.current_offset = target;
            false
        } else {
            true
        }
    }

    /// Snap to the bottom (offset 0). Used on keyboard input.
    pub fn snap_to_bottom(&mut self) {
        self.target_offset = 0;
        self.current_offset = 0.0;
        self.pixel_accumulator = 0.0;
    }

    /// Current display offset rounded to nearest line (for terminal.set_display_offset).
    pub fn current_line_offset(&self) -> usize {
        self.current_offset.round().max(0.0) as usize
    }

    /// The target offset we're animating toward.
    pub fn target_offset(&self) -> usize {
        self.target_offset
    }

    /// Whether the animation is currently in progress.
    pub fn is_animating(&self) -> bool {
        (self.target_offset as f32 - self.current_offset).abs() >= SNAP_THRESHOLD
    }

    /// Clamp offsets to a (possibly reduced) history size.
    pub fn clamp_to_history(&mut self, history_size: usize) {
        if self.target_offset > history_size {
            self.target_offset = history_size;
        }
        let max = history_size as f32;
        if self.current_offset > max {
            self.current_offset = max;
        }
    }

    /// Set target directly (for scrollbar click-to-position).
    pub fn set_target(&mut self, offset: usize, history_size: usize) {
        self.target_offset = offset.min(history_size);
        self.last_scroll_time = Some(Instant::now());
    }

    /// Set offset immediately without animation (for scrollbar drag).
    pub fn set_immediate(&mut self, offset: usize, history_size: usize) {
        let clamped = offset.min(history_size);
        self.target_offset = clamped;
        self.current_offset = clamped as f32;
        self.last_scroll_time = Some(Instant::now());
    }

    /// Apply an immediate scroll delta (for auto-scroll during drag selection).
    /// Positive lines = scroll into history, negative = scroll toward live.
    pub fn apply_auto_scroll(&mut self, lines: i32, history_size: usize) {
        let new_offset = (self.target_offset as i64 + lines as i64).max(0) as usize;
        let clamped = new_offset.min(history_size);
        self.target_offset = clamped;
        self.current_offset = clamped as f32;
        self.last_scroll_time = Some(Instant::now());
    }

    /// Scrollbar alpha based on time since last scroll activity.
    /// Returns 0.3 if within 1.5s, fades to 0.0 over next 0.3s.
    pub fn scrollbar_alpha(&self, now: Instant) -> f32 {
        self.scrollbar_alpha_at(now, 1.5, 0.3, 0.3)
    }

    /// Testable version with configurable timings.
    pub fn scrollbar_alpha_at(
        &self,
        now: Instant,
        visible_secs: f32,
        fade_secs: f32,
        max_alpha: f32,
    ) -> f32 {
        let last = match self.last_scroll_time {
            Some(t) => t,
            None => return 0.0,
        };
        let elapsed = now.duration_since(last).as_secs_f32();
        if elapsed < visible_secs {
            max_alpha
        } else if elapsed < visible_secs + fade_secs {
            let fade_progress = (elapsed - visible_secs) / fade_secs;
            max_alpha * (1.0 - fade_progress)
        } else {
            0.0
        }
    }

    /// Whether the scrollbar should be requesting redraws (visible or fading).
    pub fn scrollbar_needs_redraw(&self, now: Instant) -> bool {
        self.scrollbar_alpha(now) > 0.0
    }

    /// Last scroll time (for external auto-hide logic).
    pub fn last_scroll_time(&self) -> Option<Instant> {
        self.last_scroll_time
    }

    /// Mark scroll activity (e.g., from scrollbar interaction).
    pub fn touch(&mut self) {
        self.last_scroll_time = Some(Instant::now());
    }

    /// Begin a scrollbar thumb drag.
    pub fn begin_drag(&mut self, start_y: f32) {
        self.is_dragging_scrollbar = true;
        self.drag_start_y = start_y;
        self.drag_start_offset = self.target_offset;
        self.last_scroll_time = Some(Instant::now());
    }

    /// Update during a scrollbar thumb drag.
    /// `delta_y` is the pixel distance dragged from the start position.
    /// `track_height` is the total scrollable track height.
    /// Negative delta_y (dragging up) increases offset.
    pub fn update_drag(
        &mut self,
        current_y: f32,
        track_height: f32,
        history_size: usize,
    ) {
        if track_height <= 0.0 || history_size == 0 {
            return;
        }
        let delta_y = current_y - self.drag_start_y;
        // Dragging down = decreasing offset (toward bottom), up = increasing (toward top)
        let offset_delta = -(delta_y / track_height) * history_size as f32;
        let new_offset = (self.drag_start_offset as f32 + offset_delta)
            .round()
            .max(0.0) as usize;
        self.set_immediate(new_offset, history_size);
    }

    /// End a scrollbar thumb drag.
    pub fn end_drag(&mut self) {
        self.is_dragging_scrollbar = false;
    }
}

/// Scrollbar width in physical pixels.
const SCROLLBAR_WIDTH: f32 = 6.0;

/// Minimum thumb height in physical pixels.
const MIN_THUMB_HEIGHT: f32 = 20.0;

/// Scrollbar thumb geometry result.
#[derive(Debug, Clone, PartialEq)]
pub struct ScrollbarThumb {
    /// X position (left edge) in physical pixels.
    pub x: f32,
    /// Y position (top edge) in physical pixels.
    pub y: f32,
    /// Width in physical pixels.
    pub width: f32,
    /// Height in physical pixels.
    pub height: f32,
}

/// Compute the scrollbar thumb rectangle for a pane.
///
/// `pane_x`, `pane_y`, `pane_w`, `pane_h` are the pane content area in pixels.
/// `padding` is `[top, bottom, left, right]`.
/// `visible_rows` is the number of rows visible in the viewport.
/// `history_size` is the total scrollback lines available.
/// `display_offset` is the current scroll position (0 = bottom).
///
/// Returns `None` when there's no scrollback history.
pub fn scrollbar_thumb_rect(
    pane_x: f32,
    pane_y: f32,
    pane_w: f32,
    pane_h: f32,
    padding: [f32; 4],
    visible_rows: usize,
    history_size: usize,
    display_offset: usize,
) -> Option<ScrollbarThumb> {
    if history_size == 0 {
        return None;
    }

    let pad_top = padding[0];
    let pad_bottom = padding[1];
    let pad_right = padding[3];

    // Track area: inside padding, full height minus top/bottom padding
    let track_top = pane_y + pad_top;
    let track_height = pane_h - pad_top - pad_bottom;
    if track_height <= 0.0 {
        return None;
    }

    // Total content = visible rows + history
    let total_rows = visible_rows + history_size;
    let ratio = visible_rows as f32 / total_rows as f32;
    let thumb_height = (ratio * track_height).max(MIN_THUMB_HEIGHT).min(track_height);

    // Position: offset 0 = thumb at bottom, max offset = thumb at top
    let max_offset = history_size;
    let scroll_fraction = if max_offset > 0 {
        display_offset as f32 / max_offset as f32
    } else {
        0.0
    };
    // scroll_fraction 0 = bottom, 1 = top
    // thumb_y: at fraction 0, thumb is at bottom of track; at 1, top of track
    let available = track_height - thumb_height;
    let thumb_y = track_top + available * (1.0 - scroll_fraction);

    // X position: right edge of content area, inside padding
    let x = pane_x + pane_w - pad_right - SCROLLBAR_WIDTH;

    Some(ScrollbarThumb {
        x,
        y: thumb_y,
        width: SCROLLBAR_WIDTH,
        height: thumb_height,
    })
}

/// Hit-test result for scrollbar interaction.
#[derive(Debug, Clone, PartialEq)]
pub enum ScrollbarHit {
    /// No hit — click was outside the scrollbar region.
    None,
    /// Click on the scrollbar track (not on thumb). Contains the y position.
    Track(f32),
    /// Click on the scrollbar thumb. Contains the y position.
    Thumb(f32),
}

/// Hit-test a click position against the scrollbar region.
///
/// Returns whether the click hit the track, the thumb, or nothing.
pub fn scrollbar_hit_test(
    click_x: f32,
    click_y: f32,
    pane_x: f32,
    pane_y: f32,
    pane_w: f32,
    pane_h: f32,
    padding: [f32; 4],
    visible_rows: usize,
    history_size: usize,
    display_offset: usize,
) -> ScrollbarHit {
    let pad_right = padding[3];
    let bar_x = pane_x + pane_w - pad_right - SCROLLBAR_WIDTH;

    // Check if click is in the scrollbar column
    if click_x < bar_x || click_x > bar_x + SCROLLBAR_WIDTH {
        return ScrollbarHit::None;
    }

    // Check if click is in the vertical track area
    let pad_top = padding[0];
    let pad_bottom = padding[1];
    let track_top = pane_y + pad_top;
    let track_bottom = pane_y + pane_h - pad_bottom;
    if click_y < track_top || click_y > track_bottom {
        return ScrollbarHit::None;
    }

    // Check if click is on the thumb
    if let Some(thumb) = scrollbar_thumb_rect(
        pane_x, pane_y, pane_w, pane_h, padding,
        visible_rows, history_size, display_offset,
    ) {
        if click_y >= thumb.y && click_y <= thumb.y + thumb.height {
            return ScrollbarHit::Thumb(click_y);
        }
    }

    ScrollbarHit::Track(click_y)
}

/// Convert a click Y position on the scrollbar track to a scroll offset.
///
/// Returns the target offset (0 = bottom, history_size = top).
pub fn track_click_to_offset(
    click_y: f32,
    pane_y: f32,
    pane_h: f32,
    padding: [f32; 4],
    history_size: usize,
) -> usize {
    let pad_top = padding[0];
    let pad_bottom = padding[1];
    let track_top = pane_y + pad_top;
    let track_height = pane_h - pad_top - pad_bottom;
    if track_height <= 0.0 {
        return 0;
    }
    // Fraction: 0.0 = top of track (max offset), 1.0 = bottom (offset 0)
    let fraction = ((click_y - track_top) / track_height).clamp(0.0, 1.0);
    // Invert: top = max offset, bottom = 0
    let offset = ((1.0 - fraction) * history_size as f32).round() as usize;
    offset.min(history_size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // ── Construction ─────────────────────────────────────────────────

    #[test]
    fn new_starts_at_zero() {
        let s = ScrollState::new();
        assert_eq!(s.target_offset(), 0);
        assert_eq!(s.current_line_offset(), 0);
        assert!(!s.is_animating());
    }

    // ── Line delta ───────────────────────────────────────────────────

    #[test]
    fn line_delta_scrolls_up() {
        let mut s = ScrollState::new();
        s.apply_line_delta(1.0, 100); // 1 notch * 3 lines = 3
        assert_eq!(s.target_offset(), 3);
    }

    #[test]
    fn line_delta_scrolls_down() {
        let mut s = ScrollState::new();
        s.apply_line_delta(1.0, 100);
        assert_eq!(s.target_offset(), 3);
        s.apply_line_delta(-1.0, 100); // back down 3 lines
        assert_eq!(s.target_offset(), 0);
    }

    #[test]
    fn line_delta_clamps_to_zero() {
        let mut s = ScrollState::new();
        s.apply_line_delta(-5.0, 100); // can't go below 0
        assert_eq!(s.target_offset(), 0);
    }

    #[test]
    fn line_delta_clamps_to_history_size() {
        let mut s = ScrollState::new();
        s.apply_line_delta(100.0, 50); // 300 lines requested, only 50 available
        assert_eq!(s.target_offset(), 50);
    }

    #[test]
    fn line_delta_accumulates() {
        let mut s = ScrollState::new();
        s.apply_line_delta(1.0, 100); // +3
        s.apply_line_delta(1.0, 100); // +3 more
        assert_eq!(s.target_offset(), 6);
    }

    // ── Pixel delta ──────────────────────────────────────────────────

    #[test]
    fn pixel_delta_converts_to_lines() {
        let mut s = ScrollState::new();
        s.apply_pixel_delta(40.0, 20.0, 100); // 40px / 20px per line = 2 lines
        assert_eq!(s.target_offset(), 2);
        assert_eq!(s.current_line_offset(), 2); // applied immediately
    }

    #[test]
    fn pixel_delta_accumulates_subline() {
        let mut s = ScrollState::new();
        s.apply_pixel_delta(15.0, 20.0, 100); // 0.75 lines, no whole line yet
        assert_eq!(s.target_offset(), 0);
        s.apply_pixel_delta(10.0, 20.0, 100); // +0.5 = 1.25 total, 1 whole line
        assert_eq!(s.target_offset(), 1);
    }

    #[test]
    fn pixel_delta_clamps() {
        let mut s = ScrollState::new();
        s.apply_pixel_delta(-100.0, 20.0, 100); // negative = scroll down, but at 0
        assert_eq!(s.target_offset(), 0);
    }

    #[test]
    fn pixel_delta_zero_cell_height_noop() {
        let mut s = ScrollState::new();
        s.apply_pixel_delta(100.0, 0.0, 100);
        assert_eq!(s.target_offset(), 0);
    }

    // ── Ease-out animation ───────────────────────────────────────────

    #[test]
    fn tick_converges_to_target() {
        let mut s = ScrollState::new();
        s.apply_line_delta(1.0, 100); // target = 3
        // Run many frames
        for _ in 0..100 {
            s.tick(1.0 / 60.0);
        }
        assert_eq!(s.current_line_offset(), 3);
        assert!(!s.is_animating());
    }

    #[test]
    fn tick_returns_true_while_animating() {
        let mut s = ScrollState::new();
        s.apply_line_delta(1.0, 100);
        assert!(s.tick(1.0 / 60.0)); // first frame: still animating
    }

    #[test]
    fn tick_returns_false_when_at_target() {
        let mut s = ScrollState::new();
        assert!(!s.tick(1.0 / 60.0)); // already at target
    }

    #[test]
    fn tick_moves_toward_target() {
        let mut s = ScrollState::new();
        s.apply_line_delta(1.0, 100); // target = 3
        s.tick(1.0 / 60.0);
        // Should have moved from 0 toward 3
        assert!(s.current_offset > 0.0);
        assert!(s.current_offset < 3.0);
    }

    // ── Snap to bottom ───────────────────────────────────────────────

    #[test]
    fn snap_to_bottom_resets() {
        let mut s = ScrollState::new();
        s.apply_line_delta(5.0, 100);
        for _ in 0..100 {
            s.tick(1.0 / 60.0);
        }
        assert!(s.current_line_offset() > 0);
        s.snap_to_bottom();
        assert_eq!(s.target_offset(), 0);
        assert_eq!(s.current_line_offset(), 0);
    }

    // ── Clamping ─────────────────────────────────────────────────────

    #[test]
    fn clamp_to_history_reduces_offsets() {
        let mut s = ScrollState::new();
        s.apply_line_delta(10.0, 100); // target = 30
        s.current_offset = 30.0;
        s.clamp_to_history(10); // history shrunk to 10
        assert_eq!(s.target_offset(), 10);
        assert_eq!(s.current_offset, 10.0);
    }

    #[test]
    fn clamp_to_history_noop_when_within_range() {
        let mut s = ScrollState::new();
        s.apply_line_delta(1.0, 100); // target = 3
        s.clamp_to_history(100);
        assert_eq!(s.target_offset(), 3);
    }

    // ── Set target / immediate ───────────────────────────────────────

    #[test]
    fn set_target_clamps_and_animates() {
        let mut s = ScrollState::new();
        s.set_target(50, 100);
        assert_eq!(s.target_offset(), 50);
        assert!(s.is_animating()); // current is still 0
    }

    #[test]
    fn set_target_clamps_to_history() {
        let mut s = ScrollState::new();
        s.set_target(200, 100);
        assert_eq!(s.target_offset(), 100);
    }

    #[test]
    fn set_immediate_skips_animation() {
        let mut s = ScrollState::new();
        s.set_immediate(50, 100);
        assert_eq!(s.target_offset(), 50);
        assert_eq!(s.current_line_offset(), 50);
        assert!(!s.is_animating());
    }

    // ── Scrollbar alpha ──────────────────────────────────────────────

    #[test]
    fn alpha_zero_when_no_scroll_activity() {
        let s = ScrollState::new();
        assert_eq!(s.scrollbar_alpha(Instant::now()), 0.0);
    }

    #[test]
    fn alpha_visible_immediately_after_scroll() {
        let mut s = ScrollState::new();
        s.touch();
        let alpha = s.scrollbar_alpha_at(Instant::now(), 1.5, 0.3, 0.3);
        assert!((alpha - 0.3).abs() < 0.01);
    }

    #[test]
    fn alpha_fading_after_visible_period() {
        let mut s = ScrollState::new();
        let start = Instant::now();
        s.last_scroll_time = Some(start);
        // At 1.65s: 0.15s into the 0.3s fade (50%)
        let at = start + Duration::from_secs_f32(1.65);
        let alpha = s.scrollbar_alpha_at(at, 1.5, 0.3, 0.3);
        assert!(alpha > 0.0);
        assert!(alpha < 0.3);
    }

    #[test]
    fn alpha_zero_after_full_fade() {
        let mut s = ScrollState::new();
        let start = Instant::now();
        s.last_scroll_time = Some(start);
        let at = start + Duration::from_secs_f32(2.0); // well past 1.5 + 0.3
        let alpha = s.scrollbar_alpha_at(at, 1.5, 0.3, 0.3);
        assert_eq!(alpha, 0.0);
    }

    #[test]
    fn scrollbar_needs_redraw_tracks_visibility() {
        let mut s = ScrollState::new();
        assert!(!s.scrollbar_needs_redraw(Instant::now()));
        s.touch();
        assert!(s.scrollbar_needs_redraw(Instant::now()));
    }

    // ── Scrollbar geometry ──────────────────────────────────────────

    #[test]
    fn thumb_none_when_no_history() {
        let result = scrollbar_thumb_rect(0.0, 0.0, 800.0, 600.0, [10.0, 10.0, 10.0, 10.0], 24, 0, 0);
        assert!(result.is_none());
    }

    #[test]
    fn thumb_some_when_history_exists() {
        let result = scrollbar_thumb_rect(0.0, 0.0, 800.0, 600.0, [10.0, 10.0, 10.0, 10.0], 24, 100, 0);
        assert!(result.is_some());
    }

    #[test]
    fn thumb_at_bottom_when_offset_zero() {
        // offset 0 = at bottom of content
        let thumb = scrollbar_thumb_rect(0.0, 0.0, 800.0, 600.0, [10.0, 10.0, 10.0, 10.0], 24, 100, 0).unwrap();
        let track_bottom = 600.0 - 10.0; // pane_h - pad_bottom
        // Thumb bottom edge should be at track bottom
        assert!((thumb.y + thumb.height - track_bottom).abs() < 0.5);
    }

    #[test]
    fn thumb_at_top_when_offset_max() {
        // offset = history_size = fully scrolled up
        let thumb = scrollbar_thumb_rect(0.0, 0.0, 800.0, 600.0, [10.0, 10.0, 10.0, 10.0], 24, 100, 100).unwrap();
        let track_top = 10.0; // pad_top
        assert!((thumb.y - track_top).abs() < 0.5);
    }

    #[test]
    fn thumb_width_is_6px() {
        let thumb = scrollbar_thumb_rect(0.0, 0.0, 800.0, 600.0, [10.0, 10.0, 10.0, 10.0], 24, 100, 0).unwrap();
        assert_eq!(thumb.width, 6.0);
    }

    #[test]
    fn thumb_positioned_at_right_edge_inside_padding() {
        let thumb = scrollbar_thumb_rect(0.0, 0.0, 800.0, 600.0, [10.0, 10.0, 10.0, 10.0], 24, 100, 0).unwrap();
        // x = pane_x + pane_w - pad_right - scrollbar_width = 0 + 800 - 10 - 6 = 784
        assert_eq!(thumb.x, 784.0);
    }

    #[test]
    fn thumb_height_respects_minimum() {
        // Very large history relative to visible rows => tiny ratio, but min 20px
        let thumb = scrollbar_thumb_rect(0.0, 0.0, 800.0, 600.0, [10.0, 10.0, 10.0, 10.0], 24, 10000, 0).unwrap();
        assert!(thumb.height >= 20.0);
    }

    #[test]
    fn thumb_height_proportional_to_content() {
        // 24 visible rows + 24 history = 50% ratio
        let thumb = scrollbar_thumb_rect(0.0, 0.0, 800.0, 600.0, [10.0, 10.0, 10.0, 10.0], 24, 24, 0).unwrap();
        let track_height = 600.0 - 10.0 - 10.0; // 580
        let expected = track_height * 0.5; // 290
        assert!((thumb.height - expected).abs() < 1.0);
    }

    #[test]
    fn thumb_with_pane_offset() {
        // Pane at x=100, y=50
        let thumb = scrollbar_thumb_rect(100.0, 50.0, 400.0, 300.0, [5.0, 5.0, 5.0, 5.0], 24, 100, 0).unwrap();
        // x = 100 + 400 - 5 - 6 = 489
        assert_eq!(thumb.x, 489.0);
        // Track top = 50 + 5 = 55, track bottom = 50 + 300 - 5 = 345
        assert!(thumb.y >= 55.0);
        assert!(thumb.y + thumb.height <= 345.5);
    }

    // ── Hit testing ─────────────────────────────────────────────────

    #[test]
    fn hit_test_none_outside_scrollbar() {
        let hit = scrollbar_hit_test(
            100.0, 300.0, // click in middle of pane
            0.0, 0.0, 800.0, 600.0,
            [10.0, 10.0, 10.0, 10.0],
            24, 100, 0,
        );
        assert_eq!(hit, ScrollbarHit::None);
    }

    #[test]
    fn hit_test_track_in_scrollbar_region() {
        // Click at x=785 (inside scrollbar column at 784..790), y=100 (in track area)
        let hit = scrollbar_hit_test(
            785.0, 100.0,
            0.0, 0.0, 800.0, 600.0,
            [10.0, 10.0, 10.0, 10.0],
            24, 100, 50,
        );
        // Should be Track or Thumb depending on thumb position
        assert!(matches!(hit, ScrollbarHit::Track(_) | ScrollbarHit::Thumb(_)));
    }

    #[test]
    fn hit_test_thumb_on_thumb_area() {
        // First get the thumb position
        let thumb = scrollbar_thumb_rect(
            0.0, 0.0, 800.0, 600.0,
            [10.0, 10.0, 10.0, 10.0],
            24, 100, 50,
        ).unwrap();
        // Click in the middle of the thumb
        let hit = scrollbar_hit_test(
            thumb.x + 3.0, thumb.y + thumb.height / 2.0,
            0.0, 0.0, 800.0, 600.0,
            [10.0, 10.0, 10.0, 10.0],
            24, 100, 50,
        );
        assert!(matches!(hit, ScrollbarHit::Thumb(_)));
    }

    #[test]
    fn hit_test_none_above_track() {
        let hit = scrollbar_hit_test(
            785.0, 5.0, // above top padding
            0.0, 0.0, 800.0, 600.0,
            [10.0, 10.0, 10.0, 10.0],
            24, 100, 0,
        );
        assert_eq!(hit, ScrollbarHit::None);
    }

    // ── Track click to offset ───────────────────────────────────────

    #[test]
    fn track_click_top_gives_max_offset() {
        // Click at very top of track
        let offset = track_click_to_offset(10.0, 0.0, 600.0, [10.0, 10.0, 10.0, 10.0], 100);
        assert_eq!(offset, 100);
    }

    #[test]
    fn track_click_bottom_gives_zero_offset() {
        // Click at very bottom of track
        let offset = track_click_to_offset(589.0, 0.0, 600.0, [10.0, 10.0, 10.0, 10.0], 100);
        assert_eq!(offset, 0);
    }

    #[test]
    fn track_click_middle_gives_half_offset() {
        // Click at middle of track
        let track_top = 10.0;
        let track_bottom = 590.0;
        let mid = (track_top + track_bottom) / 2.0;
        let offset = track_click_to_offset(mid, 0.0, 600.0, [10.0, 10.0, 10.0, 10.0], 100);
        assert_eq!(offset, 50);
    }

    #[test]
    fn track_click_clamps_below_track() {
        let offset = track_click_to_offset(700.0, 0.0, 600.0, [10.0, 10.0, 10.0, 10.0], 100);
        assert_eq!(offset, 0);
    }

    // ── Drag ────────────────────────────────────────────────────────

    #[test]
    fn drag_begin_sets_state() {
        let mut s = ScrollState::new();
        s.set_immediate(50, 100);
        s.begin_drag(200.0);
        assert!(s.is_dragging_scrollbar);
        assert_eq!(s.drag_start_y, 200.0);
        assert_eq!(s.drag_start_offset, 50);
    }

    #[test]
    fn drag_up_increases_offset() {
        let mut s = ScrollState::new();
        s.set_immediate(50, 100);
        s.begin_drag(200.0);
        // Drag up by 100px with a 500px track, history_size 100
        // delta_y = 100 - 200 = -100, offset_delta = -(-100/500)*100 = +20
        s.update_drag(100.0, 500.0, 100);
        assert_eq!(s.target_offset(), 70);
    }

    #[test]
    fn drag_down_decreases_offset() {
        let mut s = ScrollState::new();
        s.set_immediate(50, 100);
        s.begin_drag(200.0);
        // Drag down by 100px
        s.update_drag(300.0, 500.0, 100);
        assert_eq!(s.target_offset(), 30);
    }

    #[test]
    fn drag_clamps_to_zero() {
        let mut s = ScrollState::new();
        s.set_immediate(10, 100);
        s.begin_drag(200.0);
        // Drag way down
        s.update_drag(800.0, 500.0, 100);
        assert_eq!(s.target_offset(), 0);
    }

    #[test]
    fn drag_clamps_to_history() {
        let mut s = ScrollState::new();
        s.set_immediate(90, 100);
        s.begin_drag(200.0);
        // Drag way up
        s.update_drag(-500.0, 500.0, 100);
        assert_eq!(s.target_offset(), 100);
    }

    #[test]
    fn drag_end_clears_state() {
        let mut s = ScrollState::new();
        s.begin_drag(200.0);
        s.end_drag();
        assert!(!s.is_dragging_scrollbar);
    }
}

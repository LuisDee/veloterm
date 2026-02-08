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
}

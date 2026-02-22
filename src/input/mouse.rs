// Mouse selection state: click counting, drag tracking, and selection management.

use std::time::Instant;

use crate::input::selection::{
    find_word_boundaries, pixel_to_cell, Selection, SelectionType,
};
use crate::renderer::grid_renderer::GridCell;

/// Maximum time between clicks to count as multi-click (double/triple), in milliseconds.
const MULTI_CLICK_THRESHOLD_MS: u64 = 300;

/// Maximum pixel distance between clicks to count as multi-click.
const MULTI_CLICK_DISTANCE: f32 = 5.0;

/// Minimum pixel movement before a press becomes a drag selection.
const DRAG_THRESHOLD: f32 = 3.0;

/// Selection drag state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragPhase {
    /// No mouse button held.
    Idle,
    /// Mouse pressed but not yet moved beyond threshold — not a selection yet.
    Pending,
    /// Mouse moved beyond threshold — active drag selection in progress.
    Active,
}

/// Mouse selection state for a single pane.
#[derive(Debug)]
pub struct MouseSelectionState {
    /// Current click count (1=single, 2=double, 3=triple).
    pub click_count: u8,
    /// Time of the last mouse press.
    last_click_time: Option<Instant>,
    /// Position of the last mouse press in pixels.
    last_click_pos: (f32, f32),
    /// Active text selection, if any.
    pub active_selection: Option<Selection>,
    /// Drag state machine phase.
    pub drag_phase: DragPhase,
    /// Pixel position where the current press started (for threshold check).
    press_origin_px: (f32, f32),
    /// Cell coordinates of the drag origin (absolute_row, col).
    drag_origin: (i32, usize),
    /// For word/line drag: the original word/line boundaries at the click origin.
    drag_anchor: Option<(i32, usize, i32, usize)>, // (start_abs_row, start_col, end_abs_row, end_col)
    /// Set to true when the next click should be swallowed (focus change).
    pub swallow_next_click: bool,
    /// Last known mouse position for auto-scroll + drag endpoint updates.
    pub last_drag_pos: (f32, f32),
    /// Auto-scroll speed: positive = scroll into history, negative = scroll to live.
    pub auto_scroll_speed: f32,
}

/// Backwards-compatible alias.
impl MouseSelectionState {
    /// Whether a drag is actively selecting (phase == Active).
    pub fn is_dragging(&self) -> bool {
        self.drag_phase == DragPhase::Active
    }
}

impl MouseSelectionState {
    pub fn new() -> Self {
        Self {
            click_count: 0,
            last_click_time: None,
            last_click_pos: (0.0, 0.0),
            active_selection: None,
            drag_phase: DragPhase::Idle,
            press_origin_px: (0.0, 0.0),
            drag_origin: (0, 0),
            drag_anchor: None,
            swallow_next_click: false,
            last_drag_pos: (0.0, 0.0),
            auto_scroll_speed: 0.0,
        }
    }

    /// Mark that the next mouse press should be swallowed (e.g. focus change).
    pub fn swallow_next(&mut self) {
        self.swallow_next_click = true;
    }

    /// Reset all selection/drag state (e.g. on pane/window focus change).
    pub fn reset_on_focus_change(&mut self) {
        self.drag_phase = DragPhase::Idle;
        self.drag_anchor = None;
        self.swallow_next_click = true;
        self.auto_scroll_speed = 0.0;
    }

    /// Handle a mouse press event. Returns the click count (1, 2, or 3).
    ///
    /// `pixel_x`, `pixel_y`: mouse position relative to the pane's content area
    /// (already adjusted for padding and tab bar).
    /// `display_offset`: current scroll offset (0 = at bottom, N = N lines into history).
    pub fn on_mouse_press(
        &mut self,
        pixel_x: f32,
        pixel_y: f32,
        cell_width: f32,
        cell_height: f32,
        cols: usize,
        rows: usize,
        cells: &[GridCell],
        display_offset: usize,
    ) -> u8 {
        let now = Instant::now();

        // Determine click count
        let is_multi = if let Some(last_time) = self.last_click_time {
            let elapsed = now.duration_since(last_time).as_millis() as u64;
            let dx = (pixel_x - self.last_click_pos.0).abs();
            let dy = (pixel_y - self.last_click_pos.1).abs();
            elapsed <= MULTI_CLICK_THRESHOLD_MS
                && dx <= MULTI_CLICK_DISTANCE
                && dy <= MULTI_CLICK_DISTANCE
        } else {
            false
        };

        if is_multi && self.click_count < 3 {
            self.click_count += 1;
        } else {
            self.click_count = 1;
        }

        self.last_click_time = Some(now);
        self.last_click_pos = (pixel_x, pixel_y);
        self.press_origin_px = (pixel_x, pixel_y);

        // Swallow focus clicks
        if self.swallow_next_click {
            self.swallow_next_click = false;
            self.drag_phase = DragPhase::Idle;
            self.active_selection = None;
            return self.click_count;
        }

        let (vp_row, col) = pixel_to_cell(
            pixel_x as f64,
            pixel_y as f64,
            cell_width,
            cell_height,
            cols,
            rows,
        );
        let abs_row = vp_row as i32 - display_offset as i32;

        self.drag_origin = (abs_row, col);

        match self.click_count {
            1 => {
                // Single click: enter Pending — don't create selection yet.
                // Selection is only created when drag threshold is exceeded.
                self.drag_phase = DragPhase::Pending;
                self.active_selection = None;
                self.drag_anchor = None;
            }
            2 => {
                // Double click: immediate word selection (no threshold needed)
                self.drag_phase = DragPhase::Active;
                let (word_start, word_end) = find_word_boundaries(cells, vp_row, col, cols);
                self.active_selection = Some(Selection {
                    start: (abs_row, word_start),
                    end: (abs_row, word_end),
                    selection_type: SelectionType::Word,
                });
                self.drag_anchor = Some((abs_row, word_start, abs_row, word_end));
            }
            3 => {
                // Triple click: immediate line selection (no threshold needed)
                self.drag_phase = DragPhase::Active;
                self.active_selection = Some(Selection {
                    start: (abs_row, 0),
                    end: (abs_row, cols.saturating_sub(1)),
                    selection_type: SelectionType::Line,
                });
                self.drag_anchor = Some((abs_row, 0, abs_row, cols.saturating_sub(1)));
            }
            _ => {}
        }

        self.click_count
    }

    /// Handle mouse drag (cursor moved while button held).
    /// `display_offset`: current scroll offset.
    pub fn on_mouse_drag(
        &mut self,
        pixel_x: f32,
        pixel_y: f32,
        cell_width: f32,
        cell_height: f32,
        cols: usize,
        rows: usize,
        cells: &[GridCell],
        display_offset: usize,
    ) {
        self.last_drag_pos = (pixel_x, pixel_y);

        // Compute auto-scroll speed based on how far outside the viewport the mouse is
        let viewport_height = rows as f32 * cell_height;
        if pixel_y < 0.0 {
            // Above viewport — scroll into history
            self.auto_scroll_speed = ((-pixel_y) / 50.0).min(5.0).max(0.5);
        } else if pixel_y > viewport_height {
            // Below viewport — scroll toward live
            self.auto_scroll_speed = -((pixel_y - viewport_height) / 50.0).min(5.0).max(0.5);
        } else {
            self.auto_scroll_speed = 0.0;
        }

        match self.drag_phase {
            DragPhase::Idle => return,
            DragPhase::Pending => {
                // Check if movement exceeds drag threshold
                let dx = (pixel_x - self.press_origin_px.0).abs();
                let dy = (pixel_y - self.press_origin_px.1).abs();
                if dx < DRAG_THRESHOLD && dy < DRAG_THRESHOLD {
                    return; // Not enough movement yet
                }
                // Threshold exceeded — transition to Active and create selection
                self.drag_phase = DragPhase::Active;
                let (origin_row, origin_col) = self.drag_origin;
                self.active_selection = Some(Selection {
                    start: (origin_row, origin_col),
                    end: (origin_row, origin_col),
                    selection_type: SelectionType::Range,
                });
            }
            DragPhase::Active => {} // Continue below
        }

        let (vp_row, col) = pixel_to_cell(
            pixel_x as f64,
            pixel_y.clamp(0.0, (rows as f32 * cell_height) - 1.0) as f64,
            cell_width,
            cell_height,
            cols,
            rows,
        );
        let abs_row = vp_row as i32 - display_offset as i32;

        if let Some(ref mut sel) = self.active_selection {
            match sel.selection_type {
                SelectionType::Range => {
                    sel.end = (abs_row, col);
                }
                SelectionType::Word => {
                    // Extend by word boundaries
                    let (word_start, word_end) = find_word_boundaries(cells, vp_row, col, cols);
                    if let Some((anchor_row, anchor_start, _, anchor_end)) = self.drag_anchor {
                        if abs_row < anchor_row || (abs_row == anchor_row && col < anchor_start) {
                            sel.start = (abs_row, word_start);
                            sel.end = (anchor_row, anchor_end);
                        } else {
                            sel.start = (anchor_row, anchor_start);
                            sel.end = (abs_row, word_end);
                        }
                    }
                }
                SelectionType::Line => {
                    // Extend by full lines
                    if let Some((anchor_row, _, _, _)) = self.drag_anchor {
                        if abs_row < anchor_row {
                            sel.start = (abs_row, 0);
                            sel.end = (anchor_row, cols.saturating_sub(1));
                        } else {
                            sel.start = (anchor_row, 0);
                            sel.end = (abs_row, cols.saturating_sub(1));
                        }
                    }
                }
                SelectionType::VisualBlock => {} // Not handled here
            }
        }
    }

    /// Handle mouse release.
    pub fn on_mouse_release(&mut self) {
        if self.drag_phase == DragPhase::Pending {
            // Click-up while still Pending = just a click, not a drag.
            // Clear any selection (it was just a click to place cursor/focus).
            self.active_selection = None;
        }
        self.drag_phase = DragPhase::Idle;
        self.auto_scroll_speed = 0.0;
    }

    /// Handle shift+click to extend selection.
    /// `display_offset`: current scroll offset.
    pub fn on_shift_click(
        &mut self,
        pixel_x: f32,
        pixel_y: f32,
        cell_width: f32,
        cell_height: f32,
        cols: usize,
        rows: usize,
        cursor_row: usize,
        cursor_col: usize,
        display_offset: usize,
    ) {
        let (vp_row, col) = pixel_to_cell(
            pixel_x as f64,
            pixel_y as f64,
            cell_width,
            cell_height,
            cols,
            rows,
        );
        let abs_row = vp_row as i32 - display_offset as i32;
        let cursor_abs_row = cursor_row as i32 - display_offset as i32;

        if let Some(ref mut sel) = self.active_selection {
            // Extend existing selection
            sel.end = (abs_row, col);
        } else {
            // Create new selection from cursor to click
            self.active_selection = Some(Selection {
                start: (cursor_abs_row, cursor_col),
                end: (abs_row, col),
                selection_type: SelectionType::Range,
            });
        }
    }

    /// Update the drag endpoint at a given pixel position and display_offset.
    /// Used when auto-scrolling or manual scrolling during an active drag.
    pub fn update_drag_endpoint(
        &mut self,
        pixel_x: f32,
        pixel_y: f32,
        cell_width: f32,
        cell_height: f32,
        cols: usize,
        rows: usize,
        display_offset: usize,
    ) {
        let clamped_y = pixel_y.clamp(0.0, (rows as f32 * cell_height) - 1.0);
        let (vp_row, col) = pixel_to_cell(
            pixel_x as f64,
            clamped_y as f64,
            cell_width,
            cell_height,
            cols,
            rows,
        );
        let abs_row = vp_row as i32 - display_offset as i32;
        if let Some(ref mut sel) = self.active_selection {
            sel.end = (abs_row, col);
        }
    }

    /// Clear the active selection.
    pub fn clear_selection(&mut self) {
        self.active_selection = None;
    }

    /// Check if there is an active non-empty selection.
    pub fn has_selection(&self) -> bool {
        if let Some(ref sel) = self.active_selection {
            sel.start != sel.end
        } else {
            false
        }
    }

    /// Handle a mouse press with explicit timestamp (for testing).
    #[cfg(test)]
    pub fn on_mouse_press_at(
        &mut self,
        pixel_x: f32,
        pixel_y: f32,
        cell_width: f32,
        cell_height: f32,
        cols: usize,
        rows: usize,
        cells: &[GridCell],
        at: Instant,
        display_offset: usize,
    ) -> u8 {
        // Same logic as on_mouse_press but with explicit time
        let is_multi = if let Some(last_time) = self.last_click_time {
            let elapsed = at.duration_since(last_time).as_millis() as u64;
            let dx = (pixel_x - self.last_click_pos.0).abs();
            let dy = (pixel_y - self.last_click_pos.1).abs();
            elapsed <= MULTI_CLICK_THRESHOLD_MS
                && dx <= MULTI_CLICK_DISTANCE
                && dy <= MULTI_CLICK_DISTANCE
        } else {
            false
        };

        if is_multi && self.click_count < 3 {
            self.click_count += 1;
        } else {
            self.click_count = 1;
        }

        self.last_click_time = Some(at);
        self.last_click_pos = (pixel_x, pixel_y);
        self.press_origin_px = (pixel_x, pixel_y);

        if self.swallow_next_click {
            self.swallow_next_click = false;
            self.drag_phase = DragPhase::Idle;
            self.active_selection = None;
            return self.click_count;
        }

        let (vp_row, col) = pixel_to_cell(
            pixel_x as f64,
            pixel_y as f64,
            cell_width,
            cell_height,
            cols,
            rows,
        );
        let abs_row = vp_row as i32 - display_offset as i32;

        self.drag_origin = (abs_row, col);

        match self.click_count {
            1 => {
                self.drag_phase = DragPhase::Pending;
                self.active_selection = None;
                self.drag_anchor = None;
            }
            2 => {
                self.drag_phase = DragPhase::Active;
                let (word_start, word_end) = find_word_boundaries(cells, vp_row, col, cols);
                self.active_selection = Some(Selection {
                    start: (abs_row, word_start),
                    end: (abs_row, word_end),
                    selection_type: SelectionType::Word,
                });
                self.drag_anchor = Some((abs_row, word_start, abs_row, word_end));
            }
            3 => {
                self.drag_phase = DragPhase::Active;
                self.active_selection = Some(Selection {
                    start: (abs_row, 0),
                    end: (abs_row, cols.saturating_sub(1)),
                    selection_type: SelectionType::Line,
                });
                self.drag_anchor = Some((abs_row, 0, abs_row, cols.saturating_sub(1)));
            }
            _ => {}
        }

        self.click_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::theme::{Color, color_new};
    use std::time::Duration;

    fn make_cells(text: &str, cols: usize) -> Vec<GridCell> {
        let fg = color_new(1.0, 1.0, 1.0, 1.0);
        let bg = color_new(0.0, 0.0, 0.0, 1.0);
        let mut cells = Vec::with_capacity(cols);
        for ch in text.chars().take(cols) {
            cells.push(GridCell::new(ch, fg, bg));
        }
        while cells.len() < cols {
            cells.push(GridCell::new(' ', fg, bg));
        }
        cells
    }

    fn make_grid(lines: &[&str], cols: usize) -> Vec<GridCell> {
        let mut cells = Vec::new();
        for line in lines {
            cells.extend(make_cells(line, cols));
        }
        cells
    }

    // ── Click count detection ─────────────────────────────────────

    #[test]
    fn single_click_returns_count_1() {
        let mut state = MouseSelectionState::new();
        let cells = make_cells("hello world", 20);
        let count = state.on_mouse_press(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, 0);
        assert_eq!(count, 1);
    }

    #[test]
    fn double_click_within_threshold_returns_count_2() {
        let mut state = MouseSelectionState::new();
        let cells = make_cells("hello world", 20);
        let t0 = Instant::now();
        let t1 = t0 + Duration::from_millis(100);
        state.on_mouse_press_at(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, t0, 0);
        let count = state.on_mouse_press_at(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, t1, 0);
        assert_eq!(count, 2);
    }

    #[test]
    fn triple_click_within_threshold_returns_count_3() {
        let mut state = MouseSelectionState::new();
        let cells = make_cells("hello world", 20);
        let t0 = Instant::now();
        let t1 = t0 + Duration::from_millis(100);
        let t2 = t0 + Duration::from_millis(200);
        state.on_mouse_press_at(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, t0, 0);
        state.on_mouse_press_at(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, t1, 0);
        let count = state.on_mouse_press_at(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, t2, 0);
        assert_eq!(count, 3);
    }

    #[test]
    fn click_after_timeout_resets_to_1() {
        let mut state = MouseSelectionState::new();
        let cells = make_cells("hello world", 20);
        let t0 = Instant::now();
        let t1 = t0 + Duration::from_millis(500); // > 300ms threshold
        state.on_mouse_press_at(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, t0, 0);
        let count = state.on_mouse_press_at(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, t1, 0);
        assert_eq!(count, 1);
    }

    #[test]
    fn click_at_different_position_resets_to_1() {
        let mut state = MouseSelectionState::new();
        let cells = make_cells("hello world", 20);
        let t0 = Instant::now();
        let t1 = t0 + Duration::from_millis(100);
        state.on_mouse_press_at(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, t0, 0);
        let count = state.on_mouse_press_at(100.0, 5.0, 10.0, 20.0, 20, 1, &cells, t1, 0);
        assert_eq!(count, 1);
    }

    #[test]
    fn fourth_click_resets_to_1() {
        let mut state = MouseSelectionState::new();
        let cells = make_cells("hello world", 20);
        let t0 = Instant::now();
        state.on_mouse_press_at(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, t0, 0);
        state.on_mouse_press_at(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, t0 + Duration::from_millis(50), 0);
        state.on_mouse_press_at(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, t0 + Duration::from_millis(100), 0);
        let count = state.on_mouse_press_at(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, t0 + Duration::from_millis(150), 0);
        assert_eq!(count, 1);
    }

    // ── Single click selection (drag threshold) ──────────────────

    #[test]
    fn single_click_enters_pending_no_selection() {
        let mut state = MouseSelectionState::new();
        let cells = make_cells("hello world", 20);
        state.on_mouse_press(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, 0);
        assert_eq!(state.drag_phase, DragPhase::Pending);
        assert!(state.active_selection.is_none(), "no selection until drag threshold");
    }

    #[test]
    fn drag_below_threshold_stays_pending() {
        let mut state = MouseSelectionState::new();
        let cells = make_cells("hello world", 20);
        state.on_mouse_press(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, 0);
        // Move only 2px — below DRAG_THRESHOLD of 3px
        state.on_mouse_drag(17.0, 5.0, 10.0, 20.0, 20, 1, &cells, 0);
        assert_eq!(state.drag_phase, DragPhase::Pending);
        assert!(state.active_selection.is_none());
    }

    #[test]
    fn drag_beyond_threshold_creates_selection() {
        let mut state = MouseSelectionState::new();
        let cells = make_cells("hello world", 20);
        state.on_mouse_press(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, 0);
        // Move 40px — well beyond threshold
        state.on_mouse_drag(55.0, 5.0, 10.0, 20.0, 20, 1, &cells, 0);
        assert_eq!(state.drag_phase, DragPhase::Active);
        let sel = state.active_selection.as_ref().unwrap();
        assert_eq!(sel.selection_type, SelectionType::Range);
        assert_eq!(sel.end, (0, 5));
    }

    #[test]
    fn release_without_movement_clears_selection() {
        let mut state = MouseSelectionState::new();
        let cells = make_cells("hello world", 20);
        state.on_mouse_press(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, 0);
        state.on_mouse_release();
        assert_eq!(state.drag_phase, DragPhase::Idle);
        assert!(state.active_selection.is_none());
    }

    #[test]
    fn release_after_drag_keeps_selection() {
        let mut state = MouseSelectionState::new();
        let cells = make_cells("hello world", 20);
        state.on_mouse_press(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, 0);
        state.on_mouse_drag(55.0, 5.0, 10.0, 20.0, 20, 1, &cells, 0);
        state.on_mouse_release();
        assert_eq!(state.drag_phase, DragPhase::Idle);
        assert!(state.active_selection.is_some());
    }

    // ── Focus swallow ─────────────────────────────────────────────

    #[test]
    fn swallow_next_click_prevents_selection() {
        let mut state = MouseSelectionState::new();
        let cells = make_cells("hello world", 20);
        state.swallow_next();
        state.on_mouse_press(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, 0);
        assert_eq!(state.drag_phase, DragPhase::Idle);
        assert!(state.active_selection.is_none());
        // Second click at different position to avoid multi-click detection
        state.on_mouse_press(75.0, 5.0, 10.0, 20.0, 20, 1, &cells, 0);
        assert_eq!(state.drag_phase, DragPhase::Pending);
    }

    #[test]
    fn reset_on_focus_change_clears_and_swallows() {
        let mut state = MouseSelectionState::new();
        let cells = make_cells("hello world", 20);
        state.on_mouse_press(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, 0);
        state.on_mouse_drag(55.0, 5.0, 10.0, 20.0, 20, 1, &cells, 0);
        assert!(state.is_dragging());
        state.reset_on_focus_change();
        assert_eq!(state.drag_phase, DragPhase::Idle);
        assert!(state.swallow_next_click);
    }

    // ── Double-click word selection ───────────────────────────────

    #[test]
    fn double_click_selects_word() {
        let mut state = MouseSelectionState::new();
        let cells = make_cells("hello world", 20);
        let t0 = Instant::now();
        state.on_mouse_press_at(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, t0, 0);
        state.on_mouse_release();
        state.on_mouse_press_at(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, t0 + Duration::from_millis(100), 0);
        let sel = state.active_selection.as_ref().unwrap();
        assert_eq!(sel.selection_type, SelectionType::Word);
        // "hello" spans cols 0-4, click at col 1
        assert_eq!(sel.start, (0, 0));
        assert_eq!(sel.end, (0, 4));
    }

    // ── Triple-click line selection ───────────────────────────────

    #[test]
    fn triple_click_selects_line() {
        let mut state = MouseSelectionState::new();
        let cells = make_cells("hello world", 20);
        let t0 = Instant::now();
        state.on_mouse_press_at(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, t0, 0);
        state.on_mouse_release();
        state.on_mouse_press_at(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, t0 + Duration::from_millis(100), 0);
        state.on_mouse_release();
        state.on_mouse_press_at(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, t0 + Duration::from_millis(200), 0);
        let sel = state.active_selection.as_ref().unwrap();
        assert_eq!(sel.selection_type, SelectionType::Line);
        assert_eq!(sel.start, (0, 0));
        assert_eq!(sel.end, (0, 19));
    }

    // ── Word drag extension ──────────────────────────────────────

    #[test]
    fn double_click_drag_extends_by_word() {
        let mut state = MouseSelectionState::new();
        let cells = make_cells("hello world test", 20);
        let t0 = Instant::now();
        // Double click on "hello" (col 1)
        state.on_mouse_press_at(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, t0, 0);
        state.on_mouse_release();
        state.on_mouse_press_at(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, t0 + Duration::from_millis(100), 0);
        // Drag to "world" (col 7, pixel 75)
        state.on_mouse_drag(75.0, 5.0, 10.0, 20.0, 20, 1, &cells, 0);
        let sel = state.active_selection.as_ref().unwrap();
        // Should extend from "hello" start (0) to "world" end (10)
        assert_eq!(sel.start, (0, 0));
        assert_eq!(sel.end, (0, 10));
    }

    // ── Line drag extension ──────────────────────────────────────

    #[test]
    fn triple_click_drag_extends_by_line() {
        let mut state = MouseSelectionState::new();
        let cells = make_grid(&["hello world", "second line"], 20);
        let t0 = Instant::now();
        // Triple click on row 0
        state.on_mouse_press_at(15.0, 5.0, 10.0, 20.0, 20, 2, &cells, t0, 0);
        state.on_mouse_release();
        state.on_mouse_press_at(15.0, 5.0, 10.0, 20.0, 20, 2, &cells, t0 + Duration::from_millis(100), 0);
        state.on_mouse_release();
        state.on_mouse_press_at(15.0, 5.0, 10.0, 20.0, 20, 2, &cells, t0 + Duration::from_millis(200), 0);
        // Drag to row 1
        state.on_mouse_drag(15.0, 25.0, 10.0, 20.0, 20, 2, &cells, 0);
        let sel = state.active_selection.as_ref().unwrap();
        assert_eq!(sel.start, (0, 0));
        assert_eq!(sel.end, (1, 19));
    }

    // ── Shift+click ──────────────────────────────────────────────

    #[test]
    fn shift_click_extends_existing_selection() {
        let mut state = MouseSelectionState::new();
        let cells = make_cells("hello world", 20);
        state.on_mouse_press(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, 0);
        state.on_mouse_drag(35.0, 5.0, 10.0, 20.0, 20, 1, &cells, 0);
        state.on_mouse_release();
        // Shift+click at col 8
        state.on_shift_click(85.0, 5.0, 10.0, 20.0, 20, 1, 0, 0, 0);
        let sel = state.active_selection.as_ref().unwrap();
        assert_eq!(sel.end, (0, 8));
    }

    #[test]
    fn shift_click_creates_selection_from_cursor_when_none() {
        let mut state = MouseSelectionState::new();
        // No existing selection, cursor at (0, 2)
        state.on_shift_click(85.0, 5.0, 10.0, 20.0, 20, 1, 0, 2, 0);
        let sel = state.active_selection.as_ref().unwrap();
        assert_eq!(sel.start, (0, 2));
        assert_eq!(sel.end, (0, 8));
        assert_eq!(sel.selection_type, SelectionType::Range);
    }

    // ── has_selection ─────────────────────────────────────────────

    #[test]
    fn has_selection_false_when_none() {
        let state = MouseSelectionState::new();
        assert!(!state.has_selection());
    }

    #[test]
    fn has_selection_true_after_drag() {
        let mut state = MouseSelectionState::new();
        let cells = make_cells("hello world", 20);
        state.on_mouse_press(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, 0);
        state.on_mouse_drag(55.0, 5.0, 10.0, 20.0, 20, 1, &cells, 0);
        state.on_mouse_release();
        assert!(state.has_selection());
    }

    // ── clear_selection ──────────────────────────────────────────

    #[test]
    fn clear_selection_removes_active() {
        let mut state = MouseSelectionState::new();
        let cells = make_cells("hello world", 20);
        state.on_mouse_press(15.0, 5.0, 10.0, 20.0, 20, 1, &cells, 0);
        state.on_mouse_drag(55.0, 5.0, 10.0, 20.0, 20, 1, &cells, 0);
        state.on_mouse_release();
        assert!(state.has_selection());
        state.clear_selection();
        assert!(!state.has_selection());
    }

    // ── Auto-scroll speed ───────────────────────────────────────

    #[test]
    fn auto_scroll_above_viewport() {
        let mut state = MouseSelectionState::new();
        let cells = make_cells("hello world", 20);
        state.on_mouse_press(15.0, 5.0, 10.0, 20.0, 20, 3, &cells, 0);
        // Drag above viewport
        state.on_mouse_drag(15.0, -50.0, 10.0, 20.0, 20, 3, &cells, 0);
        assert!(state.auto_scroll_speed > 0.0, "should scroll into history");
    }

    #[test]
    fn auto_scroll_below_viewport() {
        let mut state = MouseSelectionState::new();
        let cells = make_cells("hello world", 20);
        state.on_mouse_press(15.0, 5.0, 10.0, 20.0, 20, 3, &cells, 0);
        // Drag below viewport (3 rows * 20px = 60px viewport height)
        state.on_mouse_drag(15.0, 100.0, 10.0, 20.0, 20, 3, &cells, 0);
        assert!(state.auto_scroll_speed < 0.0, "should scroll toward live");
    }

    #[test]
    fn auto_scroll_inside_viewport_is_zero() {
        let mut state = MouseSelectionState::new();
        let cells = make_cells("hello world", 20);
        state.on_mouse_press(15.0, 5.0, 10.0, 20.0, 20, 3, &cells, 0);
        state.on_mouse_drag(15.0, 30.0, 10.0, 20.0, 20, 3, &cells, 0);
        assert_eq!(state.auto_scroll_speed, 0.0);
    }

    #[test]
    fn auto_scroll_resets_on_release() {
        let mut state = MouseSelectionState::new();
        let cells = make_cells("hello world", 20);
        state.on_mouse_press(15.0, 5.0, 10.0, 20.0, 20, 3, &cells, 0);
        state.on_mouse_drag(15.0, -50.0, 10.0, 20.0, 20, 3, &cells, 0);
        assert!(state.auto_scroll_speed != 0.0);
        state.on_mouse_release();
        assert_eq!(state.auto_scroll_speed, 0.0);
    }

    // ── update_drag_endpoint ────────────────────────────────────

    #[test]
    fn update_drag_endpoint_changes_selection_end() {
        let mut state = MouseSelectionState::new();
        state.active_selection = Some(Selection {
            start: (0, 0),
            end: (0, 5),
            selection_type: SelectionType::Range,
        });
        // Update endpoint to viewport row 2, col 3 with display_offset=1
        // abs_row = 2 - 1 = 1
        state.update_drag_endpoint(35.0, 45.0, 10.0, 20.0, 20, 3, 1);
        let sel = state.active_selection.as_ref().unwrap();
        assert_eq!(sel.end, (1, 3));
    }
}

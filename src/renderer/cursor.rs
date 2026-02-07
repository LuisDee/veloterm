// Cursor rendering: generates cursor overlay cell instances for the GPU.

use crate::config::theme::Color;
use crate::renderer::gpu::CellInstance;
use std::time::{Duration, Instant};

/// Cursor shape styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorStyle {
    Block,
    Beam,
    Underline,
}

/// Flags for cursor rendering in CellInstance.flags field.
/// Bit 0: has_glyph (existing)
/// Bit 1: is_cursor
/// Bits 2-3: cursor shape (00=block, 01=beam, 10=underline, 11=hollow_block)
pub const FLAG_CURSOR: u32 = 0x02;
pub const FLAG_CURSOR_BLOCK: u32 = 0x00 << 2;
pub const FLAG_CURSOR_BEAM: u32 = 0x01 << 2;
pub const FLAG_CURSOR_UNDERLINE: u32 = 0x02 << 2;
pub const FLAG_CURSOR_HOLLOW: u32 = 0x03 << 2;

/// Default cursor blink interval.
pub const BLINK_INTERVAL: Duration = Duration::from_millis(500);

/// Cursor foreground color (text under block cursor — dark on bright).
pub const CURSOR_FG: Color = Color::new(0.1020, 0.0941, 0.0863, 1.0); // same as bg

/// Cursor background color (cursor block color — bright).
pub const CURSOR_BG: Color = Color::new(0.9098, 0.8980, 0.8745, 1.0); // same as fg

/// Manages cursor state and generates cursor overlay instances.
pub struct CursorState {
    pub row: usize,
    pub col: usize,
    pub style: CursorStyle,
    pub visible: bool,
    pub focused: bool,
    blink_visible: bool,
    last_blink: Instant,
}

impl Default for CursorState {
    fn default() -> Self {
        Self::new()
    }
}

impl CursorState {
    /// Create a new cursor at the origin.
    pub fn new() -> Self {
        Self {
            row: 0,
            col: 0,
            style: CursorStyle::Block,
            visible: true,
            focused: true,
            blink_visible: true,
            last_blink: Instant::now(),
        }
    }

    /// Update cursor position from terminal state.
    pub fn update_position(&mut self, row: usize, col: usize) {
        self.row = row;
        self.col = col;
    }

    /// Set the cursor style.
    pub fn set_style(&mut self, style: CursorStyle) {
        self.style = style;
    }

    /// Set window focus state. Unfocused window shows hollow block cursor.
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Advance the blink timer. Returns true if blink state changed.
    pub fn tick_blink(&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since(self.last_blink) >= BLINK_INTERVAL {
            self.blink_visible = !self.blink_visible;
            self.last_blink = now;
            true
        } else {
            false
        }
    }

    /// Whether the cursor should be rendered this frame.
    pub fn should_render(&self) -> bool {
        self.visible && self.blink_visible
    }

    /// Generate the cursor CellInstance for rendering.
    /// Returns None if the cursor should not be rendered.
    pub fn to_cell_instance(&self) -> Option<CellInstance> {
        if !self.should_render() {
            return None;
        }

        Some(CellInstance {
            position: [self.col as f32, self.row as f32],
            atlas_uv: [0.0, 0.0, 0.0, 0.0],
            fg_color: [CURSOR_FG.r, CURSOR_FG.g, CURSOR_FG.b, CURSOR_FG.a],
            bg_color: [CURSOR_BG.r, CURSOR_BG.g, CURSOR_BG.b, CURSOR_BG.a],
            flags: self.cursor_flags(),
            _padding: [0; 3],
        })
    }

    /// Compute the flags value for this cursor's style and focus state.
    fn cursor_flags(&self) -> u32 {
        let shape_flags = if !self.focused {
            FLAG_CURSOR_HOLLOW
        } else {
            match self.style {
                CursorStyle::Block => FLAG_CURSOR_BLOCK,
                CursorStyle::Beam => FLAG_CURSOR_BEAM,
                CursorStyle::Underline => FLAG_CURSOR_UNDERLINE,
            }
        };
        FLAG_CURSOR | shape_flags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Cursor position extraction ──────────────────────────────────

    #[test]
    fn cursor_starts_at_origin() {
        let cursor = CursorState::new();
        assert_eq!(cursor.row, 0);
        assert_eq!(cursor.col, 0);
    }

    #[test]
    fn cursor_update_position() {
        let mut cursor = CursorState::new();
        cursor.update_position(5, 10);
        assert_eq!(cursor.row, 5);
        assert_eq!(cursor.col, 10);
    }

    #[test]
    fn cursor_default_style_is_block() {
        let cursor = CursorState::new();
        assert_eq!(cursor.style, CursorStyle::Block);
    }

    #[test]
    fn cursor_default_visible() {
        let cursor = CursorState::new();
        assert!(cursor.visible);
    }

    #[test]
    fn cursor_default_focused() {
        let cursor = CursorState::new();
        assert!(cursor.focused);
    }

    // ── Block cursor cell instance ──────────────────────────────────

    #[test]
    fn block_cursor_generates_instance() {
        let cursor = CursorState::new();
        let instance = cursor.to_cell_instance();
        assert!(instance.is_some());
    }

    #[test]
    fn block_cursor_position_matches() {
        let mut cursor = CursorState::new();
        cursor.update_position(3, 7);
        let instance = cursor.to_cell_instance().unwrap();
        assert_eq!(instance.position, [7.0, 3.0]); // [col, row]
    }

    #[test]
    fn block_cursor_has_cursor_flag() {
        let cursor = CursorState::new();
        let instance = cursor.to_cell_instance().unwrap();
        assert_ne!(instance.flags & FLAG_CURSOR, 0);
    }

    #[test]
    fn block_cursor_flags_have_block_shape() {
        let cursor = CursorState::new();
        let instance = cursor.to_cell_instance().unwrap();
        let shape_bits = instance.flags & (0x03 << 2);
        assert_eq!(shape_bits, FLAG_CURSOR_BLOCK);
    }

    #[test]
    fn block_cursor_uses_cursor_colors() {
        let cursor = CursorState::new();
        let instance = cursor.to_cell_instance().unwrap();
        // Block cursor bg should be bright (CURSOR_BG)
        assert!((instance.bg_color[0] - CURSOR_BG.r).abs() < 0.01);
    }

    // ── Beam cursor cell instance ───────────────────────────────────

    #[test]
    fn beam_cursor_has_beam_shape_flag() {
        let mut cursor = CursorState::new();
        cursor.set_style(CursorStyle::Beam);
        let instance = cursor.to_cell_instance().unwrap();
        let shape_bits = instance.flags & (0x03 << 2);
        assert_eq!(shape_bits, FLAG_CURSOR_BEAM);
    }

    #[test]
    fn beam_cursor_has_cursor_flag() {
        let mut cursor = CursorState::new();
        cursor.set_style(CursorStyle::Beam);
        let instance = cursor.to_cell_instance().unwrap();
        assert_ne!(instance.flags & FLAG_CURSOR, 0);
    }

    // ── Underline cursor cell instance ──────────────────────────────

    #[test]
    fn underline_cursor_has_underline_shape_flag() {
        let mut cursor = CursorState::new();
        cursor.set_style(CursorStyle::Underline);
        let instance = cursor.to_cell_instance().unwrap();
        let shape_bits = instance.flags & (0x03 << 2);
        assert_eq!(shape_bits, FLAG_CURSOR_UNDERLINE);
    }

    #[test]
    fn underline_cursor_has_cursor_flag() {
        let mut cursor = CursorState::new();
        cursor.set_style(CursorStyle::Underline);
        let instance = cursor.to_cell_instance().unwrap();
        assert_ne!(instance.flags & FLAG_CURSOR, 0);
    }

    // ── Hollow block cursor (unfocused) ─────────────────────────────

    #[test]
    fn unfocused_cursor_shows_hollow_block() {
        let mut cursor = CursorState::new();
        cursor.set_focused(false);
        let instance = cursor.to_cell_instance().unwrap();
        let shape_bits = instance.flags & (0x03 << 2);
        assert_eq!(shape_bits, FLAG_CURSOR_HOLLOW);
    }

    #[test]
    fn unfocused_cursor_still_has_cursor_flag() {
        let mut cursor = CursorState::new();
        cursor.set_focused(false);
        let instance = cursor.to_cell_instance().unwrap();
        assert_ne!(instance.flags & FLAG_CURSOR, 0);
    }

    #[test]
    fn refocused_cursor_restores_original_style() {
        let mut cursor = CursorState::new();
        cursor.set_style(CursorStyle::Beam);
        cursor.set_focused(false);
        // Unfocused shows hollow
        let instance = cursor.to_cell_instance().unwrap();
        assert_eq!(instance.flags & (0x03 << 2), FLAG_CURSOR_HOLLOW);
        // Refocus restores beam
        cursor.set_focused(true);
        let instance = cursor.to_cell_instance().unwrap();
        assert_eq!(instance.flags & (0x03 << 2), FLAG_CURSOR_BEAM);
    }

    // ── Cursor blink timing ─────────────────────────────────────────

    #[test]
    fn cursor_visible_initially() {
        let cursor = CursorState::new();
        assert!(cursor.should_render());
    }

    #[test]
    fn cursor_not_visible_returns_none() {
        let mut cursor = CursorState::new();
        cursor.visible = false;
        assert_eq!(cursor.to_cell_instance(), None);
    }

    #[test]
    fn blink_toggles_visibility() {
        let mut cursor = CursorState::new();
        // Force blink interval to have elapsed
        cursor.last_blink = Instant::now() - BLINK_INTERVAL - Duration::from_millis(1);
        let changed = cursor.tick_blink();
        assert!(changed);
        assert!(!cursor.blink_visible); // Was true, now false
    }

    #[test]
    fn blink_no_change_before_interval() {
        let mut cursor = CursorState::new();
        // Just created, interval hasn't elapsed
        let changed = cursor.tick_blink();
        assert!(!changed);
        assert!(cursor.blink_visible); // Still true
    }

    #[test]
    fn blink_off_hides_cursor() {
        let mut cursor = CursorState::new();
        cursor.blink_visible = false;
        assert!(!cursor.should_render());
    }

    #[test]
    fn blink_on_shows_cursor() {
        let mut cursor = CursorState::new();
        cursor.blink_visible = true;
        assert!(cursor.should_render());
    }

    // ── Set style ───────────────────────────────────────────────────

    #[test]
    fn set_style_beam() {
        let mut cursor = CursorState::new();
        cursor.set_style(CursorStyle::Beam);
        assert_eq!(cursor.style, CursorStyle::Beam);
    }

    #[test]
    fn set_style_underline() {
        let mut cursor = CursorState::new();
        cursor.set_style(CursorStyle::Underline);
        assert_eq!(cursor.style, CursorStyle::Underline);
    }
}

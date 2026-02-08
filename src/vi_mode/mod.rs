// Vi-mode: modal keyboard-driven navigation and selection for terminal scrollback.

/// The current sub-mode within vi-mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViMode {
    /// Normal mode — motions move the cursor, no selection active.
    Normal,
    /// Visual (character-wise) selection mode.
    Visual,
    /// Visual-line selection mode — entire rows selected.
    VisualLine,
    /// Visual-block (rectangular) selection mode.
    VisualBlock,
}

/// A position in the scrollback buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorPos {
    /// Row index (0 = top of scrollback).
    pub row: usize,
    /// Column index.
    pub col: usize,
}

/// The full vi-mode state for a single pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViState {
    /// Current vi sub-mode.
    pub mode: ViMode,
    /// Current cursor position.
    pub cursor: CursorPos,
    /// Anchor position for visual selections (set when entering a visual mode).
    pub anchor: Option<CursorPos>,
    /// Accumulated count prefix (None = no count, use 1).
    pub count: Option<usize>,
    /// Whether we're in the middle of a multi-key sequence (e.g., 'g' waiting for second key).
    pub pending_key: Option<char>,
}

/// Actions that the vi-mode handler can produce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViAction {
    /// Move cursor by a motion.
    Motion(Motion),
    /// Enter a visual mode (sets anchor at current cursor).
    EnterVisual(ViMode),
    /// Return to Normal mode (clear selection).
    ExitVisual,
    /// Exit vi-mode entirely.
    ExitViMode,
    /// Yank (copy) the current selection to clipboard.
    Yank,
    /// Begin forward search.
    SearchForward,
    /// Begin backward search.
    SearchBackward,
    /// Jump to next search match.
    NextMatch,
    /// Jump to previous search match.
    PrevMatch,
    /// No action (key consumed but nothing to do).
    None,
}

/// A cursor motion with an associated repeat count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    CharLeft(usize),
    CharRight(usize),
    LineUp(usize),
    LineDown(usize),
    WordForward(usize),
    WordBackward(usize),
    WordEnd(usize),
    LineStart,
    LineEnd,
    FirstNonBlank,
    BufferTop,
    BufferBottom,
    ViewportTop,
    ViewportMiddle,
    ViewportBottom,
    HalfPageUp,
    HalfPageDown,
}

impl ViState {
    /// Create a new vi-mode state, starting in Normal mode at the given position.
    pub fn new(row: usize, col: usize) -> Self {
        Self {
            mode: ViMode::Normal,
            cursor: CursorPos { row, col },
            anchor: None,
            count: None,
            pending_key: None,
        }
    }

    /// Get the effective count (defaults to 1 if no count prefix entered).
    pub fn effective_count(&self) -> usize {
        self.count.unwrap_or(1)
    }

    /// Feed a digit to the count accumulator. Returns true if the digit was consumed as a count.
    /// '0' is only consumed as count if there is already a pending count (otherwise it's LineStart).
    pub fn feed_count_digit(&mut self, digit: char) -> bool {
        if let Some(d) = digit.to_digit(10) {
            let d = d as usize;
            if d == 0 && self.count.is_none() {
                return false; // '0' without pending count → LineStart motion
            }
            let current = self.count.unwrap_or(0);
            let new_count = current.saturating_mul(10).saturating_add(d).min(9999);
            self.count = Some(new_count);
            true
        } else {
            false
        }
    }

    /// Process a key input and return the resulting action.
    /// This handles mode transitions, count prefixes, and motion commands.
    pub fn process_key(&mut self, ch: char, ctrl: bool) -> ViAction {
        // Handle pending multi-key sequences (e.g., 'g' prefix)
        if let Some(pending) = self.pending_key.take() {
            return self.process_pending(pending, ch);
        }

        // Handle Ctrl+key combinations
        if ctrl {
            return self.process_ctrl_key(ch);
        }

        // Try count prefix first
        if ch.is_ascii_digit() && self.feed_count_digit(ch) {
            return ViAction::None;
        }

        let count = self.effective_count();
        self.count = None;

        match self.mode {
            ViMode::Normal => self.process_normal(ch, count),
            ViMode::Visual | ViMode::VisualLine | ViMode::VisualBlock => {
                self.process_visual(ch, count)
            }
        }
    }

    /// Process a key in Normal mode.
    fn process_normal(&mut self, ch: char, count: usize) -> ViAction {
        match ch {
            'h' => ViAction::Motion(Motion::CharLeft(count)),
            'l' => ViAction::Motion(Motion::CharRight(count)),
            'j' => ViAction::Motion(Motion::LineDown(count)),
            'k' => ViAction::Motion(Motion::LineUp(count)),
            'w' => ViAction::Motion(Motion::WordForward(count)),
            'b' => ViAction::Motion(Motion::WordBackward(count)),
            'e' => ViAction::Motion(Motion::WordEnd(count)),
            '0' => ViAction::Motion(Motion::LineStart),
            '$' => ViAction::Motion(Motion::LineEnd),
            '^' => ViAction::Motion(Motion::FirstNonBlank),
            'G' => ViAction::Motion(Motion::BufferBottom),
            'H' => ViAction::Motion(Motion::ViewportTop),
            'M' => ViAction::Motion(Motion::ViewportMiddle),
            'L' => ViAction::Motion(Motion::ViewportBottom),
            'g' => {
                self.pending_key = Some('g');
                ViAction::None
            }
            'v' => {
                self.mode = ViMode::Visual;
                self.anchor = Some(self.cursor);
                ViAction::EnterVisual(ViMode::Visual)
            }
            'V' => {
                self.mode = ViMode::VisualLine;
                self.anchor = Some(self.cursor);
                ViAction::EnterVisual(ViMode::VisualLine)
            }
            '/' => ViAction::SearchForward,
            '?' => ViAction::SearchBackward,
            'n' => ViAction::NextMatch,
            'N' => ViAction::PrevMatch,
            '\x1b' => ViAction::ExitViMode, // Escape
            _ => ViAction::None,
        }
    }

    /// Process a key in any Visual mode.
    fn process_visual(&mut self, ch: char, count: usize) -> ViAction {
        match ch {
            // Motions work the same in visual modes
            'h' => ViAction::Motion(Motion::CharLeft(count)),
            'l' => ViAction::Motion(Motion::CharRight(count)),
            'j' => ViAction::Motion(Motion::LineDown(count)),
            'k' => ViAction::Motion(Motion::LineUp(count)),
            'w' => ViAction::Motion(Motion::WordForward(count)),
            'b' => ViAction::Motion(Motion::WordBackward(count)),
            'e' => ViAction::Motion(Motion::WordEnd(count)),
            '0' => ViAction::Motion(Motion::LineStart),
            '$' => ViAction::Motion(Motion::LineEnd),
            '^' => ViAction::Motion(Motion::FirstNonBlank),
            'G' => ViAction::Motion(Motion::BufferBottom),
            'H' => ViAction::Motion(Motion::ViewportTop),
            'M' => ViAction::Motion(Motion::ViewportMiddle),
            'L' => ViAction::Motion(Motion::ViewportBottom),
            'g' => {
                self.pending_key = Some('g');
                ViAction::None
            }
            // Yank
            'y' => {
                let action = ViAction::Yank;
                self.mode = ViMode::Normal;
                self.anchor = None;
                action
            }
            // Toggle visual modes or exit
            'v' => {
                if self.mode == ViMode::Visual {
                    self.mode = ViMode::Normal;
                    self.anchor = None;
                    ViAction::ExitVisual
                } else {
                    self.mode = ViMode::Visual;
                    self.anchor = Some(self.cursor);
                    ViAction::EnterVisual(ViMode::Visual)
                }
            }
            'V' => {
                if self.mode == ViMode::VisualLine {
                    self.mode = ViMode::Normal;
                    self.anchor = None;
                    ViAction::ExitVisual
                } else {
                    self.mode = ViMode::VisualLine;
                    self.anchor = Some(self.cursor);
                    ViAction::EnterVisual(ViMode::VisualLine)
                }
            }
            // Search
            '/' => ViAction::SearchForward,
            '?' => ViAction::SearchBackward,
            'n' => ViAction::NextMatch,
            'N' => ViAction::PrevMatch,
            // Escape → back to Normal
            '\x1b' => {
                self.mode = ViMode::Normal;
                self.anchor = None;
                ViAction::ExitVisual
            }
            _ => ViAction::None,
        }
    }

    /// Process Ctrl+key combinations.
    fn process_ctrl_key(&mut self, ch: char) -> ViAction {
        let count = self.effective_count();
        self.count = None;
        match ch {
            'u' | 'U' => ViAction::Motion(Motion::HalfPageUp),
            'd' | 'D' => ViAction::Motion(Motion::HalfPageDown),
            'v' | 'V' => {
                // Ctrl+V → Visual-Block toggle
                if self.mode == ViMode::VisualBlock {
                    self.mode = ViMode::Normal;
                    self.anchor = None;
                    ViAction::ExitVisual
                } else {
                    self.mode = ViMode::VisualBlock;
                    self.anchor = Some(self.cursor);
                    ViAction::EnterVisual(ViMode::VisualBlock)
                }
            }
            _ => {
                let _ = count; // suppress unused warning
                ViAction::None
            }
        }
    }

    /// Process the second key of a multi-key sequence.
    fn process_pending(&mut self, first: char, second: char) -> ViAction {
        match (first, second) {
            ('g', 'g') => {
                let action = ViAction::Motion(Motion::BufferTop);
                self.count = None;
                action
            }
            _ => {
                self.count = None;
                ViAction::None
            }
        }
    }

    /// Get the mode indicator text for the status bar.
    pub fn mode_text(&self) -> &'static str {
        match self.mode {
            ViMode::Normal => "-- NORMAL --",
            ViMode::Visual => "-- VISUAL --",
            ViMode::VisualLine => "-- VISUAL LINE --",
            ViMode::VisualBlock => "-- VISUAL BLOCK --",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ViState construction ────────────────────────────────────────

    #[test]
    fn new_state_is_normal_mode() {
        let state = ViState::new(10, 5);
        assert_eq!(state.mode, ViMode::Normal);
        assert_eq!(state.cursor, CursorPos { row: 10, col: 5 });
        assert_eq!(state.anchor, None);
        assert_eq!(state.count, None);
        assert_eq!(state.pending_key, None);
    }

    // ── Mode transitions: Normal → Visual ───────────────────────────

    #[test]
    fn v_enters_visual_mode() {
        let mut state = ViState::new(5, 3);
        let action = state.process_key('v', false);
        assert_eq!(state.mode, ViMode::Visual);
        assert_eq!(state.anchor, Some(CursorPos { row: 5, col: 3 }));
        assert_eq!(action, ViAction::EnterVisual(ViMode::Visual));
    }

    #[test]
    fn uppercase_v_enters_visual_line_mode() {
        let mut state = ViState::new(5, 3);
        let action = state.process_key('V', false);
        assert_eq!(state.mode, ViMode::VisualLine);
        assert_eq!(state.anchor, Some(CursorPos { row: 5, col: 3 }));
        assert_eq!(action, ViAction::EnterVisual(ViMode::VisualLine));
    }

    #[test]
    fn ctrl_v_enters_visual_block_mode() {
        let mut state = ViState::new(5, 3);
        let action = state.process_key('v', true); // Ctrl+V
        assert_eq!(state.mode, ViMode::VisualBlock);
        assert_eq!(state.anchor, Some(CursorPos { row: 5, col: 3 }));
        assert_eq!(action, ViAction::EnterVisual(ViMode::VisualBlock));
    }

    // ── Mode transitions: Visual → Normal ───────────────────────────

    #[test]
    fn escape_from_visual_returns_to_normal() {
        let mut state = ViState::new(5, 3);
        state.process_key('v', false); // enter Visual
        let action = state.process_key('\x1b', false); // Escape
        assert_eq!(state.mode, ViMode::Normal);
        assert_eq!(state.anchor, None);
        assert_eq!(action, ViAction::ExitVisual);
    }

    #[test]
    fn v_from_visual_returns_to_normal() {
        let mut state = ViState::new(5, 3);
        state.process_key('v', false); // enter Visual
        let action = state.process_key('v', false); // toggle off
        assert_eq!(state.mode, ViMode::Normal);
        assert_eq!(state.anchor, None);
        assert_eq!(action, ViAction::ExitVisual);
    }

    #[test]
    fn uppercase_v_from_visual_line_returns_to_normal() {
        let mut state = ViState::new(5, 3);
        state.process_key('V', false); // enter Visual-Line
        let action = state.process_key('V', false); // toggle off
        assert_eq!(state.mode, ViMode::Normal);
        assert_eq!(state.anchor, None);
        assert_eq!(action, ViAction::ExitVisual);
    }

    #[test]
    fn ctrl_v_from_visual_block_returns_to_normal() {
        let mut state = ViState::new(5, 3);
        state.process_key('v', true); // enter Visual-Block
        let action = state.process_key('v', true); // toggle off
        assert_eq!(state.mode, ViMode::Normal);
        assert_eq!(state.anchor, None);
        assert_eq!(action, ViAction::ExitVisual);
    }

    #[test]
    fn escape_from_visual_line_returns_to_normal() {
        let mut state = ViState::new(5, 3);
        state.process_key('V', false);
        let action = state.process_key('\x1b', false);
        assert_eq!(state.mode, ViMode::Normal);
        assert_eq!(action, ViAction::ExitVisual);
    }

    #[test]
    fn escape_from_visual_block_returns_to_normal() {
        let mut state = ViState::new(5, 3);
        state.process_key('v', true);
        let action = state.process_key('\x1b', false);
        assert_eq!(state.mode, ViMode::Normal);
        assert_eq!(action, ViAction::ExitVisual);
    }

    // ── Mode transitions: Normal → Exit vi-mode ─────────────────────

    #[test]
    fn escape_from_normal_exits_vi_mode() {
        let mut state = ViState::new(5, 3);
        let action = state.process_key('\x1b', false);
        assert_eq!(action, ViAction::ExitViMode);
    }

    // ── Motion commands from Normal mode ────────────────────────────

    #[test]
    fn h_produces_char_left_motion() {
        let mut state = ViState::new(5, 3);
        assert_eq!(
            state.process_key('h', false),
            ViAction::Motion(Motion::CharLeft(1))
        );
    }

    #[test]
    fn l_produces_char_right_motion() {
        let mut state = ViState::new(5, 3);
        assert_eq!(
            state.process_key('l', false),
            ViAction::Motion(Motion::CharRight(1))
        );
    }

    #[test]
    fn j_produces_line_down_motion() {
        let mut state = ViState::new(5, 3);
        assert_eq!(
            state.process_key('j', false),
            ViAction::Motion(Motion::LineDown(1))
        );
    }

    #[test]
    fn k_produces_line_up_motion() {
        let mut state = ViState::new(5, 3);
        assert_eq!(
            state.process_key('k', false),
            ViAction::Motion(Motion::LineUp(1))
        );
    }

    #[test]
    fn zero_produces_line_start() {
        let mut state = ViState::new(5, 3);
        assert_eq!(
            state.process_key('0', false),
            ViAction::Motion(Motion::LineStart)
        );
    }

    #[test]
    fn dollar_produces_line_end() {
        let mut state = ViState::new(5, 3);
        assert_eq!(
            state.process_key('$', false),
            ViAction::Motion(Motion::LineEnd)
        );
    }

    #[test]
    fn caret_produces_first_non_blank() {
        let mut state = ViState::new(5, 3);
        assert_eq!(
            state.process_key('^', false),
            ViAction::Motion(Motion::FirstNonBlank)
        );
    }

    #[test]
    fn w_produces_word_forward() {
        let mut state = ViState::new(5, 3);
        assert_eq!(
            state.process_key('w', false),
            ViAction::Motion(Motion::WordForward(1))
        );
    }

    #[test]
    fn b_produces_word_backward() {
        let mut state = ViState::new(5, 3);
        assert_eq!(
            state.process_key('b', false),
            ViAction::Motion(Motion::WordBackward(1))
        );
    }

    #[test]
    fn e_produces_word_end() {
        let mut state = ViState::new(5, 3);
        assert_eq!(
            state.process_key('e', false),
            ViAction::Motion(Motion::WordEnd(1))
        );
    }

    #[test]
    fn uppercase_g_produces_buffer_bottom() {
        let mut state = ViState::new(5, 3);
        assert_eq!(
            state.process_key('G', false),
            ViAction::Motion(Motion::BufferBottom)
        );
    }

    #[test]
    fn gg_produces_buffer_top() {
        let mut state = ViState::new(5, 3);
        let action1 = state.process_key('g', false);
        assert_eq!(action1, ViAction::None); // first 'g' is pending
        assert_eq!(state.pending_key, Some('g'));

        let action2 = state.process_key('g', false);
        assert_eq!(action2, ViAction::Motion(Motion::BufferTop));
        assert_eq!(state.pending_key, None);
    }

    #[test]
    fn uppercase_h_produces_viewport_top() {
        let mut state = ViState::new(5, 3);
        assert_eq!(
            state.process_key('H', false),
            ViAction::Motion(Motion::ViewportTop)
        );
    }

    #[test]
    fn uppercase_m_produces_viewport_middle() {
        let mut state = ViState::new(5, 3);
        assert_eq!(
            state.process_key('M', false),
            ViAction::Motion(Motion::ViewportMiddle)
        );
    }

    #[test]
    fn uppercase_l_produces_viewport_bottom() {
        let mut state = ViState::new(5, 3);
        assert_eq!(
            state.process_key('L', false),
            ViAction::Motion(Motion::ViewportBottom)
        );
    }

    #[test]
    fn ctrl_u_produces_half_page_up() {
        let mut state = ViState::new(5, 3);
        assert_eq!(
            state.process_key('u', true),
            ViAction::Motion(Motion::HalfPageUp)
        );
    }

    #[test]
    fn ctrl_d_produces_half_page_down() {
        let mut state = ViState::new(5, 3);
        assert_eq!(
            state.process_key('d', true),
            ViAction::Motion(Motion::HalfPageDown)
        );
    }

    // ── Count prefix ────────────────────────────────────────────────

    #[test]
    fn count_prefix_with_motion() {
        let mut state = ViState::new(5, 3);
        state.process_key('5', false); // count = 5
        let action = state.process_key('j', false);
        assert_eq!(action, ViAction::Motion(Motion::LineDown(5)));
    }

    #[test]
    fn multi_digit_count() {
        let mut state = ViState::new(5, 3);
        state.process_key('1', false);
        state.process_key('2', false);
        let action = state.process_key('l', false);
        assert_eq!(action, ViAction::Motion(Motion::CharRight(12)));
    }

    #[test]
    fn count_capped_at_9999() {
        let mut state = ViState::new(5, 3);
        for _ in 0..6 {
            state.process_key('9', false);
        }
        let action = state.process_key('j', false);
        assert_eq!(action, ViAction::Motion(Motion::LineDown(9999)));
    }

    #[test]
    fn zero_without_count_is_line_start() {
        let mut state = ViState::new(5, 3);
        let action = state.process_key('0', false);
        assert_eq!(action, ViAction::Motion(Motion::LineStart));
    }

    #[test]
    fn zero_with_pending_count_appends() {
        let mut state = ViState::new(5, 3);
        state.process_key('1', false); // count = 1
        state.process_key('0', false); // count = 10
        let action = state.process_key('j', false);
        assert_eq!(action, ViAction::Motion(Motion::LineDown(10)));
    }

    #[test]
    fn effective_count_defaults_to_one() {
        let state = ViState::new(5, 3);
        assert_eq!(state.effective_count(), 1);
    }

    #[test]
    fn count_reset_after_motion() {
        let mut state = ViState::new(5, 3);
        state.process_key('3', false);
        state.process_key('j', false);
        // After motion, count should be reset
        assert_eq!(state.count, None);
        let action = state.process_key('k', false);
        assert_eq!(action, ViAction::Motion(Motion::LineUp(1)));
    }

    // ── Motions work in Visual mode too ─────────────────────────────

    #[test]
    fn motions_work_in_visual_mode() {
        let mut state = ViState::new(5, 3);
        state.process_key('v', false); // enter Visual
        let action = state.process_key('j', false);
        assert_eq!(action, ViAction::Motion(Motion::LineDown(1)));
        assert_eq!(state.mode, ViMode::Visual); // still in Visual
    }

    #[test]
    fn count_motions_work_in_visual_mode() {
        let mut state = ViState::new(5, 3);
        state.process_key('v', false);
        state.process_key('3', false);
        let action = state.process_key('l', false);
        assert_eq!(action, ViAction::Motion(Motion::CharRight(3)));
    }

    // ── Yank ────────────────────────────────────────────────────────

    #[test]
    fn y_in_visual_mode_yanks_and_returns_to_normal() {
        let mut state = ViState::new(5, 3);
        state.process_key('v', false); // enter Visual
        let action = state.process_key('y', false);
        assert_eq!(action, ViAction::Yank);
        assert_eq!(state.mode, ViMode::Normal);
        assert_eq!(state.anchor, None);
    }

    #[test]
    fn y_in_visual_line_yanks_and_returns_to_normal() {
        let mut state = ViState::new(5, 3);
        state.process_key('V', false);
        let action = state.process_key('y', false);
        assert_eq!(action, ViAction::Yank);
        assert_eq!(state.mode, ViMode::Normal);
    }

    #[test]
    fn y_in_visual_block_yanks_and_returns_to_normal() {
        let mut state = ViState::new(5, 3);
        state.process_key('v', true); // Ctrl+V
        let action = state.process_key('y', false);
        assert_eq!(action, ViAction::Yank);
        assert_eq!(state.mode, ViMode::Normal);
    }

    // ── Search commands ─────────────────────────────────────────────

    #[test]
    fn slash_starts_forward_search() {
        let mut state = ViState::new(5, 3);
        assert_eq!(state.process_key('/', false), ViAction::SearchForward);
    }

    #[test]
    fn question_mark_starts_backward_search() {
        let mut state = ViState::new(5, 3);
        assert_eq!(state.process_key('?', false), ViAction::SearchBackward);
    }

    #[test]
    fn n_jumps_to_next_match() {
        let mut state = ViState::new(5, 3);
        assert_eq!(state.process_key('n', false), ViAction::NextMatch);
    }

    #[test]
    fn uppercase_n_jumps_to_prev_match() {
        let mut state = ViState::new(5, 3);
        assert_eq!(state.process_key('N', false), ViAction::PrevMatch);
    }

    // ── Mode text ───────────────────────────────────────────────────

    #[test]
    fn mode_text_normal() {
        let state = ViState::new(0, 0);
        assert_eq!(state.mode_text(), "-- NORMAL --");
    }

    #[test]
    fn mode_text_visual() {
        let mut state = ViState::new(0, 0);
        state.process_key('v', false);
        assert_eq!(state.mode_text(), "-- VISUAL --");
    }

    #[test]
    fn mode_text_visual_line() {
        let mut state = ViState::new(0, 0);
        state.process_key('V', false);
        assert_eq!(state.mode_text(), "-- VISUAL LINE --");
    }

    #[test]
    fn mode_text_visual_block() {
        let mut state = ViState::new(0, 0);
        state.process_key('v', true);
        assert_eq!(state.mode_text(), "-- VISUAL BLOCK --");
    }

    // ── Pending key cancellation ────────────────────────────────────

    #[test]
    fn g_followed_by_non_g_produces_none() {
        let mut state = ViState::new(5, 3);
        state.process_key('g', false);
        let action = state.process_key('x', false); // invalid sequence
        assert_eq!(action, ViAction::None);
        assert_eq!(state.pending_key, None);
    }

    // ── Visual mode switching ───────────────────────────────────────

    #[test]
    fn switch_from_visual_to_visual_line() {
        let mut state = ViState::new(5, 3);
        state.process_key('v', false); // enter Visual
        let action = state.process_key('V', false); // switch to Visual-Line
        assert_eq!(state.mode, ViMode::VisualLine);
        assert_eq!(action, ViAction::EnterVisual(ViMode::VisualLine));
    }

    #[test]
    fn switch_from_visual_to_visual_block() {
        let mut state = ViState::new(5, 3);
        state.process_key('v', false);
        let action = state.process_key('v', true); // Ctrl+V → Visual-Block
        assert_eq!(state.mode, ViMode::VisualBlock);
        assert_eq!(action, ViAction::EnterVisual(ViMode::VisualBlock));
    }

    // ── Unknown key produces None ───────────────────────────────────

    #[test]
    fn unknown_key_produces_none() {
        let mut state = ViState::new(5, 3);
        assert_eq!(state.process_key('x', false), ViAction::None);
    }

    #[test]
    fn unknown_ctrl_key_produces_none() {
        let mut state = ViState::new(5, 3);
        assert_eq!(state.process_key('x', true), ViAction::None);
    }
}

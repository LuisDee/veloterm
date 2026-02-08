// Vi-mode: modal keyboard-driven navigation and selection for terminal scrollback.

/// Direction of the last search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchDirection {
    Forward,
    Backward,
}

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
    /// Whether search input mode is active (collecting query text).
    pub search_input_active: bool,
    /// The current search query (being typed, or last executed).
    pub search_query: String,
    /// Direction of the current/last search.
    pub search_direction: SearchDirection,
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
    /// Search query confirmed (Enter pressed in search input mode).
    SearchExecute,
    /// Search input cancelled (Escape pressed in search input mode).
    SearchCancel,
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
            search_input_active: false,
            search_query: String::new(),
            search_direction: SearchDirection::Forward,
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
        // Handle search input mode first
        if self.search_input_active {
            return self.process_search_input(ch, ctrl);
        }

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
            '/' => {
                self.search_input_active = true;
                self.search_query.clear();
                self.search_direction = SearchDirection::Forward;
                ViAction::SearchForward
            }
            '?' => {
                self.search_input_active = true;
                self.search_query.clear();
                self.search_direction = SearchDirection::Backward;
                ViAction::SearchBackward
            }
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
            '/' => {
                self.search_input_active = true;
                self.search_query.clear();
                self.search_direction = SearchDirection::Forward;
                ViAction::SearchForward
            }
            '?' => {
                self.search_input_active = true;
                self.search_query.clear();
                self.search_direction = SearchDirection::Backward;
                ViAction::SearchBackward
            }
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

    /// Get the full status bar text for the current vi-mode state.
    /// During search input, shows the search prompt. Otherwise shows the mode indicator.
    pub fn status_text(&self) -> String {
        if self.search_input_active {
            self.search_prompt()
        } else {
            self.mode_text().to_string()
        }
    }

    /// Whether search input is currently active (collecting query text).
    pub fn is_search_active(&self) -> bool {
        self.search_input_active
    }

    /// Get the search prompt text for display (e.g., "/ hello" or "? hello").
    pub fn search_prompt(&self) -> String {
        let prefix = match self.search_direction {
            SearchDirection::Forward => "/",
            SearchDirection::Backward => "?",
        };
        format!("{} {}", prefix, self.search_query)
    }

    /// Move the vi cursor to a match position.
    pub fn move_to_match(&mut self, row: usize, col: usize) {
        self.cursor.row = row;
        self.cursor.col = col;
    }

    /// Process a key during search input mode.
    fn process_search_input(&mut self, ch: char, ctrl: bool) -> ViAction {
        if ctrl {
            return ViAction::None;
        }
        match ch {
            '\x1b' => {
                // Escape: cancel search
                self.search_input_active = false;
                self.search_query.clear();
                ViAction::SearchCancel
            }
            '\r' | '\n' => {
                // Enter: confirm search
                self.search_input_active = false;
                ViAction::SearchExecute
            }
            '\x08' | '\x7f' => {
                // Backspace: remove last character
                self.search_query.pop();
                ViAction::None
            }
            _ => {
                // Append character to query
                self.search_query.push(ch);
                ViAction::None
            }
        }
    }

    /// Apply a motion to the cursor position, clamping to buffer bounds.
    /// `line_len` is a callback that returns the length (number of columns) for a given row.
    pub fn apply_motion(&mut self, motion: &Motion, ctx: &BufferContext) {
        match *motion {
            Motion::CharLeft(n) => {
                self.cursor.col = self.cursor.col.saturating_sub(n);
            }
            Motion::CharRight(n) => {
                let max_col = ctx.line_len(self.cursor.row).saturating_sub(1);
                self.cursor.col = (self.cursor.col + n).min(max_col);
            }
            Motion::LineUp(n) => {
                self.cursor.row = self.cursor.row.saturating_sub(n);
                self.clamp_col(ctx);
            }
            Motion::LineDown(n) => {
                let max_row = ctx.total_rows.saturating_sub(1);
                self.cursor.row = (self.cursor.row + n).min(max_row);
                self.clamp_col(ctx);
            }
            Motion::LineStart => {
                self.cursor.col = 0;
            }
            Motion::LineEnd => {
                let max_col = ctx.line_len(self.cursor.row).saturating_sub(1);
                self.cursor.col = max_col;
            }
            Motion::FirstNonBlank => {
                let len = ctx.line_len(self.cursor.row);
                let mut col = 0;
                while col < len {
                    if let Some(ch) = ctx.char_at(self.cursor.row, col) {
                        if ch != ' ' && ch != '\t' {
                            break;
                        }
                    }
                    col += 1;
                }
                self.cursor.col = col.min(len.saturating_sub(1));
            }
            Motion::BufferTop => {
                self.cursor.row = 0;
                self.clamp_col(ctx);
            }
            Motion::BufferBottom => {
                self.cursor.row = ctx.total_rows.saturating_sub(1);
                self.clamp_col(ctx);
            }
            Motion::ViewportTop => {
                self.cursor.row = ctx.viewport_top;
                self.clamp_col(ctx);
            }
            Motion::ViewportMiddle => {
                let mid = ctx.viewport_top + ctx.viewport_rows / 2;
                self.cursor.row = mid.min(ctx.total_rows.saturating_sub(1));
                self.clamp_col(ctx);
            }
            Motion::ViewportBottom => {
                let bottom = ctx.viewport_top + ctx.viewport_rows.saturating_sub(1);
                self.cursor.row = bottom.min(ctx.total_rows.saturating_sub(1));
                self.clamp_col(ctx);
            }
            Motion::HalfPageUp => {
                let half = ctx.viewport_rows / 2;
                self.cursor.row = self.cursor.row.saturating_sub(half);
                self.clamp_col(ctx);
            }
            Motion::HalfPageDown => {
                let half = ctx.viewport_rows / 2;
                let max_row = ctx.total_rows.saturating_sub(1);
                self.cursor.row = (self.cursor.row + half).min(max_row);
                self.clamp_col(ctx);
            }
            Motion::WordForward(n) => {
                for _ in 0..n {
                    self.move_word_forward(ctx);
                }
            }
            Motion::WordBackward(n) => {
                for _ in 0..n {
                    self.move_word_backward(ctx);
                }
            }
            Motion::WordEnd(n) => {
                for _ in 0..n {
                    self.move_word_end(ctx);
                }
            }
        }
    }

    /// Clamp column to the current line's length.
    fn clamp_col(&mut self, ctx: &BufferContext) {
        let max_col = ctx.line_len(self.cursor.row).saturating_sub(1);
        self.cursor.col = self.cursor.col.min(max_col);
    }

    /// Move forward to the start of the next word.
    fn move_word_forward(&mut self, ctx: &BufferContext) {
        let max_row = ctx.total_rows.saturating_sub(1);
        let mut row = self.cursor.row;
        let mut col = self.cursor.col;
        let len = ctx.line_len(row);

        // Skip current word characters
        while col < len {
            match ctx.char_at(row, col) {
                Some(ch) if is_word_char(ch) => col += 1,
                _ => break,
            }
        }
        // Skip non-word characters (spaces, punctuation)
        loop {
            if col < ctx.line_len(row) {
                match ctx.char_at(row, col) {
                    Some(ch) if !is_word_char(ch) && ch != '\0' => col += 1,
                    _ => break,
                }
            } else {
                // Move to next line
                if row < max_row {
                    row += 1;
                    col = 0;
                    // Skip leading whitespace on new line
                    let new_len = ctx.line_len(row);
                    while col < new_len {
                        match ctx.char_at(row, col) {
                            Some(' ') | Some('\t') => col += 1,
                            _ => break,
                        }
                    }
                    break;
                } else {
                    col = ctx.line_len(row).saturating_sub(1);
                    break;
                }
            }
        }
        self.cursor.row = row;
        self.cursor.col = col.min(ctx.line_len(row).saturating_sub(1));
    }

    /// Move backward to the start of the previous word.
    fn move_word_backward(&mut self, ctx: &BufferContext) {
        let mut row = self.cursor.row;
        let mut col = self.cursor.col;

        // If at start of line, go to end of previous line
        if col == 0 && row > 0 {
            row -= 1;
            col = ctx.line_len(row).saturating_sub(1);
        } else if col > 0 {
            col = col.saturating_sub(1);
        }

        // Skip non-word characters backward
        loop {
            if col == 0 && row == 0 {
                break;
            }
            match ctx.char_at(row, col) {
                Some(ch) if is_word_char(ch) => break,
                _ => {
                    if col > 0 {
                        col -= 1;
                    } else if row > 0 {
                        row -= 1;
                        col = ctx.line_len(row).saturating_sub(1);
                    } else {
                        break;
                    }
                }
            }
        }

        // Move to start of word
        while col > 0 {
            match ctx.char_at(row, col - 1) {
                Some(ch) if is_word_char(ch) => col -= 1,
                _ => break,
            }
        }

        self.cursor.row = row;
        self.cursor.col = col;
    }

    /// Move forward to the end of the current/next word.
    fn move_word_end(&mut self, ctx: &BufferContext) {
        let max_row = ctx.total_rows.saturating_sub(1);
        let mut row = self.cursor.row;
        let mut col = self.cursor.col;

        // Move at least one position forward
        col += 1;
        if col >= ctx.line_len(row) {
            if row < max_row {
                row += 1;
                col = 0;
            } else {
                self.cursor.col = ctx.line_len(row).saturating_sub(1);
                return;
            }
        }

        // Skip non-word characters
        loop {
            if col < ctx.line_len(row) {
                match ctx.char_at(row, col) {
                    Some(ch) if is_word_char(ch) => break,
                    _ => col += 1,
                }
            } else if row < max_row {
                row += 1;
                col = 0;
            } else {
                col = ctx.line_len(row).saturating_sub(1);
                break;
            }
        }

        // Move to end of word
        while col + 1 < ctx.line_len(row) {
            match ctx.char_at(row, col + 1) {
                Some(ch) if is_word_char(ch) => col += 1,
                _ => break,
            }
        }

        self.cursor.row = row;
        self.cursor.col = col.min(ctx.line_len(row).saturating_sub(1));
    }
}

use crate::input::selection::{
    Selection, SelectionType, selected_text, selected_text_block, selected_text_lines,
};
use crate::renderer::grid_renderer::{GridCell, CELL_FLAG_VI_CURSOR};

impl ViState {
    /// Convert the current vi-mode visual selection to a Selection for rendering.
    /// Returns None if not in a visual mode or no anchor is set.
    pub fn to_selection(&self) -> Option<Selection> {
        let anchor = self.anchor?;
        let selection_type = match self.mode {
            ViMode::Visual => SelectionType::Range,
            ViMode::VisualLine => SelectionType::Line,
            ViMode::VisualBlock => SelectionType::VisualBlock,
            ViMode::Normal => return None,
        };

        let (start, end) = match self.mode {
            ViMode::VisualLine => {
                // Full rows from anchor to cursor
                let min_row = anchor.row.min(self.cursor.row);
                let max_row = anchor.row.max(self.cursor.row);
                ((min_row, 0), (max_row, usize::MAX))
            }
            _ => {
                ((anchor.row, anchor.col), (self.cursor.row, self.cursor.col))
            }
        };

        Some(Selection {
            start,
            end,
            selection_type,
        })
    }

    /// Extract the selected text for yank, based on the current visual mode.
    /// Returns None if not in a visual mode.
    pub fn yank_text(&self, cells: &[GridCell], cols: usize) -> Option<String> {
        let selection = self.to_selection()?;
        let text = match selection.selection_type {
            SelectionType::VisualBlock => selected_text_block(cells, &selection, cols),
            SelectionType::Line => selected_text_lines(cells, &selection, cols),
            _ => selected_text(cells, &selection, cols),
        };
        Some(text)
    }

    /// Set the CELL_FLAG_VI_CURSOR on the cell at the current cursor position.
    /// The cursor position is relative to the viewport: `viewport_top` is subtracted from cursor.row.
    pub fn apply_vi_cursor_flag(&self, cells: &mut [GridCell], cols: usize, viewport_top: usize) {
        if self.cursor.row < viewport_top {
            return;
        }
        let rel_row = self.cursor.row - viewport_top;
        let idx = rel_row * cols + self.cursor.col;
        if idx < cells.len() {
            cells[idx].flags |= CELL_FLAG_VI_CURSOR;
        }
    }
}

/// Clear CELL_FLAG_VI_CURSOR from all cells.
pub fn clear_vi_cursor_flags(cells: &mut [GridCell]) {
    for cell in cells.iter_mut() {
        cell.flags &= !CELL_FLAG_VI_CURSOR;
    }
}

/// Check if a character is a word character (alphanumeric or underscore).
fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

/// Context about the buffer needed for motion application.
pub struct BufferContext<'a> {
    /// Total number of rows in the scrollback buffer.
    pub total_rows: usize,
    /// Number of columns per row.
    pub cols: usize,
    /// First visible row in the viewport.
    pub viewport_top: usize,
    /// Number of visible rows in the viewport.
    pub viewport_rows: usize,
    /// Callback to get a character at a given (row, col).
    /// Returns None if out of bounds.
    pub char_at_fn: &'a dyn Fn(usize, usize) -> Option<char>,
}

impl<'a> BufferContext<'a> {
    /// Get the length of a line (number of columns).
    pub fn line_len(&self, _row: usize) -> usize {
        self.cols
    }

    /// Get the character at (row, col).
    pub fn char_at(&self, row: usize, col: usize) -> Option<char> {
        (self.char_at_fn)(row, col)
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

    // ── Motion application tests ────────────────────────────────────

    /// Helper to create a BufferContext from a slice of strings.
    fn make_ctx<'a>(lines: &'a [&str], _viewport_top: usize, _viewport_rows: usize) -> (Vec<Vec<char>>, usize) {
        let cols = lines.iter().map(|l| l.len()).max().unwrap_or(10);
        let grid: Vec<Vec<char>> = lines
            .iter()
            .map(|l| {
                let mut row: Vec<char> = l.chars().collect();
                row.resize(cols, ' ');
                row
            })
            .collect();
        (grid, cols)
    }

    macro_rules! ctx_from {
        ($grid:expr, $cols:expr, $vt:expr, $vr:expr) => {{
            let grid_ref = &$grid;
            BufferContext {
                total_rows: grid_ref.len(),
                cols: $cols,
                viewport_top: $vt,
                viewport_rows: $vr,
                char_at_fn: &|row, col| {
                    grid_ref.get(row).and_then(|r| r.get(col).copied())
                },
            }
        }};
    }

    #[test]
    fn apply_char_left_clamps_at_zero() {
        let lines = &["hello world"];
        let (grid, cols) = make_ctx(lines, 0, 1);
        let ctx = ctx_from!(&grid, cols, 0, 1);
        let mut state = ViState::new(0, 2);
        state.apply_motion(&Motion::CharLeft(5), &ctx);
        assert_eq!(state.cursor.col, 0);
    }

    #[test]
    fn apply_char_left_moves_by_count() {
        let lines = &["hello world"];
        let (grid, cols) = make_ctx(lines, 0, 1);
        let ctx = ctx_from!(&grid, cols, 0, 1);
        let mut state = ViState::new(0, 5);
        state.apply_motion(&Motion::CharLeft(3), &ctx);
        assert_eq!(state.cursor.col, 2);
    }

    #[test]
    fn apply_char_right_clamps_at_line_end() {
        let lines = &["hello world"];
        let (grid, cols) = make_ctx(lines, 0, 1);
        let ctx = ctx_from!(&grid, cols, 0, 1);
        let mut state = ViState::new(0, 8);
        state.apply_motion(&Motion::CharRight(100), &ctx);
        assert_eq!(state.cursor.col, cols - 1);
    }

    #[test]
    fn apply_char_right_moves_by_count() {
        let lines = &["hello world"];
        let (grid, cols) = make_ctx(lines, 0, 1);
        let ctx = ctx_from!(&grid, cols, 0, 1);
        let mut state = ViState::new(0, 0);
        state.apply_motion(&Motion::CharRight(3), &ctx);
        assert_eq!(state.cursor.col, 3);
    }

    #[test]
    fn apply_line_up_clamps_at_zero() {
        let lines = &["line0", "line1", "line2"];
        let (grid, cols) = make_ctx(lines, 0, 3);
        let ctx = ctx_from!(&grid, cols, 0, 3);
        let mut state = ViState::new(1, 0);
        state.apply_motion(&Motion::LineUp(5), &ctx);
        assert_eq!(state.cursor.row, 0);
    }

    #[test]
    fn apply_line_down_clamps_at_last_row() {
        let lines = &["line0", "line1", "line2"];
        let (grid, cols) = make_ctx(lines, 0, 3);
        let ctx = ctx_from!(&grid, cols, 0, 3);
        let mut state = ViState::new(1, 0);
        state.apply_motion(&Motion::LineDown(100), &ctx);
        assert_eq!(state.cursor.row, 2);
    }

    #[test]
    fn apply_line_start() {
        let lines = &["hello world"];
        let (grid, cols) = make_ctx(lines, 0, 1);
        let ctx = ctx_from!(&grid, cols, 0, 1);
        let mut state = ViState::new(0, 5);
        state.apply_motion(&Motion::LineStart, &ctx);
        assert_eq!(state.cursor.col, 0);
    }

    #[test]
    fn apply_line_end() {
        let lines = &["hello world"];
        let (grid, cols) = make_ctx(lines, 0, 1);
        let ctx = ctx_from!(&grid, cols, 0, 1);
        let mut state = ViState::new(0, 0);
        state.apply_motion(&Motion::LineEnd, &ctx);
        assert_eq!(state.cursor.col, cols - 1);
    }

    #[test]
    fn apply_first_non_blank() {
        let lines = &["   hello"];
        let (grid, cols) = make_ctx(lines, 0, 1);
        let ctx = ctx_from!(&grid, cols, 0, 1);
        let mut state = ViState::new(0, 0);
        state.apply_motion(&Motion::FirstNonBlank, &ctx);
        assert_eq!(state.cursor.col, 3);
    }

    #[test]
    fn apply_first_non_blank_no_leading_spaces() {
        let lines = &["hello"];
        let (grid, cols) = make_ctx(lines, 0, 1);
        let ctx = ctx_from!(&grid, cols, 0, 1);
        let mut state = ViState::new(0, 3);
        state.apply_motion(&Motion::FirstNonBlank, &ctx);
        assert_eq!(state.cursor.col, 0);
    }

    #[test]
    fn apply_buffer_top() {
        let lines = &["line0", "line1", "line2"];
        let (grid, cols) = make_ctx(lines, 0, 3);
        let ctx = ctx_from!(&grid, cols, 0, 3);
        let mut state = ViState::new(2, 3);
        state.apply_motion(&Motion::BufferTop, &ctx);
        assert_eq!(state.cursor.row, 0);
    }

    #[test]
    fn apply_buffer_bottom() {
        let lines = &["line0", "line1", "line2"];
        let (grid, cols) = make_ctx(lines, 0, 3);
        let ctx = ctx_from!(&grid, cols, 0, 3);
        let mut state = ViState::new(0, 0);
        state.apply_motion(&Motion::BufferBottom, &ctx);
        assert_eq!(state.cursor.row, 2);
    }

    #[test]
    fn apply_viewport_top() {
        let lines = &["l0", "l1", "l2", "l3", "l4", "l5", "l6", "l7", "l8", "l9"];
        let (grid, cols) = make_ctx(lines, 3, 5);
        let ctx = ctx_from!(&grid, cols, 3, 5);
        let mut state = ViState::new(7, 0);
        state.apply_motion(&Motion::ViewportTop, &ctx);
        assert_eq!(state.cursor.row, 3);
    }

    #[test]
    fn apply_viewport_middle() {
        let lines = &["l0", "l1", "l2", "l3", "l4", "l5", "l6", "l7", "l8", "l9"];
        let (grid, cols) = make_ctx(lines, 3, 5);
        let ctx = ctx_from!(&grid, cols, 3, 5);
        let mut state = ViState::new(0, 0);
        state.apply_motion(&Motion::ViewportMiddle, &ctx);
        assert_eq!(state.cursor.row, 5); // 3 + 5/2 = 5
    }

    #[test]
    fn apply_viewport_bottom() {
        let lines = &["l0", "l1", "l2", "l3", "l4", "l5", "l6", "l7", "l8", "l9"];
        let (grid, cols) = make_ctx(lines, 3, 5);
        let ctx = ctx_from!(&grid, cols, 3, 5);
        let mut state = ViState::new(0, 0);
        state.apply_motion(&Motion::ViewportBottom, &ctx);
        assert_eq!(state.cursor.row, 7); // 3 + 5 - 1 = 7
    }

    #[test]
    fn apply_half_page_up() {
        let lines = &["l0", "l1", "l2", "l3", "l4", "l5", "l6", "l7", "l8", "l9"];
        let (grid, cols) = make_ctx(lines, 0, 10);
        let ctx = ctx_from!(&grid, cols, 0, 10);
        let mut state = ViState::new(7, 0);
        state.apply_motion(&Motion::HalfPageUp, &ctx);
        assert_eq!(state.cursor.row, 2); // 7 - 10/2 = 2
    }

    #[test]
    fn apply_half_page_down() {
        let lines = &["l0", "l1", "l2", "l3", "l4", "l5", "l6", "l7", "l8", "l9"];
        let (grid, cols) = make_ctx(lines, 0, 10);
        let ctx = ctx_from!(&grid, cols, 0, 10);
        let mut state = ViState::new(3, 0);
        state.apply_motion(&Motion::HalfPageDown, &ctx);
        assert_eq!(state.cursor.row, 8); // 3 + 10/2 = 8
    }

    #[test]
    fn apply_half_page_up_clamps_at_zero() {
        let lines = &["l0", "l1", "l2"];
        let (grid, cols) = make_ctx(lines, 0, 10);
        let ctx = ctx_from!(&grid, cols, 0, 10);
        let mut state = ViState::new(1, 0);
        state.apply_motion(&Motion::HalfPageUp, &ctx);
        assert_eq!(state.cursor.row, 0);
    }

    #[test]
    fn apply_half_page_down_clamps_at_bottom() {
        let lines = &["l0", "l1", "l2"];
        let (grid, cols) = make_ctx(lines, 0, 10);
        let ctx = ctx_from!(&grid, cols, 0, 10);
        let mut state = ViState::new(1, 0);
        state.apply_motion(&Motion::HalfPageDown, &ctx);
        assert_eq!(state.cursor.row, 2);
    }

    #[test]
    fn apply_word_forward() {
        let lines = &["hello world foo"];
        let (grid, cols) = make_ctx(lines, 0, 1);
        let ctx = ctx_from!(&grid, cols, 0, 1);
        let mut state = ViState::new(0, 0);
        state.apply_motion(&Motion::WordForward(1), &ctx);
        assert_eq!(state.cursor.col, 6); // 'w' of "world"
    }

    #[test]
    fn apply_word_forward_twice() {
        let lines = &["hello world foo"];
        let (grid, cols) = make_ctx(lines, 0, 1);
        let ctx = ctx_from!(&grid, cols, 0, 1);
        let mut state = ViState::new(0, 0);
        state.apply_motion(&Motion::WordForward(2), &ctx);
        assert_eq!(state.cursor.col, 12); // 'f' of "foo"
    }

    #[test]
    fn apply_word_backward() {
        let lines = &["hello world foo"];
        let (grid, cols) = make_ctx(lines, 0, 1);
        let ctx = ctx_from!(&grid, cols, 0, 1);
        let mut state = ViState::new(0, 8); // middle of "world"
        state.apply_motion(&Motion::WordBackward(1), &ctx);
        assert_eq!(state.cursor.col, 6); // start of "world"
    }

    #[test]
    fn apply_word_backward_to_previous_word() {
        let lines = &["hello world"];
        let (grid, cols) = make_ctx(lines, 0, 1);
        let ctx = ctx_from!(&grid, cols, 0, 1);
        let mut state = ViState::new(0, 6); // 'w' of "world"
        state.apply_motion(&Motion::WordBackward(1), &ctx);
        assert_eq!(state.cursor.col, 0); // 'h' of "hello"
    }

    #[test]
    fn apply_word_end() {
        let lines = &["hello world"];
        let (grid, cols) = make_ctx(lines, 0, 1);
        let ctx = ctx_from!(&grid, cols, 0, 1);
        let mut state = ViState::new(0, 0);
        state.apply_motion(&Motion::WordEnd(1), &ctx);
        assert_eq!(state.cursor.col, 4); // 'o' of "hello"
    }

    #[test]
    fn apply_word_end_from_end_of_word() {
        let lines = &["hello world"];
        let (grid, cols) = make_ctx(lines, 0, 1);
        let ctx = ctx_from!(&grid, cols, 0, 1);
        let mut state = ViState::new(0, 4); // 'o' of "hello"
        state.apply_motion(&Motion::WordEnd(1), &ctx);
        assert_eq!(state.cursor.col, 10); // 'd' of "world"
    }

    #[test]
    fn apply_line_down_clamps_col() {
        // When moving to a shorter line, col should clamp
        let lines = &["hello world", "hi"];
        let (grid, cols) = make_ctx(lines, 0, 2);
        let ctx = ctx_from!(&grid, cols, 0, 2);
        let mut state = ViState::new(0, 10); // at end of first line
        state.apply_motion(&Motion::LineDown(1), &ctx);
        assert_eq!(state.cursor.row, 1);
        // Col should still be clamped to cols-1 (same width grid)
        assert_eq!(state.cursor.col, cols - 1);
    }

    #[test]
    fn full_integration_count_then_motion() {
        let lines = &["l0", "l1", "l2", "l3", "l4"];
        let (grid, cols) = make_ctx(lines, 0, 5);
        let ctx = ctx_from!(&grid, cols, 0, 5);
        let mut state = ViState::new(0, 0);
        // Type "3j" — should move down 3 lines
        state.process_key('3', false);
        let action = state.process_key('j', false);
        assert_eq!(action, ViAction::Motion(Motion::LineDown(3)));
        state.apply_motion(&Motion::LineDown(3), &ctx);
        assert_eq!(state.cursor.row, 3);
    }

    #[test]
    fn word_backward_clamps_at_buffer_start() {
        let lines = &["hello"];
        let (grid, cols) = make_ctx(lines, 0, 1);
        let ctx = ctx_from!(&grid, cols, 0, 1);
        let mut state = ViState::new(0, 0);
        state.apply_motion(&Motion::WordBackward(1), &ctx);
        assert_eq!(state.cursor.row, 0);
        assert_eq!(state.cursor.col, 0);
    }

    // ── to_selection conversion ─────────────────────────────────────

    #[test]
    fn to_selection_returns_none_in_normal_mode() {
        let state = ViState::new(5, 3);
        assert_eq!(state.to_selection(), None);
    }

    #[test]
    fn to_selection_visual_mode() {
        let mut state = ViState::new(5, 3);
        state.process_key('v', false); // enter Visual
        state.cursor = CursorPos { row: 7, col: 10 };
        let sel = state.to_selection().unwrap();
        assert_eq!(sel.start, (5, 3));
        assert_eq!(sel.end, (7, 10));
        assert_eq!(sel.selection_type, SelectionType::Range);
    }

    #[test]
    fn to_selection_visual_line_mode() {
        let mut state = ViState::new(5, 3);
        state.process_key('V', false); // enter Visual-Line
        state.cursor = CursorPos { row: 8, col: 0 };
        let sel = state.to_selection().unwrap();
        assert_eq!(sel.start, (5, 0));
        assert_eq!(sel.end.0, 8);
        assert_eq!(sel.selection_type, SelectionType::Line);
    }

    #[test]
    fn to_selection_visual_block_mode() {
        let mut state = ViState::new(5, 3);
        state.process_key('v', true); // Ctrl+V → Visual-Block
        state.cursor = CursorPos { row: 8, col: 10 };
        let sel = state.to_selection().unwrap();
        assert_eq!(sel.start, (5, 3));
        assert_eq!(sel.end, (8, 10));
        assert_eq!(sel.selection_type, SelectionType::VisualBlock);
    }

    #[test]
    fn to_selection_visual_line_reversed() {
        let mut state = ViState::new(8, 3);
        state.process_key('V', false);
        state.cursor = CursorPos { row: 5, col: 0 }; // cursor above anchor
        let sel = state.to_selection().unwrap();
        assert_eq!(sel.start, (5, 0)); // normalized: min row first
        assert_eq!(sel.end.0, 8);
    }

    // ── Yank text extraction ────────────────────────────────────────

    fn make_cells(lines: &[&str], cols: usize) -> Vec<GridCell> {
        use crate::config::theme::Color;
        let fg = Color::new(1.0, 1.0, 1.0, 1.0);
        let bg = Color::new(0.0, 0.0, 0.0, 1.0);
        let mut cells = Vec::new();
        for line in lines {
            for ch in line.chars().take(cols) {
                cells.push(GridCell::new(ch, fg, bg));
            }
            let line_len = line.chars().count().min(cols);
            for _ in line_len..cols {
                cells.push(GridCell::new(' ', fg, bg));
            }
        }
        cells
    }

    #[test]
    fn yank_text_visual_mode() {
        let cells = make_cells(&["hello world"], 20);
        let mut state = ViState::new(0, 0);
        state.process_key('v', false);
        state.cursor = CursorPos { row: 0, col: 4 };
        let text = state.yank_text(&cells, 20).unwrap();
        assert_eq!(text, "hello");
    }

    #[test]
    fn yank_text_visual_line_mode() {
        let cells = make_cells(&["hello world", "second line"], 20);
        let mut state = ViState::new(0, 3);
        state.process_key('V', false);
        state.cursor = CursorPos { row: 1, col: 0 };
        let text = state.yank_text(&cells, 20).unwrap();
        assert_eq!(text, "hello world\nsecond line");
    }

    #[test]
    fn yank_text_visual_block_mode() {
        let cells = make_cells(&["hello world", "abcde fghij"], 20);
        let mut state = ViState::new(0, 0);
        state.process_key('v', true); // Ctrl+V
        state.cursor = CursorPos { row: 1, col: 4 };
        let text = state.yank_text(&cells, 20).unwrap();
        assert_eq!(text, "hello\nabcde");
    }

    #[test]
    fn yank_text_returns_none_in_normal() {
        let cells = make_cells(&["hello"], 20);
        let state = ViState::new(0, 0);
        assert_eq!(state.yank_text(&cells, 20), None);
    }

    // ── Search input mode ─────────────────────────────────────────────

    #[test]
    fn slash_activates_search_input_forward() {
        let mut state = ViState::new(5, 3);
        let action = state.process_key('/', false);
        assert_eq!(action, ViAction::SearchForward);
        assert!(state.is_search_active());
        assert_eq!(state.search_direction, SearchDirection::Forward);
        assert_eq!(state.search_query, "");
    }

    #[test]
    fn question_mark_activates_search_input_backward() {
        let mut state = ViState::new(5, 3);
        let action = state.process_key('?', false);
        assert_eq!(action, ViAction::SearchBackward);
        assert!(state.is_search_active());
        assert_eq!(state.search_direction, SearchDirection::Backward);
    }

    #[test]
    fn search_input_collects_characters() {
        let mut state = ViState::new(5, 3);
        state.process_key('/', false);
        state.process_key('h', false);
        state.process_key('e', false);
        state.process_key('l', false);
        assert_eq!(state.search_query, "hel");
        assert!(state.is_search_active());
    }

    #[test]
    fn search_input_backspace_removes_char() {
        let mut state = ViState::new(5, 3);
        state.process_key('/', false);
        state.process_key('a', false);
        state.process_key('b', false);
        assert_eq!(state.search_query, "ab");
        state.process_key('\x7f', false); // Backspace
        assert_eq!(state.search_query, "a");
    }

    #[test]
    fn search_input_enter_confirms() {
        let mut state = ViState::new(5, 3);
        state.process_key('/', false);
        state.process_key('f', false);
        state.process_key('o', false);
        state.process_key('o', false);
        let action = state.process_key('\r', false); // Enter
        assert_eq!(action, ViAction::SearchExecute);
        assert!(!state.is_search_active());
        assert_eq!(state.search_query, "foo"); // query preserved after confirm
    }

    #[test]
    fn search_input_escape_cancels() {
        let mut state = ViState::new(5, 3);
        state.process_key('/', false);
        state.process_key('f', false);
        state.process_key('o', false);
        let action = state.process_key('\x1b', false); // Escape
        assert_eq!(action, ViAction::SearchCancel);
        assert!(!state.is_search_active());
        assert_eq!(state.search_query, ""); // query cleared on cancel
    }

    #[test]
    fn search_prompt_forward() {
        let mut state = ViState::new(5, 3);
        state.process_key('/', false);
        state.process_key('h', false);
        state.process_key('i', false);
        assert_eq!(state.search_prompt(), "/ hi");
    }

    #[test]
    fn search_prompt_backward() {
        let mut state = ViState::new(5, 3);
        state.process_key('?', false);
        state.process_key('x', false);
        assert_eq!(state.search_prompt(), "? x");
    }

    #[test]
    fn search_input_ignores_ctrl_keys() {
        let mut state = ViState::new(5, 3);
        state.process_key('/', false);
        state.process_key('a', false);
        let action = state.process_key('c', true); // Ctrl+C during search
        assert_eq!(action, ViAction::None);
        assert_eq!(state.search_query, "a"); // unchanged
        assert!(state.is_search_active());
    }

    #[test]
    fn move_to_match_updates_cursor() {
        let mut state = ViState::new(5, 3);
        state.move_to_match(10, 7);
        assert_eq!(state.cursor, CursorPos { row: 10, col: 7 });
    }

    #[test]
    fn search_from_visual_mode() {
        let mut state = ViState::new(5, 3);
        state.process_key('v', false); // enter Visual
        let action = state.process_key('/', false);
        assert_eq!(action, ViAction::SearchForward);
        assert!(state.is_search_active());
        assert_eq!(state.mode, ViMode::Visual); // still in visual mode
    }

    #[test]
    fn n_after_search_returns_next_match() {
        let mut state = ViState::new(5, 3);
        state.process_key('/', false);
        state.process_key('x', false);
        state.process_key('\r', false); // confirm search
        let action = state.process_key('n', false);
        assert_eq!(action, ViAction::NextMatch);
        assert_eq!(state.search_direction, SearchDirection::Forward);
    }

    #[test]
    fn uppercase_n_after_search_returns_prev_match() {
        let mut state = ViState::new(5, 3);
        state.process_key('?', false);
        state.process_key('y', false);
        state.process_key('\r', false); // confirm search
        let action = state.process_key('N', false);
        assert_eq!(action, ViAction::PrevMatch);
        assert_eq!(state.search_direction, SearchDirection::Backward);
    }

    #[test]
    fn new_search_clears_previous_query() {
        let mut state = ViState::new(5, 3);
        state.process_key('/', false);
        state.process_key('o', false);
        state.process_key('l', false);
        state.process_key('d', false);
        state.process_key('\r', false); // confirm
        assert_eq!(state.search_query, "old");
        state.process_key('/', false); // new search
        assert_eq!(state.search_query, ""); // cleared
        state.process_key('n', false);
        state.process_key('e', false);
        state.process_key('w', false);
        state.process_key('\r', false);
        assert_eq!(state.search_query, "new");
    }

    // ── Vi cursor flag ────────────────────────────────────────────────

    #[test]
    fn apply_vi_cursor_flag_sets_flag_at_cursor() {
        let mut cells = make_cells(&["hello world"], 20);
        let state = ViState::new(0, 3);
        state.apply_vi_cursor_flag(&mut cells, 20, 0);
        assert_ne!(cells[3].flags & CELL_FLAG_VI_CURSOR, 0);
    }

    #[test]
    fn apply_vi_cursor_flag_does_not_set_other_cells() {
        let mut cells = make_cells(&["hello world"], 20);
        let state = ViState::new(0, 3);
        state.apply_vi_cursor_flag(&mut cells, 20, 0);
        assert_eq!(cells[0].flags & CELL_FLAG_VI_CURSOR, 0);
        assert_eq!(cells[4].flags & CELL_FLAG_VI_CURSOR, 0);
    }

    #[test]
    fn apply_vi_cursor_flag_with_viewport_offset() {
        let mut cells = make_cells(&["line0", "line1", "line2"], 10);
        let state = ViState::new(5, 2); // cursor at row 5
        state.apply_vi_cursor_flag(&mut cells, 10, 4); // viewport starts at row 4
        // Row 5 relative to viewport_top 4 = row 1 in cells
        let idx = 1 * 10 + 2;
        assert_ne!(cells[idx].flags & CELL_FLAG_VI_CURSOR, 0);
    }

    #[test]
    fn apply_vi_cursor_flag_out_of_viewport_is_noop() {
        let mut cells = make_cells(&["hello"], 10);
        let state = ViState::new(0, 3); // cursor at row 0
        state.apply_vi_cursor_flag(&mut cells, 10, 5); // viewport starts at row 5
        // cursor row 0 < viewport_top 5, so no flag set
        for cell in &cells {
            assert_eq!(cell.flags & CELL_FLAG_VI_CURSOR, 0);
        }
    }

    #[test]
    fn clear_vi_cursor_flags_removes_all() {
        let mut cells = make_cells(&["hello world"], 20);
        cells[3].flags |= CELL_FLAG_VI_CURSOR;
        cells[10].flags |= CELL_FLAG_VI_CURSOR;
        clear_vi_cursor_flags(&mut cells);
        for cell in &cells {
            assert_eq!(cell.flags & CELL_FLAG_VI_CURSOR, 0);
        }
    }

    #[test]
    fn vi_cursor_flag_does_not_interfere_with_selection() {
        use crate::renderer::grid_renderer::CELL_FLAG_SELECTED;
        let mut cells = make_cells(&["hello"], 10);
        cells[3].flags |= CELL_FLAG_SELECTED;
        let state = ViState::new(0, 3);
        state.apply_vi_cursor_flag(&mut cells, 10, 0);
        // Both flags should be set
        assert_ne!(cells[3].flags & CELL_FLAG_SELECTED, 0);
        assert_ne!(cells[3].flags & CELL_FLAG_VI_CURSOR, 0);
    }

    // ── Status bar text ───────────────────────────────────────────────

    #[test]
    fn status_text_normal_mode() {
        let state = ViState::new(0, 0);
        assert_eq!(state.status_text(), "-- NORMAL --");
    }

    #[test]
    fn status_text_visual_mode() {
        let mut state = ViState::new(0, 0);
        state.process_key('v', false);
        assert_eq!(state.status_text(), "-- VISUAL --");
    }

    #[test]
    fn status_text_visual_line_mode() {
        let mut state = ViState::new(0, 0);
        state.process_key('V', false);
        assert_eq!(state.status_text(), "-- VISUAL LINE --");
    }

    #[test]
    fn status_text_visual_block_mode() {
        let mut state = ViState::new(0, 0);
        state.process_key('v', true);
        assert_eq!(state.status_text(), "-- VISUAL BLOCK --");
    }

    #[test]
    fn status_text_during_search_shows_prompt() {
        let mut state = ViState::new(0, 0);
        state.process_key('/', false);
        state.process_key('f', false);
        state.process_key('o', false);
        assert_eq!(state.status_text(), "/ fo");
    }

    #[test]
    fn status_text_after_search_confirm_shows_mode() {
        let mut state = ViState::new(0, 0);
        state.process_key('/', false);
        state.process_key('x', false);
        state.process_key('\r', false);
        assert_eq!(state.status_text(), "-- NORMAL --");
    }
}

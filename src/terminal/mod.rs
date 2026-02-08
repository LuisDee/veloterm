// Terminal state machine: wraps alacritty_terminal for VT parsing and grid state.

pub mod grid_bridge;

use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::term::Config;
use alacritty_terminal::vte::ansi;

use crate::shell_integration::listener::{
    self, EventQueue, TerminalEvent, VeloTermListener,
};
use crate::shell_integration::{self, ShellEvent, ShellState};

/// Terminal dimensions for alacritty_terminal.
pub struct TermSize {
    pub columns: usize,
    pub screen_lines: usize,
}

impl Dimensions for TermSize {
    fn total_lines(&self) -> usize {
        self.screen_lines
    }
    fn screen_lines(&self) -> usize {
        self.screen_lines
    }
    fn columns(&self) -> usize {
        self.columns
    }
}

/// Wrapper around alacritty_terminal providing VT parsing and grid state.
pub struct Terminal {
    term: alacritty_terminal::term::Term<VeloTermListener>,
    processor: ansi::Processor,
    event_queue: EventQueue,
    shell_state: ShellState,
}

impl Terminal {
    /// Create a new terminal with the given dimensions and scrollback.
    pub fn new(cols: usize, rows: usize, scrollback: usize) -> Self {
        let size = TermSize {
            columns: cols,
            screen_lines: rows,
        };
        let config = Config {
            scrolling_history: scrollback,
            ..Config::default()
        };
        let (veloterm_listener, event_queue) = listener::create_listener();
        let term = alacritty_terminal::term::Term::new(config, &size, veloterm_listener);
        let processor = ansi::Processor::new();
        Self {
            term,
            processor,
            event_queue,
            shell_state: ShellState::new(),
        }
    }

    /// Feed raw bytes from the PTY into the terminal parser.
    /// Also extracts shell integration events (OSC 7, OSC 133) from the byte stream
    /// and processes any title events from the event listener.
    pub fn feed(&mut self, bytes: &[u8]) {
        // Pre-scan for OSC 7 and OSC 133 sequences before alacritty_terminal processes them
        let shell_events = shell_integration::extract_shell_events(bytes);

        // Feed to alacritty_terminal for normal VT processing
        self.processor.advance(&mut self.term, bytes);

        // Process shell events from byte pre-scan
        let current_line = self.cursor_position().0 + self.history_size();
        for event in &shell_events {
            self.shell_state.handle_event(event, current_line);
        }

        // Process title events from the event listener
        let listener_events = listener::drain_events(&self.event_queue);
        for event in listener_events {
            match event {
                TerminalEvent::TitleChanged(title) => {
                    self.shell_state
                        .handle_event(&ShellEvent::Title(title), current_line);
                }
                TerminalEvent::TitleReset => {
                    self.shell_state.title = None;
                    self.shell_state.title_is_explicit = false;
                }
                TerminalEvent::Bell => {
                    // Bell events can be handled later for notifications
                }
            }
        }
    }

    /// Get the character at a grid position (row, col). Row 0 is top of screen.
    pub fn cell_char(&self, row: usize, col: usize) -> char {
        let point = Point::new(Line(row as i32), Column(col));
        self.term.grid()[point].c
    }

    /// Get the number of columns.
    pub fn columns(&self) -> usize {
        self.term.grid().columns()
    }

    /// Get the number of screen lines (rows).
    pub fn rows(&self) -> usize {
        self.term.grid().screen_lines()
    }

    /// Access the inner alacritty_terminal Term for grid iteration.
    pub fn inner(&self) -> &alacritty_terminal::term::Term<VeloTermListener> {
        &self.term
    }

    /// Get the cursor position as (row, col).
    pub fn cursor_position(&self) -> (usize, usize) {
        let content = self.term.renderable_content();
        let cursor = content.cursor;
        (cursor.point.line.0 as usize, cursor.point.column.0)
    }

    /// Get the current display offset (0 = bottom, increases when scrolled up).
    pub fn display_offset(&self) -> usize {
        self.term.grid().display_offset()
    }

    /// Get the number of lines in scrollback history.
    pub fn history_size(&self) -> usize {
        self.term.grid().history_size()
    }

    /// Scroll up (toward history) by the given number of lines.
    pub fn scroll_up(&mut self, lines: i32) {
        self.term.scroll_display(Scroll::Delta(lines));
    }

    /// Scroll down (toward bottom) by the given number of lines.
    pub fn scroll_down(&mut self, lines: i32) {
        self.term.scroll_display(Scroll::Delta(-lines));
    }

    /// Scroll up by one page (screen height).
    pub fn scroll_page_up(&mut self) {
        self.term.scroll_display(Scroll::PageUp);
    }

    /// Scroll down by one page (screen height).
    pub fn scroll_page_down(&mut self) {
        self.term.scroll_display(Scroll::PageDown);
    }

    /// Snap the viewport to the bottom (most recent output).
    pub fn snap_to_bottom(&mut self) {
        self.term.scroll_display(Scroll::Bottom);
    }

    /// Set the display offset directly (0 = bottom, positive = scrolled up).
    /// Clamps to the maximum available scrollback.
    pub fn set_display_offset(&mut self, offset: usize) {
        self.term.scroll_display(Scroll::Bottom);
        if offset > 0 {
            self.term.scroll_display(Scroll::Delta(offset as i32));
        }
    }

    /// Resize the terminal grid to new dimensions. Triggers content reflow.
    pub fn resize(&mut self, cols: usize, rows: usize) {
        let size = TermSize {
            columns: cols,
            screen_lines: rows,
        };
        self.term.resize(size);
    }

    /// Access the shell state for this terminal.
    pub fn shell_state(&self) -> &ShellState {
        &self.shell_state
    }

    /// Access the shell state mutably.
    pub fn shell_state_mut(&mut self) -> &mut ShellState {
        &mut self.shell_state
    }
}

/// Coalesces rapid resize events, keeping only the latest pending size.
#[derive(Default)]
pub struct ResizeDebouncer {
    pending: Option<(usize, usize)>,
}

impl ResizeDebouncer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a resize request. Overwrites any previous pending request.
    pub fn request(&mut self, cols: usize, rows: usize) {
        self.pending = Some((cols, rows));
    }

    /// Get the pending resize, if any.
    pub fn pending(&self) -> Option<(usize, usize)> {
        self.pending
    }

    /// Clear the pending resize after it has been applied.
    pub fn clear(&mut self) {
        self.pending = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Term creation ────────────────────────────────────────────────

    #[test]
    fn terminal_creates_with_correct_columns() {
        let term = Terminal::new(80, 24, 10_000);
        assert_eq!(term.columns(), 80);
    }

    #[test]
    fn terminal_creates_with_correct_rows() {
        let term = Terminal::new(80, 24, 10_000);
        assert_eq!(term.rows(), 24);
    }

    #[test]
    fn terminal_creates_with_custom_dimensions() {
        let term = Terminal::new(120, 40, 10_000);
        assert_eq!(term.columns(), 120);
        assert_eq!(term.rows(), 40);
    }

    // ── Feeding bytes / grid state ───────────────────────────────────

    #[test]
    fn feed_hello_places_chars_in_row_0() {
        let mut term = Terminal::new(80, 24, 10_000);
        term.feed(b"Hello");
        assert_eq!(term.cell_char(0, 0), 'H');
        assert_eq!(term.cell_char(0, 1), 'e');
        assert_eq!(term.cell_char(0, 2), 'l');
        assert_eq!(term.cell_char(0, 3), 'l');
        assert_eq!(term.cell_char(0, 4), 'o');
    }

    #[test]
    fn feed_empty_cells_contain_space() {
        let term = Terminal::new(80, 24, 10_000);
        assert_eq!(term.cell_char(0, 0), ' ');
    }

    #[test]
    fn feed_newline_moves_to_next_row() {
        let mut term = Terminal::new(80, 24, 10_000);
        term.feed(b"AB\r\nCD");
        assert_eq!(term.cell_char(0, 0), 'A');
        assert_eq!(term.cell_char(0, 1), 'B');
        assert_eq!(term.cell_char(1, 0), 'C');
        assert_eq!(term.cell_char(1, 1), 'D');
    }

    // ── Cursor position tracking ─────────────────────────────────────

    #[test]
    fn cursor_starts_at_origin() {
        let term = Terminal::new(80, 24, 10_000);
        assert_eq!(term.cursor_position(), (0, 0));
    }

    #[test]
    fn cursor_advances_after_text() {
        let mut term = Terminal::new(80, 24, 10_000);
        term.feed(b"Hello");
        assert_eq!(term.cursor_position(), (0, 5));
    }

    #[test]
    fn cursor_moves_to_next_line_after_newline() {
        let mut term = Terminal::new(80, 24, 10_000);
        term.feed(b"Hello\r\n");
        assert_eq!(term.cursor_position(), (1, 0));
    }

    // ── Scrollback and viewport ────────────────────────────────────────

    /// Helper: feed numbered lines to overflow the screen into scrollback.
    fn feed_overflow_lines(term: &mut Terminal, total_lines: usize) {
        for i in 0..total_lines {
            term.feed(format!("line{}\r\n", i).as_bytes());
        }
    }

    #[test]
    fn display_offset_starts_at_zero() {
        let term = Terminal::new(80, 24, 10_000);
        assert_eq!(term.display_offset(), 0);
    }

    #[test]
    fn history_size_starts_at_zero() {
        let term = Terminal::new(80, 24, 10_000);
        assert_eq!(term.history_size(), 0);
    }

    #[test]
    fn scrollback_accumulates_lines() {
        let mut term = Terminal::new(80, 5, 10_000);
        feed_overflow_lines(&mut term, 10);
        assert!(term.history_size() > 0);
    }

    #[test]
    fn scroll_up_increases_display_offset() {
        let mut term = Terminal::new(80, 5, 10_000);
        feed_overflow_lines(&mut term, 10);
        term.scroll_up(1);
        assert_eq!(term.display_offset(), 1);
    }

    #[test]
    fn scroll_down_decreases_display_offset() {
        let mut term = Terminal::new(80, 5, 10_000);
        feed_overflow_lines(&mut term, 10);
        term.scroll_up(3);
        term.scroll_down(1);
        assert_eq!(term.display_offset(), 2);
    }

    #[test]
    fn scroll_page_up_scrolls_by_page() {
        let mut term = Terminal::new(80, 5, 10_000);
        feed_overflow_lines(&mut term, 20);
        term.scroll_page_up();
        assert!(term.display_offset() > 0);
    }

    #[test]
    fn scroll_page_down_reduces_offset() {
        let mut term = Terminal::new(80, 5, 10_000);
        feed_overflow_lines(&mut term, 20);
        term.scroll_page_up();
        let offset_after_up = term.display_offset();
        term.scroll_page_down();
        assert!(term.display_offset() < offset_after_up);
    }

    #[test]
    fn snap_to_bottom_resets_offset() {
        let mut term = Terminal::new(80, 5, 10_000);
        feed_overflow_lines(&mut term, 10);
        term.scroll_up(3);
        assert!(term.display_offset() > 0);
        term.snap_to_bottom();
        assert_eq!(term.display_offset(), 0);
    }

    #[test]
    fn scroll_up_clamped_to_history() {
        let mut term = Terminal::new(80, 5, 10_000);
        feed_overflow_lines(&mut term, 10);
        term.scroll_up(99999);
        assert!(term.display_offset() <= term.history_size());
    }

    // ── Terminal resize ────────────────────────────────────────────────

    #[test]
    fn resize_changes_columns() {
        let mut term = Terminal::new(80, 24, 10_000);
        term.resize(120, 40);
        assert_eq!(term.columns(), 120);
    }

    #[test]
    fn resize_changes_rows() {
        let mut term = Terminal::new(80, 24, 10_000);
        term.resize(120, 40);
        assert_eq!(term.rows(), 40);
    }

    #[test]
    fn resize_preserves_content() {
        let mut term = Terminal::new(80, 24, 10_000);
        term.feed(b"Hello");
        term.resize(120, 40);
        // Content should still be present after resize
        assert_eq!(term.cell_char(0, 0), 'H');
        assert_eq!(term.cell_char(0, 4), 'o');
    }

    // ── Resize debouncer ───────────────────────────────────────────────

    #[test]
    fn debouncer_no_pending_initially() {
        let d = ResizeDebouncer::new();
        assert_eq!(d.pending(), None);
    }

    #[test]
    fn debouncer_stores_latest_request() {
        let mut d = ResizeDebouncer::new();
        d.request(80, 24);
        d.request(120, 40);
        assert_eq!(d.pending(), Some((120, 40)));
    }

    #[test]
    fn debouncer_clear_removes_pending() {
        let mut d = ResizeDebouncer::new();
        d.request(80, 24);
        d.clear();
        assert_eq!(d.pending(), None);
    }

    // ── Shell integration via Terminal ──────────────────────────────────

    #[test]
    fn shell_state_starts_empty() {
        let term = Terminal::new(80, 24, 10_000);
        assert!(term.shell_state().cwd.is_none());
        assert!(term.shell_state().title.is_none());
    }

    #[test]
    fn feed_osc2_title_updates_shell_state() {
        let mut term = Terminal::new(80, 24, 10_000);
        // Feed an OSC 2 title sequence
        term.feed(b"\x1b]2;my-project\x07");
        assert_eq!(term.shell_state().title.as_deref(), Some("my-project"));
        assert!(term.shell_state().title_is_explicit);
    }

    #[test]
    fn feed_osc7_updates_cwd() {
        let mut term = Terminal::new(80, 24, 10_000);
        term.feed(b"\x1b]7;file://localhost/home/user/projects\x07");
        assert_eq!(
            term.shell_state().cwd.as_deref(),
            Some("/home/user/projects")
        );
    }

    #[test]
    fn feed_osc133a_records_prompt_position() {
        let mut term = Terminal::new(80, 24, 10_000);
        term.feed(b"\x1b]133;A\x07");
        assert_eq!(term.shell_state().prompt_positions().len(), 1);
    }
}

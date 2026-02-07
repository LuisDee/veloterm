// Terminal state machine: wraps alacritty_terminal for VT parsing and grid state.

pub mod grid_bridge;

use alacritty_terminal::event::VoidListener;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::term::Config;
use alacritty_terminal::vte::ansi;

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
    term: alacritty_terminal::term::Term<VoidListener>,
    processor: ansi::Processor,
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
        let term = alacritty_terminal::term::Term::new(config, &size, VoidListener);
        let processor = ansi::Processor::new();
        Self { term, processor }
    }

    /// Feed raw bytes from the PTY into the terminal parser.
    pub fn feed(&mut self, bytes: &[u8]) {
        self.processor.advance(&mut self.term, bytes);
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
    pub fn inner(&self) -> &alacritty_terminal::term::Term<VoidListener> {
        &self.term
    }

    /// Get the cursor position as (row, col).
    pub fn cursor_position(&self) -> (usize, usize) {
        let content = self.term.renderable_content();
        let cursor = content.cursor;
        (cursor.point.line.0 as usize, cursor.point.column.0)
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
}

// Grid bridge: converts alacritty_terminal grid state to renderer GridCell data.

use crate::config::theme::Color;
use crate::renderer::grid_renderer::{GridCell, CELL_FLAG_STRIKETHROUGH, CELL_FLAG_UNDERLINE};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::term::cell::Flags as CellFlags;
use alacritty_terminal::vte::ansi::Color as AnsiColor;
use alacritty_terminal::vte::ansi::NamedColor;

/// Default foreground color (Claude Dark text_primary).
pub const DEFAULT_FG: Color = Color::new(0.9098, 0.8980, 0.8745, 1.0); // #E8E5DF

/// Default background color (Claude Dark background).
pub const DEFAULT_BG: Color = Color::new(0.1020, 0.0941, 0.0863, 1.0); // #1A1816

/// Standard 16 ANSI colors mapped to Claude Dark theme values.
pub fn ansi_named_color(name: NamedColor) -> Color {
    match name {
        NamedColor::Black => Color::new(0.07, 0.07, 0.07, 1.0),
        NamedColor::Red => Color::new(0.80, 0.27, 0.27, 1.0),
        NamedColor::Green => Color::new(0.36, 0.67, 0.36, 1.0),
        NamedColor::Yellow => Color::new(0.80, 0.73, 0.35, 1.0),
        NamedColor::Blue => Color::new(0.35, 0.55, 0.80, 1.0),
        NamedColor::Magenta => Color::new(0.70, 0.40, 0.75, 1.0),
        NamedColor::Cyan => Color::new(0.35, 0.73, 0.73, 1.0),
        NamedColor::White => Color::new(0.80, 0.78, 0.74, 1.0),
        NamedColor::BrightBlack => Color::new(0.40, 0.38, 0.35, 1.0),
        NamedColor::BrightRed => Color::new(0.91, 0.44, 0.44, 1.0),
        NamedColor::BrightGreen => Color::new(0.50, 0.82, 0.50, 1.0),
        NamedColor::BrightYellow => Color::new(0.91, 0.84, 0.50, 1.0),
        NamedColor::BrightBlue => Color::new(0.50, 0.70, 0.91, 1.0),
        NamedColor::BrightMagenta => Color::new(0.82, 0.55, 0.87, 1.0),
        NamedColor::BrightCyan => Color::new(0.50, 0.85, 0.85, 1.0),
        NamedColor::BrightWhite => Color::new(0.95, 0.93, 0.90, 1.0),
        // Foreground/Background/Cursor use defaults
        _ => DEFAULT_FG,
    }
}

/// Map a 256-color index to an RGBA color.
pub fn ansi_indexed_color(index: u8) -> Color {
    match index {
        // 0-7: standard named colors
        0 => ansi_named_color(NamedColor::Black),
        1 => ansi_named_color(NamedColor::Red),
        2 => ansi_named_color(NamedColor::Green),
        3 => ansi_named_color(NamedColor::Yellow),
        4 => ansi_named_color(NamedColor::Blue),
        5 => ansi_named_color(NamedColor::Magenta),
        6 => ansi_named_color(NamedColor::Cyan),
        7 => ansi_named_color(NamedColor::White),
        // 8-15: bright named colors
        8 => ansi_named_color(NamedColor::BrightBlack),
        9 => ansi_named_color(NamedColor::BrightRed),
        10 => ansi_named_color(NamedColor::BrightGreen),
        11 => ansi_named_color(NamedColor::BrightYellow),
        12 => ansi_named_color(NamedColor::BrightBlue),
        13 => ansi_named_color(NamedColor::BrightMagenta),
        14 => ansi_named_color(NamedColor::BrightCyan),
        15 => ansi_named_color(NamedColor::BrightWhite),
        // 16-231: 6x6x6 RGB color cube
        16..=231 => {
            let idx = index - 16;
            let r_idx = idx / 36;
            let g_idx = (idx % 36) / 6;
            let b_idx = idx % 6;
            let r = if r_idx == 0 { 0 } else { 55 + 40 * r_idx };
            let g = if g_idx == 0 { 0 } else { 55 + 40 * g_idx };
            let b = if b_idx == 0 { 0 } else { 55 + 40 * b_idx };
            Color::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
        }
        // 232-255: 24-step grayscale
        232..=255 => {
            let v = (8 + 10 * (index - 232) as u16) as f32 / 255.0;
            Color::new(v, v, v, 1.0)
        }
    }
}

/// Convert an alacritty_terminal Color to our renderer Color.
pub fn convert_color(ansi: AnsiColor, _default: Color) -> Color {
    match ansi {
        AnsiColor::Named(name) => {
            // Foreground/Background named variants use defaults
            match name {
                NamedColor::Foreground => DEFAULT_FG,
                NamedColor::Background => DEFAULT_BG,
                _ => ansi_named_color(name),
            }
        }
        AnsiColor::Spec(rgb) => Color::new(
            rgb.r as f32 / 255.0,
            rgb.g as f32 / 255.0,
            rgb.b as f32 / 255.0,
            1.0,
        ),
        AnsiColor::Indexed(idx) => ansi_indexed_color(idx),
    }
}

/// Map a normal named color to its bright variant when bold is active.
/// Already-bright colors and special colors (Foreground, Background) are unchanged.
pub fn bold_brighten_named(name: NamedColor) -> NamedColor {
    match name {
        NamedColor::Black => NamedColor::BrightBlack,
        NamedColor::Red => NamedColor::BrightRed,
        NamedColor::Green => NamedColor::BrightGreen,
        NamedColor::Yellow => NamedColor::BrightYellow,
        NamedColor::Blue => NamedColor::BrightBlue,
        NamedColor::Magenta => NamedColor::BrightMagenta,
        NamedColor::Cyan => NamedColor::BrightCyan,
        NamedColor::White => NamedColor::BrightWhite,
        other => other,
    }
}

/// Reduce color intensity for the DIM/faint attribute (~33% reduction).
pub fn apply_dim(color: Color) -> Color {
    Color::new(color.r * 0.66, color.g * 0.66, color.b * 0.66, color.a)
}

/// Extract text content from a Terminal as one String per visible row.
/// Used for link detection scanning. Trailing spaces are preserved so
/// column indices in the returned strings match grid column positions.
pub fn extract_text_lines(terminal: &super::Terminal) -> Vec<String> {
    let term = terminal.inner();
    let grid = term.grid();
    let cols = grid.columns();
    let rows = grid.screen_lines();
    let offset = grid.display_offset() as i32;
    let mut lines = Vec::with_capacity(rows);

    for row in 0..rows {
        let mut line = String::with_capacity(cols);
        for col in 0..cols {
            let point = Point::new(Line(row as i32 - offset), Column(col));
            line.push(grid[point].c);
        }
        lines.push(line);
    }

    lines
}

/// Extract GridCell data from a Terminal for the current viewport.
/// When scrolled up, reads from scrollback history; at bottom, reads the active screen.
pub fn extract_grid_cells(terminal: &super::Terminal) -> Vec<GridCell> {
    let term = terminal.inner();
    let grid = term.grid();
    let cols = grid.columns();
    let rows = grid.screen_lines();
    let offset = grid.display_offset() as i32;
    let mut cells = Vec::with_capacity(cols * rows);

    for row in 0..rows {
        for col in 0..cols {
            let point = Point::new(Line(row as i32 - offset), Column(col));
            let cell = &grid[point];
            let ch = cell.c;
            let cell_flags = cell.flags;

            // Convert base colors, applying bold→bright for named colors
            let mut fg = if cell_flags.contains(CellFlags::BOLD) {
                match cell.fg {
                    AnsiColor::Named(name) => ansi_named_color(bold_brighten_named(name)),
                    other => convert_color(other, DEFAULT_FG),
                }
            } else {
                convert_color(cell.fg, DEFAULT_FG)
            };
            let mut bg = convert_color(cell.bg, DEFAULT_BG);

            // Apply dim: reduce fg intensity
            if cell_flags.contains(CellFlags::DIM) {
                fg = apply_dim(fg);
            }

            // Apply inverse: swap fg and bg
            if cell_flags.contains(CellFlags::INVERSE) {
                std::mem::swap(&mut fg, &mut bg);
            }

            // Propagate underline and strikethrough flags
            let mut flags = 0u32;
            if cell_flags.intersects(CellFlags::UNDERLINE) {
                flags |= CELL_FLAG_UNDERLINE;
            }
            if cell_flags.contains(CellFlags::STRIKEOUT) {
                flags |= CELL_FLAG_STRIKETHROUGH;
            }

            let mut grid_cell = GridCell::new(ch, fg, bg);
            grid_cell.flags = flags;
            cells.push(grid_cell);
        }
    }

    cells
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::Terminal;
    use alacritty_terminal::vte::ansi::Rgb;

    // ── Named ANSI color mapping ─────────────────────────────────────

    #[test]
    fn named_black_maps_to_dark_color() {
        let color = ansi_named_color(NamedColor::Black);
        assert!(color.r < 0.2 && color.g < 0.2 && color.b < 0.2);
    }

    #[test]
    fn named_red_has_red_dominant() {
        let color = ansi_named_color(NamedColor::Red);
        assert!(color.r > color.g && color.r > color.b);
    }

    #[test]
    fn named_white_maps_to_light_color() {
        let color = ansi_named_color(NamedColor::White);
        assert!(color.r > 0.7 && color.g > 0.7 && color.b > 0.7);
    }

    #[test]
    fn named_bright_red_brighter_than_normal_red() {
        let normal = ansi_named_color(NamedColor::Red);
        let bright = ansi_named_color(NamedColor::BrightRed);
        assert!(bright.r >= normal.r);
    }

    // ── 256-color indexed palette ────────────────────────────────────

    #[test]
    fn indexed_0_through_7_match_named_normal() {
        let indexed = ansi_indexed_color(0);
        let named = ansi_named_color(NamedColor::Black);
        assert_eq!(indexed.r, named.r);
        assert_eq!(indexed.g, named.g);
        assert_eq!(indexed.b, named.b);
    }

    #[test]
    fn indexed_8_through_15_match_named_bright() {
        let indexed = ansi_indexed_color(8);
        let named = ansi_named_color(NamedColor::BrightBlack);
        assert_eq!(indexed.r, named.r);
        assert_eq!(indexed.g, named.g);
        assert_eq!(indexed.b, named.b);
    }

    #[test]
    fn indexed_16_is_pure_black() {
        let color = ansi_indexed_color(16);
        assert!(color.r.abs() < 0.01 && color.g.abs() < 0.01 && color.b.abs() < 0.01);
    }

    #[test]
    fn indexed_231_is_pure_white() {
        let color = ansi_indexed_color(231);
        assert!((color.r - 1.0).abs() < 0.01);
        assert!((color.g - 1.0).abs() < 0.01);
        assert!((color.b - 1.0).abs() < 0.01);
    }

    #[test]
    fn indexed_232_to_255_are_grayscale() {
        let dark = ansi_indexed_color(232);
        let light = ansi_indexed_color(255);
        assert!(dark.r < light.r);
        assert!((dark.r - dark.g).abs() < 0.01);
        assert!((dark.r - dark.b).abs() < 0.01);
    }

    // ── Color conversion (AnsiColor → Color) ─────────────────────────

    #[test]
    fn convert_named_color_uses_palette() {
        let color = convert_color(AnsiColor::Named(NamedColor::Red), DEFAULT_FG);
        let expected = ansi_named_color(NamedColor::Red);
        assert_eq!(color.r, expected.r);
    }

    #[test]
    fn convert_indexed_color_uses_palette() {
        let color = convert_color(AnsiColor::Indexed(196), DEFAULT_FG);
        let expected = ansi_indexed_color(196);
        assert_eq!(color.r, expected.r);
    }

    #[test]
    fn convert_rgb_passthrough() {
        let color = convert_color(
            AnsiColor::Spec(Rgb {
                r: 128,
                g: 64,
                b: 32,
            }),
            DEFAULT_FG,
        );
        assert!((color.r - 128.0 / 255.0).abs() < 0.01);
        assert!((color.g - 64.0 / 255.0).abs() < 0.01);
        assert!((color.b - 32.0 / 255.0).abs() < 0.01);
    }

    // ── Default color handling ────────────────────────────────────────

    #[test]
    fn default_fg_matches_theme_text_primary() {
        assert!((DEFAULT_FG.r - 0.9098).abs() < 0.01);
    }

    #[test]
    fn default_bg_matches_theme_background() {
        assert!((DEFAULT_BG.r - 0.1020).abs() < 0.01);
    }

    // ── Grid extraction ──────────────────────────────────────────────

    #[test]
    fn extract_empty_grid_has_correct_count() {
        let term = Terminal::new(80, 24, 10_000);
        let cells = extract_grid_cells(&term);
        assert_eq!(cells.len(), 80 * 24);
    }

    #[test]
    fn extract_empty_grid_cells_are_spaces() {
        let term = Terminal::new(10, 5, 10_000);
        let cells = extract_grid_cells(&term);
        assert_eq!(cells[0].ch, ' ');
    }

    #[test]
    fn extract_grid_after_text_has_characters() {
        let mut term = Terminal::new(80, 24, 10_000);
        term.feed(b"Hello");
        let cells = extract_grid_cells(&term);
        assert_eq!(cells[0].ch, 'H');
        assert_eq!(cells[1].ch, 'e');
        assert_eq!(cells[2].ch, 'l');
        assert_eq!(cells[3].ch, 'l');
        assert_eq!(cells[4].ch, 'o');
    }

    #[test]
    fn extract_grid_default_fg_is_theme_text_primary() {
        let term = Terminal::new(80, 24, 10_000);
        let cells = extract_grid_cells(&term);
        assert!((cells[0].fg.r - DEFAULT_FG.r).abs() < 0.01);
    }

    #[test]
    fn extract_grid_default_bg_is_theme_background() {
        let term = Terminal::new(80, 24, 10_000);
        let cells = extract_grid_cells(&term);
        assert!((cells[0].bg.r - DEFAULT_BG.r).abs() < 0.01);
    }

    // ── Bold → bright color mapping ────────────────────────────────

    #[test]
    fn bold_brighten_red_to_bright_red() {
        assert_eq!(bold_brighten_named(NamedColor::Red), NamedColor::BrightRed);
    }

    #[test]
    fn bold_brighten_black_to_bright_black() {
        assert_eq!(
            bold_brighten_named(NamedColor::Black),
            NamedColor::BrightBlack
        );
    }

    #[test]
    fn bold_brighten_white_to_bright_white() {
        assert_eq!(
            bold_brighten_named(NamedColor::White),
            NamedColor::BrightWhite
        );
    }

    #[test]
    fn bold_brighten_already_bright_unchanged() {
        assert_eq!(
            bold_brighten_named(NamedColor::BrightRed),
            NamedColor::BrightRed
        );
    }

    #[test]
    fn extract_bold_red_text_uses_bright_color() {
        let mut term = Terminal::new(80, 24, 10_000);
        term.feed(b"\x1b[1;31mX");
        let cells = extract_grid_cells(&term);
        let bright_red = ansi_named_color(NamedColor::BrightRed);
        assert!((cells[0].fg.r - bright_red.r).abs() < 0.01);
        assert!((cells[0].fg.g - bright_red.g).abs() < 0.01);
    }

    // ── Dim/faint attribute ────────────────────────────────────────

    #[test]
    fn apply_dim_reduces_rgb() {
        let original = Color::new(0.9, 0.6, 0.3, 1.0);
        let dimmed = apply_dim(original);
        assert!(dimmed.r < original.r);
        assert!(dimmed.g < original.g);
        assert!(dimmed.b < original.b);
    }

    #[test]
    fn apply_dim_preserves_alpha() {
        let original = Color::new(0.9, 0.6, 0.3, 1.0);
        let dimmed = apply_dim(original);
        assert_eq!(dimmed.a, original.a);
    }

    #[test]
    fn extract_dim_text_has_reduced_fg() {
        let mut term = Terminal::new(80, 24, 10_000);
        term.feed(b"\x1b[2mX");
        let cells = extract_grid_cells(&term);
        assert!(cells[0].fg.r < DEFAULT_FG.r);
    }

    // ── Inverse attribute ──────────────────────────────────────────

    #[test]
    fn extract_inverse_swaps_fg_bg() {
        let mut term = Terminal::new(80, 24, 10_000);
        term.feed(b"\x1b[7mX");
        let cells = extract_grid_cells(&term);
        // After inverse: fg becomes the background color, bg becomes the fg color
        assert!((cells[0].fg.r - DEFAULT_BG.r).abs() < 0.01);
        assert!((cells[0].bg.r - DEFAULT_FG.r).abs() < 0.01);
    }

    // ── Underline flag propagation ─────────────────────────────────

    #[test]
    fn extract_underline_sets_flag() {
        let mut term = Terminal::new(80, 24, 10_000);
        term.feed(b"\x1b[4mX");
        let cells = extract_grid_cells(&term);
        assert_ne!(cells[0].flags & CELL_FLAG_UNDERLINE, 0);
    }

    #[test]
    fn extract_no_underline_clears_flag() {
        let mut term = Terminal::new(80, 24, 10_000);
        term.feed(b"X");
        let cells = extract_grid_cells(&term);
        assert_eq!(cells[0].flags & CELL_FLAG_UNDERLINE, 0);
    }

    // ── Strikethrough flag propagation ─────────────────────────────

    #[test]
    fn extract_strikethrough_sets_flag() {
        let mut term = Terminal::new(80, 24, 10_000);
        term.feed(b"\x1b[9mX");
        let cells = extract_grid_cells(&term);
        assert_ne!(cells[0].flags & CELL_FLAG_STRIKETHROUGH, 0);
    }

    #[test]
    fn extract_no_strikethrough_clears_flag() {
        let mut term = Terminal::new(80, 24, 10_000);
        term.feed(b"X");
        let cells = extract_grid_cells(&term);
        assert_eq!(cells[0].flags & CELL_FLAG_STRIKETHROUGH, 0);
    }

    // ── Viewport-aware extraction ────────────────────────────────────

    #[test]
    fn extract_viewport_shows_scrollback_content() {
        let mut term = Terminal::new(10, 3, 10_000);
        // Feed 6 lines: "L0" through "L5" with newlines
        for i in 0..6 {
            term.feed(format!("L{}\r\n", i).as_bytes());
        }
        // Final state: screen=[L4, L5, ""], history=[L0, L1, L2, L3]
        // Scroll up 2: viewport row 0 → Line(-2)=L2, row 1 → Line(-1)=L3, row 2 → Line(0)=L4
        term.scroll_up(2);
        let cells = extract_grid_cells(&term);
        // First row should show "L2"
        assert_eq!(cells[0].ch, 'L');
        assert_eq!(cells[1].ch, '2');
    }

    #[test]
    fn extract_viewport_at_bottom_shows_latest() {
        let mut term = Terminal::new(10, 3, 10_000);
        for i in 0..6 {
            term.feed(format!("L{}\r\n", i).as_bytes());
        }
        // Not scrolled: screen shows [L4, L5, ""]
        let cells = extract_grid_cells(&term);
        assert_eq!(cells[0].ch, 'L');
        assert_eq!(cells[1].ch, '4');
    }

    // ── Text line extraction ────────────────────────────────────────

    #[test]
    fn extract_text_lines_empty_grid() {
        let term = Terminal::new(10, 3, 10_000);
        let lines = extract_text_lines(&term);
        assert_eq!(lines.len(), 3);
        // Each line should be 10 spaces
        assert_eq!(lines[0].len(), 10);
        assert!(lines[0].chars().all(|c| c == ' '));
    }

    #[test]
    fn extract_text_lines_with_content() {
        let mut term = Terminal::new(20, 3, 10_000);
        term.feed(b"Hello world");
        let lines = extract_text_lines(&term);
        assert!(lines[0].starts_with("Hello world"));
        assert_eq!(lines[0].len(), 20); // padded with spaces to column width
    }

    #[test]
    fn extract_text_lines_preserves_column_positions() {
        let mut term = Terminal::new(20, 3, 10_000);
        term.feed(b"abc https://x.com end");
        let lines = extract_text_lines(&term);
        // 'h' of https:// should be at index 4
        assert_eq!(&lines[0][4..17], "https://x.com");
    }

    #[test]
    fn extract_text_lines_multiline() {
        let mut term = Terminal::new(10, 3, 10_000);
        term.feed(b"AAA\r\nBBB\r\nCCC");
        let lines = extract_text_lines(&term);
        assert!(lines[0].starts_with("AAA"));
        assert!(lines[1].starts_with("BBB"));
        assert!(lines[2].starts_with("CCC"));
    }
}

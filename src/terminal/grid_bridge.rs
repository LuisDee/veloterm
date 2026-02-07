// Grid bridge: converts alacritty_terminal grid state to renderer GridCell data.

use crate::config::theme::Color;
use crate::renderer::grid_renderer::GridCell;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point};
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

/// Extract GridCell data from a Terminal for the entire visible grid.
pub fn extract_grid_cells(terminal: &super::Terminal) -> Vec<GridCell> {
    let term = terminal.inner();
    let grid = term.grid();
    let cols = grid.columns();
    let rows = grid.screen_lines();
    let mut cells = Vec::with_capacity(cols * rows);

    for row in 0..rows {
        for col in 0..cols {
            let point = Point::new(Line(row as i32), Column(col));
            let cell = &grid[point];
            let ch = cell.c;
            let fg = convert_color(cell.fg, DEFAULT_FG);
            let bg = convert_color(cell.bg, DEFAULT_BG);
            cells.push(GridCell::new(ch, fg, bg));
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
}

// Header bar: brand bar below the native macOS title bar.

use crate::config::theme::{Color, Theme};
use crate::pane::divider::OverlayQuad;
use crate::pane::Rect;
use crate::renderer::grid_renderer::GridCell;

/// Height of the header bar in physical pixels.
pub const HEADER_BAR_HEIGHT: f32 = 46.0;

/// Generate overlay quads for the header bar background and bottom divider.
pub fn generate_header_bar_quads(window_width: f32, theme: &Theme) -> Vec<OverlayQuad> {
    let mut quads = Vec::new();

    // Background
    quads.push(OverlayQuad {
        rect: Rect::new(0.0, 0.0, window_width, HEADER_BAR_HEIGHT),
        color: [theme.surface.r, theme.surface.g, theme.surface.b, 1.0],
        border_radius: 0.0,
    });

    // Bottom divider (1px)
    quads.push(OverlayQuad {
        rect: Rect::new(0.0, HEADER_BAR_HEIGHT - 1.0, window_width, 1.0),
        color: [theme.border.r, theme.border.g, theme.border.b, 1.0],
        border_radius: 0.0,
    });

    quads
}

/// Generate text cells for the header bar: sparkle icon + brand left, version right.
pub fn generate_header_bar_text_cells(
    window_width: f32,
    cell_width: f32,
    cell_height: f32,
    theme: &Theme,
) -> Option<(Rect, Vec<GridCell>)> {
    if cell_width == 0.0 || cell_height == 0.0 {
        return None;
    }

    let columns = (window_width / cell_width).floor() as usize;
    if columns < 10 {
        return None;
    }

    let text_y = (HEADER_BAR_HEIGHT - cell_height) / 2.0;
    let text_rect = Rect::new(0.0, text_y.max(0.0), window_width, cell_height);

    let bg = Color::new(theme.surface.r, theme.surface.g, theme.surface.b, 0.0);
    let mut cells = vec![GridCell::empty(bg); columns];

    // Left side: sparkle icon (✻) in accent + space + "Claude Terminal" in primary text
    let accent = Color::new(theme.accent.r, theme.accent.g, theme.accent.b, 1.0);
    let text_color = Color::new(theme.text.r, theme.text.g, theme.text.b, 1.0);
    let dim_color = Color::new(theme.text_dim.r, theme.text_dim.g, theme.text_dim.b, 1.0);

    // 2-cell left padding, then sparkle icon, space, brand text
    let left_pad = 2;
    let icon_col = left_pad;
    if icon_col < columns {
        cells[icon_col] = GridCell::new('*', accent, bg);
    }

    let brand = "Claude Terminal";
    let brand_start = icon_col + 2; // space after icon
    for (j, ch) in brand.chars().enumerate() {
        let col = brand_start + j;
        if col < columns {
            cells[col] = GridCell::new(ch, text_color, bg);
        }
    }

    // Right side: "v0.1.0" in dim text, right-aligned with 2-cell right padding
    let version = "v0.1.0";
    let right_pad = 2;
    let version_start = columns.saturating_sub(version.len() + right_pad);
    for (j, ch) in version.chars().enumerate() {
        let col = version_start + j;
        if col < columns {
            cells[col] = GridCell::new(ch, dim_color, bg);
        }
    }

    Some((text_rect, cells))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::theme::Theme;

    #[test]
    fn header_bar_height_constant() {
        assert_eq!(HEADER_BAR_HEIGHT, 46.0);
    }

    #[test]
    fn header_bar_quads_count() {
        let theme = Theme::claude_dark();
        let quads = generate_header_bar_quads(1280.0, &theme);
        // Background + divider
        assert_eq!(quads.len(), 2);
    }

    #[test]
    fn header_bar_quads_dimensions() {
        let theme = Theme::claude_dark();
        let quads = generate_header_bar_quads(1280.0, &theme);
        // Background spans full width and height
        assert_eq!(quads[0].rect.width, 1280.0);
        assert_eq!(quads[0].rect.height, HEADER_BAR_HEIGHT);
        // Divider is 1px at the bottom
        assert_eq!(quads[1].rect.height, 1.0);
        assert_eq!(quads[1].rect.y, HEADER_BAR_HEIGHT - 1.0);
    }

    #[test]
    fn header_bar_text_cells_generated() {
        let theme = Theme::claude_dark();
        let result = generate_header_bar_text_cells(1280.0, 10.0, 20.0, &theme);
        assert!(result.is_some());
        let (rect, cells) = result.unwrap();
        assert_eq!(rect.width, 1280.0);
        assert_eq!(cells.len(), 128); // 1280 / 10
    }

    #[test]
    fn header_bar_text_cells_none_for_zero_cell_width() {
        let theme = Theme::claude_dark();
        let result = generate_header_bar_text_cells(1280.0, 0.0, 20.0, &theme);
        assert!(result.is_none());
    }

    #[test]
    fn header_bar_text_contains_brand() {
        let theme = Theme::claude_dark();
        let (_, cells) = generate_header_bar_text_cells(1280.0, 10.0, 20.0, &theme).unwrap();
        // Brand text starts at column 4 (2 padding + hamburger + space)
        let brand: String = cells[4..19].iter().map(|c| c.ch).collect();
        assert_eq!(brand, "Claude Terminal");
    }

    #[test]
    fn header_bar_text_contains_version() {
        let theme = Theme::claude_dark();
        let (_, cells) = generate_header_bar_text_cells(1280.0, 10.0, 20.0, &theme).unwrap();
        let columns = 128;
        // Version "v0.1.0" right-aligned with 2-cell padding
        let version_start = columns - 6 - 2;
        let version: String = cells[version_start..version_start + 6]
            .iter()
            .map(|c| c.ch)
            .collect();
        assert_eq!(version, "v0.1.0");
    }
}

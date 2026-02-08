// Status bar: bottom bar with brand info, active pane indicator, and session details.

use crate::config::theme::{Color, Theme};
use crate::pane::divider::OverlayQuad;
use crate::pane::Rect;
use crate::renderer::grid_renderer::GridCell;

/// Height of the status bar in physical pixels.
pub const STATUS_BAR_HEIGHT: f32 = 36.0;

/// Generate overlay quads for the status bar background and top divider.
pub fn generate_status_bar_quads(
    window_width: f32,
    window_height: f32,
    theme: &Theme,
) -> Vec<OverlayQuad> {
    let mut quads = Vec::new();
    let bar_y = window_height - STATUS_BAR_HEIGHT;

    // Background
    quads.push(OverlayQuad {
        rect: Rect::new(0.0, bar_y, window_width, STATUS_BAR_HEIGHT),
        color: [theme.surface.r, theme.surface.g, theme.surface.b, 1.0],
        border_radius: 0.0,
    });

    // Top divider (1px)
    quads.push(OverlayQuad {
        rect: Rect::new(0.0, bar_y, window_width, 1.0),
        color: [theme.border.r, theme.border.g, theme.border.b, 1.0],
        border_radius: 0.0,
    });

    quads
}

/// Generate text cells for the status bar.
/// Left: ✻ in accent + "Claude Terminal" in dim.
/// Center: ● green dot + "Pane N" in secondary.
/// Right: session info in dim.
pub fn generate_status_bar_text_cells(
    window_width: f32,
    window_height: f32,
    cell_width: f32,
    cell_height: f32,
    active_pane_index: usize,
    theme: &Theme,
) -> Option<(Rect, Vec<GridCell>)> {
    if cell_width == 0.0 || cell_height == 0.0 {
        return None;
    }

    let columns = (window_width / cell_width).floor() as usize;
    if columns < 10 {
        return None;
    }

    let bar_y = window_height - STATUS_BAR_HEIGHT;
    let text_y = bar_y + (STATUS_BAR_HEIGHT - cell_height) / 2.0;
    let text_rect = Rect::new(0.0, text_y.max(0.0), window_width, cell_height);

    let bg = Color::new(theme.surface.r, theme.surface.g, theme.surface.b, 0.0);
    let mut cells = vec![GridCell::empty(bg); columns];

    let accent = Color::new(theme.accent.r, theme.accent.g, theme.accent.b, 1.0);
    let dim = Color::new(theme.text_dim.r, theme.text_dim.g, theme.text_dim.b, 1.0);
    let secondary = Color::new(
        theme.text_secondary.r,
        theme.text_secondary.g,
        theme.text_secondary.b,
        1.0,
    );
    let success = Color::new(theme.success.r, theme.success.g, theme.success.b, 1.0);

    // Left: * in accent + " Claude Terminal" in dim
    let left_pad = 3;
    if left_pad < columns {
        cells[left_pad] = GridCell::new('*', accent, bg);
    }
    let brand = " Claude Terminal";
    for (j, ch) in brand.chars().enumerate() {
        let col = left_pad + 1 + j;
        if col < columns {
            cells[col] = GridCell::new(ch, dim, bg);
        }
    }

    // Center: ● green dot + " Pane N"
    let pane_label = format!("\u{25CF} Pane {}", active_pane_index + 1);
    let center_start = columns / 2 - pane_label.len() / 2;
    for (j, ch) in pane_label.chars().enumerate() {
        let col = center_start + j;
        if col < columns {
            let color = if j == 0 { success } else { secondary };
            cells[col] = GridCell::new(ch, color, bg);
        }
    }

    // Right: user@host · UTF-8 · shell in dim
    let user = std::env::var("USER").unwrap_or_else(|_| "user".into());
    let right_text = format!("{user} \u{00B7} UTF-8 \u{00B7} bash");
    let right_pad = 3;
    let right_start = columns.saturating_sub(right_text.len() + right_pad);
    for (j, ch) in right_text.chars().enumerate() {
        let col = right_start + j;
        if col < columns {
            cells[col] = GridCell::new(ch, dim, bg);
        }
    }

    Some((text_rect, cells))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::theme::Theme;

    #[test]
    fn status_bar_height_constant() {
        assert_eq!(STATUS_BAR_HEIGHT, 36.0);
    }

    #[test]
    fn status_bar_quads_count() {
        let theme = Theme::claude_dark();
        let quads = generate_status_bar_quads(1280.0, 720.0, &theme);
        assert_eq!(quads.len(), 2); // background + divider
    }

    #[test]
    fn status_bar_quads_position() {
        let theme = Theme::claude_dark();
        let quads = generate_status_bar_quads(1280.0, 720.0, &theme);
        let bar_y = 720.0 - STATUS_BAR_HEIGHT;
        assert_eq!(quads[0].rect.y, bar_y);
        assert_eq!(quads[0].rect.height, STATUS_BAR_HEIGHT);
        assert_eq!(quads[1].rect.y, bar_y); // divider at top of bar
        assert_eq!(quads[1].rect.height, 1.0);
    }

    #[test]
    fn status_bar_text_cells_generated() {
        let theme = Theme::claude_dark();
        let result = generate_status_bar_text_cells(1280.0, 720.0, 10.0, 20.0, 0, &theme);
        assert!(result.is_some());
        let (_, cells) = result.unwrap();
        assert_eq!(cells.len(), 128);
    }

    #[test]
    fn status_bar_text_none_for_zero_cell_width() {
        let theme = Theme::claude_dark();
        let result = generate_status_bar_text_cells(1280.0, 720.0, 0.0, 20.0, 0, &theme);
        assert!(result.is_none());
    }

    #[test]
    fn status_bar_content_bounds_subtracts_height() {
        // Verify the expected content area reduction
        let total_height = 720.0;
        let content_height = total_height - STATUS_BAR_HEIGHT;
        assert_eq!(content_height, 684.0);
    }
}

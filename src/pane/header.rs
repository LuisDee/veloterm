// Pane header: badge, title, status dot, and shell label above each pane.

use crate::config::theme::{Color, Theme};
use crate::pane::divider::OverlayQuad;
use crate::pane::Rect;
use crate::renderer::grid_renderer::GridCell;

/// Height of the pane header in physical pixels.
pub const PANE_HEADER_HEIGHT: f32 = 36.0;

/// Accent stripe height for the active pane header.
const ACTIVE_STRIPE_HEIGHT: f32 = 2.0;

/// Accent stripe height for inactive pane headers.
const INACTIVE_STRIPE_HEIGHT: f32 = 1.0;

/// Get a digit badge for a 0-based pane index.
fn badge_char(index: usize) -> char {
    char::from_digit((index + 1) as u32 % 10, 10).unwrap_or('?')
}

/// Generate overlay quads for a single pane header (background + accent stripe).
pub fn generate_pane_header_quads(
    pane_rect: Rect,
    is_active: bool,
    theme: &Theme,
) -> Vec<OverlayQuad> {
    let mut quads = Vec::new();

    // Rounded pane border: outer filled rounded rect (border color) with inner
    // filled rounded rect (terminal_bg) inset by 1px, creating a 1px border.
    let border_color = if is_active {
        &theme.accent
    } else {
        &theme.border
    };
    let bc = [border_color.r, border_color.g, border_color.b, 1.0];
    // Outer border fill (full pane, 8px radius)
    quads.push(OverlayQuad {
        rect: Rect::new(pane_rect.x, pane_rect.y, pane_rect.width, pane_rect.height),
        color: bc,
        border_radius: 8.0,
    });
    // Inner fill (terminal bg, inset 1px, 7px radius)
    let tbg = &theme.terminal_bg;
    quads.push(OverlayQuad {
        rect: Rect::new(
            pane_rect.x + 1.0,
            pane_rect.y + 1.0,
            pane_rect.width - 2.0,
            pane_rect.height - 2.0,
        ),
        color: [tbg.r, tbg.g, tbg.b, 1.0],
        border_radius: 7.0,
    });

    // Header background (on top of inner fill)
    let bg = if is_active {
        &theme.surface_raised
    } else {
        &theme.surface
    };
    quads.push(OverlayQuad {
        rect: Rect::new(pane_rect.x + 1.0, pane_rect.y + 1.0, pane_rect.width - 2.0, PANE_HEADER_HEIGHT),
        color: [bg.r, bg.g, bg.b, 1.0],
        border_radius: 7.0,
    });

    // Accent stripe below the header (separator between header and terminal content)
    let (stripe_color, stripe_h) = if is_active {
        (&theme.accent, ACTIVE_STRIPE_HEIGHT)
    } else {
        (&theme.border_subtle, INACTIVE_STRIPE_HEIGHT)
    };
    quads.push(OverlayQuad {
        rect: Rect::new(pane_rect.x + 1.0, pane_rect.y + 1.0 + PANE_HEADER_HEIGHT, pane_rect.width - 2.0, stripe_h),
        color: [stripe_color.r, stripe_color.g, stripe_color.b, 1.0],
        border_radius: 0.0,
    });

    quads
}

/// Generate text cells for a single pane header.
/// Layout: [badge] [title] ... [status_dot] [shell_label]
pub fn generate_pane_header_text(
    pane_rect: Rect,
    pane_index: usize,
    title: &str,
    shell_name: &str,
    is_active: bool,
    cell_width: f32,
    cell_height: f32,
    theme: &Theme,
) -> Option<(Rect, Vec<GridCell>)> {
    if cell_width == 0.0 || cell_height == 0.0 || pane_rect.width < cell_width * 5.0 {
        return None;
    }

    let columns = (pane_rect.width / cell_width).floor() as usize;
    if columns < 5 {
        return None;
    }

    let text_y = pane_rect.y + (PANE_HEADER_HEIGHT - cell_height) / 2.0;
    let text_rect = Rect::new(pane_rect.x, text_y.max(0.0), pane_rect.width, cell_height);

    let bg = Color::new(0.0, 0.0, 0.0, 0.0); // transparent
    let mut cells = vec![GridCell::empty(bg); columns];

    let accent = Color::new(theme.accent.r, theme.accent.g, theme.accent.b, 1.0);
    let text_primary = Color::new(theme.text.r, theme.text.g, theme.text.b, 1.0);
    let text_secondary = Color::new(
        theme.text_secondary.r, theme.text_secondary.g, theme.text_secondary.b, 1.0,
    );
    let text_dim = Color::new(theme.text_dim.r, theme.text_dim.g, theme.text_dim.b, 1.0);
    let success = Color::new(theme.success.r, theme.success.g, theme.success.b, 1.0);

    // Left padding (2 cells)
    let left_pad = 2;
    let mut col = left_pad;

    // Badge
    let badge = badge_char(pane_index);
    let badge_color = if is_active { accent } else { text_dim };
    if col < columns {
        cells[col] = GridCell::new(badge, badge_color, bg);
        col += 1;
    }

    // Space after badge
    col += 1;

    // Title
    let title_color = if is_active { text_primary } else { text_secondary };
    let right_reserved = shell_name.len() + 4; // dot + space + shell + right pad
    let title_max = columns.saturating_sub(col + right_reserved);
    let title_chars: Vec<char> = title.chars().collect();
    let display_title = if title_chars.len() > title_max && title_max > 1 {
        let mut t: Vec<char> = title_chars[..title_max - 1].to_vec();
        t.push('\u{2026}'); // …
        t
    } else {
        title_chars[..title_chars.len().min(title_max)].to_vec()
    };
    for ch in &display_title {
        if col < columns {
            cells[col] = GridCell::new(*ch, title_color, bg);
            col += 1;
        }
    }

    // Right side: status dot + space + shell name + right padding
    let right_pad = 2;
    let right_start = columns.saturating_sub(shell_name.len() + 3 + right_pad);

    // Status dot (● U+25CF) in success green
    if right_start < columns {
        cells[right_start] = GridCell::new('\u{25CF}', success, bg);
    }

    // Space
    let shell_start = right_start + 2;
    // Shell name
    for (j, ch) in shell_name.chars().enumerate() {
        let c = shell_start + j;
        if c < columns {
            cells[c] = GridCell::new(ch, text_dim, bg);
        }
    }

    Some((text_rect, cells))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::theme::Theme;

    #[test]
    fn pane_header_height_constant() {
        assert_eq!(PANE_HEADER_HEIGHT, 36.0);
    }

    #[test]
    fn active_stripe_is_thicker() {
        assert!(ACTIVE_STRIPE_HEIGHT > INACTIVE_STRIPE_HEIGHT);
    }

    #[test]
    fn badge_char_returns_digits() {
        assert_eq!(badge_char(0), '1');
        assert_eq!(badge_char(1), '2');
        assert_eq!(badge_char(8), '9');
    }

    #[test]
    fn badge_char_wraps_for_large_index() {
        assert_eq!(badge_char(9), '0');
    }

    #[test]
    fn pane_header_quads_active_vs_inactive() {
        let theme = Theme::claude_dark();
        let rect = Rect::new(0.0, 0.0, 640.0, 400.0);

        let active_quads = generate_pane_header_quads(rect, true, &theme);
        let inactive_quads = generate_pane_header_quads(rect, false, &theme);

        // Both should have quads (outer border + inner fill + header bg + stripe)
        assert_eq!(active_quads.len(), 4);
        assert_eq!(inactive_quads.len(), 4);

        // Outer border has 8px radius, inner fill has 7px radius
        assert_eq!(active_quads[0].border_radius, 8.0);
        assert_eq!(active_quads[1].border_radius, 7.0);

        // Active stripe is 2px, inactive is 1px (index 3)
        assert_eq!(active_quads[3].rect.height, ACTIVE_STRIPE_HEIGHT);
        assert_eq!(inactive_quads[3].rect.height, INACTIVE_STRIPE_HEIGHT);
    }

    #[test]
    fn pane_header_text_generated() {
        let theme = Theme::claude_dark();
        let rect = Rect::new(0.0, 0.0, 640.0, 400.0);
        let result = generate_pane_header_text(
            rect, 0, "~/work/project", "bash", true, 10.0, 20.0, &theme,
        );
        assert!(result.is_some());
        let (text_rect, cells) = result.unwrap();
        assert_eq!(text_rect.width, 640.0);
        assert_eq!(cells.len(), 64); // 640 / 10
    }

    #[test]
    fn pane_header_text_has_badge() {
        let theme = Theme::claude_dark();
        let rect = Rect::new(0.0, 0.0, 640.0, 400.0);
        let (_, cells) = generate_pane_header_text(
            rect, 0, "~/work", "bash", true, 10.0, 20.0, &theme,
        ).unwrap();
        // Badge at column 2 (after 2-cell left padding)
        assert_eq!(cells[2].ch, '1');
    }

    #[test]
    fn pane_header_text_none_for_narrow_pane() {
        let theme = Theme::claude_dark();
        let rect = Rect::new(0.0, 0.0, 30.0, 400.0);
        let result = generate_pane_header_text(
            rect, 0, "test", "bash", true, 10.0, 20.0, &theme,
        );
        assert!(result.is_none());
    }
}

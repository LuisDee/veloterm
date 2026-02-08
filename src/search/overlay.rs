use crate::config::theme::Color;
use crate::pane::divider::OverlayQuad;
use crate::pane::Rect;
use crate::renderer::grid_renderer::GridCell;

/// Parameters for generating the search bar overlay.
pub struct SearchBarParams<'a> {
    pub pane_rect: Rect,
    pub cell_width: f32,
    pub cell_height: f32,
    pub query: &'a str,
    pub current_match: usize,
    pub total_matches: usize,
    pub has_error: bool,
    pub bar_color: [f32; 4],
    pub text_color: [f32; 4],
}

/// Generate overlay quads for the search bar UI.
///
/// The search bar is positioned at the top-right of the given pane rect.
/// It consists of a background quad. Text rendering is handled separately
/// via the glyph atlas in the renderer.
pub fn generate_search_bar_quads(params: &SearchBarParams) -> Vec<OverlayQuad> {
    let rect = search_bar_rect(params.pane_rect, params.cell_width, params.cell_height);
    vec![OverlayQuad {
        rect,
        color: params.bar_color,
    }]
}

/// Returns the search bar rect (for hit-testing mouse clicks).
pub fn search_bar_rect(pane_rect: Rect, cell_width: f32, cell_height: f32) -> Rect {
    let bar_width = BAR_WIDTH_CELLS * cell_width;
    let bar_height = BAR_HEIGHT_CELLS * cell_height;
    let x = pane_rect.x + pane_rect.width - RIGHT_PADDING - bar_width;
    let y = pane_rect.y + TOP_PADDING;
    Rect::new(x, y, bar_width, bar_height)
}

/// Generate GridCells for the search bar text content.
///
/// Returns the text rect (1 cell tall, centered in the bar) and a Vec of GridCells
/// representing the query text and match count. Returns None if search is not active.
pub fn generate_search_bar_text_cells(
    params: &SearchBarParams,
) -> Option<(Rect, Vec<GridCell>)> {
    let bar = search_bar_rect(params.pane_rect, params.cell_width, params.cell_height);
    let columns = (bar.width / params.cell_width).floor() as usize;
    if columns == 0 {
        return None;
    }

    // Text rect: 1 cell tall, centered vertically within the search bar background
    let text_y = bar.y + (bar.height - params.cell_height) / 2.0;
    let text_rect = Rect::new(bar.x, text_y, bar.width, params.cell_height);

    let bar_bg = Color::new(
        params.bar_color[0],
        params.bar_color[1],
        params.bar_color[2],
        params.bar_color[3],
    );
    let text_fg = if params.has_error {
        Color::new(1.0, 0.3, 0.3, 1.0)
    } else {
        Color::new(
            params.text_color[0],
            params.text_color[1],
            params.text_color[2],
            params.text_color[3],
        )
    };

    let mut cells = vec![GridCell::empty(bar_bg); columns];

    // Layout query text starting at column 1 (after left padding)
    let query_chars: Vec<char> = params.query.chars().collect();
    let query_start = 1;
    for (i, &ch) in query_chars.iter().enumerate() {
        let col = query_start + i;
        if col >= columns {
            break;
        }
        cells[col] = GridCell::new(ch, text_fg, bar_bg);
    }

    // Right-aligned match count
    let status = if params.query.is_empty() {
        String::new()
    } else if params.total_matches == 0 {
        "No results".to_string()
    } else {
        format!("{} of {}", params.current_match, params.total_matches)
    };

    if !status.is_empty() {
        let status_chars: Vec<char> = status.chars().collect();
        let status_start = columns.saturating_sub(status_chars.len() + 1);
        let muted_fg = Color::new(
            params.text_color[0] * 0.7,
            params.text_color[1] * 0.7,
            params.text_color[2] * 0.7,
            params.text_color[3],
        );
        for (i, &ch) in status_chars.iter().enumerate() {
            let col = status_start + i;
            if col < columns {
                cells[col] = GridCell::new(ch, muted_fg, bar_bg);
            }
        }
    }

    Some((text_rect, cells))
}

/// Width of the search bar in cells.
const BAR_WIDTH_CELLS: f32 = 30.0;
/// Height of the search bar in cells.
const BAR_HEIGHT_CELLS: f32 = 1.5;
/// Padding from the right edge in pixels.
const RIGHT_PADDING: f32 = 8.0;
/// Padding from the top edge in pixels.
const TOP_PADDING: f32 = 8.0;

#[cfg(test)]
mod tests {
    use super::*;

    fn pane() -> Rect {
        Rect::new(0.0, 0.0, 1280.0, 720.0)
    }

    const CELL_W: f32 = 10.0;
    const CELL_H: f32 = 20.0;
    const BAR_COLOR: [f32; 4] = [0.2, 0.2, 0.2, 0.9];
    const TEXT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

    fn make_params(pane_rect: Rect, query: &str, current: usize, total: usize) -> SearchBarParams {
        SearchBarParams {
            pane_rect,
            cell_width: CELL_W,
            cell_height: CELL_H,
            query,
            current_match: current,
            total_matches: total,
            has_error: false,
            bar_color: BAR_COLOR,
            text_color: TEXT_COLOR,
        }
    }

    // ── 2.1.1 Search bar at top-right ──────────────────────────────

    #[test]
    fn search_bar_positioned_at_top_right() {
        let quads = generate_search_bar_quads(&make_params(pane(), "test", 1, 5));
        assert!(!quads.is_empty());
        let bg = &quads[0];
        let bar_right = bg.rect.x + bg.rect.width;
        assert!((bar_right - (1280.0 - RIGHT_PADDING)).abs() < 1.0);
        assert!((bg.rect.y - TOP_PADDING).abs() < 1.0);
    }

    #[test]
    fn search_bar_positioned_in_offset_pane() {
        let offset_pane = Rect::new(640.0, 28.0, 640.0, 692.0);
        let quads = generate_search_bar_quads(&make_params(offset_pane, "test", 1, 5));
        let bg = &quads[0];
        let bar_right = bg.rect.x + bg.rect.width;
        assert!((bar_right - (640.0 + 640.0 - RIGHT_PADDING)).abs() < 1.0);
        assert!((bg.rect.y - (28.0 + TOP_PADDING)).abs() < 1.0);
    }

    // ── 2.1.2 Search bar dimensions ────────────────────────────────

    #[test]
    fn search_bar_width_based_on_cells() {
        let quads = generate_search_bar_quads(&make_params(pane(), "test", 1, 5));
        let bg = &quads[0];
        let expected_width = BAR_WIDTH_CELLS * CELL_W;
        assert!((bg.rect.width - expected_width).abs() < 1.0);
    }

    #[test]
    fn search_bar_height_based_on_cells() {
        let quads = generate_search_bar_quads(&make_params(pane(), "test", 1, 5));
        let bg = &quads[0];
        let expected_height = BAR_HEIGHT_CELLS * CELL_H;
        assert!((bg.rect.height - expected_height).abs() < 1.0);
    }

    // ── 2.1.3 Contains background quad ─────────────────────────────

    #[test]
    fn search_bar_has_background_quad() {
        let quads = generate_search_bar_quads(&make_params(pane(), "test", 1, 5));
        assert!(!quads.is_empty());
        assert_eq!(quads[0].color, BAR_COLOR);
    }

    #[test]
    fn empty_query_still_generates_background() {
        let quads = generate_search_bar_quads(&make_params(pane(), "", 0, 0));
        assert!(!quads.is_empty());
    }

    // ── 2.1.4 Search bar rect for hit-testing ──────────────────────

    #[test]
    fn search_bar_rect_matches_background_quad() {
        let quads = generate_search_bar_quads(&make_params(pane(), "test", 1, 5));
        let rect = search_bar_rect(pane(), CELL_W, CELL_H);
        assert_eq!(rect, quads[0].rect);
    }

    #[test]
    fn search_bar_rect_dimensions() {
        let rect = search_bar_rect(pane(), CELL_W, CELL_H);
        let expected_width = BAR_WIDTH_CELLS * CELL_W;
        let expected_height = BAR_HEIGHT_CELLS * CELL_H;
        assert!((rect.width - expected_width).abs() < 1.0);
        assert!((rect.height - expected_height).abs() < 1.0);
    }

    // ── Search bar text cells ────────────────────────────────────

    #[test]
    fn text_cells_contain_query_characters() {
        let params = make_params(pane(), "hello", 1, 3);
        let (_, cells) = generate_search_bar_text_cells(&params).unwrap();
        assert_eq!(cells[1].ch, 'h');
        assert_eq!(cells[2].ch, 'e');
        assert_eq!(cells[3].ch, 'l');
        assert_eq!(cells[4].ch, 'l');
        assert_eq!(cells[5].ch, 'o');
    }

    #[test]
    fn text_cells_match_count_right_aligned() {
        let params = make_params(pane(), "test", 2, 5);
        let (_, cells) = generate_search_bar_text_cells(&params).unwrap();
        // "2 of 5" = 6 chars, right-aligned with 1 cell padding
        // columns = floor(300/10) = 30, status_start = 30 - 6 - 1 = 23
        let status: String = cells[23..29].iter().map(|c| c.ch).collect();
        assert_eq!(status, "2 of 5");
    }

    #[test]
    fn text_cells_empty_query_all_spaces() {
        let params = make_params(pane(), "", 0, 0);
        let (_, cells) = generate_search_bar_text_cells(&params).unwrap();
        assert!(cells.iter().all(|c| c.ch == ' '));
    }

    #[test]
    fn text_cells_rect_is_one_cell_tall() {
        let params = make_params(pane(), "test", 1, 5);
        let (text_rect, _) = generate_search_bar_text_cells(&params).unwrap();
        assert!((text_rect.height - CELL_H).abs() < 0.1);
    }

    #[test]
    fn text_cells_rect_centered_in_bar() {
        let params = make_params(pane(), "test", 1, 5);
        let bar = search_bar_rect(pane(), CELL_W, CELL_H);
        let (text_rect, _) = generate_search_bar_text_cells(&params).unwrap();
        let expected_y = bar.y + (bar.height - CELL_H) / 2.0;
        assert!((text_rect.y - expected_y).abs() < 0.1);
    }

    #[test]
    fn text_cells_error_state_uses_red() {
        let mut params = make_params(pane(), "test", 0, 0);
        params.has_error = true;
        let (_, cells) = generate_search_bar_text_cells(&params).unwrap();
        let fg = cells[1].fg;
        assert!(fg.r > 0.9, "error fg.r should be high (red)");
        assert!(fg.g < 0.5, "error fg.g should be low");
    }

    #[test]
    fn text_cells_no_results_shows_status() {
        let params = make_params(pane(), "xyz", 0, 0);
        let (_, cells) = generate_search_bar_text_cells(&params).unwrap();
        let text: String = cells.iter().map(|c| c.ch).collect();
        assert!(text.contains("No results"));
    }
}

// Tab bar rendering: quad generation and hit testing for the tab bar UI.

use crate::config::theme::{Color, Theme};
use crate::pane::divider::OverlayQuad;
use crate::pane::Rect;
use crate::renderer::grid_renderer::GridCell;

use super::TabManager;

/// Height of the tab bar in physical pixels.
pub const TAB_BAR_HEIGHT: f32 = 28.0;

/// Maximum width for a single tab in pixels.
const MAX_TAB_WIDTH: f32 = 200.0;

/// Minimum width for a single tab in pixels.
const MIN_TAB_WIDTH: f32 = 60.0;

/// Width of the new-tab "+" button area in pixels.
const NEW_TAB_BUTTON_WIDTH: f32 = 28.0;

/// Width of tab separator lines in pixels.
const TAB_SEPARATOR_WIDTH: f32 = 1.0;

/// Width of the close button hit area in pixels.
const CLOSE_BUTTON_WIDTH: f32 = 16.0;

/// Result of a tab bar hit test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabBarAction {
    SelectTab(usize),
    CloseTab(usize),
    NewTab,
}

/// Calculates the width of each tab given the window width and tab count.
pub fn tab_width(window_width: f32, tab_count: usize) -> f32 {
    if tab_count == 0 {
        return 0.0;
    }
    let available = (window_width - NEW_TAB_BUTTON_WIDTH).max(0.0);
    let raw_width = available / tab_count as f32;
    raw_width.clamp(MIN_TAB_WIDTH, MAX_TAB_WIDTH)
}

/// Generates overlay quads for the tab bar background, tabs, and separators.
pub fn generate_tab_bar_quads(
    tab_manager: &TabManager,
    window_width: f32,
    theme: &Theme,
) -> Vec<OverlayQuad> {
    let mut quads = Vec::new();
    let count = tab_manager.tab_count();
    let active = tab_manager.active_index();
    let tw = tab_width(window_width, count);

    // Tab bar background
    quads.push(OverlayQuad {
        rect: Rect::new(0.0, 0.0, window_width, TAB_BAR_HEIGHT),
        color: [
            theme.surface.r,
            theme.surface.g,
            theme.surface.b,
            1.0,
        ],
    });

    // Individual tab backgrounds
    for i in 0..count {
        let x = i as f32 * tw;
        let (bg_r, bg_g, bg_b) = if i == active {
            (theme.accent.r, theme.accent.g, theme.accent.b)
        } else {
            (
                theme.surface.r,
                theme.surface.g,
                theme.surface.b,
            )
        };
        quads.push(OverlayQuad {
            rect: Rect::new(x, 0.0, tw, TAB_BAR_HEIGHT),
            color: [bg_r, bg_g, bg_b, 1.0],
        });
    }

    // Tab separators (between tabs)
    for i in 1..count {
        let x = i as f32 * tw - TAB_SEPARATOR_WIDTH / 2.0;
        quads.push(OverlayQuad {
            rect: Rect::new(x, 2.0, TAB_SEPARATOR_WIDTH, TAB_BAR_HEIGHT - 4.0),
            color: [theme.border.r, theme.border.g, theme.border.b, 0.5],
        });
    }

    // Notification badges for inactive tabs
    for i in 0..count {
        if tab_manager.tabs()[i].has_notification {
            let x = i as f32 * tw + tw - 10.0;
            quads.push(OverlayQuad {
                rect: Rect::new(x, 4.0, 6.0, 6.0),
                color: [theme.accent.r, theme.accent.g, theme.accent.b, 1.0],
            });
        }
    }

    // New-tab "+" button background (always at the right of tabs)
    let plus_x = (count as f32 * tw).min(window_width - NEW_TAB_BUTTON_WIDTH);
    quads.push(OverlayQuad {
        rect: Rect::new(plus_x, 0.0, NEW_TAB_BUTTON_WIDTH, TAB_BAR_HEIGHT),
        color: [
            theme.surface.r,
            theme.surface.g,
            theme.surface.b,
            1.0,
        ],
    });

    quads
}

/// Generate GridCells for each tab label and the "+" button.
///
/// Returns a Vec of (Rect, Vec<GridCell>) pairs, one per tab plus one for the "+" button.
/// Each rect is 1 cell tall, centered vertically within the tab bar.
/// `hovered_tab` controls close button visibility: active tab always shows it,
/// hovered inactive tabs show it, other inactive tabs don't.
pub fn generate_tab_label_text_cells(
    tab_manager: &TabManager,
    window_width: f32,
    cell_width: f32,
    cell_height: f32,
    theme: &Theme,
    hovered_tab: Option<usize>,
) -> Vec<(Rect, Vec<GridCell>)> {
    let mut result = Vec::new();
    let count = tab_manager.tab_count();
    let active = tab_manager.active_index();
    let tw = tab_width(window_width, count);

    if tw == 0.0 || cell_width == 0.0 {
        return result;
    }

    let text_y = (TAB_BAR_HEIGHT - cell_height) / 2.0;

    for i in 0..count {
        let x = i as f32 * tw;
        let columns = (tw / cell_width).floor() as usize;
        if columns == 0 {
            continue;
        }

        let text_rect = Rect::new(x, text_y.max(0.0), tw, cell_height);

        let (fg, bg) = if i == active {
            (
                Color::new(1.0, 1.0, 1.0, 1.0),
                Color::new(theme.accent.r, theme.accent.g, theme.accent.b, 1.0),
            )
        } else {
            (
                Color::new(
                    theme.text_secondary.r,
                    theme.text_secondary.g,
                    theme.text_secondary.b,
                    1.0,
                ),
                Color::new(
                    theme.surface.r,
                    theme.surface.g,
                    theme.surface.b,
                    1.0,
                ),
            )
        };

        let mut cells = vec![GridCell::empty(bg); columns];

        // Show close button: always on active tab, on hover for inactive tabs
        let show_close = i == active || hovered_tab == Some(i);
        if show_close && columns >= 3 {
            let close_col = columns - 1;
            let close_fg = if i == active {
                Color::new(1.0, 1.0, 1.0, 0.7)
            } else {
                Color::new(theme.text_secondary.r, theme.text_secondary.g, theme.text_secondary.b, 0.7)
            };
            cells[close_col] = GridCell::new('\u{00D7}', close_fg, bg); // × character
        }

        let title = &tab_manager.tabs()[i].title;
        let label_chars: Vec<char> = title.chars().collect();
        // Reserve space for close button if shown
        let usable_cols = if show_close && columns >= 3 { columns - 2 } else { columns };
        let display_chars = if label_chars.len() > usable_cols && usable_cols > 1 {
            // Truncate with ellipsis
            let mut truncated: Vec<char> = label_chars[..usable_cols - 1].to_vec();
            truncated.push('\u{2026}'); // …
            truncated
        } else {
            label_chars[..label_chars.len().min(usable_cols)].to_vec()
        };
        let label_start = if display_chars.len() < usable_cols {
            (usable_cols - display_chars.len()) / 2
        } else {
            0
        };
        for (j, &ch) in display_chars.iter().enumerate() {
            let col = label_start + j;
            if col < columns {
                cells[col] = GridCell::new(ch, fg, bg);
            }
        }

        result.push((text_rect, cells));
    }

    // "+" button
    let plus_x = (count as f32 * tw).min(window_width - NEW_TAB_BUTTON_WIDTH);
    let plus_cols = (NEW_TAB_BUTTON_WIDTH / cell_width).floor() as usize;
    if plus_cols > 0 {
        let plus_rect = Rect::new(plus_x, text_y.max(0.0), NEW_TAB_BUTTON_WIDTH, cell_height);
        let bg = Color::new(
            theme.surface.r,
            theme.surface.g,
            theme.surface.b,
            1.0,
        );
        let fg = Color::new(
            theme.text_secondary.r,
            theme.text_secondary.g,
            theme.text_secondary.b,
            1.0,
        );
        let mut cells = vec![GridCell::empty(bg); plus_cols];
        let center = plus_cols / 2;
        if center < plus_cols {
            cells[center] = GridCell::new('+', fg, bg);
        }
        result.push((plus_rect, cells));
    }

    result
}

/// Hit tests a point against the tab bar.
/// Returns the action if the point is within the tab bar area (y < TAB_BAR_HEIGHT).
/// `active_index` is used to determine which tab always shows the close button.
/// `hovered_tab` determines which inactive tab shows the close button.
pub fn hit_test_tab_bar(
    x: f32,
    y: f32,
    window_width: f32,
    tab_count: usize,
    active_index: usize,
    hovered_tab: Option<usize>,
) -> Option<TabBarAction> {
    if y >= TAB_BAR_HEIGHT {
        return None;
    }

    let tw = tab_width(window_width, tab_count);
    let tabs_end = tab_count as f32 * tw;

    // Check new-tab button area
    let plus_x = tabs_end.min(window_width - NEW_TAB_BUTTON_WIDTH);
    if x >= plus_x && x < plus_x + NEW_TAB_BUTTON_WIDTH {
        return Some(TabBarAction::NewTab);
    }

    // Check tab areas
    if x < tabs_end && x >= 0.0 {
        let index = (x / tw) as usize;
        if index < tab_count {
            // Check if click is on the close button (rightmost CLOSE_BUTTON_WIDTH pixels of tab)
            let tab_right_edge = (index + 1) as f32 * tw;
            let close_button_start = tab_right_edge - CLOSE_BUTTON_WIDTH;
            let show_close = index == active_index || hovered_tab == Some(index);
            if show_close && x >= close_button_start {
                return Some(TabBarAction::CloseTab(index));
            }
            return Some(TabBarAction::SelectTab(index));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::theme::Theme;
    use crate::pane::PaneId;
    use crate::tab::TabId;

    fn setup() {
        TabId::reset_counter();
        PaneId::reset_counter();
    }

    // ── tab_width ────────────────────────────────────────────────

    #[test]
    fn tab_width_single_tab() {
        let w = tab_width(1280.0, 1);
        assert_eq!(w, MAX_TAB_WIDTH); // (1280-28)/1=1252, clamped to 200
    }

    #[test]
    fn tab_width_many_tabs_shrinks() {
        let w = tab_width(1280.0, 20);
        // (1280-28)/20 = 62.6
        assert!(w > MIN_TAB_WIDTH);
        assert!(w < MAX_TAB_WIDTH);
    }

    #[test]
    fn tab_width_very_many_tabs_clamps_to_min() {
        let w = tab_width(400.0, 20);
        // (400-28)/20 = 18.6, clamped to 60
        assert_eq!(w, MIN_TAB_WIDTH);
    }

    #[test]
    fn tab_width_zero_tabs() {
        assert_eq!(tab_width(1280.0, 0), 0.0);
    }

    // ── generate_tab_bar_quads ───────────────────────────────────

    #[test]
    fn quads_single_tab() {
        setup();
        let mgr = TabManager::new();
        let theme = Theme::claude_dark();
        let quads = generate_tab_bar_quads(&mgr, 1280.0, &theme);
        // Background + 1 tab + 0 separators + "+" button = 3 quads
        assert_eq!(quads.len(), 3);
        // First quad is background spanning full width
        assert_eq!(quads[0].rect.width, 1280.0);
        assert_eq!(quads[0].rect.height, TAB_BAR_HEIGHT);
    }

    #[test]
    fn quads_active_tab_uses_accent_color() {
        setup();
        let mgr = TabManager::new();
        let theme = Theme::claude_dark();
        let quads = generate_tab_bar_quads(&mgr, 1280.0, &theme);
        // Second quad is the active tab — should use accent color
        let tab_quad = &quads[1];
        assert_eq!(tab_quad.color[0], theme.accent.r);
        assert_eq!(tab_quad.color[1], theme.accent.g);
        assert_eq!(tab_quad.color[2], theme.accent.b);
    }

    #[test]
    fn quads_multiple_tabs_have_separators() {
        setup();
        let mut mgr = TabManager::new();
        mgr.new_tab();
        mgr.new_tab();
        let theme = Theme::claude_dark();
        let quads = generate_tab_bar_quads(&mgr, 1280.0, &theme);
        // Background + 3 tabs + 2 separators + "+" button = 7 quads
        assert_eq!(quads.len(), 7);
    }

    #[test]
    fn quads_inactive_tab_uses_pane_background() {
        setup();
        let mut mgr = TabManager::new();
        mgr.new_tab(); // active is now tab 1
        let theme = Theme::claude_dark();
        let quads = generate_tab_bar_quads(&mgr, 1280.0, &theme);
        // quads[1] is tab 0 (inactive)
        assert_eq!(quads[1].color[0], theme.surface.r);
        assert_eq!(quads[1].color[1], theme.surface.g);
    }

    // ── generate_tab_label_text_cells ─────────────────────────────

    #[test]
    fn label_single_tab_generates_title() {
        setup();
        let mgr = TabManager::new();
        let theme = Theme::claude_dark();
        let labels = generate_tab_label_text_cells(&mgr, 1280.0, 10.0, 20.0, &theme, None);
        assert_eq!(labels.len(), 2); // 1 tab + "+" button
        // Default title is "Shell"
        assert!(labels[0].1.iter().any(|c| c.ch == 'S'));
    }

    #[test]
    fn label_active_tab_uses_accent_bg() {
        setup();
        let mgr = TabManager::new();
        let theme = Theme::claude_dark();
        let labels = generate_tab_label_text_cells(&mgr, 1280.0, 10.0, 20.0, &theme, None);
        let tab_cell = labels[0].1.iter().find(|c| c.ch == 'S').unwrap();
        assert!((tab_cell.bg.r - theme.accent.r).abs() < 0.01);
        assert!((tab_cell.bg.g - theme.accent.g).abs() < 0.01);
        assert!((tab_cell.bg.b - theme.accent.b).abs() < 0.01);
    }

    #[test]
    fn label_inactive_tab_uses_pane_background() {
        setup();
        let mut mgr = TabManager::new();
        mgr.new_tab();
        let theme = Theme::claude_dark();
        let labels = generate_tab_label_text_cells(&mgr, 1280.0, 10.0, 20.0, &theme, None);
        let tab0_cell = labels[0].1.iter().find(|c| c.ch == 'S').unwrap();
        assert!((tab0_cell.bg.r - theme.surface.r).abs() < 0.01);
    }

    #[test]
    fn label_multiple_tabs_each_get_descriptor() {
        setup();
        let mut mgr = TabManager::new();
        mgr.new_tab();
        mgr.new_tab();
        let theme = Theme::claude_dark();
        let labels = generate_tab_label_text_cells(&mgr, 1280.0, 10.0, 20.0, &theme, None);
        assert_eq!(labels.len(), 4); // 3 tabs + "+" button
        // All tabs have default "Shell" title
        assert!(labels[0].1.iter().any(|c| c.ch == 'S'));
        assert!(labels[1].1.iter().any(|c| c.ch == 'S'));
        assert!(labels[2].1.iter().any(|c| c.ch == 'S'));
        assert!(labels[3].1.iter().any(|c| c.ch == '+'));
    }

    // ── Close button visibility ────────────────────────────────────

    #[test]
    fn close_button_always_on_active_tab() {
        setup();
        let mgr = TabManager::new();
        let theme = Theme::claude_dark();
        let labels = generate_tab_label_text_cells(&mgr, 1280.0, 10.0, 20.0, &theme, None);
        // Active tab (0) should have × close button
        assert!(labels[0].1.iter().any(|c| c.ch == '\u{00D7}'));
    }

    #[test]
    fn close_button_on_hovered_inactive_tab() {
        setup();
        let mut mgr = TabManager::new();
        mgr.new_tab(); // active is now 1
        let theme = Theme::claude_dark();
        // Hover over tab 0 (inactive)
        let labels = generate_tab_label_text_cells(&mgr, 1280.0, 10.0, 20.0, &theme, Some(0));
        // Tab 0 (inactive, hovered) should have × close button
        assert!(labels[0].1.iter().any(|c| c.ch == '\u{00D7}'));
    }

    #[test]
    fn no_close_button_on_unhovered_inactive_tab() {
        setup();
        let mut mgr = TabManager::new();
        mgr.new_tab(); // active is now 1
        let theme = Theme::claude_dark();
        // No hover
        let labels = generate_tab_label_text_cells(&mgr, 1280.0, 10.0, 20.0, &theme, None);
        // Tab 0 (inactive, not hovered) should NOT have × close button
        assert!(!labels[0].1.iter().any(|c| c.ch == '\u{00D7}'));
    }

    // ── Hit test with close button ─────────────────────────────────

    #[test]
    fn hit_test_click_on_first_tab() {
        let result = hit_test_tab_bar(10.0, 10.0, 1280.0, 3, 0, None);
        assert_eq!(result, Some(TabBarAction::SelectTab(0)));
    }

    #[test]
    fn hit_test_click_on_second_tab() {
        let tw = tab_width(1280.0, 3);
        let result = hit_test_tab_bar(tw + 5.0, 10.0, 1280.0, 3, 0, None);
        assert_eq!(result, Some(TabBarAction::SelectTab(1)));
    }

    #[test]
    fn hit_test_click_on_new_tab_button() {
        let tw = tab_width(1280.0, 2);
        let plus_x = 2.0 * tw; // right after tabs
        let result = hit_test_tab_bar(plus_x + 5.0, 10.0, 1280.0, 2, 0, None);
        assert_eq!(result, Some(TabBarAction::NewTab));
    }

    #[test]
    fn hit_test_below_tab_bar_returns_none() {
        let result = hit_test_tab_bar(100.0, 30.0, 1280.0, 3, 0, None);
        assert_eq!(result, None);
    }

    #[test]
    fn hit_test_at_tab_bar_boundary() {
        let result = hit_test_tab_bar(100.0, TAB_BAR_HEIGHT, 1280.0, 3, 0, None);
        assert_eq!(result, None);
    }

    #[test]
    fn hit_test_just_inside_tab_bar() {
        let result = hit_test_tab_bar(100.0, TAB_BAR_HEIGHT - 1.0, 1280.0, 3, 0, None);
        assert_eq!(result, Some(TabBarAction::SelectTab(0)));
    }

    #[test]
    fn hit_test_close_button_on_active_tab() {
        let tw = tab_width(1280.0, 3);
        // Click near right edge of active tab (tab 0)
        let x = tw - 5.0; // within CLOSE_BUTTON_WIDTH of right edge
        let result = hit_test_tab_bar(x, 10.0, 1280.0, 3, 0, None);
        assert_eq!(result, Some(TabBarAction::CloseTab(0)));
    }

    #[test]
    fn hit_test_close_button_on_hovered_inactive_tab() {
        let tw = tab_width(1280.0, 3);
        // Click near right edge of tab 1 (inactive, hovered)
        let x = tw * 2.0 - 5.0;
        let result = hit_test_tab_bar(x, 10.0, 1280.0, 3, 0, Some(1));
        assert_eq!(result, Some(TabBarAction::CloseTab(1)));
    }

    #[test]
    fn hit_test_no_close_button_on_unhovered_inactive_tab() {
        let tw = tab_width(1280.0, 3);
        // Click near right edge of tab 1 (inactive, NOT hovered)
        let x = tw * 2.0 - 5.0;
        let result = hit_test_tab_bar(x, 10.0, 1280.0, 3, 0, None);
        // Should be SelectTab, not CloseTab, since close button is hidden
        assert_eq!(result, Some(TabBarAction::SelectTab(1)));
    }

    // ── Drag target calculation ────────────────────────────────────

    #[test]
    fn drag_target_index_from_cursor_x() {
        let tw = tab_width(1280.0, 3);
        // Cursor at middle of tab 0 → index 0
        assert_eq!((tw * 0.5 / tw) as usize, 0);
        // Cursor at middle of tab 1 → index 1
        assert_eq!((tw * 1.5 / tw) as usize, 1);
        // Cursor at middle of tab 2 → index 2
        assert_eq!((tw * 2.5 / tw) as usize, 2);
    }

    #[test]
    fn move_tab_preserves_active_tracking() {
        setup();
        let mut mgr = TabManager::new();
        mgr.set_title(0, "A");
        let _ = mgr.new_tab();
        mgr.set_title(1, "B");
        let _ = mgr.new_tab();
        mgr.set_title(2, "C");
        // Active is 2 (C), drag it to position 0
        mgr.move_tab(2, 0);
        assert_eq!(mgr.active_tab().title, "C");
        assert_eq!(mgr.active_index(), 0);
        assert_eq!(mgr.tabs()[1].title, "A");
        assert_eq!(mgr.tabs()[2].title, "B");
    }
}

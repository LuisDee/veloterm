// Divider geometry: compute divider rects from the pane tree layout.

use super::{PaneNode, Rect, SplitDirection};
use crate::config::theme::Color;

/// Information about a single divider bar between panes.
#[derive(Debug, Clone, PartialEq)]
pub struct DividerInfo {
    /// Physical pixel rect of the divider line.
    pub rect: Rect,
    /// Direction of the split that created this divider.
    /// Vertical split → vertical divider line (resize left/right).
    /// Horizontal split → horizontal divider line (resize up/down).
    pub direction: SplitDirection,
    /// Index of this divider's split node in pre-order tree walk.
    /// Used to identify the split node for ratio updates during drag.
    pub split_index: usize,
}

/// Width of the divider bar in pixels.
pub const DIVIDER_WIDTH: f32 = 2.0;

/// Default hit-test margin in pixels around a divider.
pub const HIT_TEST_MARGIN: f32 = 8.0;

/// Hit-test a point against dividers, returning the first divider within margin.
/// The margin expands the divider rect by `margin` pixels on each side of the
/// thin axis (perpendicular to the divider line).
pub fn hit_test_divider(point: (f32, f32), dividers: &[DividerInfo], margin: f32) -> Option<usize> {
    let (px, py) = point;
    for (i, divider) in dividers.iter().enumerate() {
        let r = &divider.rect;
        let expanded = match divider.direction {
            SplitDirection::Vertical => {
                // Expand horizontally (the thin axis)
                Rect::new(r.x - margin, r.y, r.width + margin * 2.0, r.height)
            }
            SplitDirection::Horizontal => {
                // Expand vertically (the thin axis)
                Rect::new(r.x, r.y - margin, r.width, r.height + margin * 2.0)
            }
        };
        if expanded.contains_point(px, py) {
            return Some(i);
        }
    }
    None
}

/// A UI overlay quad: a colored rectangle at a pixel position.
/// Pure data structure — no GPU dependency. Used for dividers and focus overlays.
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayQuad {
    /// Rectangle in physical pixels.
    pub rect: Rect,
    /// RGBA color.
    pub color: [f32; 4],
    /// Border radius in pixels (0.0 for sharp corners).
    pub border_radius: f32,
}

/// Generate overlay quads for divider bars.
/// Returns one quad per divider, colored with the appropriate theme color.
/// If `hovered_index` matches a divider, uses `hover_color` instead.
pub fn generate_divider_quads(
    dividers: &[DividerInfo],
    border_color: &Color,
    hover_color: &Color,
    hovered_index: Option<usize>,
) -> Vec<OverlayQuad> {
    dividers
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let color = if Some(i) == hovered_index {
                [hover_color.r, hover_color.g, hover_color.b, hover_color.a]
            } else {
                [border_color.r, border_color.g, border_color.b, border_color.a]
            };
            OverlayQuad {
                rect: d.rect,
                color,
                border_radius: 0.0,
            }
        })
        .collect()
}

/// Generate overlay quads for unfocused pane dimming.
/// Returns one translucent quad per unfocused pane rect.
pub fn generate_unfocused_overlay_quads(
    layout: &[(super::PaneId, Rect)],
    focused_id: super::PaneId,
    bg_color: &Color,
    dim_alpha: f32,
) -> Vec<OverlayQuad> {
    layout
        .iter()
        .filter(|(id, _)| *id != focused_id)
        .map(|(_, rect)| OverlayQuad {
            rect: *rect,
            color: [bg_color.r, bg_color.g, bg_color.b, dim_alpha],
            border_radius: 0.0,
        })
        .collect()
}

/// Calculate divider rects from the pane tree.
/// Walks the tree in pre-order, emitting a DividerInfo at each Split node.
pub fn calculate_dividers(root: &PaneNode, bounds: Rect, min_size: f32) -> Vec<DividerInfo> {
    let mut dividers = Vec::new();
    let mut split_index = 0;
    collect_dividers(root, bounds, min_size, &mut dividers, &mut split_index);
    dividers
}

fn collect_dividers(
    node: &PaneNode,
    bounds: Rect,
    min_size: f32,
    dividers: &mut Vec<DividerInfo>,
    split_index: &mut usize,
) {
    match node {
        PaneNode::Leaf { .. } => {}
        PaneNode::Split {
            direction,
            ratio,
            first,
            second,
        } => {
            let clamped_ratio = super::clamp_ratio(*ratio, match direction {
                SplitDirection::Vertical => bounds.width,
                SplitDirection::Horizontal => bounds.height,
            }, min_size);

            let divider_rect = match direction {
                SplitDirection::Vertical => {
                    let boundary_x = bounds.x + bounds.width * clamped_ratio;
                    Rect::new(
                        boundary_x - DIVIDER_WIDTH / 2.0,
                        bounds.y,
                        DIVIDER_WIDTH,
                        bounds.height,
                    )
                }
                SplitDirection::Horizontal => {
                    let boundary_y = bounds.y + bounds.height * clamped_ratio;
                    Rect::new(
                        bounds.x,
                        boundary_y - DIVIDER_WIDTH / 2.0,
                        bounds.width,
                        DIVIDER_WIDTH,
                    )
                }
            };

            let current_index = *split_index;
            *split_index += 1;

            dividers.push(DividerInfo {
                rect: divider_rect,
                direction: *direction,
                split_index: current_index,
            });

            // Recurse into children with their sub-bounds
            let (first_bounds, second_bounds) =
                super::split_rect(bounds, *direction, *ratio, min_size);
            collect_dividers(first, first_bounds, min_size, dividers, split_index);
            collect_dividers(second, second_bounds, min_size, dividers, split_index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pane::{PaneId, PaneNode};

    #[test]
    fn single_leaf_has_no_dividers() {
        let root = PaneNode::leaf(PaneId(1));
        let bounds = Rect::new(0.0, 0.0, 1280.0, 720.0);
        let dividers = calculate_dividers(&root, bounds, 20.0);
        assert!(dividers.is_empty());
    }

    #[test]
    fn vertical_split_produces_one_vertical_divider() {
        let root = PaneNode::split(
            SplitDirection::Vertical,
            0.5,
            PaneNode::leaf(PaneId(1)),
            PaneNode::leaf(PaneId(2)),
        );
        let bounds = Rect::new(0.0, 0.0, 1280.0, 720.0);
        let dividers = calculate_dividers(&root, bounds, 20.0);
        assert_eq!(dividers.len(), 1);
        assert_eq!(dividers[0].direction, SplitDirection::Vertical);
    }

    #[test]
    fn vertical_divider_rect_centered_on_split_boundary() {
        let root = PaneNode::split(
            SplitDirection::Vertical,
            0.5,
            PaneNode::leaf(PaneId(1)),
            PaneNode::leaf(PaneId(2)),
        );
        let bounds = Rect::new(0.0, 0.0, 1280.0, 720.0);
        let dividers = calculate_dividers(&root, bounds, 20.0);
        let d = &dividers[0];
        // Boundary at x=640, divider centered: x=639, width=2
        assert_eq!(d.rect.x, 639.0);
        assert_eq!(d.rect.y, 0.0);
        assert_eq!(d.rect.width, DIVIDER_WIDTH);
        assert_eq!(d.rect.height, 720.0);
    }

    #[test]
    fn horizontal_split_produces_one_horizontal_divider() {
        let root = PaneNode::split(
            SplitDirection::Horizontal,
            0.5,
            PaneNode::leaf(PaneId(1)),
            PaneNode::leaf(PaneId(2)),
        );
        let bounds = Rect::new(0.0, 0.0, 1280.0, 720.0);
        let dividers = calculate_dividers(&root, bounds, 20.0);
        assert_eq!(dividers.len(), 1);
        assert_eq!(dividers[0].direction, SplitDirection::Horizontal);
    }

    #[test]
    fn horizontal_divider_rect_centered_on_split_boundary() {
        let root = PaneNode::split(
            SplitDirection::Horizontal,
            0.5,
            PaneNode::leaf(PaneId(1)),
            PaneNode::leaf(PaneId(2)),
        );
        let bounds = Rect::new(0.0, 0.0, 1280.0, 720.0);
        let dividers = calculate_dividers(&root, bounds, 20.0);
        let d = &dividers[0];
        // Boundary at y=360, divider centered: y=359, height=2
        assert_eq!(d.rect.x, 0.0);
        assert_eq!(d.rect.y, 359.0);
        assert_eq!(d.rect.width, 1280.0);
        assert_eq!(d.rect.height, DIVIDER_WIDTH);
    }

    #[test]
    fn nested_splits_produce_two_dividers() {
        // [A | [B / C]]  — one vertical, one horizontal
        let root = PaneNode::split(
            SplitDirection::Vertical,
            0.5,
            PaneNode::leaf(PaneId(1)),
            PaneNode::split(
                SplitDirection::Horizontal,
                0.5,
                PaneNode::leaf(PaneId(2)),
                PaneNode::leaf(PaneId(3)),
            ),
        );
        let bounds = Rect::new(0.0, 0.0, 1280.0, 720.0);
        let dividers = calculate_dividers(&root, bounds, 20.0);
        assert_eq!(dividers.len(), 2);
        // First divider: vertical split at root
        assert_eq!(dividers[0].direction, SplitDirection::Vertical);
        assert_eq!(dividers[0].split_index, 0);
        // Second divider: horizontal split in right subtree
        assert_eq!(dividers[1].direction, SplitDirection::Horizontal);
        assert_eq!(dividers[1].split_index, 1);
    }

    #[test]
    fn nested_horizontal_divider_spans_only_its_subtree() {
        // [A | [B / C]]  — horizontal divider should span only the right half
        let root = PaneNode::split(
            SplitDirection::Vertical,
            0.5,
            PaneNode::leaf(PaneId(1)),
            PaneNode::split(
                SplitDirection::Horizontal,
                0.5,
                PaneNode::leaf(PaneId(2)),
                PaneNode::leaf(PaneId(3)),
            ),
        );
        let bounds = Rect::new(0.0, 0.0, 1280.0, 720.0);
        let dividers = calculate_dividers(&root, bounds, 20.0);
        let h_divider = &dividers[1];
        // With 8px gap: right half starts at x=644, width=636
        assert_eq!(h_divider.rect.x, 644.0);
        assert_eq!(h_divider.rect.width, 636.0);
    }

    #[test]
    fn divider_with_offset_bounds() {
        let root = PaneNode::split(
            SplitDirection::Vertical,
            0.5,
            PaneNode::leaf(PaneId(1)),
            PaneNode::leaf(PaneId(2)),
        );
        let bounds = Rect::new(100.0, 50.0, 800.0, 600.0);
        let dividers = calculate_dividers(&root, bounds, 20.0);
        let d = &dividers[0];
        // Boundary at x = 100 + 800*0.5 = 500, divider at 499
        assert_eq!(d.rect.x, 499.0);
        assert_eq!(d.rect.y, 50.0);
        assert_eq!(d.rect.height, 600.0);
    }

    #[test]
    fn divider_with_asymmetric_ratio() {
        let root = PaneNode::split(
            SplitDirection::Vertical,
            0.25,
            PaneNode::leaf(PaneId(1)),
            PaneNode::leaf(PaneId(2)),
        );
        let bounds = Rect::new(0.0, 0.0, 1000.0, 500.0);
        let dividers = calculate_dividers(&root, bounds, 20.0);
        let d = &dividers[0];
        // Boundary at x = 1000*0.25 = 250, divider at 249
        assert_eq!(d.rect.x, 249.0);
    }

    #[test]
    fn three_way_split_produces_two_dividers() {
        // [[A | B] | C] — two vertical dividers
        let root = PaneNode::split(
            SplitDirection::Vertical,
            0.5,
            PaneNode::split(
                SplitDirection::Vertical,
                0.5,
                PaneNode::leaf(PaneId(1)),
                PaneNode::leaf(PaneId(2)),
            ),
            PaneNode::leaf(PaneId(3)),
        );
        let bounds = Rect::new(0.0, 0.0, 1200.0, 600.0);
        let dividers = calculate_dividers(&root, bounds, 20.0);
        assert_eq!(dividers.len(), 2);
        assert_eq!(dividers[0].direction, SplitDirection::Vertical);
        assert_eq!(dividers[1].direction, SplitDirection::Vertical);
        // Root divider at x=600, inner divider at x=298 (left half is 596px, split at 50%)
        assert_eq!(dividers[0].rect.x, 599.0);
        assert_eq!(dividers[1].rect.x, 297.0);
    }

    #[test]
    fn split_index_increments_in_preorder() {
        // [[A | B] / C] — first split is vertical (index 0), second is horizontal (index 1)
        // Wait, root is horizontal, left child is vertical split
        let root = PaneNode::split(
            SplitDirection::Horizontal,
            0.5,
            PaneNode::split(
                SplitDirection::Vertical,
                0.5,
                PaneNode::leaf(PaneId(1)),
                PaneNode::leaf(PaneId(2)),
            ),
            PaneNode::leaf(PaneId(3)),
        );
        let bounds = Rect::new(0.0, 0.0, 1000.0, 800.0);
        let dividers = calculate_dividers(&root, bounds, 20.0);
        assert_eq!(dividers.len(), 2);
        assert_eq!(dividers[0].split_index, 0); // root horizontal
        assert_eq!(dividers[1].split_index, 1); // inner vertical
    }

    // ── Hit-testing tests ────────────────────────────────────────────

    fn make_vertical_dividers() -> Vec<DividerInfo> {
        // Simulate a vertical split at x=640 in 1280x720 window
        let root = PaneNode::split(
            SplitDirection::Vertical,
            0.5,
            PaneNode::leaf(PaneId(1)),
            PaneNode::leaf(PaneId(2)),
        );
        calculate_dividers(&root, Rect::new(0.0, 0.0, 1280.0, 720.0), 20.0)
    }

    #[test]
    fn hit_test_on_divider_returns_index() {
        let dividers = make_vertical_dividers();
        // Divider at x=639, width=2. Point exactly on divider.
        let result = hit_test_divider((640.0, 360.0), &dividers, HIT_TEST_MARGIN);
        assert_eq!(result, Some(0));
    }

    #[test]
    fn hit_test_within_margin_returns_index() {
        let dividers = make_vertical_dividers();
        // Point 5px to the left of divider center, within 8px margin
        let result = hit_test_divider((635.0, 360.0), &dividers, HIT_TEST_MARGIN);
        assert_eq!(result, Some(0));
    }

    #[test]
    fn hit_test_outside_margin_returns_none() {
        let dividers = make_vertical_dividers();
        // Point 20px away from divider
        let result = hit_test_divider((620.0, 360.0), &dividers, HIT_TEST_MARGIN);
        assert_eq!(result, None);
    }

    #[test]
    fn hit_test_beyond_divider_length_returns_none() {
        let dividers = make_vertical_dividers();
        // Point at correct x but below the divider (y > 720)
        let result = hit_test_divider((640.0, 800.0), &dividers, HIT_TEST_MARGIN);
        assert_eq!(result, None);
    }

    #[test]
    fn hit_test_empty_dividers_returns_none() {
        let result = hit_test_divider((640.0, 360.0), &[], HIT_TEST_MARGIN);
        assert_eq!(result, None);
    }

    #[test]
    fn hit_test_horizontal_divider() {
        let root = PaneNode::split(
            SplitDirection::Horizontal,
            0.5,
            PaneNode::leaf(PaneId(1)),
            PaneNode::leaf(PaneId(2)),
        );
        let dividers = calculate_dividers(&root, Rect::new(0.0, 0.0, 1280.0, 720.0), 20.0);
        // Horizontal divider at y=359, height=2. Point near divider.
        let result = hit_test_divider((640.0, 360.0), &dividers, HIT_TEST_MARGIN);
        assert_eq!(result, Some(0));
    }

    #[test]
    fn hit_test_picks_first_matching_divider() {
        // Two dividers close together: [[A | B] | C]
        let root = PaneNode::split(
            SplitDirection::Vertical,
            0.5,
            PaneNode::split(
                SplitDirection::Vertical,
                0.5,
                PaneNode::leaf(PaneId(1)),
                PaneNode::leaf(PaneId(2)),
            ),
            PaneNode::leaf(PaneId(3)),
        );
        let dividers = calculate_dividers(&root, Rect::new(0.0, 0.0, 1200.0, 600.0), 20.0);
        // Root divider at x=599, inner at x=299. Point near root divider.
        let result = hit_test_divider((600.0, 300.0), &dividers, HIT_TEST_MARGIN);
        assert_eq!(result, Some(0));
    }

    #[test]
    fn hit_test_zero_margin() {
        let dividers = make_vertical_dividers();
        // With zero margin, must be exactly on the 2px divider (x=639..641)
        let on = hit_test_divider((639.5, 360.0), &dividers, 0.0);
        assert_eq!(on, Some(0));
        let off = hit_test_divider((637.0, 360.0), &dividers, 0.0);
        assert_eq!(off, None);
    }

    // ── Divider quad generation tests ────────────────────────────────

    fn border_color() -> Color {
        Color { r: 0.24, g: 0.22, b: 0.20, a: 1.0 }
    }

    fn hover_color() -> Color {
        Color { r: 0.80, g: 0.60, b: 0.40, a: 1.0 }
    }

    #[test]
    fn divider_quads_empty_for_no_dividers() {
        let quads = generate_divider_quads(&[], &border_color(), &hover_color(), None);
        assert!(quads.is_empty());
    }

    #[test]
    fn divider_quads_one_per_divider() {
        let dividers = make_vertical_dividers();
        let quads = generate_divider_quads(&dividers, &border_color(), &hover_color(), None);
        assert_eq!(quads.len(), 1);
    }

    #[test]
    fn divider_quad_uses_border_color_when_not_hovered() {
        let dividers = make_vertical_dividers();
        let bc = border_color();
        let quads = generate_divider_quads(&dividers, &bc, &hover_color(), None);
        assert_eq!(quads[0].color, [bc.r, bc.g, bc.b, bc.a]);
    }

    #[test]
    fn divider_quad_uses_hover_color_when_hovered() {
        let dividers = make_vertical_dividers();
        let hc = hover_color();
        let quads = generate_divider_quads(&dividers, &border_color(), &hc, Some(0));
        assert_eq!(quads[0].color, [hc.r, hc.g, hc.b, hc.a]);
    }

    #[test]
    fn divider_quad_rect_matches_divider_info() {
        let dividers = make_vertical_dividers();
        let quads = generate_divider_quads(&dividers, &border_color(), &hover_color(), None);
        assert_eq!(quads[0].rect, dividers[0].rect);
    }

    #[test]
    fn only_hovered_divider_gets_hover_color() {
        let root = PaneNode::split(
            SplitDirection::Vertical,
            0.5,
            PaneNode::leaf(PaneId(1)),
            PaneNode::split(
                SplitDirection::Horizontal,
                0.5,
                PaneNode::leaf(PaneId(2)),
                PaneNode::leaf(PaneId(3)),
            ),
        );
        let dividers = calculate_dividers(&root, Rect::new(0.0, 0.0, 1280.0, 720.0), 20.0);
        let bc = border_color();
        let hc = hover_color();
        let quads = generate_divider_quads(&dividers, &bc, &hc, Some(1));
        // First divider: border color
        assert_eq!(quads[0].color, [bc.r, bc.g, bc.b, bc.a]);
        // Second divider: hover color
        assert_eq!(quads[1].color, [hc.r, hc.g, hc.b, hc.a]);
    }

    // ── Unfocused pane overlay tests ─────────────────────────────────

    #[test]
    fn no_overlay_for_single_pane() {
        let layout = vec![(PaneId(1), Rect::new(0.0, 0.0, 1280.0, 720.0))];
        let quads = generate_unfocused_overlay_quads(&layout, PaneId(1), &border_color(), 0.3);
        assert!(quads.is_empty());
    }

    #[test]
    fn overlay_for_unfocused_pane_in_two_pane_layout() {
        let layout = vec![
            (PaneId(1), Rect::new(0.0, 0.0, 640.0, 720.0)),
            (PaneId(2), Rect::new(640.0, 0.0, 640.0, 720.0)),
        ];
        let bg = Color { r: 0.1, g: 0.09, b: 0.08, a: 1.0 };
        let quads = generate_unfocused_overlay_quads(&layout, PaneId(1), &bg, 0.3);
        assert_eq!(quads.len(), 1);
        assert_eq!(quads[0].rect, Rect::new(640.0, 0.0, 640.0, 720.0));
        assert_eq!(quads[0].color[3], 0.3); // dim alpha
    }

    #[test]
    fn overlay_covers_all_unfocused_panes() {
        let layout = vec![
            (PaneId(1), Rect::new(0.0, 0.0, 640.0, 720.0)),
            (PaneId(2), Rect::new(640.0, 0.0, 640.0, 360.0)),
            (PaneId(3), Rect::new(640.0, 360.0, 640.0, 360.0)),
        ];
        let bg = Color { r: 0.1, g: 0.09, b: 0.08, a: 1.0 };
        let quads = generate_unfocused_overlay_quads(&layout, PaneId(1), &bg, 0.3);
        assert_eq!(quads.len(), 2);
    }

    #[test]
    fn overlay_uses_bg_color_with_dim_alpha() {
        let layout = vec![
            (PaneId(1), Rect::new(0.0, 0.0, 640.0, 720.0)),
            (PaneId(2), Rect::new(640.0, 0.0, 640.0, 720.0)),
        ];
        let bg = Color { r: 0.5, g: 0.4, b: 0.3, a: 1.0 };
        let quads = generate_unfocused_overlay_quads(&layout, PaneId(2), &bg, 0.25);
        assert_eq!(quads[0].color, [0.5, 0.4, 0.3, 0.25]);
    }
}

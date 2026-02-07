// Divider geometry: compute divider rects from the pane tree layout.

use super::{PaneNode, Rect, SplitDirection};

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
        // Right half starts at x=640, width=640
        assert_eq!(h_divider.rect.x, 640.0);
        assert_eq!(h_divider.rect.width, 640.0);
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
        // Root divider at x=600, inner divider at x=300 (left half split)
        assert_eq!(dividers[0].rect.x, 599.0);
        assert_eq!(dividers[1].rect.x, 299.0);
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
}

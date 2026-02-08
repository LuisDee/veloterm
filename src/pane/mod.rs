// Pane layout engine: binary tree data structure for terminal pane management.

pub mod divider;
pub mod interaction;

use std::sync::atomic::{AtomicU32, Ordering};

/// Global monotonically increasing pane ID counter.
static NEXT_PANE_ID: AtomicU32 = AtomicU32::new(1);

/// Unique identifier for a terminal pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PaneId(pub u32);

impl PaneId {
    /// Generate a new unique PaneId.
    pub fn next() -> Self {
        Self(NEXT_PANE_ID.fetch_add(1, Ordering::Relaxed))
    }

    /// Reset the global counter (for testing only).
    #[cfg(test)]
    pub(crate) fn reset_counter() {
        NEXT_PANE_ID.store(1, Ordering::Relaxed);
    }
}

/// Direction of a split in the pane tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    /// Horizontal split: panes stacked top/bottom.
    Horizontal,
    /// Vertical split: panes side by side left/right.
    Vertical,
}

/// A rectangle in physical pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    /// Create a new rectangle.
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Check if a point (px, py) is inside this rectangle.
    pub fn contains_point(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }

    /// Center point of this rectangle.
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }
}

/// A node in the binary pane tree.
#[derive(Debug)]
pub enum PaneNode {
    /// A leaf node containing a terminal pane.
    Leaf { id: PaneId },
    /// An internal split node with two children.
    Split {
        direction: SplitDirection,
        ratio: f32,
        first: Box<PaneNode>,
        second: Box<PaneNode>,
    },
}

impl PaneNode {
    /// Create a new leaf node.
    pub fn leaf(id: PaneId) -> Self {
        PaneNode::Leaf { id }
    }

    /// Create a new split node.
    pub fn split(direction: SplitDirection, ratio: f32, first: PaneNode, second: PaneNode) -> Self {
        PaneNode::Split {
            direction,
            ratio,
            first: Box::new(first),
            second: Box::new(second),
        }
    }

    /// Check if this node is a leaf.
    pub fn is_leaf(&self) -> bool {
        matches!(self, PaneNode::Leaf { .. })
    }

    /// Get the pane ID if this is a leaf node.
    pub fn pane_id(&self) -> Option<PaneId> {
        match self {
            PaneNode::Leaf { id } => Some(*id),
            PaneNode::Split { .. } => None,
        }
    }

    /// Count the number of leaf nodes in this subtree.
    pub fn leaf_count(&self) -> usize {
        match self {
            PaneNode::Leaf { .. } => 1,
            PaneNode::Split { first, second, .. } => first.leaf_count() + second.leaf_count(),
        }
    }

    /// Collect all leaf PaneIds in this subtree.
    pub fn leaf_ids(&self) -> Vec<PaneId> {
        match self {
            PaneNode::Leaf { id } => vec![*id],
            PaneNode::Split { first, second, .. } => {
                let mut ids = first.leaf_ids();
                ids.extend(second.leaf_ids());
                ids
            }
        }
    }

    /// Calculate layout rects for all leaf nodes given a bounding rect.
    /// Returns a Vec of (PaneId, Rect) pairs.
    pub fn calculate_layout(&self, bounds: Rect, min_size: f32) -> Vec<(PaneId, Rect)> {
        match self {
            PaneNode::Leaf { id } => vec![(*id, bounds)],
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let (first_bounds, second_bounds) =
                    split_rect(bounds, *direction, *ratio, min_size);
                let mut result = first.calculate_layout(first_bounds, min_size);
                result.extend(second.calculate_layout(second_bounds, min_size));
                result
            }
        }
    }

    /// Find and split the leaf with the given id. Returns the new pane id if found.
    pub fn split_leaf(&mut self, target: PaneId, direction: SplitDirection) -> Option<PaneId> {
        match self {
            PaneNode::Leaf { id } if *id == target => {
                let new_id = PaneId::next();
                let original = PaneNode::leaf(*id);
                let new_pane = PaneNode::leaf(new_id);
                *self = PaneNode::split(direction, 0.5, original, new_pane);
                Some(new_id)
            }
            PaneNode::Leaf { .. } => None,
            PaneNode::Split { first, second, .. } => {
                if let Some(id) = first.split_leaf(target, direction) {
                    return Some(id);
                }
                second.split_leaf(target, direction)
            }
        }
    }

    /// Remove the leaf with the given id. Returns the surviving subtree if the
    /// leaf was found and removed, or None if this node IS the target leaf.
    pub fn remove_leaf(&mut self, target: PaneId) -> RemoveResult {
        match self {
            PaneNode::Leaf { id } if *id == target => RemoveResult::RemovedSelf,
            PaneNode::Leaf { .. } => RemoveResult::NotFound,
            PaneNode::Split { first, second, .. } => {
                match first.remove_leaf(target) {
                    RemoveResult::RemovedSelf => {
                        // First child was the target; replace self with second
                        let surviving = std::mem::replace(
                            second.as_mut(),
                            PaneNode::Leaf {
                                id: PaneId(0), // placeholder
                            },
                        );
                        *self = surviving;
                        RemoveResult::Removed
                    }
                    RemoveResult::Removed => RemoveResult::Removed,
                    RemoveResult::NotFound => match second.remove_leaf(target) {
                        RemoveResult::RemovedSelf => {
                            let surviving = std::mem::replace(
                                first.as_mut(),
                                PaneNode::Leaf {
                                    id: PaneId(0), // placeholder
                                },
                            );
                            *self = surviving;
                            RemoveResult::Removed
                        }
                        other => other,
                    },
                }
            }
        }
    }
}

/// Result of a remove_leaf operation.
#[derive(Debug, PartialEq)]
pub enum RemoveResult {
    /// The node itself was the target and should be replaced by its parent.
    RemovedSelf,
    /// The target was found and removed within this subtree.
    Removed,
    /// The target was not found in this subtree.
    NotFound,
}

/// Split a rect into two sub-rects along a direction with a given ratio.
/// Clamps the ratio so neither sub-rect is smaller than min_size pixels.
pub(crate) fn split_rect(bounds: Rect, direction: SplitDirection, ratio: f32, min_size: f32) -> (Rect, Rect) {
    match direction {
        SplitDirection::Vertical => {
            let total = bounds.width;
            let clamped_ratio = clamp_ratio(ratio, total, min_size);
            let first_w = total * clamped_ratio;
            let second_w = total - first_w;
            (
                Rect::new(bounds.x, bounds.y, first_w, bounds.height),
                Rect::new(bounds.x + first_w, bounds.y, second_w, bounds.height),
            )
        }
        SplitDirection::Horizontal => {
            let total = bounds.height;
            let clamped_ratio = clamp_ratio(ratio, total, min_size);
            let first_h = total * clamped_ratio;
            let second_h = total - first_h;
            (
                Rect::new(bounds.x, bounds.y, bounds.width, first_h),
                Rect::new(bounds.x, bounds.y + first_h, bounds.width, second_h),
            )
        }
    }
}

/// Clamp a split ratio so neither side is smaller than min_size.
pub(crate) fn clamp_ratio(ratio: f32, total: f32, min_size: f32) -> f32 {
    if total <= 0.0 || total < 2.0 * min_size {
        return 0.5; // Can't satisfy constraint; split evenly
    }
    let min_ratio = min_size / total;
    let max_ratio = 1.0 - min_ratio;
    ratio.clamp(min_ratio, max_ratio)
}

/// Direction for focus navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusDirection {
    Up,
    Down,
    Left,
    Right,
}

/// The pane tree: manages the root node, focus, and zoom state.
pub struct PaneTree {
    root: PaneNode,
    focused: PaneId,
    zoomed: Option<PaneId>,
}

impl PaneTree {
    /// Create a new PaneTree with a single root pane.
    pub fn new() -> Self {
        let id = PaneId::next();
        Self {
            root: PaneNode::leaf(id),
            focused: id,
            zoomed: None,
        }
    }

    /// Get the currently focused pane ID.
    pub fn focused_pane_id(&self) -> PaneId {
        self.focused
    }

    /// Get all leaf pane IDs.
    pub fn pane_ids(&self) -> Vec<PaneId> {
        self.root.leaf_ids()
    }

    /// Get the number of panes.
    pub fn pane_count(&self) -> usize {
        self.root.leaf_count()
    }

    /// Split the focused pane in the given direction.
    /// Returns the new pane's ID, or None if the split fails.
    /// If zoomed, exits zoom first.
    pub fn split_focused(&mut self, direction: SplitDirection) -> Option<PaneId> {
        if self.zoomed.is_some() {
            self.zoomed = None;
        }
        let new_id = self.root.split_leaf(self.focused, direction)?;
        self.focused = new_id;
        Some(new_id)
    }

    /// Close the focused pane. Returns None if this was the last pane.
    /// If zoomed, exits zoom first.
    pub fn close_focused(&mut self) -> Option<PaneId> {
        if self.zoomed.is_some() {
            self.zoomed = None;
        }
        if self.pane_count() <= 1 {
            return None; // Last pane
        }
        let target = self.focused;
        match self.root.remove_leaf(target) {
            RemoveResult::Removed => {
                // Focus the first available pane
                let ids = self.pane_ids();
                self.focused = ids[0];
                Some(self.focused)
            }
            _ => None,
        }
    }

    /// Calculate layout rects for all leaf panes given window dimensions.
    pub fn calculate_layout(&self, window_width: f32, window_height: f32) -> Vec<(PaneId, Rect)> {
        let min_size = 20.0; // Minimum pane size in pixels
        let bounds = Rect::new(0.0, 0.0, window_width, window_height);

        if let Some(zoomed_id) = self.zoomed {
            // When zoomed, only the zoomed pane gets the full window
            return vec![(zoomed_id, bounds)];
        }

        self.root.calculate_layout(bounds, min_size)
    }

    /// Get the IDs of visible panes (all if not zoomed, just the zoomed one if zoomed).
    pub fn visible_panes(&self) -> Vec<PaneId> {
        if let Some(zoomed_id) = self.zoomed {
            vec![zoomed_id]
        } else {
            self.pane_ids()
        }
    }

    /// Toggle zoom on the focused pane.
    pub fn zoom_toggle(&mut self) {
        if self.pane_count() <= 1 {
            return; // No-op on single pane
        }
        if self.zoomed.is_some() {
            self.zoomed = None;
        } else {
            self.zoomed = Some(self.focused);
        }
    }

    /// Check if a pane is currently zoomed.
    pub fn is_zoomed(&self) -> bool {
        self.zoomed.is_some()
    }

    /// Navigate focus in the given direction based on pane layout rects.
    pub fn focus_direction(
        &mut self,
        direction: FocusDirection,
        window_width: f32,
        window_height: f32,
    ) {
        let layout = self.calculate_layout(window_width, window_height);
        let current_rect = layout
            .iter()
            .find(|(id, _)| *id == self.focused)
            .map(|(_, r)| *r);

        let current_rect = match current_rect {
            Some(r) => r,
            None => return,
        };

        let (cx, cy) = current_rect.center();

        // Find the nearest pane in the requested direction
        let mut best: Option<(PaneId, f32)> = None;

        for &(id, rect) in &layout {
            if id == self.focused {
                continue;
            }
            let (px, py) = rect.center();

            let is_in_direction = match direction {
                FocusDirection::Left => px < cx,
                FocusDirection::Right => px > cx,
                FocusDirection::Up => py < cy,
                FocusDirection::Down => py > cy,
            };

            if !is_in_direction {
                continue;
            }

            let dist = (px - cx).powi(2) + (py - cy).powi(2);
            if best.is_none() || dist < best.unwrap().1 {
                best = Some((id, dist));
            }
        }

        if let Some((id, _)) = best {
            self.focused = id;
        }
    }

    /// Get a reference to the root PaneNode.
    pub fn root(&self) -> &PaneNode {
        &self.root
    }

    /// Set focus to a specific pane ID. No-op if the pane doesn't exist.
    pub fn set_focus(&mut self, pane_id: PaneId) {
        if self.root.leaf_ids().contains(&pane_id) {
            self.focused = pane_id;
        }
    }

    /// Update the ratio of the split node at the given pre-order split_index.
    /// Returns true if the split was found and updated.
    pub fn set_split_ratio_by_index(&mut self, split_index: usize, new_ratio: f32) -> bool {
        let mut current = 0;
        Self::set_ratio_recursive(&mut self.root, split_index, new_ratio, &mut current)
    }

    fn set_ratio_recursive(
        node: &mut PaneNode,
        target: usize,
        new_ratio: f32,
        current: &mut usize,
    ) -> bool {
        match node {
            PaneNode::Leaf { .. } => false,
            PaneNode::Split {
                ratio,
                first,
                second,
                ..
            } => {
                if *current == target {
                    *ratio = new_ratio;
                    return true;
                }
                *current += 1;
                if Self::set_ratio_recursive(first, target, new_ratio, current) {
                    return true;
                }
                Self::set_ratio_recursive(second, target, new_ratio, current)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── PaneId tests ──────────────────────────────────────────────────

    #[test]
    fn pane_id_uniqueness_from_generator() {
        PaneId::reset_counter();
        let id1 = PaneId::next();
        let id2 = PaneId::next();
        let id3 = PaneId::next();
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }

    #[test]
    fn pane_id_monotonically_increasing() {
        PaneId::reset_counter();
        let id1 = PaneId::next();
        let id2 = PaneId::next();
        assert!(id2.0 > id1.0);
    }

    #[test]
    fn pane_id_equality() {
        let a = PaneId(42);
        let b = PaneId(42);
        assert_eq!(a, b);
    }

    #[test]
    fn pane_id_inequality() {
        let a = PaneId(1);
        let b = PaneId(2);
        assert_ne!(a, b);
    }

    // ── SplitDirection tests ──────────────────────────────────────────

    #[test]
    fn split_direction_horizontal_variant() {
        let dir = SplitDirection::Horizontal;
        assert_eq!(dir, SplitDirection::Horizontal);
        assert_ne!(dir, SplitDirection::Vertical);
    }

    #[test]
    fn split_direction_vertical_variant() {
        let dir = SplitDirection::Vertical;
        assert_eq!(dir, SplitDirection::Vertical);
        assert_ne!(dir, SplitDirection::Horizontal);
    }

    // ── Rect tests ────────────────────────────────────────────────────

    #[test]
    fn rect_construction() {
        let r = Rect::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(r.x, 10.0);
        assert_eq!(r.y, 20.0);
        assert_eq!(r.width, 100.0);
        assert_eq!(r.height, 50.0);
    }

    #[test]
    fn rect_contains_point_inside() {
        let r = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert!(r.contains_point(50.0, 50.0));
    }

    #[test]
    fn rect_contains_point_on_top_left_edge() {
        let r = Rect::new(10.0, 20.0, 100.0, 50.0);
        assert!(r.contains_point(10.0, 20.0));
    }

    #[test]
    fn rect_does_not_contain_point_on_bottom_right_edge() {
        let r = Rect::new(0.0, 0.0, 100.0, 100.0);
        // Exclusive on the right/bottom edge
        assert!(!r.contains_point(100.0, 100.0));
    }

    #[test]
    fn rect_does_not_contain_point_outside() {
        let r = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert!(!r.contains_point(-1.0, 50.0));
        assert!(!r.contains_point(50.0, -1.0));
        assert!(!r.contains_point(101.0, 50.0));
        assert!(!r.contains_point(50.0, 101.0));
    }

    #[test]
    fn rect_center() {
        let r = Rect::new(0.0, 0.0, 100.0, 200.0);
        assert_eq!(r.center(), (50.0, 100.0));
    }

    // ── PaneNode tests ────────────────────────────────────────────────

    #[test]
    fn pane_node_leaf_creation() {
        let id = PaneId(1);
        let node = PaneNode::leaf(id);
        assert!(node.is_leaf());
        assert_eq!(node.pane_id(), Some(id));
    }

    #[test]
    fn pane_node_split_creation() {
        let node = PaneNode::split(
            SplitDirection::Vertical,
            0.5,
            PaneNode::leaf(PaneId(1)),
            PaneNode::leaf(PaneId(2)),
        );
        assert!(!node.is_leaf());
        assert_eq!(node.pane_id(), None);
    }

    #[test]
    fn pane_node_leaf_count_single() {
        let node = PaneNode::leaf(PaneId(1));
        assert_eq!(node.leaf_count(), 1);
    }

    #[test]
    fn pane_node_leaf_count_split() {
        let node = PaneNode::split(
            SplitDirection::Vertical,
            0.5,
            PaneNode::leaf(PaneId(1)),
            PaneNode::leaf(PaneId(2)),
        );
        assert_eq!(node.leaf_count(), 2);
    }

    #[test]
    fn pane_node_leaf_ids() {
        let node = PaneNode::split(
            SplitDirection::Vertical,
            0.5,
            PaneNode::leaf(PaneId(1)),
            PaneNode::leaf(PaneId(2)),
        );
        let ids = node.leaf_ids();
        assert_eq!(ids, vec![PaneId(1), PaneId(2)]);
    }

    // ── PaneTree tests ────────────────────────────────────────────────

    #[test]
    fn new_tree_has_single_root_pane() {
        let tree = PaneTree::new();
        assert_eq!(tree.pane_count(), 1);
    }

    #[test]
    fn new_tree_focused_pane_is_root() {
        let tree = PaneTree::new();
        let ids = tree.pane_ids();
        assert_eq!(tree.focused_pane_id(), ids[0]);
    }

    #[test]
    fn vertical_split_produces_two_leaves() {
        let mut tree = PaneTree::new();
        let new_id = tree.split_focused(SplitDirection::Vertical);
        assert!(new_id.is_some());
        assert_eq!(tree.pane_count(), 2);
    }

    #[test]
    fn horizontal_split_produces_two_leaves() {
        let mut tree = PaneTree::new();
        let new_id = tree.split_focused(SplitDirection::Horizontal);
        assert!(new_id.is_some());
        assert_eq!(tree.pane_count(), 2);
    }

    #[test]
    fn split_updates_focus_to_new_pane() {
        let mut tree = PaneTree::new();
        let new_id = tree.split_focused(SplitDirection::Vertical).unwrap();
        assert_eq!(tree.focused_pane_id(), new_id);
    }

    #[test]
    fn close_pane_with_sibling_promotes_sibling() {
        let mut tree = PaneTree::new();
        let original_id = tree.focused_pane_id();
        tree.split_focused(SplitDirection::Vertical);
        // Focus is now on the new pane; close it
        let surviving = tree.close_focused();
        assert!(surviving.is_some());
        assert_eq!(tree.pane_count(), 1);
        assert_eq!(tree.focused_pane_id(), original_id);
    }

    #[test]
    fn close_last_pane_returns_none() {
        let mut tree = PaneTree::new();
        let result = tree.close_focused();
        assert!(result.is_none());
        assert_eq!(tree.pane_count(), 1); // Still has the pane
    }

    #[test]
    fn nested_split_then_close_preserves_tree_structure() {
        let mut tree = PaneTree::new();
        let root_id = tree.focused_pane_id();
        // Split root → [root | B]
        let b_id = tree.split_focused(SplitDirection::Vertical).unwrap();
        // Focus is B, split B → [root | [B | C]]
        let _c_id = tree.split_focused(SplitDirection::Horizontal).unwrap();
        assert_eq!(tree.pane_count(), 3);

        // Close C (focused) → should leave [root | B]
        tree.close_focused();
        assert_eq!(tree.pane_count(), 2);
        let ids = tree.pane_ids();
        assert!(ids.contains(&root_id));
        assert!(ids.contains(&b_id));
    }

    // ── Layout calculation tests ──────────────────────────────────────

    #[test]
    fn single_pane_gets_full_rect() {
        let tree = PaneTree::new();
        let layout = tree.calculate_layout(1280.0, 720.0);
        assert_eq!(layout.len(), 1);
        assert_eq!(layout[0].1, Rect::new(0.0, 0.0, 1280.0, 720.0));
    }

    #[test]
    fn vertical_split_divides_width_by_ratio() {
        let mut tree = PaneTree::new();
        let root_id = tree.focused_pane_id();
        tree.split_focused(SplitDirection::Vertical);
        let layout = tree.calculate_layout(1280.0, 720.0);
        assert_eq!(layout.len(), 2);
        // Root is first child (left), new pane is second (right)
        let root_rect = layout.iter().find(|(id, _)| *id == root_id).unwrap().1;
        assert_eq!(root_rect.x, 0.0);
        assert_eq!(root_rect.width, 640.0);
        assert_eq!(root_rect.height, 720.0);
    }

    #[test]
    fn horizontal_split_divides_height_by_ratio() {
        let mut tree = PaneTree::new();
        let root_id = tree.focused_pane_id();
        tree.split_focused(SplitDirection::Horizontal);
        let layout = tree.calculate_layout(1280.0, 720.0);
        assert_eq!(layout.len(), 2);
        let root_rect = layout.iter().find(|(id, _)| *id == root_id).unwrap().1;
        assert_eq!(root_rect.y, 0.0);
        assert_eq!(root_rect.height, 360.0);
        assert_eq!(root_rect.width, 1280.0);
    }

    #[test]
    fn nested_splits_produce_correct_rects() {
        let mut tree = PaneTree::new();
        tree.split_focused(SplitDirection::Vertical);
        tree.split_focused(SplitDirection::Horizontal);
        let layout = tree.calculate_layout(1280.0, 720.0);
        assert_eq!(layout.len(), 3);
        // All rects should cover the window without overlap
        let total_area: f32 = layout.iter().map(|(_, r)| r.width * r.height).sum();
        assert!((total_area - 1280.0 * 720.0).abs() < 1.0);
    }

    #[test]
    fn minimum_pane_size_enforced_by_clamping_ratio() {
        // Create a tiny window and try to split
        let node = PaneNode::split(
            SplitDirection::Vertical,
            0.01, // Very extreme ratio
            PaneNode::leaf(PaneId(1)),
            PaneNode::leaf(PaneId(2)),
        );
        let layout = node.calculate_layout(Rect::new(0.0, 0.0, 100.0, 100.0), 20.0);
        // Both panes should be at least 20px wide
        for (_, rect) in &layout {
            assert!(
                rect.width >= 19.9, // slight float tolerance
                "pane width {} should be >= min_size 20",
                rect.width
            );
        }
    }

    #[test]
    fn zero_size_window_produces_valid_rects() {
        let tree = PaneTree::new();
        let layout = tree.calculate_layout(0.0, 0.0);
        assert_eq!(layout.len(), 1);
        assert_eq!(layout[0].1, Rect::new(0.0, 0.0, 0.0, 0.0));
    }

    // ── Focus navigation tests ────────────────────────────────────────

    #[test]
    fn focus_right_from_left_pane_moves_to_right_pane() {
        let mut tree = PaneTree::new();
        let left_id = tree.focused_pane_id();
        let right_id = tree.split_focused(SplitDirection::Vertical).unwrap();
        // Focus is on right pane; move focus to left first
        tree.focused = left_id;
        tree.focus_direction(FocusDirection::Right, 1280.0, 720.0);
        assert_eq!(tree.focused_pane_id(), right_id);
    }

    #[test]
    fn focus_left_from_right_pane_moves_to_left_pane() {
        let mut tree = PaneTree::new();
        let left_id = tree.focused_pane_id();
        tree.split_focused(SplitDirection::Vertical).unwrap();
        // Focus is on right pane
        tree.focus_direction(FocusDirection::Left, 1280.0, 720.0);
        assert_eq!(tree.focused_pane_id(), left_id);
    }

    #[test]
    fn focus_down_from_top_pane_moves_to_bottom_pane() {
        let mut tree = PaneTree::new();
        let top_id = tree.focused_pane_id();
        let bottom_id = tree.split_focused(SplitDirection::Horizontal).unwrap();
        tree.focused = top_id;
        tree.focus_direction(FocusDirection::Down, 1280.0, 720.0);
        assert_eq!(tree.focused_pane_id(), bottom_id);
    }

    #[test]
    fn focus_up_from_bottom_pane_moves_to_top_pane() {
        let mut tree = PaneTree::new();
        let top_id = tree.focused_pane_id();
        tree.split_focused(SplitDirection::Horizontal).unwrap();
        // Focus is on bottom
        tree.focus_direction(FocusDirection::Up, 1280.0, 720.0);
        assert_eq!(tree.focused_pane_id(), top_id);
    }

    #[test]
    fn focus_in_direction_with_no_neighbor_stays_on_current() {
        let mut tree = PaneTree::new();
        let id = tree.focused_pane_id();
        tree.focus_direction(FocusDirection::Right, 1280.0, 720.0);
        assert_eq!(tree.focused_pane_id(), id);
    }

    #[test]
    fn focus_navigation_in_3_pane_layout_picks_spatially_nearest() {
        let mut tree = PaneTree::new();
        let a_id = tree.focused_pane_id();
        // Split A vertically → [A | B]
        let _b_id = tree.split_focused(SplitDirection::Vertical).unwrap();
        // Split B horizontally → [A | [B / C]]
        let _c_id = tree.split_focused(SplitDirection::Horizontal).unwrap();
        // Focus is C (bottom-right). Go left → should go to A
        tree.focus_direction(FocusDirection::Left, 1280.0, 720.0);
        assert_eq!(tree.focused_pane_id(), a_id);
    }

    // ── Zoom tests ────────────────────────────────────────────────────

    #[test]
    fn zoom_sets_zoomed_pane_id() {
        let mut tree = PaneTree::new();
        tree.split_focused(SplitDirection::Vertical);
        tree.zoom_toggle();
        assert!(tree.is_zoomed());
    }

    #[test]
    fn visible_panes_returns_only_zoomed_pane_when_zoomed() {
        let mut tree = PaneTree::new();
        tree.split_focused(SplitDirection::Vertical);
        let focused = tree.focused_pane_id();
        tree.zoom_toggle();
        let visible = tree.visible_panes();
        assert_eq!(visible, vec![focused]);
    }

    #[test]
    fn unzoom_restores_all_panes_as_visible() {
        let mut tree = PaneTree::new();
        tree.split_focused(SplitDirection::Vertical);
        tree.zoom_toggle();
        tree.zoom_toggle(); // unzoom
        assert!(!tree.is_zoomed());
        assert_eq!(tree.visible_panes().len(), 2);
    }

    #[test]
    fn split_while_zoomed_exits_zoom_first() {
        let mut tree = PaneTree::new();
        tree.split_focused(SplitDirection::Vertical);
        tree.zoom_toggle();
        assert!(tree.is_zoomed());
        tree.split_focused(SplitDirection::Horizontal);
        assert!(!tree.is_zoomed());
        assert_eq!(tree.pane_count(), 3);
    }

    #[test]
    fn close_while_zoomed_exits_zoom_first() {
        let mut tree = PaneTree::new();
        tree.split_focused(SplitDirection::Vertical);
        tree.zoom_toggle();
        assert!(tree.is_zoomed());
        tree.close_focused();
        assert!(!tree.is_zoomed());
    }

    #[test]
    fn zoom_on_single_pane_is_no_op() {
        let mut tree = PaneTree::new();
        tree.zoom_toggle();
        assert!(!tree.is_zoomed());
    }

    // ── root() accessor ──────────────────────────────────────────────

    #[test]
    fn root_returns_reference_to_root_node() {
        let tree = PaneTree::new();
        assert!(tree.root().is_leaf());
    }

    #[test]
    fn root_reflects_splits() {
        let mut tree = PaneTree::new();
        tree.split_focused(SplitDirection::Vertical);
        assert!(!tree.root().is_leaf());
        assert_eq!(tree.root().leaf_count(), 2);
    }

    // ── set_focus() ──────────────────────────────────────────────────

    #[test]
    fn set_focus_changes_focused_pane() {
        let mut tree = PaneTree::new();
        let first_id = tree.focused_pane_id();
        let second_id = tree.split_focused(SplitDirection::Vertical).unwrap();
        assert_eq!(tree.focused_pane_id(), second_id);
        tree.set_focus(first_id);
        assert_eq!(tree.focused_pane_id(), first_id);
    }

    #[test]
    fn set_focus_with_invalid_id_is_no_op() {
        let mut tree = PaneTree::new();
        let original = tree.focused_pane_id();
        tree.set_focus(PaneId(99999));
        assert_eq!(tree.focused_pane_id(), original);
    }

    // ── set_split_ratio_by_index() ──────────────────────────────────

    #[test]
    fn set_split_ratio_updates_root_split() {
        let mut tree = PaneTree::new();
        tree.split_focused(SplitDirection::Vertical);
        assert!(tree.set_split_ratio_by_index(0, 0.3));
        // Verify by checking layout: first pane should be ~30% of width
        let layout = tree.calculate_layout(1000.0, 500.0);
        let first_width = layout[0].1.width;
        assert!((first_width - 300.0).abs() < 1.0);
    }

    #[test]
    fn set_split_ratio_updates_nested_split() {
        let mut tree = PaneTree::new();
        // [A | B] → [A | [B / C]]
        tree.split_focused(SplitDirection::Vertical); // index 0
        tree.split_focused(SplitDirection::Horizontal); // index 1
        assert!(tree.set_split_ratio_by_index(1, 0.25));
        // The horizontal split is at index 1
        let layout = tree.calculate_layout(1000.0, 1000.0);
        // Right side is split horizontally: top should be ~25% of 1000
        // Right side starts at x=500, so find the panes with x >= 500
        let right_panes: Vec<_> = layout.iter().filter(|(_, r)| r.x >= 499.0).collect();
        assert_eq!(right_panes.len(), 2);
        let top_height = right_panes.iter().map(|(_, r)| r.height).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        assert!(top_height < 300.0, "top pane should be ~25% of height");
    }

    #[test]
    fn set_split_ratio_with_invalid_index_returns_false() {
        let mut tree = PaneTree::new();
        tree.split_focused(SplitDirection::Vertical);
        assert!(!tree.set_split_ratio_by_index(5, 0.3));
    }

    #[test]
    fn set_split_ratio_on_single_pane_returns_false() {
        let mut tree = PaneTree::new();
        assert!(!tree.set_split_ratio_by_index(0, 0.5));
    }
}

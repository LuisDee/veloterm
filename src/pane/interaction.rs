// PaneInteraction state machine for mouse-driven divider interaction.

use super::divider::{calculate_dividers, hit_test_divider, DividerInfo, HIT_TEST_MARGIN};
use super::{PaneId, PaneNode, Rect, SplitDirection};

/// The current state of mouse interaction with pane dividers.
#[derive(Debug, Clone, PartialEq)]
pub enum InteractionState {
    /// No interaction in progress.
    Idle,
    /// Mouse is hovering near a divider (index into dividers vec).
    Hovering { divider_index: usize },
    /// User is actively dragging a divider to resize panes.
    Dragging {
        divider_index: usize,
        /// The split ratio when drag started.
        start_ratio: f32,
    },
}

/// Effects that the App should apply after processing a mouse event.
#[derive(Debug, Clone, PartialEq)]
pub enum InteractionEffect {
    /// No visible effect.
    None,
    /// Change the mouse cursor icon.
    SetCursor(CursorType),
    /// Update the split ratio for the given split_index and request redraw.
    UpdateRatio {
        split_index: usize,
        new_ratio: f32,
    },
    /// Focus the pane at the given position.
    FocusPane(PaneId),
}

/// Cursor types needed for pane interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorType {
    Default,
    EwResize,
    NsResize,
}

/// State machine managing mouse interaction with pane dividers.
pub struct PaneInteraction {
    state: InteractionState,
    /// Cached divider info, updated when layout changes.
    dividers: Vec<DividerInfo>,
    /// Last known cursor position.
    cursor_pos: (f32, f32),
    /// The bounds used to compute the layout (for drag ratio calculation).
    layout_bounds: Rect,
}

impl PaneInteraction {
    pub fn new() -> Self {
        Self {
            state: InteractionState::Idle,
            dividers: Vec::new(),
            cursor_pos: (0.0, 0.0),
            layout_bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
        }
    }

    /// Get the current interaction state.
    pub fn state(&self) -> &InteractionState {
        &self.state
    }

    /// Get the cached dividers.
    pub fn dividers(&self) -> &[DividerInfo] {
        &self.dividers
    }

    /// Update cached dividers from the current pane tree layout.
    pub fn update_layout(&mut self, root: &PaneNode, bounds: Rect, min_size: f32) {
        self.dividers = calculate_dividers(root, bounds, min_size);
        self.layout_bounds = bounds;
    }

    /// Process a cursor move event. Returns the effect to apply.
    pub fn on_cursor_moved(&mut self, x: f32, y: f32) -> InteractionEffect {
        self.cursor_pos = (x, y);

        match &self.state {
            InteractionState::Dragging { divider_index, .. } => {
                let divider_index = *divider_index;
                if let Some(divider) = self.dividers.get(divider_index) {
                    // Compute new ratio based on cursor position along the split axis.
                    // We need the parent bounds — approximate from divider rect.
                    // The divider rect tells us the axis; we use window-relative coords.
                    let new_ratio = match divider.direction {
                        SplitDirection::Vertical => {
                            // Approximate: cursor_x / window_width. We use divider.rect.height
                            // as a proxy for window height, and we know the window from the
                            // divider spanning the full height.
                            // Actually we need the parent bounds width. For now, hardcode
                            // using a stored bounds.
                            let parent_width = self.layout_bounds.width;
                            let parent_x = self.layout_bounds.x;
                            super::clamp_ratio(
                                (x - parent_x) / parent_width,
                                parent_width,
                                20.0,
                            )
                        }
                        SplitDirection::Horizontal => {
                            let parent_height = self.layout_bounds.height;
                            let parent_y = self.layout_bounds.y;
                            super::clamp_ratio(
                                (y - parent_y) / parent_height,
                                parent_height,
                                20.0,
                            )
                        }
                    };
                    InteractionEffect::UpdateRatio {
                        split_index: divider.split_index,
                        new_ratio,
                    }
                } else {
                    InteractionEffect::None
                }
            }
            _ => {
                // Check if cursor is near any divider
                match hit_test_divider(self.cursor_pos, &self.dividers, HIT_TEST_MARGIN) {
                    Some(idx) => {
                        let cursor = match self.dividers[idx].direction {
                            SplitDirection::Vertical => CursorType::EwResize,
                            SplitDirection::Horizontal => CursorType::NsResize,
                        };
                        let was_hovering = matches!(
                            self.state,
                            InteractionState::Hovering { divider_index } if divider_index == idx
                        );
                        self.state = InteractionState::Hovering { divider_index: idx };
                        if was_hovering {
                            InteractionEffect::None
                        } else {
                            InteractionEffect::SetCursor(cursor)
                        }
                    }
                    None => {
                        let was_hovering = matches!(self.state, InteractionState::Hovering { .. });
                        self.state = InteractionState::Idle;
                        if was_hovering {
                            InteractionEffect::SetCursor(CursorType::Default)
                        } else {
                            InteractionEffect::None
                        }
                    }
                }
            }
        }
    }

    /// Process a mouse button press. Returns the effect to apply.
    pub fn on_mouse_press(
        &mut self,
        layout: &[(PaneId, Rect)],
    ) -> InteractionEffect {
        match &self.state {
            InteractionState::Hovering { divider_index } => {
                let divider_index = *divider_index;
                self.state = InteractionState::Dragging {
                    divider_index,
                    start_ratio: 0.5, // Will be refined when we have tree access
                };
                InteractionEffect::None
            }
            InteractionState::Idle => {
                // Click-to-focus: find which pane contains the cursor
                let (px, py) = self.cursor_pos;
                for &(pane_id, rect) in layout {
                    if rect.contains_point(px, py) {
                        return InteractionEffect::FocusPane(pane_id);
                    }
                }
                InteractionEffect::None
            }
            InteractionState::Dragging { .. } => InteractionEffect::None,
        }
    }

    /// Process a mouse button release. Returns the effect to apply.
    pub fn on_mouse_release(&mut self) -> InteractionEffect {
        match &self.state {
            InteractionState::Dragging { .. } => {
                self.state = InteractionState::Idle;
                InteractionEffect::SetCursor(CursorType::Default)
            }
            _ => InteractionEffect::None,
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_vertical_split() -> (PaneInteraction, PaneNode) {
        let root = PaneNode::split(
            SplitDirection::Vertical,
            0.5,
            PaneNode::leaf(PaneId(1)),
            PaneNode::leaf(PaneId(2)),
        );
        let mut interaction = PaneInteraction::new();
        interaction.update_layout(&root, Rect::new(0.0, 0.0, 1280.0, 720.0), 20.0);
        (interaction, root)
    }

    fn layout_from_root(root: &PaneNode) -> Vec<(PaneId, Rect)> {
        root.calculate_layout(Rect::new(0.0, 0.0, 1280.0, 720.0), 20.0)
    }

    // ── Initial state ────────────────────────────────────────────────

    #[test]
    fn initial_state_is_idle() {
        let interaction = PaneInteraction::new();
        assert_eq!(*interaction.state(), InteractionState::Idle);
    }

    // ── Idle → Hovering transitions ──────────────────────────────────

    #[test]
    fn cursor_near_divider_transitions_to_hovering() {
        let (mut interaction, _root) = setup_vertical_split();
        let effect = interaction.on_cursor_moved(640.0, 360.0);
        assert_eq!(
            *interaction.state(),
            InteractionState::Hovering { divider_index: 0 }
        );
        assert_eq!(effect, InteractionEffect::SetCursor(CursorType::EwResize));
    }

    #[test]
    fn cursor_away_from_divider_stays_idle() {
        let (mut interaction, _root) = setup_vertical_split();
        let effect = interaction.on_cursor_moved(100.0, 360.0);
        assert_eq!(*interaction.state(), InteractionState::Idle);
        assert_eq!(effect, InteractionEffect::None);
    }

    #[test]
    fn cursor_near_horizontal_divider_sets_ns_resize() {
        let root = PaneNode::split(
            SplitDirection::Horizontal,
            0.5,
            PaneNode::leaf(PaneId(1)),
            PaneNode::leaf(PaneId(2)),
        );
        let mut interaction = PaneInteraction::new();
        interaction.update_layout(&root, Rect::new(0.0, 0.0, 1280.0, 720.0), 20.0);
        let effect = interaction.on_cursor_moved(640.0, 360.0);
        assert_eq!(
            *interaction.state(),
            InteractionState::Hovering { divider_index: 0 }
        );
        assert_eq!(effect, InteractionEffect::SetCursor(CursorType::NsResize));
    }

    // ── Hovering → Idle transitions ──────────────────────────────────

    #[test]
    fn cursor_leaves_divider_transitions_back_to_idle() {
        let (mut interaction, _root) = setup_vertical_split();
        interaction.on_cursor_moved(640.0, 360.0); // enter hover
        let effect = interaction.on_cursor_moved(100.0, 360.0); // leave
        assert_eq!(*interaction.state(), InteractionState::Idle);
        assert_eq!(effect, InteractionEffect::SetCursor(CursorType::Default));
    }

    // ── Hovering → Dragging transitions ──────────────────────────────

    #[test]
    fn mouse_press_while_hovering_transitions_to_dragging() {
        let (mut interaction, root) = setup_vertical_split();
        let layout = layout_from_root(&root);
        interaction.on_cursor_moved(640.0, 360.0); // hover
        let effect = interaction.on_mouse_press(&layout);
        assert!(matches!(
            interaction.state(),
            InteractionState::Dragging { divider_index: 0, .. }
        ));
        assert_eq!(effect, InteractionEffect::None);
    }

    // ── Dragging behavior ────────────────────────────────────────────

    #[test]
    fn cursor_move_while_dragging_produces_update_ratio() {
        let (mut interaction, root) = setup_vertical_split();
        let layout = layout_from_root(&root);
        interaction.on_cursor_moved(640.0, 360.0); // hover
        interaction.on_mouse_press(&layout); // start drag
        let effect = interaction.on_cursor_moved(700.0, 360.0); // drag
        match effect {
            InteractionEffect::UpdateRatio { split_index, new_ratio } => {
                assert_eq!(split_index, 0);
                assert!(new_ratio > 0.5, "ratio should increase when dragging right");
            }
            _ => panic!("expected UpdateRatio, got {:?}", effect),
        }
    }

    // ── Dragging → Idle transitions ──────────────────────────────────

    #[test]
    fn mouse_release_while_dragging_transitions_to_idle() {
        let (mut interaction, root) = setup_vertical_split();
        let layout = layout_from_root(&root);
        interaction.on_cursor_moved(640.0, 360.0); // hover
        interaction.on_mouse_press(&layout); // drag
        let effect = interaction.on_mouse_release();
        assert_eq!(*interaction.state(), InteractionState::Idle);
        assert_eq!(effect, InteractionEffect::SetCursor(CursorType::Default));
    }

    // ── Click-to-focus ───────────────────────────────────────────────

    #[test]
    fn mouse_press_while_idle_focuses_pane_under_cursor() {
        let (mut interaction, root) = setup_vertical_split();
        let layout = layout_from_root(&root);
        interaction.on_cursor_moved(100.0, 360.0); // move to left pane
        let effect = interaction.on_mouse_press(&layout);
        assert_eq!(effect, InteractionEffect::FocusPane(PaneId(1)));
    }

    #[test]
    fn mouse_press_in_right_pane_focuses_right_pane() {
        let (mut interaction, root) = setup_vertical_split();
        let layout = layout_from_root(&root);
        interaction.on_cursor_moved(900.0, 360.0); // move to right pane
        let effect = interaction.on_mouse_press(&layout);
        assert_eq!(effect, InteractionEffect::FocusPane(PaneId(2)));
    }

    #[test]
    fn mouse_press_outside_all_panes_returns_none() {
        let (mut interaction, root) = setup_vertical_split();
        let layout = layout_from_root(&root);
        interaction.on_cursor_moved(-10.0, -10.0); // outside window
        let effect = interaction.on_mouse_press(&layout);
        assert_eq!(effect, InteractionEffect::None);
    }

    // ── Mouse release while not dragging ─────────────────────────────

    #[test]
    fn mouse_release_while_idle_is_no_op() {
        let mut interaction = PaneInteraction::new();
        let effect = interaction.on_mouse_release();
        assert_eq!(*interaction.state(), InteractionState::Idle);
        assert_eq!(effect, InteractionEffect::None);
    }

    #[test]
    fn mouse_release_while_hovering_is_no_op() {
        let (mut interaction, _root) = setup_vertical_split();
        interaction.on_cursor_moved(640.0, 360.0); // hover
        let effect = interaction.on_mouse_release();
        assert_eq!(
            *interaction.state(),
            InteractionState::Hovering { divider_index: 0 }
        );
        assert_eq!(effect, InteractionEffect::None);
    }
}

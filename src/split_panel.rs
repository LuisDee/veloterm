// SplitPanel widget — horizontal split with draggable vertical divider.

use iced_core::layout::{Limits, Node};
use iced_core::mouse;
use iced_core::renderer::Style;
use iced_core::widget::Tree;
use iced_core::{
    Clipboard, Element, Event, Layout, Length, Rectangle, Shell, Size, Widget,
};

/// Default divider width in logical pixels.
const DEFAULT_DIVIDER_WIDTH: f32 = 4.0;
/// Default minimum ratio (20%).
const DEFAULT_MIN_RATIO: f32 = 0.2;
/// Default maximum ratio (80%).
const DEFAULT_MAX_RATIO: f32 = 0.8;
/// Default (reset) ratio (50%).
const DEFAULT_RATIO: f32 = 0.5;
/// Double-click detection threshold in milliseconds.
const DOUBLE_CLICK_MS: u128 = 400;

/// Clamp a ratio to the given min/max bounds.
pub fn clamp_ratio(ratio: f32, min: f32, max: f32) -> f32 {
    ratio.clamp(min, max)
}

/// Test whether cursor_x is within the divider hit zone.
pub fn divider_hit_test(cursor_x: f32, total_width: f32, ratio: f32, divider_width: f32) -> bool {
    let left_w = left_panel_width(total_width, ratio, divider_width);
    let divider_start = left_w;
    let divider_end = divider_start + divider_width;
    cursor_x >= divider_start && cursor_x <= divider_end
}

/// Compute the width of the left panel.
pub fn left_panel_width(total_width: f32, ratio: f32, divider_width: f32) -> f32 {
    (total_width - divider_width) * ratio
}

/// Convert a cursor x position to a split ratio.
pub fn ratio_from_cursor(cursor_x: f32, total_width: f32, min: f32, max: f32) -> f32 {
    if total_width <= 0.0 {
        return DEFAULT_RATIO;
    }
    clamp_ratio(cursor_x / total_width, min, max)
}

/// A horizontal split panel with a draggable divider.
///
/// Renders `left` and `right` children separated by a vertical divider.
/// The divider can be dragged to resize. Double-click resets to 50%.
pub struct SplitPanel<'a, Message> {
    left: Element<'a, Message, iced_core::Theme, iced_wgpu::Renderer>,
    right: Element<'a, Message, iced_core::Theme, iced_wgpu::Renderer>,
    ratio: f32,
    on_resize: Option<Box<dyn Fn(f32) -> Message + 'a>>,
    on_reset: Option<Message>,
    divider_width: f32,
    min_ratio: f32,
    max_ratio: f32,
}

impl<'a, Message> SplitPanel<'a, Message> {
    pub fn new(
        left: impl Into<Element<'a, Message, iced_core::Theme, iced_wgpu::Renderer>>,
        right: impl Into<Element<'a, Message, iced_core::Theme, iced_wgpu::Renderer>>,
        ratio: f32,
    ) -> Self {
        Self {
            left: left.into(),
            right: right.into(),
            ratio: clamp_ratio(ratio, DEFAULT_MIN_RATIO, DEFAULT_MAX_RATIO),
            on_resize: None,
            on_reset: None,
            divider_width: DEFAULT_DIVIDER_WIDTH,
            min_ratio: DEFAULT_MIN_RATIO,
            max_ratio: DEFAULT_MAX_RATIO,
        }
    }

    pub fn on_resize(mut self, f: impl Fn(f32) -> Message + 'a) -> Self {
        self.on_resize = Some(Box::new(f));
        self
    }

    pub fn on_reset(mut self, msg: Message) -> Self {
        self.on_reset = Some(msg);
        self
    }

    pub fn divider_width(mut self, width: f32) -> Self {
        self.divider_width = width;
        self
    }

    pub fn min_ratio(mut self, min: f32) -> Self {
        self.min_ratio = min;
        self
    }

    pub fn max_ratio(mut self, max: f32) -> Self {
        self.max_ratio = max;
        self
    }
}

/// Internal state for the SplitPanel widget (drag tracking).
struct SplitPanelState {
    dragging: bool,
    last_click: Option<std::time::Instant>,
}

impl Default for SplitPanelState {
    fn default() -> Self {
        Self {
            dragging: false,
            last_click: None,
        }
    }
}

impl<'a, Message> Widget<Message, iced_core::Theme, iced_wgpu::Renderer>
    for SplitPanel<'a, Message>
where
    Message: Clone,
{
    fn tag(&self) -> iced_core::widget::tree::Tag {
        iced_core::widget::tree::Tag::of::<SplitPanelState>()
    }

    fn state(&self) -> iced_core::widget::tree::State {
        iced_core::widget::tree::State::new(SplitPanelState::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.left), Tree::new(&self.right)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.left, &self.right]);
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &iced_wgpu::Renderer, limits: &Limits) -> Node {
        let bounds = limits.max();
        let left_w = left_panel_width(bounds.width, self.ratio, self.divider_width);
        let right_w = bounds.width - left_w - self.divider_width;

        let left_limits = Limits::new(Size::ZERO, Size::new(left_w, bounds.height));
        let left_node = self.left
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, &left_limits)
            .move_to(iced_core::Point::ORIGIN);

        let divider_node = Node::new(Size::new(self.divider_width, bounds.height))
            .move_to(iced_core::Point::new(left_w, 0.0));

        let right_limits = Limits::new(Size::ZERO, Size::new(right_w, bounds.height));
        let right_node = self.right
            .as_widget_mut()
            .layout(&mut tree.children[1], renderer, &right_limits)
            .move_to(iced_core::Point::new(left_w + self.divider_width, 0.0));

        Node::with_children(bounds, vec![left_node, divider_node, right_node])
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced_wgpu::Renderer,
        theme: &iced_core::Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let children: Vec<_> = layout.children().collect();
        if children.len() < 3 {
            return;
        }

        // Draw left child
        self.left.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            children[0],
            cursor,
            viewport,
        );

        // Draw divider line
        let divider_bounds = children[1].bounds();
        iced_core::Renderer::fill_quad(
            renderer,
            iced_core::renderer::Quad {
                bounds: divider_bounds,
                border: iced_core::Border {
                    color: iced_core::Color::TRANSPARENT,
                    width: 0.0,
                    radius: 0.0.into(),
                },
                shadow: iced_core::Shadow::default(),
                snap: true,
            },
            iced_core::Background::Color(iced_core::Color::from_rgba(1.0, 1.0, 1.0, 0.15)),
        );

        // Draw right child
        self.right.as_widget().draw(
            &tree.children[1],
            renderer,
            theme,
            style,
            children[2],
            cursor,
            viewport,
        );
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced_wgpu::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<SplitPanelState>();
        let bounds = layout.bounds();
        let children: Vec<_> = layout.children().collect();
        if children.len() < 3 {
            return;
        }

        // Handle divider-specific events first — if the divider captures
        // the event we return early so children don't also process it.
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(pos) = cursor.position_in(bounds) {
                    if divider_hit_test(pos.x, bounds.width, self.ratio, self.divider_width) {
                        // Check for double-click
                        let now = std::time::Instant::now();
                        if let Some(last) = state.last_click {
                            if now.duration_since(last).as_millis() < DOUBLE_CLICK_MS {
                                // Double-click: reset
                                if let Some(msg) = &self.on_reset {
                                    shell.publish(msg.clone());
                                }
                                state.last_click = None;
                                shell.capture_event();
                                return;
                            }
                        }
                        state.last_click = Some(now);
                        state.dragging = true;
                        shell.capture_event();
                        return;
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.dragging {
                    state.dragging = false;
                    shell.capture_event();
                    return;
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if state.dragging {
                    if let Some(pos) = cursor.position_in(bounds) {
                        if let Some(on_resize) = &self.on_resize {
                            let new_ratio = ratio_from_cursor(
                                pos.x,
                                bounds.width,
                                self.min_ratio,
                                self.max_ratio,
                            );
                            shell.publish(on_resize(new_ratio));
                        }
                    }
                    shell.capture_event();
                    return;
                }
            }
            _ => {}
        }

        // Forward events to left child (children[0] layout)
        self.left.as_widget_mut().update(
            &mut tree.children[0],
            event,
            children[0],
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        // Forward events to right child (children[2] layout, tree index 1)
        self.right.as_widget_mut().update(
            &mut tree.children[1],
            event,
            children[2],
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced_wgpu::Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<SplitPanelState>();
        let bounds = layout.bounds();

        if state.dragging {
            return mouse::Interaction::ResizingHorizontally;
        }

        if let Some(pos) = cursor.position_in(bounds) {
            if divider_hit_test(pos.x, bounds.width, self.ratio, self.divider_width) {
                return mouse::Interaction::ResizingHorizontally;
            }
        }

        // Delegate to children
        let children: Vec<_> = layout.children().collect();
        if children.len() >= 3 {
            let left = self.left.as_widget().mouse_interaction(
                &tree.children[0],
                children[0],
                cursor,
                viewport,
                renderer,
            );
            if left != mouse::Interaction::default() {
                return left;
            }
            let right = self.right.as_widget().mouse_interaction(
                &tree.children[1],
                children[2],
                cursor,
                viewport,
                renderer,
            );
            if right != mouse::Interaction::default() {
                return right;
            }
        }

        mouse::Interaction::default()
    }
}

impl<'a, Message> From<SplitPanel<'a, Message>>
    for Element<'a, Message, iced_core::Theme, iced_wgpu::Renderer>
where
    Message: Clone + 'a,
{
    fn from(panel: SplitPanel<'a, Message>) -> Self {
        Self::new(panel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_panel_clamp_ratio_below_minimum() {
        assert!((clamp_ratio(0.1, 0.2, 0.8) - 0.2).abs() < f32::EPSILON);
    }

    #[test]
    fn split_panel_clamp_ratio_above_maximum() {
        assert!((clamp_ratio(0.9, 0.2, 0.8) - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn split_panel_clamp_ratio_within_range() {
        assert!((clamp_ratio(0.4, 0.2, 0.8) - 0.4).abs() < f32::EPSILON);
    }

    #[test]
    fn split_panel_clamp_ratio_at_boundaries() {
        assert!((clamp_ratio(0.2, 0.2, 0.8) - 0.2).abs() < f32::EPSILON);
        assert!((clamp_ratio(0.8, 0.2, 0.8) - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn split_panel_default_ratio_is_valid() {
        assert!(DEFAULT_RATIO >= DEFAULT_MIN_RATIO && DEFAULT_RATIO <= DEFAULT_MAX_RATIO);
    }

    #[test]
    fn split_panel_reset_ratio() {
        assert!((DEFAULT_RATIO - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn split_panel_divider_hit_test() {
        // total=1000, ratio=0.5, divider=4 -> left=498, divider at 498..502
        assert!(divider_hit_test(499.0, 1000.0, 0.5, 4.0));
        assert!(divider_hit_test(498.0, 1000.0, 0.5, 4.0));
        assert!(divider_hit_test(502.0, 1000.0, 0.5, 4.0));
        // Outside divider
        assert!(!divider_hit_test(200.0, 1000.0, 0.5, 4.0));
        assert!(!divider_hit_test(700.0, 1000.0, 0.5, 4.0));
    }

    #[test]
    fn split_panel_left_width_calculation() {
        let w = left_panel_width(1000.0, 0.5, 4.0);
        assert!((w - 498.0).abs() < f32::EPSILON);
    }

    #[test]
    fn split_panel_right_width_calculation() {
        let left = left_panel_width(1000.0, 0.3, 4.0);
        assert!((left - 298.8).abs() < 0.01);
        let right = 1000.0 - left - 4.0;
        assert!((right - 697.2).abs() < 0.01);
    }

    #[test]
    fn split_panel_ratio_from_cursor_position() {
        let r1 = ratio_from_cursor(500.0, 1000.0, 0.2, 0.8);
        assert!((r1 - 0.5).abs() < f32::EPSILON);
        // 200/1000 = 0.2, at minimum
        let r2 = ratio_from_cursor(200.0, 1000.0, 0.2, 0.8);
        assert!((r2 - 0.2).abs() < f32::EPSILON);
        // Below minimum gets clamped
        let r3 = ratio_from_cursor(100.0, 1000.0, 0.2, 0.8);
        assert!((r3 - 0.2).abs() < f32::EPSILON);
    }
}

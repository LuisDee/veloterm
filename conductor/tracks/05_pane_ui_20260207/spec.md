<!-- ARCHITECT CONTEXT | Track: 05_pane_ui | Wave: 3 | CC: v1 -->

## Cross-Cutting Constraints
- Performance Budget: divider rendering must not add draw calls — integrate into existing pass
- Platform Abstraction: mouse cursor icons may differ per platform
- Testing: TDD, visual validation via screenshots

## Interfaces

### Owns
- Divider bar hit-testing and rendering
- Drag-to-resize interaction
- Mouse-click-to-focus pane interaction

### Consumes
- `PaneTree` (Track 04) — pane rects and split positions
- `Config` (Track 03) — divider color, width, theme

## Dependencies
- Track 04_pane_layout: PaneTree with layout calculation
- Track 03_config: theme colors for dividers

<!-- END ARCHITECT CONTEXT -->

# Track 05: Pane UI & Interaction — Specification

## Overview

This track adds the visual and interactive layer for split panes in VeloTerm. It delivers divider bar rendering between panes, mouse-driven drag-to-resize, click-to-focus pane switching, and pane zoom visual feedback. The result is an intuitive, mouse-driven pane management experience that complements the existing keyboard shortcuts from Track 04.

## Functional Requirements

### FR-1: Divider Bar Rendering
- Render solid-line divider bars (2px wide) between adjacent panes using GPU quads in the existing wgpu render pass.
- Divider quads are appended to the instance buffer alongside glyph/cell quads — **no additional draw calls**.
- Divider color sourced from `Theme::border` (configurable per theme).
- Dividers only appear when there are 2+ visible panes (not in zoomed mode).
- Divider positions are derived from `PaneTree::calculate_layout()` results — computed from the gap between adjacent pane `Rect`s.

### FR-2: Divider Hover State
- When the mouse cursor hovers within a configurable hit-test zone (8px) around a divider, change the cursor icon:
  - Vertical divider (left/right split): `CursorIcon::EwResize`
  - Horizontal divider (top/bottom split): `CursorIcon::NsResize`
- Optionally highlight the divider (e.g., use `Theme::accent_hover` color) on hover.

### FR-3: Drag-to-Resize
- When the user clicks and drags on a divider, adjust the split ratio of the corresponding `PaneNode::Split`.
- Enforce minimum pane size (20px, matching `PaneTree`'s existing minimum) during drag.
- Continuously recalculate layout and re-render during drag for smooth feedback.
- On mouse release, finalize the new ratio.

### FR-4: Click-to-Focus
- When the user clicks inside a pane area (not on a divider), set that pane as focused via `PaneTree`.
- The click event is consumed for focus — it should not be forwarded to the PTY as terminal input on the initial focus-changing click.

### FR-5: Focus Indicator — Translucent Overlay on Unfocused Panes
- Render a translucent overlay (semi-transparent quad) over all unfocused panes.
- Overlay color: `Theme::background` with configurable alpha (default ~0.3).
- The focused pane remains fully opaque, creating clear visual hierarchy.
- Overlay quads are appended to the instance buffer after pane content, before dividers.

### FR-6: Pane Zoom Visual Transition
- When zoom is toggled (`PaneTree::zoom_toggle()`), the zoomed pane fills the entire window.
- Unfocused panes are hidden (not rendered) — handled by `PaneTree::visible_panes()`.
- Dividers are hidden in zoomed mode (only 1 visible pane).
- Transition is immediate (no animation in this track).

### FR-7: PaneInteraction State Machine
- A dedicated `PaneInteraction` module manages mouse interaction state:
  - **Idle**: Default state, no interaction in progress.
  - **Hovering(DividerInfo)**: Cursor is near a divider, cursor icon changed.
  - **Dragging(DividerInfo, start_ratio)**: User is actively dragging a divider.
- State transitions:
  - `CursorMoved` → check hit-test → transition to Hovering or back to Idle.
  - `MouseInput(Pressed)` while Hovering → transition to Dragging.
  - `CursorMoved` while Dragging → update split ratio.
  - `MouseInput(Released)` while Dragging → finalize ratio, transition to Idle.

## Non-Functional Requirements

### NFR-1: Performance
- Zero additional draw calls — divider and overlay quads are part of the existing render pass.
- Hit-testing is O(n) where n = number of dividers (typically 1-4) — negligible cost.
- Drag updates must feel smooth: layout recalculation on every `CursorMoved` during drag.

### NFR-2: Testability
- `PaneInteraction` state machine is fully unit-testable without GPU.
- Divider rect calculation is pure geometry — unit-testable.
- Hit-testing logic is unit-testable with mock pane layouts.
- Visual rendering validated via Playwright screenshot tool.

### NFR-3: Platform Compatibility
- Cursor icon changes use winit's cross-platform `CursorIcon` enum.
- Mouse coordinates use winit's `PhysicalPosition` (handles HiDPI).

## Acceptance Criteria

1. Divider bars render between panes as 2px solid lines using theme border color.
2. Cursor changes to resize icon when hovering near a divider.
3. Dragging a divider resizes adjacent panes smoothly.
4. Minimum pane size (20px) is enforced during drag.
5. Clicking a pane sets it as focused.
6. Unfocused panes have a translucent overlay dimming effect.
7. Zoomed mode hides dividers and shows only the zoomed pane.
8. All mouse interactions are handled by a testable state machine module.
9. No additional draw calls are introduced.

## Out of Scope
- Pane layout data structures and split/close logic (Track 04).
- Tab bar rendering (Track 06).
- Pane content rendering (existing renderer).
- Animated transitions for zoom.
- Right-click context menus on panes.
- Touch/trackpad gestures beyond standard mouse events.

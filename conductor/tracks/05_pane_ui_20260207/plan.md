# Track 05: Pane UI & Interaction — Plan

## Phase 1: Divider Geometry & Interaction State Machine [checkpoint: 24fc738]

### Task 1.1: Divider Rect Calculation <!-- 04005b9 -->
- [x] Write tests for computing divider `Rect`s from a pane layout
- [x] Implement `calculate_dividers(root: &PaneNode, bounds: Rect, min_size: f32) -> Vec<DividerInfo>` in `src/pane/divider.rs`
- `DividerInfo` contains: rect, direction (H/V), and reference to the parent Split node

### Task 1.2: Hit-Testing <!-- a6a80fe -->
- [x] Write tests for point-in-divider detection with configurable hit zone (8px)
- [x] Implement `hit_test_divider(point: (f32, f32), dividers: &[DividerInfo], margin: f32) -> Option<usize>`

### Task 1.3: PaneInteraction State Machine <!-- 78d5d17 -->
- [x] Write tests for state transitions: Idle ↔ Hovering ↔ Dragging
- [x] Implement `PaneInteraction` struct with `on_cursor_moved()`, `on_mouse_press()`, `on_mouse_release()` in `src/pane/interaction.rs`
- Returns `InteractionEffect` enum (SetCursor, StartDrag, UpdateRatio, FocusPane, None)

### Phase 1 Completion
- [x] Phase completion verification and checkpointing

## Phase 2: Rendering & Mouse Integration

### Task 2.1: Divider Quad Generation
- [ ] Write tests for generating `CellInstance` quads for divider bars
- [ ] Implement `generate_divider_instances()` that produces GPU quads (bg_color = theme.border, no glyph) appended to the instance buffer in `render_panes()`

### Task 2.2: Unfocused Pane Overlay
- [ ] Write tests for generating translucent overlay quads for unfocused panes
- [ ] Implement overlay quad generation (one full-pane quad per unfocused pane, bg with alpha ~0.3), appended after pane content

### Task 2.3: Wire Mouse Events into App
- [ ] Write tests for mouse event routing (CursorMoved, MouseInput → PaneInteraction)
- [ ] Add `CursorMoved` and `MouseInput` handlers in `App::window_event()`, delegate to `PaneInteraction`, apply effects (cursor icon change, request redraw)

### Task 2.4: Click-to-Focus
- [ ] Write tests for click-to-focus logic (click inside pane area → set focused pane)
- [ ] Implement click-to-focus: on mouse press in pane area (not divider), call `pane_tree.set_focus(pane_id)` and consume the click

### Phase 2 Completion
- [ ] Phase completion verification and checkpointing

## Phase 3: Drag-to-Resize & Polish

### Task 3.1: Drag-to-Resize
- [ ] Write tests for ratio updates during drag with minimum pane size enforcement
- [ ] Implement drag-to-resize: on CursorMoved while Dragging, compute new ratio, clamp to min size, update PaneTree split ratio, trigger layout recalculation and redraw

### Task 3.2: Hover Highlight & Zoom Mode
- [ ] Write tests for divider hover color change and divider hiding in zoom mode
- [ ] Implement: use `Theme::accent_hover` for divider color when hovered; skip divider rendering when `pane_tree.zoomed.is_some()`

### Task 3.3: Visual Validation
- [ ] Run application and validate via Playwright screenshots:
  - Divider bars visible between split panes
  - Unfocused pane overlay dimming effect
  - Cursor changes on divider hover
  - Drag-to-resize visual feedback
  - Zoom mode hides dividers

### Phase 3 Completion
- [ ] Phase completion verification and checkpointing

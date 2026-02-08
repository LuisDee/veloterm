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

## Phase 2: Rendering & Mouse Integration [checkpoint: 2fb91bf]

### Task 2.1: Divider Quad Generation <!-- 96e4c18 -->
- [x] Write tests for generating overlay quads for divider bars
- [x] Implement `generate_divider_quads()` in `src/pane/divider.rs` and overlay shader + pipeline in renderer

### Task 2.2: Unfocused Pane Overlay <!-- 96e4c18 -->
- [x] Write tests for generating translucent overlay quads for unfocused panes
- [x] Implement overlay quad generation (one full-pane quad per unfocused pane, bg with alpha ~0.3), appended after pane content

### Task 2.3: Wire Mouse Events into App <!-- 2789a5c -->
- [x] Write tests for mouse event routing (CursorMoved, MouseInput → PaneInteraction)
- [x] Add `CursorMoved` and `MouseInput` handlers in `App::window_event()`, delegate to `PaneInteraction`, apply effects (cursor icon change, request redraw)

### Task 2.4: Click-to-Focus <!-- 2789a5c -->
- [x] Write tests for click-to-focus logic (click inside pane area → set focused pane)
- [x] Implement click-to-focus: on mouse press in pane area (not divider), call `pane_tree.set_focus(pane_id)` and consume the click

### Phase 2 Completion
- [x] Phase completion verification and checkpointing

## Phase 3: Drag-to-Resize & Polish [checkpoint: pending]

### Task 3.1: Drag-to-Resize <!-- cd99e98 -->
- [x] Write tests for ratio updates during drag with minimum pane size enforcement
- [x] Implement drag-to-resize: on CursorMoved while Dragging, compute new ratio, clamp to min size, update PaneTree split ratio, trigger layout recalculation and redraw

### Task 3.2: Hover Highlight & Zoom Mode <!-- cd99e98 -->
- [x] Write tests for divider hover color change and divider hiding in zoom mode
- [x] Implement: use `Theme::accent` for divider color when hovered; skip divider rendering when `pane_tree.is_zoomed()`

### Task 3.3: Visual Validation <!-- cd99e98 -->
- [x] Run application and validate via screenshots:
  - [x] Divider bars visible between split panes
  - [x] Unfocused pane overlay dimming effect
  - [x] Cursor changes on divider hover (verified via 13 interaction unit tests)
  - [x] Drag-to-resize visual feedback (verified via 5 drag unit tests + App integration test)
  - [x] Zoom mode hides dividers

### Phase 3 Completion
- [x] Phase completion verification and checkpointing

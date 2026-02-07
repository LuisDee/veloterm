# Plan: Pane Layout Engine

## Phase 1: PaneTree Data Structure & Layout Calculation

- [x] Task 1: Implement PaneId, SplitDirection, Rect, and PaneNode types <!-- 12fda83 -->
  - [x] Write tests: PaneId uniqueness from generator, SplitDirection variants, Rect construction and contains_point, PaneNode leaf creation, PaneNode split creation
  - [x] Implement types in new module `src/pane/mod.rs`

- [x] Task 2: Implement PaneTree with split and close operations <!-- 12fda83 -->
  - [x] Write tests: new tree has single root pane, vertical split produces two leaves, horizontal split produces two leaves, close pane with sibling promotes sibling to parent, close last pane returns None, nested split then close preserves tree structure, split updates focus to new pane
  - [x] Implement PaneTree struct with `split_focused()`, `close_focused()`, `focused_pane_id()`, and pane iteration

- [x] Task 3: Implement layout calculation (window rect to per-pane rects) <!-- 12fda83 -->
  - [x] Write tests: single pane gets full rect, vertical split divides width by ratio, horizontal split divides height by ratio, nested splits produce correct rects, minimum pane size is enforced by clamping ratio, zero-size window produces valid (clamped) rects
  - [x] Implement `calculate_layout()` that recursively assigns Rect to each leaf node

- [x] Task: Conductor - User Manual Verification 'Phase 1: PaneTree Data Structure & Layout Calculation' (Protocol in workflow.md)

## Phase 2: Focus Navigation & Zoom

- [x] Task 1: Implement directional focus navigation <!-- 12fda83 -->
  - [x] Write tests: focus right from left pane moves to right pane, focus left from right pane moves to left pane, focus down from top pane moves to bottom pane, focus up from bottom pane moves to top pane, focus in direction with no neighbor stays on current pane, focus navigation in 3-pane layout picks spatially nearest pane
  - [x] Implement `focus_direction(direction)` using pane rects to find nearest neighbor in requested direction

- [x] Task 2: Implement pane zoom toggle <!-- 12fda83 -->
  - [x] Write tests: zoom sets zoomed pane id, visible_panes returns only zoomed pane when zoomed, unzoom restores all panes as visible, split while zoomed exits zoom first, close while zoomed exits zoom first, zoom on single pane is a no-op
  - [x] Add `zoom_toggle()`, `is_zoomed()`, and `visible_panes()` methods to PaneTree

- [x] Task: Conductor - User Manual Verification 'Phase 2: Focus Navigation & Zoom' (Protocol in workflow.md)

## Phase 3: Pane-Aware Rendering

- [ ] Task 1: Add per-pane grid dimensions and instance data generation
  - [ ] Write tests: pane grid dimensions computed from pane rect and cell size, per-pane instances have correct positions within pane bounds, two panes produce separate instance vecs with correct counts
  - [ ] Implement helpers to compute GridDimensions from a pane Rect, and generate instances scoped to a pane's grid

- [ ] Task 2: Implement multi-pane render loop with scissor rects
  - [ ] Write tests: scissor rect matches pane pixel rect, single pane scissor covers full surface, two-pane layout produces two scissor regions that tile the window
  - [ ] Modify Renderer to accept a list of pane render descriptors (rect, cells) and render each with appropriate scissor rect and per-pane uniforms

- [ ] Task 3: Add per-pane DamageState tracking
  - [ ] Write tests: each pane has independent damage state, change in pane A does not mark pane B dirty, resize triggers full damage on all panes, new pane starts with full damage
  - [ ] Give each pane its own DamageState, wire into the render loop

- [ ] Task: Conductor - User Manual Verification 'Phase 3: Pane-Aware Rendering' (Protocol in workflow.md)

## Phase 4: Integration — Wire PaneTree into App Event Loop

- [ ] Task 1: Refactor App to hold PaneTree instead of single terminal/pty
  - [ ] Write tests: App creates single-pane tree on startup, each pane has its own Terminal and PtySession, pane count matches tree leaf count
  - [ ] Replace App's `terminal`/`pty` fields with PaneTree, spawn PTY+Terminal per pane leaf

- [ ] Task 2: Wire keyboard input routing and pane commands
  - [ ] Write tests: normal keys route to focused pane PTY, pane-split keybinding triggers split, pane-close keybinding triggers close, focus-direction keybinding switches focus, zoom keybinding toggles zoom
  - [ ] Intercept pane command keys before PTY dispatch, route remaining input to focused pane

- [ ] Task 3: Wire resize and multi-pane render frame
  - [ ] Write tests: window resize recalculates all pane rects, window resize triggers PTY resize on all panes, render loop processes all visible panes
  - [ ] Integrate pane layout recalculation into App::resize, update render_frame to iterate visible panes

- [ ] Task: Conductor - User Manual Verification 'Phase 4: Integration — Wire PaneTree into App Event Loop' (Protocol in workflow.md)

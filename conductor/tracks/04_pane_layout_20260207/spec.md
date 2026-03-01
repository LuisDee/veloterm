# Spec: Pane Layout Engine

## Overview

The pane layout engine enables users to split the terminal window into multiple independent panes, each running its own shell. This is VeloTerm's primary differentiator — native split panes that replace tmux. The layout uses a binary tree data structure where leaf nodes are terminal panes and internal nodes are horizontal or vertical splits with adjustable ratios.

## Design Decisions

- **DD-1: Single Renderer with viewport scissoring** — One shared GPU surface. Each pane is rendered into its own rectangular viewport region using scissor rects. Shares atlas, pipeline, and bind group across all panes.
- **DD-2: Floating-point ratio (0.0–1.0) with minimum pixel constraint** — Splits store a ratio that scales naturally on window resize. A minimum pane size (e.g., 2 columns x 1 row) prevents unusably small panes.
- **DD-3: Directional focus navigation (up/down/left/right)** — Focus moves spatially based on pane pixel positions. Matches tmux mental model.
- **DD-4: New pane starts in $HOME** — Simple and predictable. CWD inheritance deferred to shell integration (Track 10).
- **DD-5: Zoom hides other panes** — Zoomed pane takes full window rect. Other panes stop rendering but retain terminal state.

## Functional Requirements

### FR-1: Binary Tree Pane Data Structure
- `PaneNode` enum with `Leaf { id, terminal, pty }` and `Split { direction, ratio, first, second }` variants
- `PaneTree` struct wrapping the root node, focused pane ID, and zoom state
- `PaneId` as a unique identifier (monotonically increasing u32)
- `SplitDirection` enum: `Horizontal` (top/bottom) and `Vertical` (left/right)

### FR-2: Split Operations
- Split the focused pane vertically (side by side) or horizontally (stacked)
- New pane gets a fresh PTY session starting a shell in $HOME
- The split creates a new `Split` node replacing the current `Leaf`, with the original pane and new pane as children
- Default split ratio: 0.5 (equal)
- Focus moves to the newly created pane after splitting

### FR-3: Close Operations
- Close the focused pane: destroy its PTY session and terminal state
- Collapse the parent `Split` node: replace it with the surviving sibling
- If the last pane is closed, the application exits
- Focus moves to the surviving sibling after close

### FR-4: Layout Calculation
- Given the window's physical pixel dimensions, recursively compute a `Rect { x, y, width, height }` for each leaf pane
- Splits divide the available rect according to direction and ratio
- Enforce minimum pane size: clamp ratio so no pane is smaller than a threshold (e.g., 2 cell widths)
- Layout recalculation runs only on split, close, resize, or zoom toggle — not every frame

### FR-5: Focus Management
- Track the currently focused pane by `PaneId`
- Directional focus switching: given the focused pane's rect, find the nearest pane in the requested direction (up/down/left/right)
- If no pane exists in that direction, focus stays on current pane

### FR-6: Pane Zoom
- Toggle zoom on focused pane: zoomed pane renders at full window rect
- Other panes are skipped during rendering but retain their terminal state
- Unzoom restores the previous layout
- Splitting or closing while zoomed exits zoom first

### FR-7: Resize Handling
- On window resize, recalculate all pane rects from the new window dimensions
- Resize each pane's PTY to match its new cell dimensions (rows x cols)
- Trigger full damage on all panes after resize

### FR-8: Renderer Integration
- Render loop iterates over visible leaf panes
- For each pane: set scissor rect, call `update_cells()` with that pane's grid, draw instances
- Uniforms (cell_size_ndc, grid_size) are per-pane since pane dimensions may differ
- Share atlas texture and sampler across all panes

## Acceptance Criteria

- AC-1: Can split a pane vertically and horizontally, producing two independent shells
- AC-2: Can close a pane, collapsing the tree correctly with surviving sibling promoted
- AC-3: Directional focus navigation moves to the spatially correct pane
- AC-4: Window resize correctly recalculates all pane rects and PTY dimensions
- AC-5: Zoom toggle maximizes focused pane and restores layout on unzoom
- AC-6: All panes render correctly with independent content via scissor rects
- AC-7: Closing the last pane exits the application

## Out of Scope

- Divider bar rendering and mouse interaction (Track 05)
- Drag-to-resize via mouse (Track 05)
- Tab support (Track 06)
- CWD inheritance for new panes (Track 10)
- Session save/restore of pane layout (Track 12)
- Keyboard shortcut configuration (using hardcoded defaults for now)

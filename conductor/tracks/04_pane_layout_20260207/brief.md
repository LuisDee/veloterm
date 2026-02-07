<!-- ARCHITECT CONTEXT | Track: 04_pane_layout | Wave: 2 | CC: v1 -->

## Cross-Cutting Constraints
- Graceful Shutdown: close all pane PTY sessions cleanly on exit
- Error Handling: PaneError enum for split failures, close failures
- Testing: TDD, comprehensive tree manipulation tests
- Platform Abstraction: keybindings use Cmd on macOS, Ctrl on Linux

## Interfaces

### Owns
- `PaneTree` — binary tree of pane nodes (split/close/resize/focus)
- `PaneNode` enum (Leaf with terminal+PTY, Split with direction+ratio+children)
- `PaneId` — unique pane identifier
- Layout calculation: pixel rect per pane from window size

### Consumes
- `Config` (Track 03) — keybindings for split/close/focus, default split ratio
- `Terminal::new()` (existing) — create terminal instance per new pane
- `PtySession::new()` (existing) — spawn shell per new pane
- `Renderer` (existing) — render each pane's grid independently

## Dependencies
- Track 03_config: keybinding definitions and default pane settings

<!-- END ARCHITECT CONTEXT -->

# Track 04: Pane Layout Engine

## What This Track Delivers

The core split pane system that lets users divide the terminal window into multiple independent panes, each running its own shell. This is VeloTerm's primary differentiator — native split panes that replace tmux. The layout engine uses a binary tree data structure where leaf nodes are terminal panes and internal nodes are horizontal or vertical splits with adjustable ratios.

## Scope

### IN
- Binary tree pane data structure (`PaneNode` enum: Leaf | Split)
- Split operations: vertical split, horizontal split (creates new pane with fresh shell)
- Close operations: remove pane, collapse parent split, reparent surviving sibling
- Focus management: track focused pane, switch focus (up/down/left/right)
- Layout calculation: given window pixel dimensions, compute each pane's pixel rect
- Keyboard shortcuts for split, close, and focus navigation
- Pane zoom: temporarily maximize focused pane (toggle)
- Per-pane resize handling: when window resizes, recalculate all pane rects and resize all PTYs

### OUT
- Divider bar rendering and mouse interaction (Track 05 — pane_ui)
- Tab support (Track 06 — tabs)
- Drag-to-resize via mouse (Track 05 — pane_ui)
- Session save/restore of pane layout (Track 12 — session_persistence)

## Key Design Decisions

1. **Pane-to-renderer mapping**: One Renderer per pane (independent GPU surfaces) vs single Renderer with viewport scissoring per pane?
   Trade-off: independent renderers are simpler per-pane but waste GPU resources; scissoring shares resources but needs viewport management

2. **Split ratio model**: Floating-point ratio (0.0-1.0) vs fixed pixel sizes vs constraint-based (minimum pane size)?
   Trade-off: ratios scale naturally on resize; pixels give precise control; constraints prevent unusably small panes

3. **Focus navigation model**: Directional (move focus left/right/up/down based on spatial position) vs tree-based (next/prev sibling, parent)?
   Trade-off: directional feels natural but is complex to compute in nested splits; tree-based is simple but unintuitive

4. **New pane shell**: Inherit CWD from focused pane vs always start in $HOME vs configurable?
   Trade-off: CWD inheritance is the expected tmux behavior; $HOME is simpler; configurable adds complexity

5. **Zoom implementation**: Hide other panes (fast, simple) vs render zoomed pane on top (preserves other pane state visibility)?
   Trade-off: hiding is simpler; overlay preserves context but adds rendering complexity

## Architectural Notes

- Each leaf pane owns a `Terminal` + `PtySession` + its own PTY reader thread — resource management is critical
- The existing `App` struct in `window.rs` holds single optional terminal/pty — this must be refactored to hold the `PaneTree` instead
- Layout calculation is O(tree_depth) and only runs on user-driven events (split, close, resize) — not on every frame
- The renderer's `update_cells()` and `render_frame()` must be called once per visible pane per frame — consider batch rendering
- This is the most architecturally impactful track: it changes how the event loop dispatches input and how rendering works

## Complexity: L
## Estimated Phases: ~4

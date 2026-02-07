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

# Track 05: Pane UI & Interaction

## What This Track Delivers

The visual and interactive layer for split panes — rendering divider bars between panes, supporting drag-to-resize via mouse, click-to-focus pane switching, and pane zoom visual feedback. This makes the split pane system discoverable and intuitive to use with mouse interaction, complementing the keyboard shortcuts from Track 04.

## Scope

### IN
- Divider bar rendering between adjacent panes (2-4px lines)
- Divider hover state (cursor change, highlight color)
- Drag-to-resize: mouse drag on divider adjusts split ratio
- Click-to-focus: clicking inside a pane sets it as focused
- Focus indicator: subtle border or glow on the active pane
- Pane zoom visual transition (focused pane fills window, others hidden)
- Minimum pane size enforcement during drag

### OUT
- Pane layout data structure and split/close logic (Track 04 — pane_layout)
- Tab bar rendering (Track 06 — tabs)
- Pane content rendering (existing renderer handles this)

## Key Design Decisions

1. **Divider rendering approach**: GPU quads in the existing render pass vs egui overlay vs custom UI layer?
   Trade-off: GPU quads are fastest but least flexible; egui adds dependency but handles hit-testing; custom layer is most work

2. **Divider visual style**: Solid line vs gap between panes vs raised/3D appearance?
   Trade-off: solid line is simplest; gap feels more native on macOS; 3D adds visual affordance but complexity

3. **Focus indicator style**: Colored border (2px) vs background tint vs accent-colored divider segments?
   Trade-off: border is most visible; tint is subtle; divider color requires per-segment coloring

4. **Mouse event handling**: Integrate mouse events into existing winit handler vs separate mouse state machine?
   Trade-off: inline is simpler but clutters window.rs; state machine is cleaner but adds indirection

## Architectural Notes

- winit provides `CursorMoved`, `MouseInput`, and cursor icon setting — use these for divider interaction
- The renderer currently has no concept of multiple viewports — this track or Track 04 must add viewport/scissor support
- Divider hit-testing needs the pane layout rects from `PaneTree` — design a clean query interface
- Consider egui integration here if it simplifies UI chrome (dividers, focus indicators) — this decision affects Track 06 (tabs) and Track 09 (search overlay) too

## Complexity: M
## Estimated Phases: ~3

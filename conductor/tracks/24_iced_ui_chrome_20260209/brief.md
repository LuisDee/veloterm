<!-- ARCHITECT CONTEXT | Track: 24_iced_ui_chrome | Wave: 11 | CC: v1 -->

## Cross-Cutting Constraints
- Grid cell rendering (grid.wgsl) is NOT touched -- only overlay chrome
- Must preserve all existing keyboard shortcuts and mouse interactions (tabs, search, pane dividers)
- Tab drag-to-reorder must continue working
- Search TextInput must handle keyboard input (currently routed through window.rs)
- Context menu (NSMenu on macOS) is native and stays native -- not part of this track
- Hot-reload of theme colors must propagate to iced widgets

## Interfaces

### Owns
- iced widget tree for all UI chrome (tab bar, header bar, status bar, search overlay, pane headers, pane dividers, focus dimming)
- Widget message types for tab actions, search input, pane interactions
- Theme-to-iced-style mapping layer

### Consumes
- `iced::Engine` + `iced::Renderer` (Track 23) -- rendering infrastructure
- `TabManager` (Track 06) -- tab state for widget rendering
- `PaneTree` / `PaneLayout` (Track 04) -- layout rects for dividers and headers
- `SearchState` (Track 09) -- search active/query/match state
- `Theme` (Track 03) -- Anthropic Dark color tokens
- `InteractionState` (Track 05) -- divider hover state

## Dependencies
- Track 23_iced_foundation: iced Engine/Renderer/UserInterface wired into render loop

<!-- END ARCHITECT CONTEXT -->

# Track 24: UI Chrome Migration to iced Widgets

## What This Track Delivers

Replaces the entire hand-rolled overlay rendering pipeline with native iced widgets. Today, every piece of UI chrome (tab bar, header bar, status bar, pane dividers, pane headers, search overlay, focus dimming) is built by generating `OverlayQuad` structs that are uploaded as GPU instances and drawn by `overlay.wgsl` -- a custom SDF rounded-rectangle shader. This track migrates all of that to iced `Container`, `Row`, `Column`, `Text`, `TextInput`, and `Button` widgets, then deletes the overlay shader, `OverlayInstance`, `OverlayUniforms`, the overlay render pipeline, and every `generate_*_quads()` / `generate_*_text_cells()` function. The result is proper toolkit-level text rendering (cosmic-text), accessible widget layout, and a maintainable UI layer that can evolve without touching GPU code.

## Scope

### IN
- Replace `generate_header_bar_quads()` + `generate_header_bar_text_cells()` in `src/header_bar.rs` with an iced Row widget (title label, traffic-light spacing)
- Replace `generate_tab_bar_quads()` + `generate_tab_label_text_cells()` in `src/tab/bar.rs` with an iced Row of tab Button widgets (numbered labels, close buttons, new-tab button, active accent stripe)
- Replace `generate_status_bar_quads()` + `generate_status_bar_text_cells()` in `src/status_bar.rs` with an iced Row widget (brand icon, pane indicator, session info)
- Replace `generate_pane_header_quads()` + `generate_pane_header_text()` in `src/pane/header.rs` with iced Container per pane
- Replace `generate_divider_quads()` + `generate_unfocused_overlay_quads()` in `src/pane/divider.rs` with iced-rendered dividers or a thin retained-mode overlay
- Replace `generate_search_bar_quads()` + `generate_search_bar_text_cells()` in `src/search/overlay.rs` with an iced Container + TextInput widget
- Delete `shaders/overlay.wgsl` (96 LOC SDF shader)
- Delete `OverlayInstance`, `OverlayUniforms`, `create_overlay_pipeline()`, `create_overlay_bind_group_layout()` from `src/renderer/gpu.rs`
- Delete `update_overlays()` from `src/renderer/mod.rs`
- Remove `generate_overlay_quads()` orchestration in `src/window.rs` and the overlay pass from `render_panes()`
- Wire iced widget messages back to existing `TabManager`, `SearchState`, and `InteractionState` APIs
- Ensure Anthropic Dark theme tokens apply to all iced widget styles

### OUT
- Grid cell rendering -- `grid.wgsl` and the entire terminal content pipeline are untouched
- Native context menus (NSMenu) -- remain macOS-native
- New UI features (command palette, quick terminal dropdown) -- separate tracks
- iced-based terminal text rendering -- that is Track 25 (glyphon text)
- Cursor rendering -- stays in the grid pipeline
- Background/transparency effects

## Key Design Decisions

1. **Widget message architecture**: Should the iced widget tree produce its own message enum that window.rs matches against, or should widgets directly mutate shared state (TabManager, SearchState) via closures/callbacks?
   - Own message enum: clean separation, testable, but adds a translation layer between iced Messages and existing app state
   - Direct mutation: fewer indirections, but couples iced widgets to internal state types and makes testing harder

2. **Tab drag-to-reorder in iced**: The current system uses raw mouse position tracking in window.rs for tab reordering. Should this be reimplemented as iced drag-and-drop, kept as raw mouse tracking alongside iced widgets, or replaced with a simpler click-to-move interaction?
   - iced drag-and-drop: native feel, but iced's drag-and-drop support may be limited for horizontal tab reordering
   - Raw mouse tracking: proven to work, but means iced tab widgets and mouse handling are partially decoupled
   - Simpler interaction: lowest effort, but loses existing UX

3. **Search TextInput keyboard routing**: Currently window.rs intercepts keyboard events and routes them to SearchState. With an iced TextInput widget, should keyboard focus be fully delegated to iced's focus system, or should window.rs continue intercepting and forwarding?
   - iced focus: cleaner, TextInput handles its own cursor/selection, but Ctrl+F toggle and Escape dismiss need custom handling
   - window.rs interception: preserves existing flow, but duplicates input handling and may fight iced's internal state

4. **Focus dimming approach**: Unfocused panes are currently dimmed via a semi-transparent OverlayQuad drawn over the entire pane rect. With the overlay shader removed, how should this effect be achieved?
   - iced overlay Container with semi-transparent background drawn above the grid viewport
   - Modify grid.wgsl to accept a per-pane dim factor uniform
   - Use wgpu blend state on the grid pass itself
   - Trade-off: iced overlay is simplest but may z-order incorrectly with grid content; grid shader modification is clean but crosses scope boundary; blend state is GPU-efficient but less flexible

5. **Pane divider rendering**: Dividers need hover highlighting and cursor-change on mouse-over. Should dividers become iced widgets with mouse event handling, or remain as positioned rectangles with hit-testing in window.rs?
   - iced widgets: consistent with the migration, hover/cursor handled by iced
   - Positioned rects: simpler for thin 1px lines that need sub-pixel accuracy, but means partial migration

6. **Text overlay removal strategy**: The current renderer has a separate "text overlay" pass that renders GridCells generated by header/tab/status/search modules through the glyph atlas. When iced takes over text rendering, should this pass be removed entirely or kept as a fallback?
   - Remove entirely: clean break, all chrome text goes through iced/cosmic-text
   - Keep as fallback: safety net during migration, but dead code risk
   - Trade-off: full removal is the goal but means all text must work through iced before the overlay pass can be deleted

7. **Migration ordering**: Should all components be migrated simultaneously (big-bang) or one-by-one (incremental) with the overlay pipeline kept alive until the last component is migrated?
   - Big-bang: fewer intermediate states, but high risk of regressions and harder to debug
   - Incremental: each component can be validated independently, overlay pipeline stays until fully replaced, but requires coexistence of both rendering paths

## Architectural Notes

- The overlay pipeline is tightly integrated in `render_panes()` in `src/renderer/mod.rs` -- it runs between the grid pass and the text overlay pass. Removing it requires careful attention to render pass ordering so iced composites correctly.
- `OverlayQuad` is defined in `src/pane/divider.rs` but used by all chrome modules. It will need to be removed after all consumers are migrated.
- The text overlay pass (GridCell-based) renders tab labels, search text, header text, and status text through the glyph atlas. This is a second system to remove alongside the quad system.
- `hit_test_tab_bar()` in `src/tab/bar.rs` does pixel-level hit testing for tab clicks and close buttons -- iced widgets handle this natively, so this function can be deleted.
- `SearchBarParams` carries cell_width/cell_height because the current search bar is sized in grid cells. iced TextInput does not need this coupling.
- Theme hot-reload currently triggers a re-generation of overlay quads on next frame. The iced equivalent is updating the iced theme/style and requesting a redraw.
- Constants like `TAB_BAR_HEIGHT` (28px), `HEADER_BAR_HEIGHT`, `STATUS_BAR_HEIGHT` (36px) should be preserved or defined in iced widget styles to maintain layout compatibility.
- The `window.rs` `generate_overlay_quads()` method (line ~782) orchestrates all quad generation and offset calculations. This entire method and its callers should be replaced by the iced widget tree's `view()` function.
- All existing tests for quad generation (`divider.rs` has ~10 tests, `header_bar.rs` has ~5, `tab/bar.rs` has tests) will be replaced with widget-level tests.

## Test Strategy

- **Framework**: `cargo test` (Rust standard test harness)
- **Unit tests**: Widget message handling (tab select/close/new, search query changes, pane focus changes), theme-to-style conversion, layout calculations
- **Integration tests**: Full widget tree rendering produces correct iced primitives, keyboard events reach TextInput correctly, tab reorder produces correct TabManager state changes
- **Prerequisites**: Track 23 (iced foundation) must be complete so iced Engine/Renderer are available
- **Quality threshold**: 80% line coverage (advisory)
- **Key test scenarios**:
  1. Tab bar renders correct number of tabs with labels and responds to click messages
  2. Search TextInput receives focus on Ctrl+F and produces correct query-change messages
  3. Theme color change propagates to all widget styles within one frame
  4. Pane divider hover state changes cursor and highlight color
  5. Status bar content updates when active pane changes

## Complexity: L
## Estimated Phases: ~5

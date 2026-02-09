# Track 23: iced_wgpu Integration Foundation

**Track ID:** 23_iced_foundation
**Wave:** 10 | **Complexity:** L | **Status:** new
**Dependencies:** 01_window_gpu (complete)

---

## What This Track Delivers

This track introduces iced (via iced_wgpu, iced_winit, and iced_widget) as a retained-mode UI toolkit layered on top of VeloTerm's existing custom wgpu renderer. It establishes the foundational plumbing: creating an iced Engine and Renderer that share the existing wgpu device/queue/adapter, converting winit events through iced's conversion layer, running the iced UserInterface build/update/draw/present lifecycle each frame, and compositing iced's output onto the same TextureView that the custom renderer already writes to. The deliverable is a proven integration where a simple iced widget (e.g., a colored rectangle or text label) renders visibly on top of the terminal content, validating the full pipeline end-to-end.

---

## Scope IN

- Add `iced_wgpu`, `iced_winit`, `iced_widget`, and `iced_runtime` as Cargo dependencies (targeting iced 0.14.x)
- Create an iced `Engine` that reuses the existing `wgpu::Device`, `wgpu::Queue`, and `wgpu::Adapter` from `Renderer` (no second GPU context)
- Create an iced `iced_wgpu::Renderer` wrapping that engine
- Set up iced `Viewport` from the window's physical size and scale factor, updating on resize
- Convert relevant winit `WindowEvent` variants through `iced_winit::conversion` into iced events
- Implement the iced `UserInterface` lifecycle: build a widget tree from a root `Element`, call `update()` with accumulated events, call `draw()`, call `renderer.present()` to record draw commands onto the shared TextureView
- Integrate into the existing render loop in `Renderer::render_panes()` so iced rendering happens AFTER the custom 3-phase pipeline (grid cells, overlay quads, text overlays) but BEFORE the frame is submitted/presented
- Expose the iced integration behind a feature flag or compile-time toggle so it can be disabled without code churn
- Prove the integration with a minimal iced widget visible on screen (a colored container, a text label, or similar)
- Ensure the existing terminal rendering is completely unaffected when iced has no visible widgets
- Handle iced's clipboard and keyboard interaction basics (enough for the proof widget)

## Scope OUT

- **Full UI chrome migration** -- replacing the custom overlay pipeline (tab bar, status bar, search bar, pane headers) with iced widgets belongs in Track 24 (iced UI Chrome)
- **Text rendering via glyphon/iced_text** -- Track 25 handles glyphon integration for text
- **iced theming system** -- mapping VeloTerm's Theme to iced's styling is a Track 24 concern
- **Command palette, settings UI, or modal dialogs** -- future tracks that consume the iced foundation
- **iced subscriptions or async commands** -- not needed for the proof-of-concept layer
- **Input focus arbitration** -- deciding when iced vs terminal gets keyboard input is a Track 24 problem; this track just ensures events can flow to both

---

## Key Design Decisions

1. **Where does iced state live?**
   The existing `Renderer` struct owns all GPU state. Should iced's `Engine` + `iced_wgpu::Renderer` + `Viewport` + `UserInterface` state live inside `Renderer`, in a new sibling struct (e.g., `IcedLayer`), or in `App` (window.rs)?
   - *Inside Renderer*: simplest access to device/queue, but bloats an already large struct (1,310 LOC)
   - *Sibling struct held by Renderer*: clean separation, Renderer passes device/queue refs
   - *In App*: closer to event handling, but needs to reach into Renderer's GPU resources

2. **How to share the wgpu device/queue with iced?**
   Currently `Renderer` owns `device: wgpu::Device` and `queue: wgpu::Queue` directly. iced_wgpu::Engine needs references to these. Should the device/queue be wrapped in `Arc` and shared, should they be extracted into a separate `GpuContext` struct that both Renderer and iced reference, or should iced get raw references scoped to each frame?
   - *Arc wrapping*: most flexible, some refactoring of Renderer fields
   - *Shared GpuContext*: the `gpu.rs` file already has a `GpuContext` struct (currently only used headless) -- could unify
   - *Scoped references*: zero refactoring, but lifetime gymnastics with UserInterface

3. **When in the frame does iced render?**
   The custom pipeline uses a single render pass with 3 phases. iced_wgpu's `present()` creates its own render pass. Should iced render in the same command encoder (as a second render pass), in a separate command encoder submitted after the custom one, or should the custom pipeline's final phase be restructured to leave the render pass open for iced?
   - *Same encoder, second pass*: single submit, clean ordering, iced sees the same TextureView
   - *Separate encoder*: simpler isolation, two submits per frame (slight overhead)
   - *Shared render pass*: would require iced to not create its own pass (not supported by iced_wgpu API)

4. **Feature flag or always-on?**
   Should the iced integration be behind a Cargo feature flag (`features = ["iced-ui"]`), a runtime config toggle, or always compiled in?
   - *Feature flag*: clean opt-in, no binary size impact when off, but increases CI matrix
   - *Runtime toggle*: always compiled, skip iced lifecycle when disabled, simpler builds
   - *Always on*: simplest, but adds ~30s compile time and binary size even if unused

5. **Event routing strategy?**
   winit events currently flow directly to App's handler methods. iced_winit::conversion can translate them to iced events. Should all events go to both systems (iced + terminal), should there be a priority/consumed flag, or should only mouse events in certain screen regions go to iced?
   - *Dual delivery*: simple, but iced might consume keypresses meant for the terminal
   - *Region-based routing*: mouse events over iced widgets go to iced, rest to terminal; requires hit-testing
   - *Consumed flag*: iced processes first, if it returns "captured" the terminal doesn't see it

6. **iced version pinning strategy?**
   iced 0.14.x is the target, but iced's API changes frequently between minor versions. Should the Cargo.toml pin to exact `=0.14.0`, use `0.14`, or use a wider range?
   - *Exact pin*: reproducible, but misses patch fixes
   - *Minor range (0.14)*: gets patches, may break on 0.14.x API changes (iced has done this)
   - *Lock file only*: use `0.14` in Cargo.toml, rely on Cargo.lock for reproducibility

---

## Architectural Notes

- **Surface format compatibility**: VeloTerm uses `Bgra8UnormSrgb`. iced_wgpu needs to be told this format when creating its Engine/Renderer. Verify iced supports this format (it typically auto-detects from the surface, but in integration mode you pass it explicitly).

- **Viewport and scale factor**: iced uses `iced_wgpu::graphics::Viewport::with_physical_size()` which takes a `Size<u32>` and scale factor. VeloTerm already detects scale factor via CoreGraphics on macOS (see `platform::macos::detect_display_scale`). The iced Viewport must use the same scale factor for correct layout.

- **UserInterface lifetime**: iced's `UserInterface::build()` borrows the widget tree and renderer. The UI must be rebuilt each frame (or cached and updated). This is a key integration challenge -- the borrow checker will enforce that iced rendering cannot overlap with custom renderer mutations.

- **Existing overlay pipeline impact**: The custom overlay pipeline (Phase 2 in render_panes) draws dividers, focus overlays, tab bar backgrounds, search bar backgrounds, and status bar backgrounds using `OverlayInstance` quads. This track does NOT replace that pipeline. Track 24 will migrate those overlays to iced widgets. During the transition, both systems coexist.

- **render_panes() return value**: Currently `render_panes()` returns `Ok(SurfaceTexture)` and the caller presents it. iced's `present()` must happen between the custom render and the surface present. This may require restructuring the return flow or adding a hook.

- **wgpu version alignment**: VeloTerm uses wgpu 24. iced 0.14 also uses wgpu internally. Verify that iced_wgpu 0.14 depends on wgpu 24 (not 23 or 22) to avoid duplicate wgpu versions in the dependency tree, which would make device/queue sharing impossible.

- **Compilation time impact**: iced brings in a significant dependency tree (wgpu is already shared, but iced_core, iced_style, iced_widget, etc. are new). Expect 30-60s added to clean builds. Incremental builds should be less affected.

---

## Test Strategy

- **Test framework**: `cargo test` (Rust's built-in test harness)
- **Unit tests**:
  - iced Engine creation with mock/headless wgpu device succeeds
  - Viewport construction from physical size + scale factor produces correct logical dimensions
  - winit-to-iced event conversion produces expected iced event variants (mouse move, resize, keyboard)
  - Feature flag / toggle correctly skips iced lifecycle when disabled
- **Integration tests**:
  - Full render loop with iced integration produces a frame without panicking (headless wgpu)
  - iced widget tree build/update/draw cycle completes without errors
  - Custom renderer output is unmodified when iced widget tree is empty
- **Prerequisites**: Track 01 (window_gpu) must be complete (it is)
- **Quality threshold**: 80% line coverage (advisory)
- **Key test scenarios**:
  1. Create iced Engine sharing existing device/queue -- no second adapter request
  2. Resize event updates both custom renderer surface config AND iced Viewport
  3. Empty iced widget tree adds zero overhead to frame (no extra draw calls)
  4. iced render pass writes to the same TextureView as custom pipeline
  5. wgpu dependency tree has exactly one wgpu version (no duplicates)

---

## Complexity

**L** (Large) -- This track requires modifying core rendering infrastructure, introducing a major new dependency (iced), resolving wgpu device sharing across two rendering systems, and restructuring the render loop to accommodate a second renderer's lifecycle. The iced UserInterface lifetime/borrow constraints add non-trivial Rust complexity.

## Estimated Phases

~4 phases:
1. Dependency setup + wgpu version verification + Engine creation
2. Event conversion layer + Viewport management
3. UserInterface lifecycle integration into render loop
4. Proof-of-concept widget + validation + feature flag cleanup

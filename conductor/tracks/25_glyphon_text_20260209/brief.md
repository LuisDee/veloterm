<!-- ARCHITECT CONTEXT | Track: 25_glyphon_text | Wave: 10 | CC: v1 -->

## Cross-Cutting Constraints
- Atlas texture format may change from R8 to RGBA -- grid.wgsl sampling must match
- Font metrics (cell_width, cell_height) must produce identical grid layout after the swap
- rebuild_atlas() on config change (font size, family, line height) must work with glyphon
- Scale factor detection (src/platform/macos.rs) remains the authority for DPI scaling
- Shader cache: if grid.wgsl changes, `cargo clean -p veloterm` required (include_str!() not tracked)
- All 1143 existing tests must continue to pass without modification

## Interfaces

### Owns
- Glyph rasterization backend (replacing GlyphAtlas + CoreText rasterizer with glyphon)
- Atlas texture creation and GPU upload
- Font metric computation (cell_width, cell_height)

### Consumes
- `Config` (Track 03) -- font_size, font_family, line_height_multiplier, scale_factor
- `GridRenderer` (Track core) -- CellInstance.atlas_uv UV lookup
- `Renderer` (Track core) -- atlas texture bind group, sampler

## Dependencies
- None (independent track, can run in parallel with any wave)

<!-- END ARCHITECT CONTEXT -->

# Track 25: Glyphon Glyph Rasterizer Swap

## What This Track Delivers

Replaces VeloTerm's hand-rolled glyph atlas system (GlyphAtlas struct + CoreText rasterizer on macOS, cosmic-text + swash on other platforms) with glyphon 0.8, which uses cosmic-text internally for HiDPI-aware text shaping, proper font hinting, and subpixel positioning. This is a surgical swap of the atlas/rasterization backend that fixes blurry Retina text and eliminates the platform-specific CoreText FFI code, while keeping the custom grid pipeline (grid.wgsl, CellInstance, damage tracking, scissor rects) entirely unchanged. glyphon 0.8 is an exact dependency match (wgpu 24, cosmic-text 0.12, winit 0.30) requiring no version bumps.

## Scope

### IN
- Add `glyphon = "0.8"` to Cargo.toml
- Replace `GlyphAtlas::new()` internals with glyphon's FontSystem + SwashCache + TextAtlas (or TextRenderer)
- Delete `src/renderer/coretext_raster.rs` (379 LOC of macOS-only CoreText FFI)
- Delete or gut `src/renderer/glyph_atlas.rs` (502 LOC), replacing with glyphon-backed equivalent
- Maintain identical `GlyphInfo { uv: [f32; 4] }` API or equivalent so grid_renderer.rs UV lookups work
- Maintain identical font metrics (cell_width, cell_height) computation
- Handle atlas texture format transition (R8 to whatever glyphon produces)
- Update grid.wgsl fragment shader if atlas texture format changes (e.g., R8 to RGBA sampling)
- Ensure `rebuild_atlas()` works for config hot-reload (font size / family changes)
- Retina / HiDPI rendering correctness on macOS (the primary motivation)
- Cross-platform glyph rendering (macOS, Linux, Windows) via single code path

### OUT
- Changes to grid.wgsl beyond texture format adaptation (no new shader features)
- Changes to CellInstance vertex layout
- Changes to grid_renderer.rs instance generation logic
- Changes to damage tracking, scissor rects, or multi-pane rendering
- Changes to cursor rendering (block/beam/underline/hollow)
- Rich text / multi-font support
- Ligature support (separate future track)
- Any UI chrome or overlay text changes (those are Track 24's domain)

## Key Design Decisions

1. **Integration approach: full TextRenderer vs atlas-only extraction?**
   - **Option A (TextRenderer):** Use glyphon's TextRenderer.prepare() + render() to draw text directly into the render pass. This replaces grid.wgsl entirely. Pro: cleanest integration, full glyphon feature set. Con: loses custom shader features (cursor shapes drawn in-shader, background cell colors, selection highlighting, underline/strikethrough flags, vi-mode cursor).
   - **Option B (Atlas extraction):** Use glyphon's FontSystem + SwashCache + TextAtlas to rasterize glyphs, then extract the atlas texture handle and bind it to the existing grid.wgsl pipeline. Keep CellInstance, UV lookups, all shader features. Pro: minimal changes, keeps every custom feature. Con: depends on glyphon exposing its internal atlas texture (may need to access `TextAtlas::texture()` or equivalent).
   - **Option C (Hybrid):** Use glyphon's SwashCache for rasterization only (get pixel data per glyph), then pack into a custom atlas texture as today. Pro: no dependency on glyphon's atlas internals. Con: reimplements atlas packing that glyphon already does.
   - Trade-off: preserving the grid.wgsl pipeline is critical since it carries cursor, selection, underline, and damage tracking -- losing these would require reimplementing them in a completely different paradigm.

2. **Atlas texture format: R8 vs RGBA?**
   - Current atlas is R8 (single-channel alpha/coverage mask). glyphon's TextAtlas typically uses color textures (RGBA or similar) for subpixel rendering.
   - If glyphon produces RGBA, the grid.wgsl fragment shader needs to sample `.a` (or `.r`) instead of the current single-channel sample. Or the atlas extraction could convert RGBA back to R8.
   - Trade-off: adapting the shader is simpler than format conversion; but RGBA atlas uses 4x VRAM for what is currently a monochrome glyph mask.

3. **Font metric equivalence: how to validate cell dimensions match?**
   - The grid layout depends on exact `cell_width` and `cell_height` values. If glyphon computes slightly different metrics (due to different hinting or rounding), the entire grid shifts.
   - Option A: Assert metric equality in tests (within epsilon).
   - Option B: Override glyphon metrics with the current computation.
   - Option C: Accept metric differences and update tests/expectations.
   - Trade-off: metric drift could cause subtle visual regressions across the entire UI.

4. **GlyphInfo API preservation vs new API?**
   - Currently `grid_renderer.rs` calls `atlas.glyph_info(ch)` to get UV rects. With glyphon, the UV lookup mechanism may be different (glyphon manages its own atlas layout).
   - Should the new code expose the same `HashMap<char, GlyphInfo>` API, or should grid_renderer.rs be updated to use glyphon's native lookup?
   - Trade-off: preserving the API minimizes changes but may fight glyphon's design; updating grid_renderer is more work but cleaner.

5. **Bundled font handling: how does glyphon load JetBrains Mono?**
   - Currently VeloTerm embeds JetBrains Mono via `include_bytes!()` and avoids system fonts (Nerd Font variants cause metric issues).
   - glyphon's FontSystem can load fonts from bytes via cosmic-text. Verify that the bundled font path works and that glyphon does not fall back to system fonts with different metrics.
   - Trade-off: if glyphon picks up system JetBrains Mono Nerd Font, spacing breaks.

6. **CoreText dependency cleanup: remove or keep?**
   - After deleting `coretext_raster.rs`, the `core-text`, `core-graphics`, `core-foundation` crates may become unused (check if other code like macOS platform detection still needs them).
   - Trade-off: removing unused deps shrinks build time; but objc2-app-kit still pulls in Foundation, and platform/macos.rs may use core-graphics for scale factor detection.

## Architectural Notes

- **Dependency fit is exact:** glyphon 0.8 requires wgpu ^24, cosmic-text ^0.12, winit ^0.30 -- VeloTerm already has all three at matching versions. No Cargo.toml version bumps needed beyond adding glyphon itself.
- **The grid pipeline (grid.wgsl + CellInstance + grid_renderer.rs) is the heart of VeloTerm's rendering.** It carries cursor shapes, selection highlighting, underline/strikethrough flags, per-cell foreground/background colors, and damage tracking. Any integration approach that replaces this pipeline requires reimplementing all of these features. Approach B (atlas extraction) is strongly favored.
- **Atlas rebuild on config change:** `rebuild_atlas()` is called when the user changes font size, font family, or line height multiplier via config hot-reload. The glyphon integration must support re-creating the atlas without leaking GPU resources.
- **GLYPH_PADDING constant (2px):** The current atlas adds 2px padding per glyph slot to prevent clipping of descenders and anti-aliased edges. glyphon's atlas packing handles this internally -- verify the padding is sufficient.
- **UI characters:** The current atlas includes 14 extra UI characters beyond ASCII (box-drawing, arrows, etc.). Ensure glyphon rasterizes these correctly or that they are handled separately.
- **No cross-track dependencies:** This track is fully independent. It does not depend on Track 23 (iced foundation) or Track 24 (iced UI chrome). It can run in parallel.
- **Test impact:** Existing tests that create a GlyphAtlas in test harnesses will need their construction calls updated, but assertions about rendering behavior (grid cells, colors, flags) should remain unchanged.

## Test Strategy

- **Framework:** `cargo test` (Rust built-in test framework, consistent with all 1143 existing tests)
- **Unit tests:**
  - Font metric equivalence: assert cell_width and cell_height from glyphon match expected values (within epsilon of current CoreText/cosmic-text values)
  - Glyph UV lookup: verify all ASCII printable characters (0x20..=0x7E) plus UI characters produce valid UV rects
  - Atlas dimensions: verify power-of-two sizing, minimum 512px constraint
  - Atlas rebuild: verify re-creation with different font size/scale produces correct new metrics
- **Integration tests:**
  - Grid rendering with glyphon atlas: verify CellInstance generation produces valid atlas_uv values
  - Full render pass: verify the pipeline renders without GPU errors (wgpu validation)
  - Config hot-reload: change font size, verify atlas rebuild and re-render
- **Regression tests:**
  - All 1143 existing tests must pass without modification (or with minimal constructor updates)
- **Key test scenarios:**
  1. Atlas creation with bundled JetBrains Mono at 13pt, 2x scale -- metrics match expected values
  2. All ASCII printable characters have valid, non-overlapping UV rects in the atlas
  3. Atlas rebuild after font size change (13pt to 16pt) produces correct new dimensions
  4. Grid rendering pipeline produces identical CellInstance output with new atlas backend
  5. Cross-platform: atlas creation works on macOS (previously CoreText) and non-macOS (previously cosmic-text) via single glyphon code path
- **Quality threshold:** 80% line coverage (advisory), 100% pass rate

## Complexity: M
## Estimated Phases: ~3

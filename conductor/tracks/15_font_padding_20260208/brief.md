<!-- ARCHITECT CONTEXT | Track: 15_font_padding | Wave: 6 | CC: v2 -->

## Cross-Cutting Constraints
- Performance Budget: glyph atlas rebuild on font change must complete in <200ms; no frame drops during normal rendering
- Configuration Management: font family, font size, line-height, padding all configurable via `veloterm.toml` with hot-reload
- Testing: TDD for padding calculations, font metrics extraction, atlas rebuild logic
- UI Reference Compliance: all visual output must match the reference mockup aesthetic

## Interfaces

### Owns
- Font loading and fallback chain resolution
- Glyph atlas rebuild on font/size change
- Terminal content padding/margin system
- Runtime font size adjustment (Cmd+Plus/Cmd+Minus)

### Consumes
- `Config` (Track 03) — font.family, font.size, font.line_height, padding settings
- `GlyphAtlas` (Track 01) — existing atlas infrastructure to extend
- `GridRenderer` (Track 01) — cell size and grid positioning

## Dependencies
- Track 01_window_gpu_pipeline: GlyphAtlas and GridRenderer infrastructure
- Track 03_config: font and padding configuration fields

<!-- END ARCHITECT CONTEXT -->

# Track 15: Font Rendering & Terminal Padding

## UI Reference

The visual aesthetic MUST match the reference implementation:
- **Reference Cargo.toml:** `/Users/luisdeburnay/Downloads/Cargo.toml`
- **Reference main.rs:** `/Users/luisdeburnay/Downloads/src/main.rs`

Key reference values: 13px monospace font, 1.5x line-height, 16px content padding, warm dark background `#181715`.

## What This Track Delivers

Professional-grade font rendering with a modern monospace font (JetBrains Mono, SF Mono, or Menlo), properly anti-aliased and correctly sized with configurable line-height and letter-spacing. Adds configurable internal padding/margins so terminal text is not flush against window edges. Adds runtime font size adjustment via Cmd+Plus/Cmd+Minus keyboard shortcuts, with live glyph atlas rebuild.

## Scope

### IN
- Font loading with explicit fallback chain (e.g., JetBrains Mono → SF Mono → Menlo → system monospace)
- Anti-aliased glyph rendering at correct metrics (advance width, ascent, descent)
- Configurable line-height (default 1.5x as per reference)
- Configurable letter-spacing
- Configurable terminal padding on all four sides (minimum 12px default)
- Padding-aware grid positioning (content area inset from window edges)
- Runtime font size adjustment: Cmd+Plus to increase, Cmd+Minus to decrease, Cmd+0 to reset
- Glyph atlas invalidation and rebuild when font or size changes
- Config hot-reload for all font and padding settings

### OUT
- Ligature support (future track)
- Custom font loading from arbitrary file paths (use system fonts)
- Per-pane font settings (all panes share the same font)
- Font rendering for non-Latin scripts beyond basic Unicode (future enhancement)

## Key Design Decisions

1. **Default font selection**: JetBrains Mono (popular, free, excellent readability) vs SF Mono (macOS native, already installed) vs Menlo (macOS fallback, universally available)?
   Trade-off: JetBrains Mono requires bundling or system install; SF Mono is platform-specific; Menlo is safe but dated

2. **Font discovery mechanism**: cosmic-text system font database vs explicit path lookup vs bundled font file?
   Trade-off: system font DB is cross-platform but adds startup time; explicit paths are fragile; bundling increases binary size

3. **Padding implementation**: GPU viewport offset (shift all rendering) vs grid coordinate offset (adjust cell positions) vs container inset (render into a sub-region)?
   Trade-off: viewport offset is simplest but may affect mouse coordinate mapping; grid offset requires changes to all renderers; container inset is cleanest but needs scissor rect

4. **Font size step increment**: 1px per step vs 2px per step vs percentage-based scaling?
   Trade-off: 1px gives fine control but many clicks needed; 2px is practical; percentage maintains proportions across base sizes

## Architectural Notes

- The existing `GlyphAtlas` in `src/renderer/glyph_atlas.rs` uses cosmic-text's `SwashCache` — extending it to load specific named fonts should be straightforward
- Current line-height is hardcoded at 1.6x in the atlas — this needs to become configurable
- Padding affects mouse-to-cell coordinate mapping in selection, click-to-focus, and URL detection — all hit-testing must account for padding offsets
- Font size changes require full atlas rebuild (clear + re-rasterize) which causes a brief flicker — consider double-buffering the atlas
- The reference uses `Font::MONOSPACE` at size 13 with `LineHeight::Relative(1.5)` — match these defaults

## Complexity: M
## Estimated Phases: ~3

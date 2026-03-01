# Track 26: UI Chrome Redesign — Claude Terminal Design Spec

## Scope

Complete visual redesign of VeloTerm's iced_wgpu UI chrome layer to match the Claude Terminal Design Specification. This replaces all existing chrome colors, layout zones, and widget styling with a warm Anthropic-branded design using exact hex colors, pixel-perfect typography, and a 4-zone vertical layout.

## Key Changes

1. **Color Palette**: Replace existing theme tokens with spec-exact warm grays and gold (#D4A574) accent
2. **4-Zone Layout**: Title Bar (38px) + Chrome Bar (40px) + Sidebar/Content + Status Bar (30px)
3. **Title Bar**: macOS traffic lights + centered "✦ Claude Terminal — Anthropic" + version
4. **Chrome Bar**: New zone — hamburger + green dot + active tab name + shell pill
5. **Sidebar**: Preview thumbnails with 3 faux content bars, dashed "+ New Tab" button, gold active states
6. **Status Bar**: ✦ sparkle + "Claude Terminal" | green dot + "Pane N" | username pill · UTF-8 · shell
7. **Pane Chrome**: Remove circled-digit badges, simplify headers

## Files Modified

- `src/config/theme.rs` — New palette tokens matching spec
- `src/renderer/iced_layer.rs` — Complete chrome widget redesign
- `src/window.rs` — Chrome bar height constant, layout adjustments

## Reference

- Design Spec: `/Users/luisdeburnay/Downloads/CLAUDE_TERMINAL_SPEC.md`
- Reference Iced 0.14 impl: `/Users/luisdeburnay/Downloads/main (1).rs`

## Dependencies

- Track 24 (iced_wgpu UI Chrome) — COMPLETE

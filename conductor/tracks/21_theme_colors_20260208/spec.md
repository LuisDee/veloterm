<!-- ARCHITECT CONTEXT | Track: 21_theme_colors | Wave: 8 | CC: v2 -->

## Cross-Cutting Constraints
- UI Reference Compliance: theme MUST match the reference implementation EXACTLY — all colors, spacing, border radii
- Testing: TDD for color conversion, ANSI color mapping, theme application
- Configuration Management: theme selectable via config with hot-reload

## Interfaces

### Owns
- Anthropic dark theme token system (all colors, spacing, radii from reference)
- ANSI 256-color and TrueColor rendering verification
- Colored prompt, ls, git status rendering verification
- macOS window chrome integration (title bar color matching)
- Cursor style and blink rate visual configuration

### Consumes
- `Config` (Track 03) — theme selection, cursor config
- `Theme` (Track 03) — existing theme infrastructure
- `GridBridge` (Track 02) — ANSI color to RGBA conversion
- `GlyphAtlas` (Track 15) — font metrics for consistent rendering

## Dependencies
- Track 03_config: theme configuration and hot-reload
- Track 15_font_padding: font metrics and padding system

<!-- END ARCHITECT CONTEXT -->

# Track 21: Anthropic Dark Theme & Color Rendering — Specification

## Overview

Update VeloTerm's visual appearance to match the Anthropic dark theme reference exactly. This involves updating the `claude_dark` theme's color tokens to the reference values, extending the `Theme` struct with new semantic fields (surface, terminal_bg, text_dim, border_subtle, blue), applying these tokens across all UI elements (tab bar, pane borders, backgrounds, cursor), integrating the macOS title bar chrome with the theme, and verifying that standard ANSI color rendering works correctly for colored terminal output.

## Design Decisions

- **DD1: Theme architecture** — Update `claude_dark` in-place with reference design tokens. Extend the `Theme` struct to include all reference fields. Update `claude_light` and `claude_warm` with sensible values for new fields.
- **DD2: macOS title bar** — Use native title bar with `NSWindow.backgroundColor` set to match theme surface color. No custom title bar drawing.
- **DD3: ANSI color mapping** — Use standard ANSI colors. The existing 16 named ANSI colors, 256-color cube, and TrueColor pass-through remain as standard values. Verify correctness only.

## Reference Design Tokens

These hex values MUST be the `claude_dark` theme defaults:

| Token | Hex | Usage |
|-------|-----|-------|
| background | #141413 | Window and deepest background |
| surface | #1E1D1B | Elevated surface — tab bar, chrome bars |
| surface_raised | #282724 | Hover states, subtle cards |
| terminal_bg | #181715 | Terminal content area background |
| text | #FAF9F5 | Primary text — cream white |
| text_secondary | #B0AEA5 | Supporting text |
| text_dim | #6B6662 | Timestamps, labels |
| border | #33312E | Borders and dividers |
| border_subtle | #262522 | Less emphasis borders |
| accent | #D97757 | Terracotta orange — brand accent |
| success | #788C5D | Sage green |
| blue | #6A9BCC | Info — muted blue |
| error | #C44242 | Danger |

## Functional Requirements

### FR-1: Theme Struct Extension
Extend the `Theme` struct with new fields matching the reference tokens: `surface`, `surface_raised`, `terminal_bg`, `text_secondary`, `text_dim`, `border_subtle`, `blue`. Update all three themes (`claude_dark`, `claude_light`, `claude_warm`) with appropriate values for these fields. Rename existing fields to align with the reference naming where appropriate (e.g. `text_primary` → `text`, `pane_background` → `terminal_bg`).

### FR-2: claude_dark Token Update
Replace all `claude_dark` hex values with the exact reference tokens from the table above. This changes background from #1A1816 → #141413, accent from #E89171 → #D97757, text from #E8E5DF → #FAF9F5, etc.

### FR-3: Tab Bar Styling
Update the tab bar to use theme tokens: `surface` for background, `text` for active tab label, `text_secondary` for inactive tab labels, `accent` for the active tab indicator/stripe, `border` for tab separators.

### FR-4: Terminal Content Area
Use `terminal_bg` (#181715) as the terminal content background instead of `background`. The window chrome area uses `background` (#141413) while the content area uses the slightly lighter `terminal_bg`.

### FR-5: Pane Border Styling
Active pane borders use `accent` color. Inactive pane borders use `border` or `border_subtle`. Border width and styling should be consistent with existing implementation.

### FR-6: macOS Title Bar Integration
On macOS, set `NSWindow.backgroundColor` to match the theme `surface` color so the native title bar blends with the app chrome. This should be applied when the window is created and when the theme changes (hot-reload).

### FR-7: Grid Bridge Default Colors
Update `DEFAULT_FG` and `DEFAULT_BG` constants in `grid_bridge.rs` to match the new theme tokens (`text` #FAF9F5 for fg, `terminal_bg` #181715 for bg).

### FR-8: ANSI Color Verification
Verify the existing standard ANSI color implementation renders correctly. The 16 named colors, 256-color cube, and TrueColor pass-through should use standard values. No remapping needed.

### FR-9: Cursor Rendering
Cursor should use the `accent` color (#D97757) for block cursor fill and `text` color for the character under the cursor.

### FR-10: Update All Theme Consumers
All code that references `Theme` fields must be updated for any renamed/restructured fields. This includes: renderer, tab bar, pane borders, search overlay, selection rendering, scrollbar, context menu.

## Non-Functional Requirements

- All existing tests that assert specific color values must be updated to match new theme tokens
- Theme hot-reload must continue to work after the struct changes
- No visual regression in `claude_light` and `claude_warm` themes (they get sensible new field values)

## Acceptance Criteria

1. `claude_dark` theme renders with exact reference hex values
2. Tab bar uses surface/accent/text tokens from reference
3. Terminal content area uses `terminal_bg` (#181715)
4. macOS title bar blends with app chrome (no visible seam)
5. Standard ANSI colors render correctly for colored terminal output
6. Cursor renders in accent color (#D97757)
7. All existing tests pass (updated for new values)
8. Theme hot-reload still works

## Out of Scope

- Additional theme creation beyond updating existing three
- Theme editor UI
- Per-pane theming
- Background opacity/blur/transparency
- Custom-drawn title bar
- ANSI color remapping to theme tones

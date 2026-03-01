<!-- ARCHITECT CONTEXT | Track: 17_selection_clipboard | Wave: 6 | CC: v2 -->

## Cross-Cutting Constraints
- Testing: TDD for selection coordinate math, word boundary detection, clipboard integration
- Platform Abstraction: Cmd+C/V on macOS, Ctrl+Shift+C/V on Linux
- UI Reference Compliance: selection highlight color must match theme aesthetic

## Interfaces

### Owns
- Double-click word selection
- Triple-click line selection
- Click-and-drag selection with visible highlight
- Clipboard copy (Cmd+C) of selected text
- Clipboard paste (Cmd+V) with bracketed paste mode
- Basic right-click context menu: Copy, Paste, Select All

### Consumes
- `Selection` (Track 02/05) — existing selection infrastructure
- `Clipboard` (Track 02) — existing arboard clipboard integration
- `Config` (Track 03) — theme colors for selection highlight
- `Terminal` (Track 02) — grid content for text extraction

## Dependencies
- Track 02_core_terminal_emulation: Terminal grid and selection types
- Track 05_pane_ui: Selection rendering in panes
- Track 15_font_padding: padding offsets affect mouse-to-cell coordinate mapping

<!-- END ARCHITECT CONTEXT -->

# Track 17: Text Selection & Clipboard

## UI Reference

The visual aesthetic MUST match the reference mockup:
- **Reference Cargo.toml:** `/Users/luisdeburnay/Downloads/Cargo.toml`
- **Reference main.rs:** `/Users/luisdeburnay/Downloads/src/main.rs`

Selection highlight should use a warm, semi-transparent color consistent with the Anthropic dark theme.

## What This Track Delivers

Full production-grade text selection with click-and-drag highlighting, double-click word selection, and triple-click line selection. Wires up the existing selection architecture (which has `SelectionType::Word` and `SelectionType::Line` enums but incomplete mouse event handling) to the window event handler. Adds a basic right-click context menu with Copy, Paste, and Select All options.

## Scope

### IN
- Click-and-drag character-level selection with visible highlight color
- Double-click to select word (detect word boundaries: whitespace, punctuation)
- Triple-click to select entire line
- Click count detection in window event handler (single/double/triple click timing)
- Shift+click to extend selection
- Cmd+A to select all visible content
- Cmd+C to copy selected text to system clipboard
- Cmd+V to paste from system clipboard (with bracketed paste mode)
- Right-click context menu with: Copy, Paste, Select All
- Selection cleared on click or on new typing
- Selection respects terminal padding offsets

### OUT
- Rectangular/block selection (already in vi-mode, Track 11)
- Rich text / HTML copy (plain text only)
- Clipboard history
- Middle-click paste (X11 primary selection)
- Context menu items beyond Copy/Paste/Select All (other items in Track 20)

## Key Design Decisions

1. **Click count detection**: Time-based (clicks within 300ms) vs event-count from winit vs custom state machine?
   Trade-off: time-based is standard; winit may not provide click count on all platforms; state machine is most reliable

2. **Word boundary definition**: Unicode word segmentation (UAX #29) vs simple whitespace/punctuation split vs match iTerm2/Terminal.app behavior?
   Trade-off: Unicode is correct but complex; simple split handles 95% of cases; matching existing terminals is most familiar

3. **Context menu rendering**: egui popup vs custom GPU-rendered overlay vs native OS context menu?
   Trade-off: egui integrates with existing UI; custom matches aesthetic perfectly; native is platform-correct but loses theme

4. **Selection across panes**: Selection confined to active pane (standard) vs selection can span pane boundaries?
   Trade-off: confined is standard and simpler; spanning is rarely useful and complex

## Architectural Notes

- The existing `SelectionType` enum in `src/input/selection.rs` already has `Character`, `Word`, and `Line` variants — the gap is in the window event handler not detecting double/triple clicks
- The `arboard` clipboard integration in `src/input/clipboard.rs` already works for Cmd+C/V — verify it handles large text and multi-line correctly
- Mouse coordinate to terminal cell mapping must account for the new padding (Track 15) — coordinate math: `cell_col = (mouse_x - padding_left) / cell_width`
- Context menu is the first non-terminal overlay UI — this establishes the pattern for Track 20's expanded context menus
- Bracketed paste mode (`\x1b[200~` ... `\x1b[201~`) is already implemented — verify it works with the shell's paste handling

## Complexity: M
## Estimated Phases: ~3

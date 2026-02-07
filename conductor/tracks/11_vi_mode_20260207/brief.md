<!-- ARCHITECT CONTEXT | Track: 11_vi_mode | Wave: 4 | CC: v1 -->

## Cross-Cutting Constraints
- Testing: TDD, motion command tests, selection boundary tests
- Platform Abstraction: keybindings identical across platforms (vi is universal)

## Interfaces

### Owns
- Vi-mode state machine (normal, visual, visual-line, visual-block)
- Motion commands (h/j/k/l, w/b/e, 0/$, gg/G, etc.)
- Selection via visual mode

### Consumes
- `Config` (Track 03) — vi-mode enable/disable, entry keybinding
- Existing selection module (`src/input/selection.rs`)
- Terminal scrollback (existing)

## Dependencies
- Track 03_config: vi-mode toggle and keybinding

<!-- END ARCHITECT CONTEXT -->

# Track 11: Vi-Mode Selection

## What This Track Delivers

A modal vi-mode for keyboard-driven text navigation and selection in the terminal scrollback. When activated (e.g., via Ctrl+Shift+Space), the terminal enters a mode where vi motion keys navigate a cursor through the scrollback content, and visual/visual-line/visual-block modes enable precise text selection without using the mouse.

## Scope

### IN
- Vi-mode entry/exit (toggle keybinding, Escape to exit)
- Cursor movement: h/j/k/l, w/b/e (word motions), 0/$ (line start/end), gg/G (buffer start/end)
- Visual mode: character-wise selection
- Visual-line mode: line-wise selection
- Visual-block mode: rectangular block selection
- Yank (y): copy selection to clipboard
- Search within vi-mode: / and ? for forward/backward search
- Count prefix: e.g., 5j to move 5 lines down
- Vi cursor rendering: distinct block cursor at vi-mode position

### OUT
- Command mode (:commands) — not a text editor
- Text modification (d, c, x) — terminal content is read-only
- Vi-mode within the input line (shell handles this)
- Custom motion definitions

## Key Design Decisions

1. **Mode indicator**: Status bar text ("-- VISUAL --") vs cursor style change vs colored border?
   Trade-off: status bar is standard vi convention; cursor change is subtle; border is visible from distance

2. **Vi-mode scope**: Per-pane vi-mode vs global (all panes enter vi-mode)?
   Trade-off: per-pane is more flexible; global is simpler to implement

3. **Search integration**: Reuse Track 09 search engine vs independent vi-mode search?
   Trade-off: reuse avoids duplication; independent search has different UX (/ prompt vs search overlay)

4. **Visual-block selection**: True rectangular selection vs line-by-line with column constraints?
   Trade-off: true rectangular matches vim behavior; line-by-line is simpler but less useful

## Architectural Notes

- Vi-mode is a modal input state: when active, keyboard input goes to the vi-mode handler instead of the PTY
- The existing selection module (`src/input/selection.rs`) should be extended to support vi-mode selections rather than building a parallel system
- Vi-mode cursor position is independent of the terminal cursor — it's a scrollback navigation cursor
- Consider using the same search backend as Track 09 for / and ? commands

## Complexity: M
## Estimated Phases: ~3

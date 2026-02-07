# Spec: Core Terminal Emulation

## Overview

Transform VeloTerm from a static GPU-rendered test pattern into a fully interactive terminal emulator. This track integrates terminal emulation (via `alacritty_terminal`), PTY management (via `portable-pty`), keyboard input handling, cursor rendering, ANSI color support, scrollback, text selection, and clipboard — producing a single-pane terminal that can run shells and TUI applications.

This track builds directly on top of the completed GPU rendering pipeline (window, wgpu, glyph atlas, grid renderer).

## Functional Requirements

### FR-1: PTY Management
- Spawn the user's default shell by reading the `$SHELL` environment variable.
- If `$SHELL` is unset, fall back to `/bin/sh`.
- Create a PTY pair (master/slave) using `portable-pty`.
- Set initial PTY size to match the grid dimensions (columns × rows) from the renderer.
- A dedicated reader thread performs blocking `read()` on the PTY master fd and sends raw bytes to the main thread via `crossbeam-channel`.
- The main thread writes keyboard input directly to the PTY master fd.

### FR-2: Terminal State Machine
- Integrate `alacritty_terminal` as the VT100/VT220/xterm escape sequence parser and terminal grid state machine.
- One `Term` instance manages the terminal grid, cursor state, and scrollback.
- Feed raw PTY output bytes into the `alacritty_terminal` parser on the main thread.
- The terminal state machine handles: cursor movement, text insertion, line wrapping, scrolling, screen clearing, and all standard ANSI escape sequences.

### FR-3: Grid Bridge (Terminal State → GPU Renderer)
- Bridge `alacritty_terminal`'s grid state to the existing GPU renderer's `CellInstance` buffer.
- For each cell in the terminal grid, extract: character, foreground color, background color, and cell flags (bold, italic, underline, inverse).
- Map `alacritty_terminal` color values (Named, Indexed 256, RGB true color) to the renderer's `[f32; 4]` RGBA format.
- Replace the static test pattern with live terminal content.
- Update the instance buffer whenever the terminal state changes (damage-driven).

### FR-4: Keyboard Input Pipeline
- Capture keyboard events from winit on the main thread.
- Translate winit key events into byte sequences appropriate for the terminal:
  - Printable characters → UTF-8 bytes
  - Special keys (Enter, Backspace, Tab, Arrow keys, Home, End, PageUp, PageDown, Delete, Insert, Escape, Function keys) → appropriate ANSI/xterm escape sequences
  - Modifier combinations (Ctrl+C, Ctrl+D, Ctrl+Z, Ctrl+L, etc.) → correct control codes
- Write translated bytes to the PTY master fd.
- Handle key repeat correctly (winit provides repeat events).

### FR-5: Cursor Rendering
- Render the terminal cursor at the position reported by `alacritty_terminal`.
- Support three cursor styles: block (filled rectangle), beam (thin vertical line), underline (thin horizontal line at cell bottom).
- Implement cursor blink with a configurable rate (default: 500ms on, 500ms off).
- The cursor style and blink state should be controllable via terminal escape sequences (DECSCUSR).
- When the terminal window loses focus, show an unfilled (hollow) block cursor regardless of style.

### FR-6: ANSI Color Mapping
- Support the full ANSI color palette:
  - 16 named colors (8 standard + 8 bright) mapped to the Claude Dark theme palette.
  - 256 indexed colors (standard xterm-256color palette).
  - 24-bit true color (RGB) passed through directly.
- Support SGR attributes that affect rendering: bold (brighter color or bold font), dim/faint, italic, underline, inverse (swap fg/bg), strikethrough.
- Default foreground: theme `text_primary`. Default background: theme `pane_background`.

### FR-7: Scrollback Buffer
- Maintain a fixed 10,000-line scrollback history managed by `alacritty_terminal`.
- Keyboard scroll navigation:
  - Shift+PageUp / Shift+PageDown: scroll one page.
  - Shift+UpArrow / Shift+DownArrow: scroll one line.
- When scrolled up, new output should not auto-scroll to bottom (hold position).
- Any keyboard input (typing) should snap back to the bottom of the scrollback.
- Display a visual indicator when the viewport is scrolled away from the bottom.

### FR-8: Text Selection and Clipboard
- Click-and-drag to select a region of text.
- Double-click to select a word (delimited by whitespace/punctuation).
- Triple-click to select an entire line.
- Selected text is visually highlighted (inverse colors or themed selection color from the Claude Dark theme).
- Cmd+C (macOS) / Ctrl+Shift+C (Linux) copies selected text to the system clipboard via the `arboard` crate.
- Cmd+V (macOS) / Ctrl+Shift+V (Linux) pastes clipboard content into the PTY as if typed.
- Support bracketed paste mode (wrap pasted content in escape sequences when enabled).
- Selection is cleared when the user clicks elsewhere or types.

### FR-9: Window Resize Handling
- On window resize, recalculate grid dimensions (existing renderer logic).
- Send updated size to the PTY via `ioctl(TIOCSWINSZ)` (handled by `portable-pty`).
- `alacritty_terminal` reflows content to match the new dimensions.
- The renderer rebuilds the instance buffer for the new grid size.

## Non-Functional Requirements

### NFR-1: Performance
- Input latency (keystroke to screen): target <16ms (one frame at 60fps).
- PTY reader thread should use a 64KB read buffer for throughput.
- Only update GPU buffers for cells that changed (damage tracking).

### NFR-2: Correctness
- Must correctly render common TUI applications: vim, htop, less, man pages, git log.
- Must handle UTF-8 multi-byte characters and wide characters (CJK) correctly.
- Must pass basic `vttest` terminal emulation tests.

### NFR-3: Robustness
- Gracefully handle shell exit (close terminal or show "[Process exited]" message).
- Handle PTY read errors without crashing.
- Handle rapid resize events (debounce or coalesce).

## Acceptance Criteria

1. Launch VeloTerm → user's default shell appears with a working prompt.
2. Type commands (ls, echo, pwd, etc.) → output renders correctly.
3. Run `vim` → full TUI renders, navigation and editing work.
4. Run `htop` → real-time updating TUI renders correctly.
5. Colors display correctly (run a 256-color test script).
6. Cursor blinks and changes style via escape sequences.
7. Scroll up through command history with Shift+PageUp.
8. Select text with mouse, copy with Cmd+C, paste with Cmd+V.
9. Resize window → terminal reflows correctly, no rendering artifacts.
10. Exit shell (type `exit`) → terminal handles shutdown cleanly.

## Out of Scope

- Split panes and tabs (Phase 2 of product roadmap)
- Mouse reporting to applications (e.g., mouse-aware vim)
- Clickable URLs and file paths
- TOML configuration file and hot-reload
- Shell integration / semantic prompts
- Kitty graphics or keyboard protocol
- Ligature support
- Search in scrollback
- Custom keybindings

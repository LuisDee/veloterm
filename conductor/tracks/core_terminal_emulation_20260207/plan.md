# Plan: Core Terminal Emulation

## Phase 1: PTY Spawning and Raw I/O [checkpoint: 66f48c4]

- [x] Task: Write tests for PTY creation and shell spawning <!-- 23cf6cc -->
  - Write tests for PTY pair creation via `portable-pty`
  - Write tests for reading `$SHELL` with `/bin/sh` fallback
  - Write tests for PTY size initialization from grid dimensions
  - Write tests for basic read/write on the PTY master fd
  - Run tests and confirm they fail (Red phase)

- [x] Task: Implement PTY management module <!-- 23cf6cc -->
  - Create `src/pty/mod.rs` with PTY lifecycle management
  - Add `portable-pty` and `crossbeam-channel` dependencies to `Cargo.toml`
  - Spawn the user's default shell (`$SHELL` or `/bin/sh`)
  - Set initial PTY size (columns × rows)
  - Implement blocking read on a dedicated reader thread sending bytes via `crossbeam-channel`
  - Implement write-to-PTY from the main thread
  - Run tests and confirm they pass (Green phase)

- [x] Task: Conductor - User Manual Verification 'PTY Spawning and Raw I/O' (Protocol in workflow.md) <!-- 66f48c4 -->

## Phase 2: Terminal State Machine Integration [checkpoint: a6dd41a]

- [x] Task: Write tests for alacritty_terminal integration <!-- d88ecce -->
  - Write tests for `Term` creation with correct grid dimensions
  - Write tests for feeding raw bytes into the parser and observing grid state changes
  - Write tests for basic text insertion (feed "Hello" → grid row 0 contains "Hello")
  - Write tests for cursor position tracking after text insertion
  - Run tests and confirm they fail (Red phase)

- [x] Task: Implement terminal state machine wrapper <!-- d88ecce -->
  - Add `alacritty_terminal` dependency to `Cargo.toml`
  - Create `src/terminal/mod.rs` as a thin wrapper around `alacritty_terminal::Term`
  - Initialize `Term` with grid dimensions and scrollback size (10,000 lines)
  - Implement a method to feed raw PTY bytes into the terminal parser
  - Expose grid state, cursor position, and dirty flags
  - Run tests and confirm they pass (Green phase)

- [x] Task: Conductor - User Manual Verification 'Terminal State Machine Integration' (Protocol in workflow.md) <!-- a6dd41a -->

## Phase 3: Grid Bridge — Terminal State to GPU Renderer [checkpoint: 79a38ce]

- [x] Task: Write tests for terminal-to-renderer grid bridge <!-- d2d7533 -->
  - Write tests for extracting character, fg color, and bg color from each `alacritty_terminal` cell
  - Write tests for mapping named ANSI colors (16) to theme RGBA values
  - Write tests for mapping 256-color indexed palette to RGBA
  - Write tests for passing through 24-bit true color (RGB) values
  - Write tests for default fg/bg color assignment from theme
  - Run tests and confirm they fail (Red phase)

- [x] Task: Implement grid bridge and color mapping <!-- d2d7533 -->
  - Create `src/terminal/grid_bridge.rs` to convert terminal grid state to renderer `CellInstance` data
  - Implement color conversion: Named → theme colors, Indexed → xterm-256 palette, RGB → passthrough
  - Map cell flags (bold, dim, inverse) to rendering attributes
  - Replace `generate_test_pattern()` call with live terminal grid extraction
  - Update the renderer to accept dynamic cell data from the terminal
  - Run tests and confirm they pass (Green phase)

- [x] Task: Conductor - User Manual Verification 'Grid Bridge — Terminal State to GPU Renderer' (Protocol in workflow.md) <!-- 79a38ce -->

## Phase 4: Keyboard Input Pipeline [checkpoint: 7b9e49b]

- [x] Task: Write tests for keyboard input translation <!-- 4b91c0b -->
  - Write tests for printable character encoding (ASCII and UTF-8)
  - Write tests for special key translation (Enter → `\r`, Backspace → `\x7f`, Tab → `\t`, Escape → `\x1b`)
  - Write tests for arrow keys → ANSI escape sequences (`\x1b[A`, `\x1b[B`, etc.)
  - Write tests for control key combinations (Ctrl+C → `\x03`, Ctrl+D → `\x04`, Ctrl+Z → `\x1a`)
  - Write tests for function keys and Home/End/Delete/Insert/PageUp/PageDown
  - Run tests and confirm they fail (Red phase)

- [x] Task: Implement keyboard input handling <!-- 4b91c0b -->
  - Create `src/input/mod.rs` for keyboard event translation
  - Translate winit `KeyEvent` to terminal byte sequences
  - Handle modifier keys (Ctrl, Shift, Alt/Option) correctly
  - Write translated bytes to PTY master fd from the main thread
  - Integrate with the event loop in `main.rs`
  - Run tests and confirm they pass (Green phase)

- [x] Task: Conductor - User Manual Verification 'Keyboard Input Pipeline' (Protocol in workflow.md) <!-- 7b9e49b -->

## Phase 5: Cursor Rendering

- [ ] Task: Write tests for cursor rendering
  - Write tests for cursor position extraction from terminal state
  - Write tests for block cursor cell instance generation (filled rectangle)
  - Write tests for beam cursor cell instance generation (thin vertical line)
  - Write tests for underline cursor cell instance generation (thin horizontal line)
  - Write tests for hollow block cursor when window is unfocused
  - Write tests for cursor blink timing (500ms on/off cycle)
  - Run tests and confirm they fail (Red phase)

- [ ] Task: Implement cursor rendering
  - Create `src/renderer/cursor.rs` for cursor state and rendering
  - Generate cursor overlay cell instance(s) based on cursor style and position
  - Implement blink timer toggling cursor visibility at 500ms intervals
  - Support DECSCUSR escape sequence to change cursor style
  - Render hollow block when window focus is lost
  - Integrate cursor instances into the render pipeline
  - Run tests and confirm they pass (Green phase)

- [ ] Task: Conductor - User Manual Verification 'Cursor Rendering' (Protocol in workflow.md)

## Phase 6: ANSI Color and SGR Attribute Support

- [ ] Task: Write tests for full color palette and SGR attributes
  - Write tests for 16 named ANSI colors mapped to Claude Dark theme values
  - Write tests for 256-color xterm palette generation
  - Write tests for bold attribute brightening colors
  - Write tests for dim/faint attribute reducing color intensity
  - Write tests for inverse attribute swapping fg/bg
  - Write tests for underline and strikethrough flag propagation
  - Run tests and confirm they fail (Red phase)

- [ ] Task: Implement color palette and SGR attribute rendering
  - Define the full 16-color Claude Dark theme ANSI palette
  - Generate the standard xterm-256 color lookup table
  - Apply bold → bright color mapping (or bold font weight)
  - Apply dim → reduced alpha/intensity
  - Apply inverse → swap fg and bg colors
  - Pass underline and strikethrough flags through to the shader/renderer
  - Run tests and confirm they pass (Green phase)

- [ ] Task: Conductor - User Manual Verification 'ANSI Color and SGR Attribute Support' (Protocol in workflow.md)

## Phase 7: Scrollback Buffer and Scroll Navigation

- [ ] Task: Write tests for scrollback and scroll navigation
  - Write tests for scrollback history accumulation (lines scrolled off-screen are preserved)
  - Write tests for Shift+PageUp/PageDown scrolling one page
  - Write tests for Shift+UpArrow/DownArrow scrolling one line
  - Write tests for viewport hold (new output does not auto-scroll when scrolled up)
  - Write tests for snap-to-bottom on keyboard input
  - Run tests and confirm they fail (Red phase)

- [ ] Task: Implement scrollback buffer and scroll navigation
  - Configure `alacritty_terminal` with 10,000-line scrollback
  - Implement scroll viewport tracking (display offset from bottom)
  - Map Shift+PageUp/PageDown to page scroll in the viewport
  - Map Shift+UpArrow/DownArrow to single-line scroll
  - Hold viewport position when new output arrives during scroll-back
  - Snap to bottom when the user types any non-scroll key
  - Add a visual scroll indicator when viewport is not at bottom
  - Update the grid bridge to render from the correct viewport offset
  - Run tests and confirm they pass (Green phase)

- [ ] Task: Conductor - User Manual Verification 'Scrollback Buffer and Scroll Navigation' (Protocol in workflow.md)

## Phase 8: Text Selection

- [ ] Task: Write tests for text selection
  - Write tests for click-and-drag rectangular selection (start cell → end cell)
  - Write tests for double-click word selection (word boundaries at whitespace/punctuation)
  - Write tests for triple-click line selection
  - Write tests for selection highlight rendering (inverse or themed selection color)
  - Write tests for selection cleared on click-elsewhere or keypress
  - Run tests and confirm they fail (Red phase)

- [ ] Task: Implement text selection
  - Create `src/input/selection.rs` for selection state management
  - Track mouse press/drag/release events from winit to define selection region
  - Implement word boundary detection for double-click
  - Implement line selection for triple-click
  - Render selected cells with selection highlight color from Claude Dark theme
  - Clear selection on single click elsewhere or on any keyboard input
  - Extract selected text content from the terminal grid as a UTF-8 string
  - Run tests and confirm they pass (Green phase)

- [ ] Task: Conductor - User Manual Verification 'Text Selection' (Protocol in workflow.md)

## Phase 9: Clipboard Integration

- [ ] Task: Write tests for clipboard operations
  - Write tests for copy-selected-text-to-clipboard (Cmd+C / Ctrl+Shift+C)
  - Write tests for paste-from-clipboard-to-PTY (Cmd+V / Ctrl+Shift+V)
  - Write tests for platform-appropriate keybinding detection (macOS vs Linux)
  - Write tests for paste with bracketed paste mode escape sequences
  - Run tests and confirm they fail (Red phase)

- [ ] Task: Implement clipboard integration
  - Add `arboard` dependency to `Cargo.toml`
  - Implement copy: extract selected text → write to system clipboard via `arboard`
  - Implement paste: read from system clipboard → write to PTY as keystrokes
  - Support bracketed paste mode (wrap pasted content in `\x1b[200~` ... `\x1b[201~` when enabled)
  - Use Cmd+C/V on macOS, Ctrl+Shift+C/V on Linux
  - Run tests and confirm they pass (Green phase)

- [ ] Task: Conductor - User Manual Verification 'Clipboard Integration' (Protocol in workflow.md)

## Phase 10: Window Resize, Shell Exit, and Integration Polish

- [ ] Task: Write tests for resize and shell exit handling
  - Write tests for window resize → PTY size update via `TIOCSWINSZ`
  - Write tests for terminal content reflow after resize
  - Write tests for renderer grid recalculation on resize
  - Write tests for graceful shell exit detection (PTY read returns EOF)
  - Write tests for rapid resize debouncing/coalescing
  - Run tests and confirm they fail (Red phase)

- [ ] Task: Implement resize handling, shell exit, and integration polish
  - On window resize: update PTY size, let `alacritty_terminal` reflow, rebuild renderer grid
  - Detect shell exit (PTY EOF) and display "[Process exited]" or close the window
  - Debounce rapid resize events to avoid excessive reflow
  - Verify end-to-end: launch → type commands → run vim/htop → scroll → select/copy → resize → exit
  - Ensure UTF-8 multi-byte and wide character (CJK) rendering correctness
  - Run tests and confirm they pass (Green phase)

- [ ] Task: Conductor - User Manual Verification 'Window Resize, Shell Exit, and Integration Polish' (Protocol in workflow.md)

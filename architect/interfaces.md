# Interface Contracts

> Track-to-track API and event contracts. Each interface has exactly one owner.
> VeloTerm is a single-process desktop app — interfaces are Rust module boundaries, not HTTP APIs.
>
> Updated by: `/architect-decompose` (initial), `/architect-sync` (discovery-driven changes)

---

## Module Interfaces

### Existing (Completed Tracks)

**Window & Event Loop** (`src/window.rs`) — Owned by: Track 1 (complete)
- `App` struct with `ApplicationHandler` implementation
- `WindowConfig` for window dimensions, title, resizable flag
- Event dispatch: keyboard → input module, resize → renderer + PTY + terminal

**GPU Renderer** (`src/renderer/`) — Owned by: Track 1 (complete)
- `Renderer::new(window)` — initialize GPU pipeline
- `Renderer::resize(width, height)` — reconfigure surface
- `Renderer::update_cells(cells)` — update instance buffer from terminal state
- `Renderer::render_frame()` — submit GPU draw calls
- `GridDimensions` — columns, rows, cell size, window size

**Terminal State** (`src/terminal/`) — Owned by: Track 2 (complete)
- `Terminal::new(cols, rows, scrollback)` — create terminal instance
- `Terminal::feed(bytes)` — process PTY output through ANSI parser
- `Terminal::resize(cols, rows)` — resize terminal grid
- `Terminal::cursor_position()` — get cursor (row, col)
- `grid_bridge::extract_grid_cells(terminal)` — convert to `GridCell` array

**PTY Session** (`src/pty/`) — Owned by: Track 2 (complete)
- `PtySession::new(shell, cols, rows)` — spawn shell with PTY
- `PtySession::write(bytes)` — send input to shell
- `PtySession::resize(cols, rows)` — resize PTY
- `reader_rx` — crossbeam channel receiving PTY output bytes

**Input Translation** (`src/input/`) — Owned by: Track 2 (complete)
- `translate_key(logical_key, text, state, modifiers)` — winit key → terminal bytes
- `clipboard` submodule — clipboard read/write operations
- `selection` submodule — text selection state

### New (Upcoming Tracks)

**Configuration** (`src/config/`) — Owned by: Track 03_config
- `Config::load(path)` — parse TOML file
- `Config::watch(path, callback)` — hot-reload with notify
- `Config::diff(old, new)` — determine what changed
- Consumed by: ALL subsequent tracks

**Pane Layout** (`src/layout/`) — Owned by: Track 04_pane_layout
- `PaneTree` — binary tree of pane nodes
- `PaneTree::split(pane_id, direction)` — create split
- `PaneTree::close(pane_id)` — remove pane, collapse tree
- `PaneTree::resize(pane_id, delta)` — adjust split ratio
- `PaneTree::focused()` — get focused pane ID
- Consumed by: Track 05_pane_ui, Track 06_tabs, Track 12_session

**Tab Manager** (`src/tabs/`) — Owned by: Track 06_tabs
- `TabManager::new_tab()` — create tab with fresh pane tree
- `TabManager::close_tab(id)` — close tab and all panes
- `TabManager::switch_tab(id)` — change active tab
- Consumed by: Track 12_session, Track 13_command_palette

---

## Shared Data Schemas

### GridCell (existing)
```rust
GridCell {
    character: char,
    fg: [f32; 4],      // RGBA foreground
    bg: [f32; 4],      // RGBA background
    flags: u32,         // underline, strikethrough, selected, cursor
}
```
Owned by: Track 2 (complete). Used by: renderer, grid_bridge.

### PaneNode (new)
```rust
enum PaneNode {
    Leaf { id: PaneId, terminal: Terminal, pty: PtySession },
    Split { direction: SplitDirection, ratio: f32, first: Box<PaneNode>, second: Box<PaneNode> },
}
```
Owned by: Track 04_pane_layout. Used by: Track 05_pane_ui, Track 06_tabs.

---

## Contract Change Protocol

When a module interface needs to change:
1. Owner track proposes change in interfaces.md
2. All consuming tracks are checked:
   - NOT_STARTED: auto-inherit
   - IN_PROGRESS: flag for developer review
   - COMPLETE: patch phase if backward-incompatible
3. Rust's type system enforces most contract changes at compile time

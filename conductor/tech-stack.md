# Technology Stack — VeloTerm

## Language

**Rust** (latest stable)

Rust provides zero-cost abstractions, memory safety without garbage collection, and fearless concurrency — critical for a multi-threaded terminal emulator with shared mutable state across IO, parsing, and rendering threads. The performance is equivalent to C (within 0-5% in compute-bound tasks) while eliminating entire classes of bugs (data races, use-after-free, buffer overflows) at compile time. The ecosystem (crates.io, cargo, clippy, rust-analyzer) accelerates development.

## Terminal Emulation

**`alacritty_terminal`** (pinned version, vendored for stability)

Battle-tested VT100/VT220/xterm escape sequence parser and terminal grid state machine extracted from Alacritty. Handles CSI, OSC, DCS parsing, ANSI color (16/256/24-bit true color), text selection, URL detection, and scrollback buffer. Purely a state machine — no IO, no rendering. ~15K lines of focused terminal emulation code.

- One `Term` instance per pane (independent state)
- Thin adapter/wrapper layer to isolate VeloTerm from Alacritty-specific API internals
- Does not include Kitty keyboard/graphics protocols — these will be added separately in later phases

## PTY Management

**`portable-pty`**

Cross-platform PTY layer extracted from WezTerm. On Unix, calls the same system calls as raw implementations (`openpty()`, `fork()`, `exec()`, `read()`, `write()`, `ioctl(TIOCSWINSZ)`) with zero measurable overhead — the trait boundary and struct wrapper are monomorphized away at compile time. Handles edge cases: session leaders, controlling terminals, CLOEXEC flags, signal handling during fork.

## Windowing

**`winit`**

Cross-platform window creation, event loop, keyboard/mouse input, and DPI scaling. Standard in the Rust ecosystem. Supports macOS (Cocoa) and Linux (X11).

## GPU Rendering

**`wgpu`** with custom terminal renderer

GPU abstraction layer supporting Metal (macOS) and Vulkan/OpenGL (Linux). Architecture:

```
winit (window + event loop + platform integration)
wgpu (GPU abstraction: Metal/Vulkan/OpenGL)
Custom Terminal Renderer
├── GlyphAtlas (rasterize + cache glyphs in VRAM)
├── GridRenderer (instanced quad draw per cell)
├── CursorRenderer
└── SelectionHighlight
Simple UI Overlay (for split borders, tabs, etc.)
└── egui OR custom minimal widgets
```

- **Glyph atlas pattern:** Pre-rasterize ASCII range + common symbols into GPU texture atlas. Lazy-load uncommon glyphs. Rendering = 1-2 draw calls for the entire grid.
- **Damage tracking:** Track changed cells via bitfield (`bitvec` crate). Only update vertex buffer for dirty cells. For interactive typing, this means updating 1-5 cells per frame instead of 10,000.
- **Two draw calls per frame:** Background pass (colored quads) + foreground pass (textured quads from atlas).

## Text Rendering

**`cosmic-text`** + **CoreText** (macOS native rasterizer)

For glyph shaping and rasterization. Glyphs are baked into the GPU texture atlas. On macOS, glyphs are rasterized via CoreText for native-quality rendering; on other platforms, cosmic-text's SwashCache handles rasterization.

- **Bundled font:** JetBrains Mono (~300KB, compiled into binary via `include_bytes!()`)
- **Font fallback chains:** Terminal content (JetBrains Mono → SF Mono → Menlo), UI chrome (Inter → SF Pro), Display (Georgia)
- **Runtime font size adjustment:** Cmd+Plus/Minus/0 with atlas rebuild

## Threading Model

**Dedicated threads with `crossbeam-channel`** (no async runtime)

```
Thread 1: Event Loop (main thread)
├── Receives: keyboard/mouse events from winit
├── Sends: keystrokes to PTY write channel
├── Receives: parsed grid updates from Thread 2
├── Triggers: re-render when grid changes
└── Renders: GPU frame submission

Thread 2: PTY Reader + Parser (per pane)
├── Blocking read() on PTY master fd (64KB buffer)
├── Feed bytes into alacritty_terminal parser
├── Produce grid state diffs
└── Send diffs to Thread 1 via channel

Thread 3 (optional): PTY Writer
├── Receive keystrokes from Thread 1 via channel
└── Blocking write() to PTY master fd
```

No Tokio or async runtime. A terminal emulator has exactly 3 IO sources — keyboard input, PTY output, and config file watching. This is not a high-concurrency problem. Threads with `crossbeam-channel` (~20ns per send/receive) are simpler, lower latency, and easier to debug.

## Configuration

**TOML** via `toml` + `serde` crates

Configuration file at `~/.config/veloterm/veloterm.toml`. TOML is unambiguous (no implicit type coercion footguns like YAML's `no` → `false` or `3.10` → `3.1`), has a simpler spec, and the Rust `toml` crate is faster and more actively maintained than `serde_yaml`.

Hot-reload via the `notify` crate — changes to the config file are detected and applied without restarting the terminal.

## Split Pane Architecture

**Binary tree layout**

```rust
enum PaneNode {
    Leaf { id: PaneId, terminal: TerminalInstance },
    Split { direction: SplitDirection, ratio: f32, first: Box<PaneNode>, second: Box<PaneNode> },
}
```

Recursive layout calculation, O(depth) resize operations, clean pane close (replace parent Split with surviving sibling). Layout tree has zero impact on rendering — calculation only happens on user-driven split/resize events.

## Target Platforms

### macOS — Apple Silicon only
- **Architecture:** aarch64-apple-darwin
- **GPU backend:** Metal (via wgpu, automatic)
- **Display:** Retina/HiDPI support via winit
- **Distribution:** Native `.app` bundle via `cargo-bundle`

### Linux — X11 on CentOS 9
- **Architecture:** x86_64-unknown-linux-gnu
- **GPU backend:** Vulkan (primary), OpenGL ES 3.0 (automatic fallback via wgpu)
- **Display server:** X11 (primary target on CentOS 9)
- **Distribution:** RPM and/or AppImage
- **CentOS 9 specifics:**
  - Mesa 22.x ships with CentOS Stream 9 — decent Vulkan support for AMD/Intel GPUs
  - NVIDIA Vulkan requires the proprietary driver; wgpu falls back to OpenGL ES 3.0 automatically
  - CI must build against CentOS 9's glibc version (2.34) for binary portability
  - Build dependencies: `libxkbcommon-devel`, X11 development packages
  - Wayland support not targeted initially but `winit` supports it if needed later

## Core Dependencies

```toml
[dependencies]
# Window + Event Loop
winit = "0.30"

# GPU Rendering
wgpu = "24"

# Terminal Emulation
alacritty_terminal = "0.24"

# PTY
portable-pty = "0.9"

# Text Rendering
cosmic-text = "0.12"

# Threading
crossbeam-channel = "0.5"

# Configuration
toml = "0.8"
serde = { version = "1", features = ["derive"] }
notify = "7"

# Utilities
log = "0.4"
env_logger = "0.11"
bitvec = "1"
unicode-width = "0.2"
dirs = "5"
arboard = "3"
```

### Phase 2+ Dependencies

```toml
# UI Overlay (for split borders, search bar, etc.)
egui = "0.30"
egui-wgpu = "0.30"
egui-winit = "0.30"

# URL Detection
linkify = "0.10"

# Regex search in scrollback
regex = "1"
```

## Performance Targets

| Metric | Target | How |
|--------|--------|-----|
| Input latency (key-to-screen) | <10ms | Dedicated IO thread, damage-tracked rendering, no async overhead |
| Memory per terminal instance | <10MB | Compact grid (~2-4MB for 200x50 + 10K scrollback), shared glyph atlas (~1-2MB VRAM) |
| Startup time | <100ms | Minimal runtime, pre-rasterized ASCII atlas (~10-20ms), winit window (~30ms), wgpu init (~20-50ms) |

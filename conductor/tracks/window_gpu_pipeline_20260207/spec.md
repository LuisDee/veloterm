# Spec: Cross-Platform Window and GPU Rendering Pipeline

## Overview

This track establishes the foundational rendering infrastructure for VeloTerm. By the end of this track, the application will open a GPU-accelerated window on both target platforms (macOS Apple Silicon via Metal, Linux X11/CentOS 9 via Vulkan/OpenGL), render a monospace glyph atlas to a GPU texture, and display a static grid of characters on screen using instanced quad rendering.

This is the highest-risk technical validation in the project — proving that the wgpu + winit rendering pipeline works correctly cross-platform before any terminal emulation logic is added.

## Goals

1. **Initialize the Rust project** — Set up the Cargo workspace, configure dependencies, establish the project directory structure as defined in the tech stack.
2. **Create a cross-platform window** — Use winit to open a resizable window with proper DPI/HiDPI handling on both macOS and Linux X11.
3. **Initialize the GPU pipeline** — Set up wgpu with Metal backend on macOS and Vulkan/OpenGL fallback on Linux. Configure the render surface, device, and queue.
4. **Build a glyph atlas** — Rasterize the ASCII printable range (0x20–0x7E) from a monospace font (JetBrains Mono primary, with fallbacks) into a GPU texture atlas using `cosmic-text` or `ab_glyph`.
5. **Render a static character grid** — Display a grid of characters on screen using instanced quad rendering (background pass + foreground glyph pass). Use the Claude Dark theme colors as default.
6. **Handle window events** — Respond to resize events by recalculating the grid dimensions and re-rendering. Handle close events cleanly.

## Non-Goals (for this track)

- No terminal emulation (no PTY, no shell, no escape sequence parsing)
- No keyboard input handling (beyond window close)
- No scrollback, selection, or clipboard
- No split panes or tabs
- No configuration file loading
- No damage tracking (full re-render every frame is acceptable)

## Technical Specifications

### Project Structure

```
veloterm/
├── Cargo.toml
├── Cargo.lock
├── rustfmt.toml
├── src/
│   ├── main.rs              # Entry point, window creation, event loop
│   ├── renderer/
│   │   ├── mod.rs            # Render orchestration
│   │   ├── gpu.rs            # wgpu device, surface, pipeline setup
│   │   ├── glyph_atlas.rs    # Glyph rasterization + GPU texture atlas
│   │   └── grid_renderer.rs  # Terminal grid → instanced quads
│   └── config/
│       └── theme.rs          # Claude theme color definitions
└── shaders/
    └── grid.wgsl             # Vertex + fragment shader for grid cells
```

### Dependencies (from tech-stack.md)

```toml
[dependencies]
winit = "0.30"
wgpu = "24"
cosmic-text = "0.12"
log = "0.4"
env_logger = "0.11"
unicode-width = "0.2"
```

### GPU Rendering Pipeline

**Glyph Atlas:**
- Rasterize ASCII range (0x20–0x7E, 95 glyphs) plus common symbols
- Font: JetBrains Mono at 13px (as per UI guide), with DPI scaling
- Store as a single GPU texture (RGBA8, power-of-two dimensions)
- Record UV coordinates per glyph for texture sampling

**Grid Renderer:**
- Each cell is represented as an instance with: position, atlas UV coords, foreground color, background color
- Two render passes per frame:
  1. Background pass: colored quads for cell backgrounds
  2. Foreground pass: textured quads sampling from glyph atlas
- Vertex shader expands vertex index (0-5, two triangles) to quad corners
- Fragment shader samples glyph alpha from atlas and blends fg/bg colors

**Shader (grid.wgsl):**
```wgsl
struct CellInstance {
    @location(0) position: vec2<f32>,
    @location(1) atlas_uv: vec4<f32>,
    @location(2) fg_color: vec4<f32>,
    @location(3) bg_color: vec4<f32>,
    @location(4) flags: u32,
};
```

### Theme Colors (Claude Dark — Default)

From `conductor/ui-guide.md`:
- Background: `#1A1816`
- Pane Background: `#252320`
- Text Primary: `#E8E5DF`
- Text Muted: `#9B9389`
- Accent: `#E89171`
- Border: `#3D3833`

### Window Configuration

- Default size: 1280x720 (resizable)
- Title: "VeloTerm"
- DPI-aware: scale font size and grid dimensions based on monitor scale factor
- Background clear color: `#1A1816` (Claude Dark background)

### Grid Calculation

- Cell width = glyph advance width (from font metrics, scaled for DPI)
- Cell height = line height (font size * 1.6 line height ratio, per UI guide)
- Grid columns = floor(window_width / cell_width)
- Grid rows = floor(window_height / cell_height)
- Recalculate on window resize

### Static Content for Validation

Fill the grid with a test pattern to validate rendering:
- Row 0: "VeloTerm v0.1.0" (in accent color `#E89171`)
- Row 1: Empty
- Row 2: Full ASCII printable range (0x20–0x7E) to validate all glyphs
- Row 3: "claude@anthropic ~ $" (prompt colors from UI guide)
- Remaining rows: Checkerboard pattern of spaces and block characters to validate cell alignment

## Acceptance Criteria

1. `cargo build` succeeds with no warnings on macOS Apple Silicon
2. `cargo build` succeeds with no warnings on x86_64 Linux (CentOS 9 compatible glibc)
3. Application opens a window and renders the static grid with correct font glyphs
4. Grid uses Claude Dark theme colors correctly
5. Window resize recalculates grid dimensions and re-renders without artifacts
6. Window close exits cleanly
7. All tests pass with >80% coverage on renderer modules
8. Code passes `cargo fmt --check` and `cargo clippy -- -D warnings`

## References

- `conductor/tech-stack.md` — Crate versions, GPU architecture, threading model
- `conductor/ui-guide.md` — Theme colors, font specifications, layout dimensions
- `conductor/product.md` — Feature priorities, performance targets
- `projectVeloTerm` — Detailed rendering pipeline architecture, shader pseudocode

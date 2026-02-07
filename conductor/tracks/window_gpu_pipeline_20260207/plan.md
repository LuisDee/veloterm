# Plan: Cross-Platform Window and GPU Rendering Pipeline

## Phase 1: Project Initialization and Window Creation [checkpoint: 9f911e1]

- [x] Task: Initialize Rust project with Cargo workspace and directory structure <!-- d36e071 -->
  - Create `Cargo.toml` with dependencies (winit, wgpu, cosmic-text, log, env_logger, unicode-width)
  - Create `rustfmt.toml` with `max_width = 100`
  - Create `src/main.rs` with minimal entry point
  - Create module directories: `src/renderer/`, `src/config/`, `shaders/`
  - Verify `cargo build` succeeds

- [x] Task: Write tests for theme color definitions <!-- d03aa27 -->
  - Create `src/config/theme.rs` with test module
  - Write tests validating Claude Dark, Light, and Warm theme color values match UI guide hex codes
  - Write tests for color conversion (hex string to RGBA float)
  - Run tests and confirm they fail (Red phase)

- [x] Task: Implement theme color definitions <!-- d03aa27 -->
  - Define `Theme` struct with all color fields (background, pane_background, border, text_primary, text_muted, accent, accent_hover, prompt, success, error, selection)
  - Implement `claude_dark()`, `claude_light()`, `claude_warm()` constructors with exact hex values from UI guide
  - Implement hex-to-RGBA conversion utility
  - Run tests and confirm they pass (Green phase)

- [x] Task: Write tests for window creation and event loop setup <!-- d62a12d -->
  - Write tests for window configuration (default size 1280x720, title "VeloTerm", resizable)
  - Write tests for DPI scale factor handling
  - Run tests and confirm they fail (Red phase)

- [x] Task: Implement window creation with winit <!-- d62a12d -->
  - Create window with winit: 1280x720 default size, title "VeloTerm", resizable
  - Set up event loop with proper close handling
  - Handle DPI scale factor for HiDPI displays
  - Log window creation details (size, scale factor, backend)
  - Run tests and confirm they pass (Green phase)

- [x] Task: Conductor - User Manual Verification 'Project Initialization and Window Creation' (Protocol in workflow.md) <!-- 9f911e1 -->

## Phase 2: GPU Pipeline Setup [checkpoint: d9f26fb]

- [x] Task: Write tests for wgpu device and surface initialization <!-- 2d31114 -->
  - Write tests for GPU adapter selection (prefer high-performance)
  - Write tests for surface configuration (format, present mode)
  - Write tests for device limits and features
  - Run tests and confirm they fail (Red phase)

- [x] Task: Implement wgpu device, surface, and render pipeline <!-- 2d31114 -->
  - Create `src/renderer/gpu.rs` with GPU state management
  - Request adapter with power preference high-performance
  - Create device and queue
  - Configure surface with window size and preferred format
  - Handle surface resize on window resize events
  - Clear screen to Claude Dark background color (`#1A1816`)
  - Run tests and confirm they pass (Green phase)

- [x] Task: Write tests for render pipeline creation <!-- d474a3e -->
  - Write tests for shader compilation (grid.wgsl)
  - Write tests for pipeline layout (bind groups, vertex buffers)
  - Write tests for vertex buffer layout matching CellInstance struct
  - Run tests and confirm they fail (Red phase)

- [x] Task: Implement render pipeline and shaders <!-- d474a3e -->
  - Create `shaders/grid.wgsl` with vertex and fragment shaders
  - Vertex shader: expand vertex index to quad corners, apply cell position and size
  - Fragment shader: sample glyph atlas texture, blend fg/bg colors based on glyph alpha
  - Create render pipeline with correct vertex buffer layout for CellInstance
  - Set up bind groups for uniform buffer (cell_size, grid_size) and texture sampler
  - Run tests and confirm they pass (Green phase)

- [x] Task: Conductor - User Manual Verification 'GPU Pipeline Setup' (Protocol in workflow.md) <!-- d9f26fb -->

## Phase 3: Glyph Atlas

- [x] Task: Write tests for glyph rasterization and atlas packing <!-- 4cff54d -->
  - Write tests for font loading (JetBrains Mono with fallbacks)
  - Write tests for glyph metrics (advance width, height, bearing)
  - Write tests for atlas texture dimensions (power-of-two)
  - Write tests for UV coordinate calculation per glyph
  - Write tests for ASCII range coverage (0x20–0x7E, 95 glyphs)
  - Run tests and confirm they fail (Red phase)

- [x] Task: Implement glyph atlas rasterization <!-- 4cff54d -->
  - Create `src/renderer/glyph_atlas.rs`
  - Load monospace font using cosmic-text (JetBrains Mono → Fira Code → SF Mono → system monospace)
  - Rasterize ASCII printable range (0x20–0x7E) at configured font size (13px * DPI scale)
  - Pack glyphs into a texture atlas (RGBA8, power-of-two dimensions)
  - Record per-glyph metadata: UV coordinates, advance width, bearing offset
  - Calculate cell dimensions from font metrics (width = advance, height = font_size * 1.6)
  - Run tests and confirm they pass (Green phase)

- [x] Task: Write tests for atlas GPU texture upload <!-- 541b27d -->
  - Write tests for texture creation with correct dimensions and format
  - Write tests for texture view and sampler configuration
  - Run tests and confirm they fail (Red phase)

- [x] Task: Implement atlas GPU texture upload <!-- 541b27d -->
  - Upload rasterized atlas data to a wgpu texture
  - Create texture view and sampler with linear filtering
  - Bind atlas texture to the render pipeline's bind group
  - Run tests and confirm they pass (Green phase)

- [ ] Task: Conductor - User Manual Verification 'Glyph Atlas' (Protocol in workflow.md)

## Phase 4: Grid Renderer and Static Display

- [ ] Task: Write tests for grid dimension calculation
  - Write tests for column/row calculation from window size and cell dimensions
  - Write tests for grid recalculation on window resize
  - Write tests for DPI-scaled cell sizing
  - Run tests and confirm they fail (Red phase)

- [ ] Task: Implement grid dimension calculation
  - Create `src/renderer/grid_renderer.rs`
  - Calculate grid columns = floor(window_width / cell_width)
  - Calculate grid rows = floor(window_height / cell_height)
  - Recalculate on window resize events
  - Store grid state (dimensions, cell size, instance buffer)
  - Run tests and confirm they pass (Green phase)

- [ ] Task: Write tests for cell instance buffer generation
  - Write tests for CellInstance struct layout (position, atlas_uv, fg_color, bg_color, flags)
  - Write tests for instance buffer generation from a grid of characters
  - Write tests for correct UV lookup from glyph atlas for each character
  - Write tests for color assignment from theme
  - Run tests and confirm they fail (Red phase)

- [ ] Task: Implement cell instance buffer and rendering
  - Define CellInstance struct matching shader layout
  - Generate instance data for each cell: screen position, glyph UV, fg/bg colors
  - Upload instance buffer to GPU
  - Render frame: clear background → draw background quads → draw foreground glyphs
  - Run tests and confirm they pass (Green phase)

- [ ] Task: Write tests for static test pattern content
  - Write tests for test pattern generation (VeloTerm header, ASCII range, prompt line, checkerboard)
  - Write tests for correct color assignment per test pattern element (accent for header, prompt colors, primary for text)
  - Run tests and confirm they fail (Red phase)

- [ ] Task: Implement static test pattern display
  - Fill grid with test pattern:
    - Row 0: "VeloTerm v0.1.0" in accent color (#E89171)
    - Row 1: Empty
    - Row 2: Full ASCII printable range to validate all glyphs
    - Row 3: "claude@anthropic ~ $" with prompt colors from UI guide
    - Remaining rows: Alternating characters to validate cell alignment
  - Verify rendering on window resize (grid recalculates and re-renders)
  - Run tests and confirm they pass (Green phase)

- [ ] Task: Conductor - User Manual Verification 'Grid Renderer and Static Display' (Protocol in workflow.md)

## Phase 5: Render Orchestration and Polish

- [ ] Task: Write tests for render orchestration module
  - Write tests for renderer initialization (creates GPU state, atlas, grid renderer)
  - Write tests for render frame lifecycle (acquire surface → encode commands → present)
  - Write tests for window resize handling (reconfigure surface, recalculate grid)
  - Run tests and confirm they fail (Red phase)

- [ ] Task: Implement render orchestration
  - Create `src/renderer/mod.rs` as the top-level render coordinator
  - Initialize GPU state, glyph atlas, and grid renderer
  - Implement `render_frame()`: acquire surface texture → create command encoder → render passes → submit → present
  - Handle surface lost/outdated errors (reconfigure and retry)
  - Handle window resize: reconfigure surface, recalculate grid, re-render
  - Run tests and confirm they pass (Green phase)

- [ ] Task: Write tests for clean shutdown
  - Write tests for proper resource cleanup (GPU device, surface, buffers)
  - Write tests for window close event handling
  - Run tests and confirm they fail (Red phase)

- [ ] Task: Implement clean shutdown and final integration
  - Handle window close event → exit event loop cleanly
  - Drop GPU resources in correct order
  - Add env_logger initialization for debug output
  - Add log statements at key points (GPU backend selected, atlas size, grid dimensions)
  - Verify `cargo clippy -- -D warnings` passes
  - Verify `cargo fmt --check` passes
  - Run full test suite and verify >80% coverage
  - Run tests and confirm they pass (Green phase)

- [ ] Task: Conductor - User Manual Verification 'Render Orchestration and Polish' (Protocol in workflow.md)

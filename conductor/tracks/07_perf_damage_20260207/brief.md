<!-- ARCHITECT CONTEXT | Track: 07_perf_damage | Wave: 1 | CC: v1 -->

## Cross-Cutting Constraints
- Performance Budget: <10ms input-to-screen, 2 draw calls/frame, only update dirty cells
- Testing: TDD, cargo test --lib, clippy clean

## Interfaces

### Owns
- Dirty cell tracking bitfield API
- Selective instance buffer update API
- Frame budget metrics

### Consumes
- `Renderer::update_cells()` (existing — currently rebuilds entire buffer)
- `GridCell` and `CellInstance` types (existing)

## Dependencies
- None (Wave 1 — optimizes existing renderer)

<!-- END ARCHITECT CONTEXT -->

# Track 07: Performance & Damage Tracking

## What This Track Delivers

A damage tracking system that identifies which terminal cells changed since the last frame and updates only those cells in the GPU instance buffer. Currently, every frame rebuilds the entire instance buffer (~10,000 cells for a 200x50 grid). With damage tracking, interactive typing updates only 1-5 cells per frame, dramatically reducing CPU-to-GPU data transfer.

## Scope

### IN
- Dirty cell bitfield (`bitvec` crate) tracking changed cells per frame
- Selective instance buffer updates (only write dirty cell instances to GPU)
- Damage region coalescing (merge adjacent dirty cells for efficient buffer writes)
- Frame timing metrics (measure render time, buffer update time)
- Full-damage fallback for resize, scroll, and theme changes

### OUT
- GPU-side optimizations (shader changes, compute shaders)
- Multi-pass rendering changes (background + foreground pass structure stays)
- Scrollback virtualization (only render visible rows — already implicit in grid bridge)
- Benchmarking harness or profiling tools

## Key Design Decisions

1. **Dirty tracking granularity**: Per-cell bitfield vs per-row dirty flags vs rectangular damage regions?
   Trade-off: per-cell is most precise but largest bitfield; per-row is simpler but over-updates; rectangles are efficient for scroll but complex to merge

2. **Buffer update strategy**: Partial `queue.write_buffer()` with byte offsets vs rebuild-and-upload dirty subset vs GPU-side ring buffer?
   Trade-off: partial writes are simplest; ring buffer avoids stalls but adds complexity

3. **Damage source tracking**: Terminal state diff (compare old grid vs new) vs event-based (terminal parser emits dirty ranges)?
   Trade-off: diffing is decoupled but O(cells); events are O(changes) but require parser integration

4. **Full-damage triggers**: Explicit list of events that force full redraw (resize, scroll, theme change) vs heuristic (if >50% dirty, do full)?
   Trade-off: explicit is predictable; heuristic adapts to pathological cases (e.g., `cat large_file`)

## Architectural Notes

- Current `Renderer::update_cells()` creates a new buffer every frame — the first optimization is to reuse the buffer and only write changed regions
- `wgpu::Queue::write_buffer()` supports byte offset + data for partial updates
- The `bitvec` crate is already in Cargo.toml dependencies (tech-stack.md)
- Damage tracking must integrate with the grid bridge (`extract_grid_cells`) — either the bridge produces a diff or the renderer diffs the new cells against the previous frame
- Be careful with scrolling: a scroll-by-1 makes every visible cell "dirty" if using naive comparison. Consider treating scroll as a special case with buffer rotation.

## Complexity: M
## Estimated Phases: ~3

# Spec: Performance & Damage Tracking

## Overview

Implement a damage tracking system that identifies which terminal rows changed since the last frame and updates only those rows in the GPU instance buffer. Currently, every frame rebuilds the entire instance buffer (~10,000 CellInstance structs at 72 bytes each for a 200x50 grid). With damage tracking, interactive typing updates only 1 row (~200 cells) per frame instead of all rows, dramatically reducing CPU work and CPU-to-GPU data transfer.

## Functional Requirements

### FR-1: DamageTracker

A `DamageTracker` struct that manages per-frame dirty state:

- Maintains a `Vec<bool>` of per-row dirty flags (one entry per visible row).
- Provides `mark_row_dirty(row: usize)` to flag a specific row.
- Provides `mark_all_dirty()` to force full damage (all rows flagged dirty).
- Provides `dirty_rows() -> impl Iterator<Item = usize>` to iterate over dirty row indices.
- Provides `clear()` to reset all flags after a frame is rendered.
- Resizes the dirty flags vector when grid dimensions change.

### FR-2: Grid-Level Diff

Detect which rows changed by comparing the current frame's grid cells against a cached copy of the previous frame:

- After `extract_grid_cells()` produces the current `Vec<GridCell>`, compare it row-by-row against the cached previous frame.
- A row is dirty if any cell in that row differs (character, foreground color, background color, or flags).
- `GridCell` must implement `PartialEq` to enable comparison.
- Cache the current frame's cells as the "previous frame" after diffing.
- On the first frame (no previous cache), all rows are dirty.

### FR-3: Partial Instance Buffer Updates

Replace the current full-buffer recreation with partial writes to a persistent buffer:

- Allocate the instance buffer once at initialization (and on resize) with capacity for `cols * rows` instances.
- For each dirty row, compute the byte offset: `row_index * cols * size_of::<CellInstance>()`.
- Use `queue.write_buffer(instance_buffer, offset, &row_instance_bytes)` to write only the dirty rows.
- The buffer already has `COPY_DST` usage flag set (required for `write_buffer`).

### FR-4: Full-Damage Events

The following events bypass row-level diffing and force all rows dirty (full redraw):

1. **Window resize** — grid dimensions change, buffer is reallocated.
2. **Scroll position change** — all visible rows shift, every row is "new".
3. **Theme/color scheme change** — all cell colors change.
4. **Font size change** — grid dimensions change, atlas regenerated.
5. **Initial frame** — no previous state to diff against.

### FR-5: Frame Timing Metrics

Basic instrumentation to measure render performance:

- Measure time spent in damage detection (grid diff).
- Measure time spent in buffer updates (partial writes).
- Measure total frame time (from update_cells entry to render_frame exit).
- Log metrics at `debug` level, with periodic summary at `info` level (e.g., every 60 frames).

## Non-Functional Requirements

- **Performance:** The damage tracking overhead (diff + partial write) must be less than the cost of a full buffer rebuild for the common case (1-5 dirty rows out of 50).
- **Memory:** Previous frame cache adds ~one `Vec<GridCell>` (~10,000 cells). Dirty flags add one `Vec<bool>` (~50 entries). Total overhead <100KB.
- **Compatibility:** No changes to the WGSL shader, render pipeline, or draw call structure. No changes to `extract_grid_cells()` or the grid bridge.
- **Testability:** DamageTracker and grid diff logic must be unit-testable without GPU context.

## Acceptance Criteria

1. Typing a single character results in only 1 row being written to the GPU buffer (not all rows).
2. Window resize triggers a full buffer rebuild (all rows dirty).
3. Scroll triggers full damage (all rows dirty).
4. Theme change triggers full damage (all rows dirty).
5. All existing tests continue to pass.
6. Frame timing metrics are logged at debug level.
7. `DamageTracker` has >80% test coverage.
8. Grid diff logic has >80% test coverage.

## Out of Scope

- GPU-side optimizations (shader changes, compute shaders).
- Multi-pass rendering changes (background + foreground pass structure unchanged).
- Scrollback virtualization.
- Benchmarking harness or profiling tools.
- Damage region coalescing beyond row-level granularity.
- Ring buffers or double-buffering strategies.

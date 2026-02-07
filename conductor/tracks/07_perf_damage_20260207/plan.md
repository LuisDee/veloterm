# Plan: Performance & Damage Tracking

## Phase 1: DamageTracker Core

- [x] Task 1: Implement DamageTracker struct with per-row dirty flags <!-- b20449a -->
  - [x] Write tests: construction with row count, mark_row_dirty, mark_all_dirty, dirty_rows iteration, clear resets all flags, resize adjusts flag count, out-of-bounds row index is ignored
  - [x] Implement DamageTracker in new module `src/renderer/damage.rs`
  - [x] Export from `src/renderer/mod.rs`

- [x] Task 2: Derive PartialEq on GridCell and Color <!-- b9d13a3 -->
  - [x] Write tests: GridCell equality when all fields match, inequality when char differs, inequality when fg differs, inequality when bg differs, inequality when flags differ
  - [x] Add `PartialEq` derive to GridCell and Color structs

- [ ] Task: Conductor - User Manual Verification 'Phase 1: DamageTracker Core' (Protocol in workflow.md)

## Phase 2: Grid Diff & Previous Frame Cache

- [ ] Task 1: Implement row-level grid diff function
  - [ ] Write tests: identical grids produce no dirty rows, single cell change marks only that row dirty, changes in multiple rows mark all affected rows, empty grids produce no dirty rows, grids with different dimensions trigger full damage
  - [ ] Implement `diff_grid_rows(prev: &[GridCell], curr: &[GridCell], cols: usize) -> Vec<bool>` in `src/renderer/damage.rs`

- [ ] Task 2: Implement DamageState that manages previous frame cache and diffing
  - [ ] Write tests: first frame (no cache) returns all-dirty, second frame with no changes returns no dirty rows, second frame with one row changed returns that row dirty, cache updates after each diff, resize clears cache and returns all-dirty
  - [ ] Implement `DamageState` struct wrapping DamageTracker + previous frame cache

- [ ] Task: Conductor - User Manual Verification 'Phase 2: Grid Diff & Previous Frame Cache' (Protocol in workflow.md)

## Phase 3: Renderer Integration

- [ ] Task 1: Convert to persistent instance buffer with partial row writes
  - [ ] Write tests: verify byte offset calculation for row N is `row * cols * 72`, verify generate_row_instances produces correct instance data for a single row, verify full-frame generates instances for all rows
  - [ ] Replace `update_cells()` buffer recreation with persistent buffer + `queue.write_buffer()` partial writes per dirty row
  - [ ] Allocate persistent buffer in `Renderer::new()` with full grid capacity

- [ ] Task 2: Wire full-damage events into DamageState
  - [ ] Write tests: resize triggers full damage and cache clear, theme change triggers full damage, font size change triggers full damage, scroll position change triggers full damage
  - [ ] Add `force_full_damage()` method and call it from resize, theme change, font size change, and scroll handlers

- [ ] Task 3: Add frame timing metrics
  - [ ] Write tests: FrameMetrics records diff_time, update_time, and total_time; periodic summary computes averages over N frames
  - [ ] Implement FrameMetrics struct with `std::time::Instant` measurements
  - [ ] Log at debug level per-frame, info level summary every 60 frames

- [ ] Task: Conductor - User Manual Verification 'Phase 3: Renderer Integration' (Protocol in workflow.md)

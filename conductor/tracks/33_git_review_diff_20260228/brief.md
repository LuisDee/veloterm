# Track 33: Git Review — Diff View

## Source Requirements

From `new_feature` spec, Phase 6:
- Implement diff computation via git2
- Build line alignment algorithm
- Implement Canvas-based dual-pane rendering with line numbers
- Add synchronized scrolling
- Success: Side-by-side diff that works correctly for all change types

## Cross-Cutting Constraints

- TDD workflow, all cross-cutting v1 constraints
- iced 0.14 Canvas widget for custom rendering
- Performance: virtual scrolling for large diffs, background thread for computation
- Monospace font (JetBrains Mono) for diff content

## Dependencies

- Track 28 (Overlay Foundation) — provides SplitPanel right panel slot
- Track 32 (Git Review Files) — provides file selection events and git repository handle

## What This Track Delivers

The right panel of the Git Review overlay: a side-by-side diff view with aligned line pairs, line numbers, change indicators, synchronized scrolling, and correct handling of added/deleted/renamed/binary files.

## Scope IN

- Empty state: "Select a file to view changes"
- Diff computation via git2:
  - Staged: diff_tree_to_index (HEAD vs index)
  - Unstaged: diff_index_to_workdir (index vs working dir)
  - Untracked: entire file as additions
- Line alignment algorithm: build left_lines / right_lines vectors
  - Context lines: both sides, same row
  - Deletions: left only, spacer on right
  - Additions: spacer on left, right only
- Canvas-based dual-pane rendering:
  - Two side-by-side text columns
  - Line number gutters on outer edges
  - Change indicator strips (green=added, red=deleted, yellow=modified)
  - Row background tints for changed lines
- Synchronized scrolling: shared vertical scroll offset
- Horizontal scrolling (both panes together)
- Hunk headers: @@ -start,count +start,count @@ separator bars
- Handling: added files, deleted files, renamed files, binary files
- Virtual scrolling for large diffs

## Scope OUT

- Word-level intra-line diff highlighting (Track 34)
- Syntax highlighting in diff content (Track 34)
- Hunk collapse/expand (Track 34)
- Interactive hunk staging (Track 34)

## Key Design Decisions

1. **Canvas vs text widgets**: The spec explicitly calls for Canvas. This gives full control over layout but means implementing text rendering, scrolling, and hit testing manually. The existing glyph atlas/grid renderer won't apply here — this is pure iced Canvas with draw_text.

2. **Line alignment data structure**: How to represent aligned pairs? Options: (a) Vec<(Option<Line>, Option<Line>)> — simple but wasteful, (b) enum AlignedRow { Both, LeftOnly, RightOnly } — typed, (c) parallel vecs with indices — complex but efficient.

3. **Diff caching**: Cache computed diffs per file or recompute on every selection? Caching is important for snappy switching between files, but diffs can become stale.

4. **Scroll synchronization**: Single scroll state shared between panes vs coordinated separate states? Since lines are aligned with spacers, a single scroll offset is simplest.

## Test Strategy

- Unit tests for line alignment algorithm:
  - Pure additions (empty left, all right)
  - Pure deletions (all left, empty right)
  - Mixed changes (interleaved context/add/delete)
  - Modification detection (adjacent delete+add → modification pair)
- Unit tests for diff computation from git2 data structures
- Unit tests for hunk header parsing
- Unit tests for virtual scrolling (visible row range from scroll offset)
- Unit tests for renamed file detection and display
- Unit tests for binary file detection
- Unit tests for scroll synchronization logic
- Framework: `cargo test --lib`

## Complexity

L (Large)

## Estimated Phases

4 phases:
1. Diff computation via git2 + line alignment algorithm
2. Canvas-based dual-pane rendering with line numbers
3. Synchronized scrolling + virtual scrolling
4. Hunk headers + special cases (added/deleted/renamed/binary)

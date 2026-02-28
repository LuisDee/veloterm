# Track 34: Git Review — Diff Polish

## Source Requirements

From `new_feature` spec, Phase 7:
- Add word-level (intra-line) diff highlighting via `similar`
- Add hunk headers and collapse/expand
- Add interactive hunk staging
- Handle renamed, deleted, binary files
- Syntax highlighting in diff content
- Success: Diff view that matches Sublime Merge quality

## Cross-Cutting Constraints

- TDD workflow, all cross-cutting v1 constraints
- iced 0.14 Canvas widget patterns established in Track 33
- similar crate for word-level diffing
- syntect for syntax highlighting
- git2 for hunk staging operations

## Dependencies

- Track 33 (Git Review Diff) — provides base diff rendering
- Track 32 (Git Review Files) — provides staging infrastructure

## What This Track Delivers

Polish features for the Git Review diff view: word-level intra-line highlighting (showing exactly which characters changed), syntax highlighting of diff content, collapsible hunks, and interactive hunk staging (stage/unstage individual hunks without staging the whole file).

## Scope IN

- Word-level diff via `similar` crate:
  - TextDiff::from_words(old_line, new_line) for modification pairs
  - Equal spans: normal text color
  - Delete spans: red/orange background highlight (left pane)
  - Insert spans: green background highlight (right pane)
- Syntax highlighting of diff content via syntect:
  - Language detection from file extension
  - Per-line highlighting applied to both panes
  - Highlighting colors blend with diff background tints
- Hunk collapse/expand:
  - Chevron on hunk header bar
  - Collapsed state shows only header with line count summary
- Interactive hunk staging:
  - "Stage hunk" button (+) on hover over hunk header (for unstaged files)
  - "Unstage hunk" button (-) on hover (for staged files)
  - Apply partial diff via git2 (construct patch, apply to index)
  - Refresh diff view after staging/unstaging
- Renamed file display: "old/path.rs → new/path.rs" in header

## Scope OUT

- Line-level staging (selecting individual lines to stage — too complex for v1)
- Merge conflict resolution (Track 35)
- Three-way diff view (out of scope entirely)

## Key Design Decisions

1. **Hunk staging implementation**: git2's `Repository::apply()` with a constructed Diff vs constructing a patch string and applying via git CLI? git2's apply API is the cleaner approach but may have edge cases.

2. **Syntax highlighting + diff colors blending**: How to combine syntect token colors with diff background tints? Options: (a) render syntax colors on top of diff backgrounds, (b) adjust syntax colors based on diff context, (c) use opacity blending.

3. **similar crate word boundary**: `from_words` vs `from_chars` vs `from_graphemes`? Words are the right granularity for most code diffs (matches Sublime Merge behavior).

4. **Hunk state after partial staging**: After staging a hunk, should the diff view: (a) remove the hunk and close gaps, (b) show the hunk as "staged" with different styling, (c) fully refresh the diff?

## Test Strategy

- Unit tests for word-level diff:
  - Single word change in a line
  - Multiple word changes in a line
  - Line with only whitespace changes
  - Empty old/new line edge cases
- Unit tests for hunk collapse/expand state management
- Unit tests for hunk staging:
  - Stage single hunk from multi-hunk diff
  - Unstage single hunk
  - Verify remaining hunks after partial staging
- Unit tests for syntax highlighting integration:
  - Correct language detection
  - Highlight spans applied to diff lines
- Unit tests for renamed file header formatting
- Framework: `cargo test --lib`

## Complexity

M (Medium)

## Estimated Phases

3 phases:
1. Word-level diff highlighting with similar crate
2. Syntax highlighting in diff content + hunk collapse/expand
3. Interactive hunk staging via git2

# Track 35: Final Polish

## Source Requirements

From `new_feature` spec, Phase 8:
- Animation passes (hover transitions, panel resize smoothness, scroll momentum)
- Performance optimization (profile with large repos: Linux kernel, Chromium)
- Edge case handling (empty repos, merge conflicts, submodules, symlinks)
- Accessibility (keyboard-only navigation works end-to-end)
- Success: A developer could use these overlays as their primary workflow

## Cross-Cutting Constraints

- TDD workflow, all cross-cutting v1 constraints
- Performance budget: <10ms response for all interactions
- Platform abstraction: macOS + Linux CentOS 9

## Dependencies

- Track 31 (File Browser Polish) — all file browser features complete
- Track 34 (Git Review Polish) — all git review features complete

## What This Track Delivers

Final quality pass across both overlays: smooth animations, performance optimization for large repositories, edge case handling, keyboard accessibility audit, and cross-platform verification.

## Scope IN

- Hover transitions: ~100ms smooth background fade on tree items, file list items
- Panel resize smoothness: 60fps divider drag
- Overlay open/close transitions: ~150ms fade/slide
- Scroll momentum on trackpad
- Performance profiling with large repos (10K+ files, 1000+ line diffs)
- Edge cases: empty repositories, repos with no commits, submodules, symlinks
- Merge conflict file indicators (not resolution — just display)
- Keyboard-only navigation audit: ensure all features accessible without mouse
- Cross-platform testing: macOS + Linux
- Memory cleanup: ensure overlay state is properly released on close
- Config integration: keyboard shortcut customization via TOML

## Scope OUT

- New features beyond spec Phase 8
- Merge conflict resolution UI
- File editing capabilities
- Terminal integration (sending commands to terminal panes)

## Key Design Decisions

1. **Animation framework**: iced doesn't have built-in animation support. Options: (a) manual animation via tick-based state interpolation, (b) use iced_futures for animation subscriptions, (c) CSS-like transition system with easing functions.

2. **Performance targets**: What constitutes "acceptable" for large repos? Need concrete metrics: (a) max time to open overlay, (b) max time to expand a directory with 1000 files, (c) max time to compute a diff with 10000 lines.

3. **Submodule handling**: Show submodules as special directory entries or ignore them? Git status for submodules is complex (dirty submodule, uninitialized, etc.).

## Test Strategy

- Performance benchmarks: tree rendering with 10K items, diff with 10K lines
- Edge case tests: empty repo, no commits, submodule detection
- Keyboard navigation integration tests: full workflow without mouse
- Memory tests: open/close overlay 100 times, verify no leaks
- Cross-platform compilation check
- Framework: `cargo test --lib`

## Complexity

M (Medium)

## Estimated Phases

3 phases:
1. Animations + transitions
2. Performance optimization + large repo handling
3. Edge cases + accessibility + cross-platform verification

# Track 29: File Browser — Navigation

## Source Requirements

From `new_feature` spec, Phase 2:
- Implement tree view data model with lazy loading
- Render tree with proper indentation, icons, expand/collapse
- Add breadcrumb bar
- Add keyboard navigation
- Implement virtual scrolling
- Add file system watching
- Success: Can navigate a large project's file tree fluidly

## Cross-Cutting Constraints

- TDD workflow, all cross-cutting v1 constraints
- iced 0.14 widget system patterns
- Async file I/O (but NOT tokio — project uses crossbeam-channel threading model)
- Performance: virtual scrolling critical for repos with thousands of files

## Dependencies

- Track 28 (Overlay Foundation) — provides SplitPanel, InputMode::FileBrowser, overlay shell

## What This Track Delivers

The left panel of the File Browser overlay: a hierarchical file/directory tree with lazy loading, expand/collapse, breadcrumb navigation, keyboard navigation, virtual scrolling, and file system watching via notify crate.

## Scope IN

- FileTree data model: nodes for files and directories, lazy-loaded children
- Tree rendering: indentation (depth × 16-20px), chevron icons, file-type icons
- Directory expand/collapse with state tracking
- Breadcrumb bar showing current path with clickable segments
- Root directory detection from focused terminal pane's cwd
- Sorting: directories first, then files, case-insensitive alphabetical
- Virtual scrolling: flatten expanded tree to Vec<VisibleRow>, render only viewport
- Fixed row height (~28px) for efficient scrolling
- Keyboard navigation: arrows, left/right collapse/expand, Enter to select
- File system watching via `notify` crate with 200ms debounce
- Hover and selection visual states
- Hidden files hidden by default (toggle deferred to Track 31)

## Scope OUT

- File preview/viewer (Track 30)
- Fuzzy search/filter (Track 31)
- Context menus (Track 31)
- Git status indicators on tree nodes (Track 31)
- Hidden files toggle UI (Track 31)

## Key Design Decisions

1. **Threading model for file I/O**: The project does NOT use tokio. Options: (a) spawn dedicated thread with crossbeam-channel for directory reads, (b) use std::fs synchronous reads on background thread, (c) add tokio just for file I/O. Given existing architecture, crossbeam-channel + background thread is most consistent.

2. **Tree data structure**: Vec-based flat tree (like VS Code's splice model) vs recursive tree struct? Flat vec is better for virtual scrolling but harder for lazy loading. Recursive tree is natural for lazy loading but needs flattening for rendering.

3. **Virtual scrolling implementation**: iced `lazy` widget vs custom Canvas-based rendering vs computed visible slice from scrollable? The existing scrollbar implementation in the project may provide patterns.

4. **File type icons**: Unicode glyphs vs embedded icon assets? Need icons for ~20 common file types (rs, py, js, ts, json, toml, md, etc.)

5. **notify crate integration**: Watch individual expanded directories vs recursive watch from root? Individual watches are more efficient but need lifecycle management.

## Test Strategy

- Unit tests for FileTree data model: insert, expand, collapse, sort order
- Unit tests for tree flattening (expanded tree → visible rows)
- Unit tests for virtual scrolling calculations (scroll offset → visible range)
- Unit tests for breadcrumb path parsing and navigation
- Unit tests for keyboard navigation state transitions
- Unit tests for file type icon mapping
- Unit tests for directory sorting (dirs first, case-insensitive alpha)
- Integration test: expand directory populates children
- Framework: `cargo test --lib`

## Complexity

L (Large)

## Estimated Phases

4 phases:
1. FileTree data model + sorting + expand/collapse logic
2. Tree rendering with virtual scrolling
3. Breadcrumb bar + keyboard navigation
4. File system watching via notify + auto-refresh

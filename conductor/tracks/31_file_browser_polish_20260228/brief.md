# Track 31: File Browser — Polish

## Source Requirements

From `new_feature` spec, Phase 4:
- Fuzzy search/filter
- Context menus
- Git status indicators on tree nodes
- Hidden files toggle
- File metadata display
- Success: Feature-complete file explorer that rivals VS Code's sidebar

## Cross-Cutting Constraints

- TDD workflow, all cross-cutting v1 constraints
- iced 0.14 widget system patterns
- nucleo crate for fuzzy matching (same as Helix editor)
- git2 crate for status indicators

## Dependencies

- Track 29 (File Browser Nav) — tree view to add features onto
- Track 30 (File Browser Viewer) — viewer for context menu "open" actions

## What This Track Delivers

Polish features for the File Browser: fuzzy file search with nucleo, right-click context menus, git status indicators (M/U/S badges) on tree nodes with parent propagation, hidden files toggle, and comprehensive file metadata.

## Scope IN

- Fuzzy search/filter input at top of tree panel
- nucleo crate integration for fuzzy matching
- Matched character highlighting in search results
- Flat results view when filtering (relative paths shown)
- Right-click context menus: copy path, copy relative path, copy filename, reveal in file manager, open in terminal, delete/trash, new file/directory
- Git status indicators: Modified (orange M), Untracked (green U), Staged (green S), Ignored (dimmed)
- Status propagation: directory shows indicator if any child has changes
- git2 integration for repository status reading
- Hidden files toggle button (eye icon) in panel header
- `.gitignore` respect for hidden files

## Scope OUT

- Git Review overlay (Tracks 32-34)
- Animation polish (Track 35)

## Key Design Decisions

1. **nucleo integration pattern**: Synchronous matching vs background thread? For small-to-medium repos (<10K files), synchronous may be fine. For large repos, background thread with incremental results.

2. **Context menu implementation**: Custom iced widget vs overlay popup? The existing codebase doesn't have context menus. Need to build a reusable context menu widget.

3. **Git status caching**: Refresh on every expand vs cache with file watcher invalidation? git2 statuses() can be slow on large repos. Need a caching strategy.

4. **trash crate vs std::fs::remove**: The spec mentions "Move to trash" — the `trash` crate provides platform-aware trash behavior. Need to add this dependency.

## Test Strategy

- Unit tests for fuzzy matching: scoring, result ordering, character highlight positions
- Unit tests for git status mapping: StatusEntry flags → display categories
- Unit tests for status propagation through directory tree
- Unit tests for context menu action dispatch
- Unit tests for hidden files filtering logic
- Unit tests for .gitignore integration
- Integration test: type in search, see filtered results
- Framework: `cargo test --lib`

## Complexity

M (Medium)

## Estimated Phases

3 phases:
1. Fuzzy search with nucleo + filtered results view
2. Git status indicators + parent propagation
3. Context menus + hidden files toggle + trash integration

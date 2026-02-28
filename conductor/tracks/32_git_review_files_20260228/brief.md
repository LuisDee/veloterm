# Track 32: Git Review — Changed Files List

## Source Requirements

From `new_feature` spec, Phase 5:
- Implement git status reading via git2
- Render three sections (staged, changed, untracked)
- Implement stage/unstage/discard actions
- Add batch actions
- Add commit interface
- Success: Can view all changes, stage/unstage files, and commit — all without terminal

## Cross-Cutting Constraints

- TDD workflow, all cross-cutting v1 constraints
- iced 0.14 widget system patterns
- git2 crate for all git operations
- Confirmation dialogs for destructive actions (discard changes)

## Dependencies

- Track 28 (Overlay Foundation) — provides SplitPanel, InputMode::GitReview, overlay shell

## What This Track Delivers

The left panel of the Git Review overlay: a categorized file list showing staged, unstaged, and untracked files with stage/unstage/discard actions, batch operations, and a commit message input with commit button.

## Scope IN

- Git repository detection via git2::Repository::discover()
- "Not in a git repository" toast when overlay opened outside repo
- Three collapsible sections with count badges:
  - Staged Changes (green accent) — files in index
  - Changes (yellow accent) — modified tracked files
  - Untracked (gray accent) — new files
- File entries showing: status icon (A/M/D/R/?), relative path, dimmed directory
- Hover actions per file: stage (+), unstage (-), discard (undo)
- Discard confirmation popover: "Discard changes to X? Cannot be undone."
- Batch actions: Stage All, Unstage All buttons
- Commit interface: multiline text input, placeholder "Commit message..."
- Commit button: enabled only when staged changes exist AND message non-empty
- Commit execution via git2 (create commit, refresh file list)
- Success toast after commit
- Keyboard navigation: arrows across sections, Enter to view diff
- Auto-refresh on file system changes (debounced)
- Rename detection in git2 diff options

## Scope OUT

- Diff view (Track 33)
- Word-level diff (Track 34)
- Hunk staging (Track 34)
- Merge conflict handling (Track 35)

## Key Design Decisions

1. **git2 threading**: git2 Repository is not Send/Sync. Options: (a) open repository on each operation, (b) wrap in Mutex, (c) run git2 calls on dedicated thread with message passing. Given existing crossbeam-channel pattern, dedicated thread is most consistent.

2. **Stage/unstage implementation**: git2 Index API (add_path, remove_path) vs shelling out to git CLI? git2 is preferred for consistency but some operations (like `git reset HEAD`) map to different API calls.

3. **Commit interface position**: Top of panel vs bottom? The spec suggests testing both. VS Code puts it at top, GitKraken at bottom.

4. **Refresh strategy**: Poll git status periodically vs file watcher + manual refresh? File watcher is more efficient but git status can change without file system events (e.g., git commands in terminal).

## Test Strategy

- Unit tests for git status categorization: StatusEntry flags → Staged/Changed/Untracked
- Unit tests for file sorting within sections
- Unit tests for stage/unstage state transitions
- Unit tests for commit button enablement logic
- Unit tests for rename detection
- Unit tests for section collapse/expand state
- Unit tests for batch action effects
- Unit tests for repository detection (in repo, not in repo, nested repo)
- Integration test: stage file, verify it moves to staged section
- Framework: `cargo test --lib`

## Complexity

L (Large)

## Estimated Phases

4 phases:
1. Git repository detection + status reading + data model
2. Three-section file list rendering + selection
3. Stage/unstage/discard actions + confirmation dialogs
4. Commit interface + batch actions + auto-refresh

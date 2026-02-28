# Track 35 Specification: Overlay Final Polish

## Overview

Quality pass across both overlay modules: edge case handling (empty repos, merge conflicts, submodules, symlinks), keyboard accessibility improvements, performance guardrails for large trees/diffs, configurable keyboard shortcuts, and memory cleanup on overlay close.

**Non-goal**: Smooth animations (iced 0.14 lacks built-in transition support — would require a custom animation framework which is out of scope for a polish track).

---

## Phase 1: Edge Cases — Git Review

### Merge Conflict Detection

**File:** `src/git_review/status.rs`

Add a `Conflicted` variant to `FileStatus`:

```rust
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed { from: PathBuf },
    Untracked,
    Conflicted,  // NEW
}
```

In `GitStatus::from_repo()`, detect conflicted files from `git2::Status`:
- `STATUS_CONFLICTED` flag → `FileStatus::Conflicted`
- Display label: `"C"` (or `"!!"`)
- Conflicted files appear in the Changed section with a distinct indicator

### Submodule Detection

**File:** `src/git_review/status.rs`

When iterating status entries, detect submodules via `git2::Repository::submodules()` or by checking `entry.index_to_workdir()` for submodule status flags. Tag entries with a `is_submodule: bool` field on `StatusEntry`. Display submodules with a `"S"` label prefix.

### Empty Repository Handling

**File:** `src/git_review/mod.rs`

When `repo.head()` returns `Err` with `NotFound` (no commits yet):
- Set `git_status` to an empty `GitStatus` with all sections empty
- Display a message: "No commits yet — stage files and make your first commit"
- Do NOT treat as an error (`self.error` stays None)

---

## Phase 2: Edge Cases — File Browser

### Symlink Handling

**File:** `src/file_browser/tree.rs`

In `expand()`, detect symlinks via `entry.file_type()?.is_symlink()`:
- Add `Symlink { target: PathBuf }` variant to `NodeType`
- Symlinks are shown with a `→` suffix: `"link_name → /target/path"`
- Symlinks to directories are expandable (follow the link)
- Broken symlinks (target doesn't exist) shown with warning indicator

### Hidden Files Toggle

**File:** `src/file_browser/tree.rs`

The tree already filters hidden files. Add a `show_hidden: bool` field to `FileTree`:
- When `show_hidden = true`, include dotfiles in `expand()`
- Default: `false` (current behavior)
- Toggle via `Ctrl+H` keybinding when file browser is active

### Large Directory Guard

**File:** `src/file_browser/tree.rs`

In `expand()`, if a directory has more than 10,000 entries:
- Load only the first 1,000 entries
- Add a virtual "... N more files (too many to display)" node
- Log a warning

---

## Phase 3: Keyboard Accessibility & Config

### Configurable Overlay Shortcuts

**File:** `src/input/mod.rs`

Extend `match_overlay_command()` to accept configurable bindings from `KeysConfig`:

```rust
pub fn match_overlay_command(
    logical_key: &Key,
    modifiers: ModifiersState,
    bindings: &HashMap<String, String>,
) -> Option<OverlayCommand> {
    // Check custom bindings first, then fall back to defaults
}
```

Config TOML format:
```toml
[keys.bindings]
toggle_file_browser = "ctrl+e"
toggle_git_review = "ctrl+g"
```

### Keyboard Navigation Completeness

Verify and add missing keyboard shortcuts:

**File Browser (when active):**
- `Ctrl+H` — toggle hidden files
- `Home` / `End` — jump to first/last row
- `/` — focus search input (if search module exists)

**Git Review (when active):**
- `s` — stage selected file
- `u` — unstage selected file
- `d` → confirm `y` — discard selected file
- `c` — focus commit message input
- `Home` / `End` — jump to first/last file

### Memory Cleanup

**File:** `src/file_browser/mod.rs`, `src/git_review/mod.rs`

Add `fn reset(&mut self)` methods that:
- Clear cached data (file tree nodes, diff cache, preview data)
- Reset scroll positions
- Keep `split_ratio` and `focused_panel` (preserved across opens)
- Called when overlay is closed (InputMode returns to Normal) IF the overlay has been open for a long time or memory pressure is detected

For now, implement as a manual `reset()` method tested by unit tests. The App can optionally call it on close.

---

## Acceptance Criteria

1. **Merge conflicts detected**: Conflicted files show `FileStatus::Conflicted` with "C" label
2. **Submodules detected**: Submodule entries have `is_submodule: true` flag
3. **Empty repo handled**: Opening git review in a repo with no commits shows a message, not an error
4. **Symlinks shown**: Symlinks display with `→ target` suffix and correct icon
5. **Hidden files toggle**: `show_hidden` field on FileTree controls dotfile visibility
6. **Large dir guard**: Directories with >10K entries are truncated with a message
7. **Configurable shortcuts**: overlay shortcuts read from `KeysConfig.bindings`
8. **Home/End navigation**: Jump to first/last in both tree and file list
9. **Memory reset**: `reset()` clears cached data while preserving layout preferences
10. **All existing 1845 tests pass**: No regressions

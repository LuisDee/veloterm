# Track 35 Implementation Plan: Overlay Final Polish

## Phase 1: Git Review Edge Cases

### Tests to Write FIRST

**File: `src/git_review/status.rs`** (add to existing tests)

```
test file_status_conflicted_label
  - FileStatus::Conflicted.label() returns "C"

test file_status_conflicted_ne_modified
  - FileStatus::Conflicted != FileStatus::Modified

test status_entry_is_submodule_default_false
  - StatusEntry::from_path() has is_submodule = false by default

test status_entry_submodule_flag
  - StatusEntry with is_submodule = true can be constructed
```

**File: `src/git_review/mod.rs`** (add to existing tests)

```
test empty_repo_shows_message_not_error
  - Create a git repo with no commits (git init only)
  - open_from_cwd should set repo_path = Some, error = None
  - git_status should be Some with all empty sections
  - A special flag or message indicates "no commits yet"

test reset_clears_cached_data
  - Create state, set cached_diff, commit_message, selected, error
  - Call reset()
  - cached_diff = None, commit_message empty, selected = None, error = None
  - split_ratio and focused_panel preserved

test reset_preserves_layout
  - Set split_ratio = 0.3, focused_panel = Right
  - Call reset()
  - split_ratio still 0.3, focused_panel still Right
```

### Implementation Steps

1. **Add `FileStatus::Conflicted`** to `src/git_review/status.rs`:
   - Add variant with label "C"
   - In `from_git2_status()`, map `STATUS_CONFLICTED` flag

2. **Add `is_submodule` field** to `StatusEntry` in `src/git_review/status.rs`:
   - Default false in `from_path()`
   - Add a `with_submodule(mut self, is_sub: bool) -> Self` builder

3. **Handle empty repos** in `src/git_review/mod.rs`:
   - In `open_from_cwd()`, check if `repo.head()` fails with unborn branch
   - If so, create empty GitStatus and set a `no_commits: bool` flag
   - Do not set `self.error`

4. **Add `reset()` method** to `GitReviewState`:
   - Clear: cached_diff, commit_message, selected, error, discard_confirm, diff_scroll
   - Preserve: split_ratio, focused_panel, repo_path

5. **Run tests**

### Commit

```
feat(git-review): handle merge conflicts, submodules, empty repos, and add reset()
```

---

## Phase 2: File Browser Edge Cases

### Tests to Write FIRST

**File: `src/file_browser/tree.rs`** (add to existing tests)

```
test symlink_node_type_exists
  - NodeType::Symlink { target } can be constructed
  - It is not equal to NodeType::File or NodeType::Directory

test show_hidden_default_false
  - FileTree::new() has show_hidden = false

test show_hidden_includes_dotfiles
  - Create a dir with .hidden and visible.txt
  - With show_hidden = false: expand shows only visible.txt
  - With show_hidden = true: expand shows both

test large_dir_truncation
  - Create a dir with 11000 files (or mock the constant)
  - expand() should load at most MAX_DIR_ENTRIES entries
  - A truncation indicator is present

test nav_home_jumps_to_first
  - TreeNavAction::Home sets selected_visible_row to Some(0)

test nav_end_jumps_to_last
  - TreeNavAction::End sets selected_visible_row to Some(last_row)
```

**File: `src/file_browser/mod.rs`** (add to existing tests)

```
test reset_clears_tree_and_preview
  - Set file_tree, preview, visible_rows
  - Call reset()
  - file_tree = None, preview = None, visible_rows empty
  - split_ratio and focused_panel preserved

test reset_preserves_layout
  - Set split_ratio = 0.7, focused_panel = Right
  - Call reset()
  - Both preserved
```

### Implementation Steps

1. **Add `NodeType::Symlink`** to `src/file_browser/tree.rs`:
   - New variant: `Symlink { target: PathBuf }`
   - In `expand()`, use `entry.file_type()` to detect symlinks
   - Display name includes `→ target`

2. **Add `show_hidden` field** to `FileTree`:
   - Default `false`
   - In `expand()`, conditionally skip dotfiles based on flag
   - Add `set_show_hidden(&mut self, show: bool)` + clear children to re-expand

3. **Add large directory guard** to `expand()`:
   - Constant `MAX_DIR_ENTRIES: usize = 10_000`
   - If read_dir yields more, truncate and log warning

4. **Add `TreeNavAction::Home` and `TreeNavAction::End`**:
   - Home: set selected to 0
   - End: set selected to visible_rows.len() - 1

5. **Add `reset()` method** to `FileBrowserState`:
   - Clear: file_tree, visible_rows, view_state, breadcrumb, preview, preview_view
   - Preserve: split_ratio, focused_panel, syntax_set, theme_set

6. **Run tests**

### Commit

```
feat(file-browser): handle symlinks, hidden files toggle, large dirs, Home/End nav, reset()
```

---

## Phase 3: Configurable Shortcuts & Git Review Navigation

### Tests to Write FIRST

**File: `src/input/mod.rs`** (add to existing tests)

```
test overlay_cmd_custom_binding_file_browser
  - Create bindings map with "toggle_file_browser" = "ctrl+b"
  - match_overlay_command with Ctrl+B returns Some(ToggleFileBrowser)
  - match_overlay_command with Ctrl+E returns None (overridden)

test overlay_cmd_custom_binding_git_review
  - Create bindings map with "toggle_git_review" = "ctrl+r"
  - match_overlay_command with Ctrl+R returns Some(ToggleGitReview)

test overlay_cmd_default_when_no_custom
  - Empty bindings map -> Ctrl+E still works (default)

test overlay_cmd_invalid_binding_uses_default
  - bindings with "toggle_file_browser" = "invalid" -> falls back to Ctrl+E
```

**File: `src/git_review/mod.rs`** (add to existing tests)

```
test navigate_home_selects_first
  - With multiple entries, navigate_home() sets selected to first entry

test navigate_end_selects_last
  - With multiple entries, navigate_end() sets selected to last entry
```

### Implementation Steps

1. **Extend `match_overlay_command()`** in `src/input/mod.rs`:
   - Add `bindings: &HashMap<String, String>` parameter
   - Parse binding strings like "ctrl+e", "ctrl+shift+b"
   - Check custom bindings first, fall back to defaults
   - Add a helper `parse_keybinding(s: &str) -> Option<(Key, ModifiersState)>`

2. **Add `navigate_home()` and `navigate_end()`** to `GitReviewState`

3. **Update callers** of `match_overlay_command()` in `src/window.rs`:
   - Pass `self.config.keys.bindings` to the function

4. **Run full test suite**

### Commit

```
feat(overlay): configurable keyboard shortcuts and Home/End navigation
```

---

## Files Created/Modified Summary

| File | Changes |
|------|---------|
| `src/git_review/status.rs` | Add `Conflicted` variant, `is_submodule` field |
| `src/git_review/mod.rs` | Empty repo handling, reset(), navigate_home/end |
| `src/file_browser/tree.rs` | Symlink NodeType, show_hidden, large dir guard, Home/End nav |
| `src/file_browser/mod.rs` | reset() method |
| `src/input/mod.rs` | Configurable overlay shortcuts with binding parser |
| `src/window.rs` | Pass config bindings to match_overlay_command |

// View logic for the git review changed files panel.
// Provides data structures and helpers for rendering the file list.

use std::path::PathBuf;

use crate::git_review::status::{FileStatus, GitStatus, SectionState, StatusEntry};

/// Which section a file entry belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Staged,
    Changed,
    Untracked,
}

/// A flat list item for rendering — either a section header or a file entry.
#[derive(Debug, Clone)]
pub enum ListItem {
    SectionHeader {
        section: Section,
        label: String,
        count: usize,
        collapsed: bool,
    },
    FileEntry {
        section: Section,
        index: usize,
        path: PathBuf,
        display_name: String,
        display_dir: Option<String>,
        status_label: String,
        selected: bool,
    },
}

/// Actions available on a file entry depending on its section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileAction {
    Stage,
    Unstage,
    Discard,
}

/// Returns the available hover actions for a file in a given section.
pub fn actions_for_section(section: Section) -> Vec<FileAction> {
    match section {
        Section::Staged => vec![FileAction::Unstage],
        Section::Changed => vec![FileAction::Stage, FileAction::Discard],
        Section::Untracked => vec![FileAction::Stage],
    }
}

/// Whether the commit button should be enabled.
pub fn commit_button_enabled(staged_count: usize, message: &str) -> bool {
    staged_count > 0 && !message.trim().is_empty()
}

/// Whether batch "Stage All" should be enabled (there are unstaged or untracked files).
pub fn stage_all_enabled(status: &GitStatus) -> bool {
    !status.changed.is_empty() || !status.untracked.is_empty()
}

/// Whether batch "Unstage All" should be enabled (there are staged files).
pub fn unstage_all_enabled(status: &GitStatus) -> bool {
    !status.staged.is_empty()
}

/// Build a flat list of items for rendering from the current status and section state.
pub fn build_list_items(
    status: &GitStatus,
    section_state: &SectionState,
    selected: Option<(Section, usize)>,
) -> Vec<ListItem> {
    let mut items = Vec::new();

    // Staged section
    if !status.staged.is_empty() {
        items.push(ListItem::SectionHeader {
            section: Section::Staged,
            label: format!("Staged Changes ({})", status.staged.len()),
            count: status.staged.len(),
            collapsed: section_state.staged_collapsed,
        });
        if !section_state.staged_collapsed {
            for (i, entry) in status.staged.iter().enumerate() {
                items.push(entry_to_list_item(
                    entry,
                    Section::Staged,
                    i,
                    selected == Some((Section::Staged, i)),
                ));
            }
        }
    }

    // Changed section
    if !status.changed.is_empty() {
        items.push(ListItem::SectionHeader {
            section: Section::Changed,
            label: format!("Changes ({})", status.changed.len()),
            count: status.changed.len(),
            collapsed: section_state.changed_collapsed,
        });
        if !section_state.changed_collapsed {
            for (i, entry) in status.changed.iter().enumerate() {
                items.push(entry_to_list_item(
                    entry,
                    Section::Changed,
                    i,
                    selected == Some((Section::Changed, i)),
                ));
            }
        }
    }

    // Untracked section
    if !status.untracked.is_empty() {
        items.push(ListItem::SectionHeader {
            section: Section::Untracked,
            label: format!("Untracked ({})", status.untracked.len()),
            count: status.untracked.len(),
            collapsed: section_state.untracked_collapsed,
        });
        if !section_state.untracked_collapsed {
            for (i, entry) in status.untracked.iter().enumerate() {
                items.push(entry_to_list_item(
                    entry,
                    Section::Untracked,
                    i,
                    selected == Some((Section::Untracked, i)),
                ));
            }
        }
    }

    items
}

fn entry_to_list_item(
    entry: &StatusEntry,
    section: Section,
    index: usize,
    selected: bool,
) -> ListItem {
    ListItem::FileEntry {
        section,
        index,
        path: entry.path.clone(),
        display_name: entry.display_name.clone(),
        display_dir: entry.display_dir.clone(),
        status_label: entry.status.label().to_string(),
        selected,
    }
}

/// Navigate selection within the flat list (arrow keys).
/// Returns the new (Section, index) for the next selectable file entry,
/// or None if no file entries exist.
pub fn navigate_selection(
    status: &GitStatus,
    section_state: &SectionState,
    current: Option<(Section, usize)>,
    direction: i32, // -1 for up, +1 for down
) -> Option<(Section, usize)> {
    let items = build_list_items(status, section_state, current);
    let file_entries: Vec<(Section, usize)> = items
        .iter()
        .filter_map(|item| {
            if let ListItem::FileEntry { section, index, .. } = item {
                Some((*section, *index))
            } else {
                None
            }
        })
        .collect();

    if file_entries.is_empty() {
        return None;
    }

    match current {
        None => {
            if direction >= 0 {
                Some(file_entries[0])
            } else {
                Some(*file_entries.last().unwrap())
            }
        }
        Some(cur) => {
            let pos = file_entries.iter().position(|e| *e == cur);
            match pos {
                Some(idx) => {
                    let new_idx = if direction > 0 {
                        if idx + 1 < file_entries.len() {
                            idx + 1
                        } else {
                            idx
                        }
                    } else if idx > 0 {
                        idx - 1
                    } else {
                        0
                    };
                    Some(file_entries[new_idx])
                }
                None => Some(file_entries[0]),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git_review::status::FileStatus;
    use std::path::Path;

    fn make_status(
        staged: &[(&str, FileStatus)],
        changed: &[(&str, FileStatus)],
        untracked: &[&str],
    ) -> GitStatus {
        GitStatus {
            staged: staged
                .iter()
                .map(|(p, s)| StatusEntry::from_path(Path::new(p), s.clone()))
                .collect(),
            changed: changed
                .iter()
                .map(|(p, s)| StatusEntry::from_path(Path::new(p), s.clone()))
                .collect(),
            untracked: untracked
                .iter()
                .map(|p| StatusEntry::from_path(Path::new(p), FileStatus::Untracked))
                .collect(),
        }
    }

    // -- commit_button_enabled --

    #[test]
    fn commit_enabled_with_staged_and_message() {
        assert!(commit_button_enabled(3, "fix: bug"));
    }

    #[test]
    fn commit_disabled_no_staged() {
        assert!(!commit_button_enabled(0, "fix: bug"));
    }

    #[test]
    fn commit_disabled_empty_message() {
        assert!(!commit_button_enabled(3, ""));
    }

    #[test]
    fn commit_disabled_whitespace_message() {
        assert!(!commit_button_enabled(3, "   \n  "));
    }

    // -- stage_all / unstage_all enabled --

    #[test]
    fn stage_all_enabled_with_changes() {
        let status = make_status(&[], &[("a.rs", FileStatus::Modified)], &[]);
        assert!(stage_all_enabled(&status));
    }

    #[test]
    fn stage_all_enabled_with_untracked() {
        let status = make_status(&[], &[], &["new.rs"]);
        assert!(stage_all_enabled(&status));
    }

    #[test]
    fn stage_all_disabled_all_staged() {
        let status = make_status(&[("a.rs", FileStatus::Added)], &[], &[]);
        assert!(!stage_all_enabled(&status));
    }

    #[test]
    fn unstage_all_enabled_with_staged() {
        let status = make_status(&[("a.rs", FileStatus::Added)], &[], &[]);
        assert!(unstage_all_enabled(&status));
    }

    #[test]
    fn unstage_all_disabled_no_staged() {
        let status = make_status(&[], &[("a.rs", FileStatus::Modified)], &[]);
        assert!(!unstage_all_enabled(&status));
    }

    // -- actions_for_section --

    #[test]
    fn staged_actions() {
        let actions = actions_for_section(Section::Staged);
        assert_eq!(actions, vec![FileAction::Unstage]);
    }

    #[test]
    fn changed_actions() {
        let actions = actions_for_section(Section::Changed);
        assert_eq!(actions, vec![FileAction::Stage, FileAction::Discard]);
    }

    #[test]
    fn untracked_actions() {
        let actions = actions_for_section(Section::Untracked);
        assert_eq!(actions, vec![FileAction::Stage]);
    }

    // -- build_list_items --

    #[test]
    fn list_items_empty_status() {
        let status = make_status(&[], &[], &[]);
        let items = build_list_items(&status, &SectionState::default(), None);
        assert!(items.is_empty());
    }

    #[test]
    fn list_items_one_section() {
        let status = make_status(&[], &[], &["a.txt", "b.txt"]);
        let items = build_list_items(&status, &SectionState::default(), None);
        assert_eq!(items.len(), 3); // 1 header + 2 entries
        assert!(matches!(
            &items[0],
            ListItem::SectionHeader {
                section: Section::Untracked,
                count: 2,
                ..
            }
        ));
        assert!(matches!(
            &items[1],
            ListItem::FileEntry {
                section: Section::Untracked,
                index: 0,
                ..
            }
        ));
    }

    #[test]
    fn list_items_all_sections() {
        let status = make_status(
            &[("s.rs", FileStatus::Added)],
            &[("c.rs", FileStatus::Modified)],
            &["u.rs"],
        );
        let items = build_list_items(&status, &SectionState::default(), None);
        // 3 headers + 3 entries = 6
        assert_eq!(items.len(), 6);
    }

    #[test]
    fn list_items_collapsed_section() {
        let status = make_status(&[], &[], &["a.txt", "b.txt"]);
        let mut section_state = SectionState::default();
        section_state.toggle_untracked();
        let items = build_list_items(&status, &section_state, None);
        // Header only, entries hidden
        assert_eq!(items.len(), 1);
        assert!(matches!(
            &items[0],
            ListItem::SectionHeader {
                collapsed: true,
                ..
            }
        ));
    }

    #[test]
    fn list_items_selected_entry() {
        let status = make_status(&[], &[], &["a.txt"]);
        let items =
            build_list_items(&status, &SectionState::default(), Some((Section::Untracked, 0)));
        assert!(matches!(
            &items[1],
            ListItem::FileEntry {
                selected: true,
                ..
            }
        ));
    }

    #[test]
    fn list_items_header_label_format() {
        let status = make_status(&[("a.rs", FileStatus::Added)], &[], &[]);
        let items = build_list_items(&status, &SectionState::default(), None);
        if let ListItem::SectionHeader { label, .. } = &items[0] {
            assert_eq!(label, "Staged Changes (1)");
        } else {
            panic!("Expected section header");
        }
    }

    #[test]
    fn list_items_changed_label() {
        let status = make_status(
            &[],
            &[
                ("a.rs", FileStatus::Modified),
                ("b.rs", FileStatus::Deleted),
            ],
            &[],
        );
        let items = build_list_items(&status, &SectionState::default(), None);
        if let ListItem::SectionHeader { label, .. } = &items[0] {
            assert_eq!(label, "Changes (2)");
        } else {
            panic!("Expected section header");
        }
    }

    // -- navigate_selection --

    #[test]
    fn navigate_no_entries() {
        let status = make_status(&[], &[], &[]);
        let result = navigate_selection(&status, &SectionState::default(), None, 1);
        assert_eq!(result, None);
    }

    #[test]
    fn navigate_from_none_down() {
        let status = make_status(&[], &[], &["a.txt", "b.txt"]);
        let result = navigate_selection(&status, &SectionState::default(), None, 1);
        assert_eq!(result, Some((Section::Untracked, 0)));
    }

    #[test]
    fn navigate_from_none_up() {
        let status = make_status(&[], &[], &["a.txt", "b.txt"]);
        let result = navigate_selection(&status, &SectionState::default(), None, -1);
        assert_eq!(result, Some((Section::Untracked, 1)));
    }

    #[test]
    fn navigate_down_within_section() {
        let status = make_status(&[], &[], &["a.txt", "b.txt"]);
        let result = navigate_selection(
            &status,
            &SectionState::default(),
            Some((Section::Untracked, 0)),
            1,
        );
        assert_eq!(result, Some((Section::Untracked, 1)));
    }

    #[test]
    fn navigate_up_within_section() {
        let status = make_status(&[], &[], &["a.txt", "b.txt"]);
        let result = navigate_selection(
            &status,
            &SectionState::default(),
            Some((Section::Untracked, 1)),
            -1,
        );
        assert_eq!(result, Some((Section::Untracked, 0)));
    }

    #[test]
    fn navigate_across_sections() {
        let status = make_status(
            &[("s.rs", FileStatus::Added)],
            &[("c.rs", FileStatus::Modified)],
            &[],
        );
        // Down from staged to changed
        let result = navigate_selection(
            &status,
            &SectionState::default(),
            Some((Section::Staged, 0)),
            1,
        );
        assert_eq!(result, Some((Section::Changed, 0)));
    }

    #[test]
    fn navigate_at_bottom_stays() {
        let status = make_status(&[], &[], &["a.txt"]);
        let result = navigate_selection(
            &status,
            &SectionState::default(),
            Some((Section::Untracked, 0)),
            1,
        );
        assert_eq!(result, Some((Section::Untracked, 0)));
    }

    #[test]
    fn navigate_at_top_stays() {
        let status = make_status(&[], &[], &["a.txt"]);
        let result = navigate_selection(
            &status,
            &SectionState::default(),
            Some((Section::Untracked, 0)),
            -1,
        );
        assert_eq!(result, Some((Section::Untracked, 0)));
    }

    #[test]
    fn navigate_skips_collapsed() {
        let status = make_status(
            &[("s.rs", FileStatus::Added)],
            &[],
            &["u.rs"],
        );
        let mut section_state = SectionState::default();
        section_state.toggle_staged(); // collapse staged
        // Navigate down from none — should land on untracked since staged is collapsed
        let result = navigate_selection(&status, &section_state, None, 1);
        assert_eq!(result, Some((Section::Untracked, 0)));
    }

    #[test]
    fn file_entry_status_label() {
        let status = make_status(
            &[("a.rs", FileStatus::Added)],
            &[("b.rs", FileStatus::Modified)],
            &["c.rs"],
        );
        let items = build_list_items(&status, &SectionState::default(), None);
        // Check that status_label is correct
        if let ListItem::FileEntry { status_label, .. } = &items[1] {
            assert_eq!(status_label, "A");
        }
        if let ListItem::FileEntry { status_label, .. } = &items[3] {
            assert_eq!(status_label, "M");
        }
        if let ListItem::FileEntry { status_label, .. } = &items[5] {
            assert_eq!(status_label, "?");
        }
    }
}

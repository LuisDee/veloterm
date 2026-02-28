// Diff computation and line alignment for the git review diff view.

use git2::{DiffFormat, DiffOptions, Repository};
use std::path::Path;

/// A single line in the diff view.
#[derive(Debug, Clone)]
pub struct DiffLine {
    pub content: String,
    pub line_number: Option<usize>,
    pub change_type: ChangeType,
}

/// Type of change for a diff line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    Context,
    Added,
    Deleted,
    Modified,
}

/// Aligned pair of lines for side-by-side display.
#[derive(Debug, Clone)]
pub struct AlignedRow {
    pub left: Option<DiffLine>,
    pub right: Option<DiffLine>,
}

/// A diff hunk with header and aligned rows.
#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub header: String,
    pub old_start: usize,
    pub new_start: usize,
    pub rows: Vec<AlignedRow>,
}

/// Complete diff for a file.
#[derive(Debug, Clone)]
pub struct FileDiff {
    pub path: String,
    pub hunks: Vec<DiffHunk>,
    pub diff_type: DiffType,
}

/// Type of file change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffType {
    Modified,
    Added,
    Deleted,
    Renamed { from: String },
    Binary,
}

/// Raw hunk data collected from git2 callbacks.
#[derive(Debug, Clone)]
struct RawHunk {
    header: String,
    old_start: usize,
    new_start: usize,
    old_lines: Vec<String>,
    new_lines: Vec<String>,
    /// Sequence of operations: 'C' context, 'D' delete, 'A' add
    ops: Vec<char>,
}

/// Compute the diff for a file using git2.
///
/// `staged`: if true, diff HEAD vs index (staged changes); if false, diff index vs workdir.
/// For untracked files, reads the file and returns all lines as Added.
pub fn compute_diff(
    repo: &Repository,
    path: &Path,
    staged: bool,
    untracked: bool,
) -> Result<FileDiff, git2::Error> {
    let path_str = path.to_string_lossy().to_string();

    if untracked {
        return compute_untracked_diff(repo, path, &path_str);
    }

    let mut diff_opts = DiffOptions::new();
    diff_opts.pathspec(&path_str);

    let diff = if staged {
        let head = repo.head()?.peel_to_tree()?;
        let index = repo.index()?;
        repo.diff_tree_to_index(Some(&head), Some(&index), Some(&mut diff_opts))?
    } else {
        repo.diff_index_to_workdir(None, Some(&mut diff_opts))?
    };

    // Check if the file is binary or detect diff type
    let mut diff_type = DiffType::Modified;
    let mut is_binary = false;

    diff.foreach(
        &mut |delta, _| {
            match delta.status() {
                git2::Delta::Added => diff_type = DiffType::Added,
                git2::Delta::Deleted => diff_type = DiffType::Deleted,
                git2::Delta::Renamed => {
                    let from = delta
                        .old_file()
                        .path()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default();
                    diff_type = DiffType::Renamed { from };
                }
                _ => {}
            }
            if delta.old_file().is_binary() || delta.new_file().is_binary() {
                is_binary = true;
            }
            true
        },
        None,
        None,
        None,
    )?;

    if is_binary {
        return Ok(FileDiff {
            path: path_str,
            hunks: vec![],
            diff_type: DiffType::Binary,
        });
    }

    // Collect hunks with line data using print() which uses a single callback
    let mut raw_hunks: Vec<RawHunk> = Vec::new();

    diff.print(DiffFormat::Patch, |_delta, hunk, line| {
        match line.origin() {
            'H' => {
                // Hunk header line
                if let Some(hunk) = hunk {
                    let header = String::from_utf8_lossy(hunk.header()).trim().to_string();
                    raw_hunks.push(RawHunk {
                        header,
                        old_start: hunk.old_start() as usize,
                        new_start: hunk.new_start() as usize,
                        old_lines: Vec::new(),
                        new_lines: Vec::new(),
                        ops: Vec::new(),
                    });
                }
            }
            ' ' => {
                if let Some(current_hunk) = raw_hunks.last_mut() {
                    let content = String::from_utf8_lossy(line.content())
                        .trim_end_matches('\n')
                        .to_string();
                    current_hunk.old_lines.push(content.clone());
                    current_hunk.new_lines.push(content);
                    current_hunk.ops.push('C');
                }
            }
            '-' => {
                if let Some(current_hunk) = raw_hunks.last_mut() {
                    let content = String::from_utf8_lossy(line.content())
                        .trim_end_matches('\n')
                        .to_string();
                    current_hunk.old_lines.push(content);
                    current_hunk.ops.push('D');
                }
            }
            '+' => {
                if let Some(current_hunk) = raw_hunks.last_mut() {
                    let content = String::from_utf8_lossy(line.content())
                        .trim_end_matches('\n')
                        .to_string();
                    current_hunk.new_lines.push(content);
                    current_hunk.ops.push('A');
                }
            }
            _ => {}
        }
        true
    })?;

    // Convert raw hunks to aligned hunks
    let hunks = raw_hunks
        .into_iter()
        .map(|raw| {
            let rows = align_raw_hunk(&raw);
            DiffHunk {
                header: raw.header,
                old_start: raw.old_start,
                new_start: raw.new_start,
                rows,
            }
        })
        .collect();

    Ok(FileDiff {
        path: path_str,
        hunks,
        diff_type,
    })
}

/// Compute diff for an untracked file (all lines as added).
fn compute_untracked_diff(
    repo: &Repository,
    path: &Path,
    path_str: &str,
) -> Result<FileDiff, git2::Error> {
    let workdir = repo.workdir().ok_or_else(|| {
        git2::Error::from_str("Repository has no working directory")
    })?;
    let full_path = workdir.join(path);

    let content = match std::fs::read_to_string(&full_path) {
        Ok(c) => c,
        Err(_) => {
            // Binary or unreadable — treat as binary
            return Ok(FileDiff {
                path: path_str.to_string(),
                hunks: vec![],
                diff_type: DiffType::Binary,
            });
        }
    };

    let lines: Vec<&str> = content.lines().collect();
    let rows = lines
        .iter()
        .enumerate()
        .map(|(i, line)| AlignedRow {
            left: None,
            right: Some(DiffLine {
                content: line.to_string(),
                line_number: Some(i + 1),
                change_type: ChangeType::Added,
            }),
        })
        .collect();

    Ok(FileDiff {
        path: path_str.to_string(),
        hunks: vec![DiffHunk {
            header: format!("@@ -0,0 +1,{} @@", lines.len()),
            old_start: 0,
            new_start: 1,
            rows,
        }],
        diff_type: DiffType::Added,
    })
}

/// Align a raw hunk into side-by-side rows.
///
/// The ops sequence tells us the order of context (C), delete (D), add (A) lines.
/// Adjacent D+A sequences are paired as Modified on the same row.
fn align_raw_hunk(raw: &RawHunk) -> Vec<AlignedRow> {
    let mut rows = Vec::new();
    let mut old_idx = 0usize;
    let mut new_idx = 0usize;
    let mut old_line_num = raw.old_start;
    let mut new_line_num = raw.new_start;

    let ops = &raw.ops;
    let mut i = 0;
    while i < ops.len() {
        match ops[i] {
            'C' => {
                // Context line — both sides
                rows.push(AlignedRow {
                    left: Some(DiffLine {
                        content: raw.old_lines[old_idx].clone(),
                        line_number: Some(old_line_num),
                        change_type: ChangeType::Context,
                    }),
                    right: Some(DiffLine {
                        content: raw.new_lines[new_idx].clone(),
                        line_number: Some(new_line_num),
                        change_type: ChangeType::Context,
                    }),
                });
                old_idx += 1;
                new_idx += 1;
                old_line_num += 1;
                new_line_num += 1;
                i += 1;
            }
            'D' => {
                // Count consecutive deletes
                let del_start = i;
                while i < ops.len() && ops[i] == 'D' {
                    i += 1;
                }
                let del_count = i - del_start;

                // Count consecutive adds immediately after
                let add_start = i;
                while i < ops.len() && ops[i] == 'A' {
                    i += 1;
                }
                let add_count = i - add_start;

                // Pair them up
                let max = del_count.max(add_count);
                for j in 0..max {
                    let left = if j < del_count {
                        let line = DiffLine {
                            content: raw.old_lines[old_idx].clone(),
                            line_number: Some(old_line_num),
                            change_type: if j < add_count {
                                ChangeType::Modified
                            } else {
                                ChangeType::Deleted
                            },
                        };
                        old_idx += 1;
                        old_line_num += 1;
                        Some(line)
                    } else {
                        None
                    };

                    let right = if j < add_count {
                        let line = DiffLine {
                            content: raw.new_lines[new_idx].clone(),
                            line_number: Some(new_line_num),
                            change_type: if j < del_count {
                                ChangeType::Modified
                            } else {
                                ChangeType::Added
                            },
                        };
                        new_idx += 1;
                        new_line_num += 1;
                        Some(line)
                    } else {
                        None
                    };

                    rows.push(AlignedRow { left, right });
                }
            }
            'A' => {
                // Pure add (no preceding delete)
                rows.push(AlignedRow {
                    left: None,
                    right: Some(DiffLine {
                        content: raw.new_lines[new_idx].clone(),
                        line_number: Some(new_line_num),
                        change_type: ChangeType::Added,
                    }),
                });
                new_idx += 1;
                new_line_num += 1;
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    rows
}

/// Build aligned rows from explicit old/new lines and operations.
/// This is a public test-friendly version of the alignment algorithm.
pub fn align_lines(
    old_lines: &[String],
    new_lines: &[String],
    ops: &[char],
) -> Vec<AlignedRow> {
    let raw = RawHunk {
        header: String::new(),
        old_start: 1,
        new_start: 1,
        old_lines: old_lines.to_vec(),
        new_lines: new_lines.to_vec(),
        ops: ops.to_vec(),
    };
    align_raw_hunk(&raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn setup_test_repo() -> (tempfile::TempDir, Repository) {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();
        (dir, repo)
    }

    fn make_initial_commit(dir: &Path, repo: &Repository) {
        std::fs::write(dir.join("initial.txt"), "initial").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("initial.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();
    }

    // -- ChangeType equality tests --

    #[test]
    fn change_type_equality() {
        assert_eq!(ChangeType::Context, ChangeType::Context);
        assert_eq!(ChangeType::Added, ChangeType::Added);
        assert_eq!(ChangeType::Deleted, ChangeType::Deleted);
        assert_eq!(ChangeType::Modified, ChangeType::Modified);
        assert_ne!(ChangeType::Added, ChangeType::Deleted);
    }

    // -- DiffType equality tests --

    #[test]
    fn diff_type_equality() {
        assert_eq!(DiffType::Modified, DiffType::Modified);
        assert_eq!(DiffType::Added, DiffType::Added);
        assert_eq!(DiffType::Deleted, DiffType::Deleted);
        assert_eq!(DiffType::Binary, DiffType::Binary);
        assert_eq!(
            DiffType::Renamed { from: "a.rs".into() },
            DiffType::Renamed { from: "a.rs".into() }
        );
        assert_ne!(DiffType::Added, DiffType::Deleted);
    }

    // -- align_lines tests --

    #[test]
    fn align_pure_context() {
        let old = vec!["line1".into(), "line2".into(), "line3".into()];
        let new = vec!["line1".into(), "line2".into(), "line3".into()];
        let ops = vec!['C', 'C', 'C'];
        let rows = align_lines(&old, &new, &ops);
        assert_eq!(rows.len(), 3);
        for row in &rows {
            assert!(row.left.is_some());
            assert!(row.right.is_some());
            assert_eq!(row.left.as_ref().unwrap().change_type, ChangeType::Context);
            assert_eq!(row.right.as_ref().unwrap().change_type, ChangeType::Context);
        }
        // Check line numbers
        assert_eq!(rows[0].left.as_ref().unwrap().line_number, Some(1));
        assert_eq!(rows[2].right.as_ref().unwrap().line_number, Some(3));
    }

    #[test]
    fn align_pure_additions() {
        let old: Vec<String> = vec![];
        let new = vec!["added1".into(), "added2".into()];
        let ops = vec!['A', 'A'];
        let rows = align_lines(&old, &new, &ops);
        assert_eq!(rows.len(), 2);
        for row in &rows {
            assert!(row.left.is_none());
            assert!(row.right.is_some());
            assert_eq!(row.right.as_ref().unwrap().change_type, ChangeType::Added);
        }
        assert_eq!(rows[0].right.as_ref().unwrap().line_number, Some(1));
        assert_eq!(rows[1].right.as_ref().unwrap().line_number, Some(2));
    }

    #[test]
    fn align_pure_deletions() {
        let old = vec!["del1".into(), "del2".into()];
        let new: Vec<String> = vec![];
        let ops = vec!['D', 'D'];
        let rows = align_lines(&old, &new, &ops);
        assert_eq!(rows.len(), 2);
        for row in &rows {
            assert!(row.left.is_some());
            assert!(row.right.is_none());
            assert_eq!(row.left.as_ref().unwrap().change_type, ChangeType::Deleted);
        }
        assert_eq!(rows[0].left.as_ref().unwrap().line_number, Some(1));
        assert_eq!(rows[1].left.as_ref().unwrap().line_number, Some(2));
    }

    #[test]
    fn align_modification_pair() {
        // One delete followed by one add = modification pair
        let old = vec!["old_line".into()];
        let new = vec!["new_line".into()];
        let ops = vec!['D', 'A'];
        let rows = align_lines(&old, &new, &ops);
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert!(row.left.is_some());
        assert!(row.right.is_some());
        assert_eq!(row.left.as_ref().unwrap().change_type, ChangeType::Modified);
        assert_eq!(row.right.as_ref().unwrap().change_type, ChangeType::Modified);
        assert_eq!(row.left.as_ref().unwrap().content, "old_line");
        assert_eq!(row.right.as_ref().unwrap().content, "new_line");
    }

    #[test]
    fn align_modification_with_extra_adds() {
        // 1 delete + 3 adds: first pair = modified, remaining 2 = added
        let old = vec!["old".into()];
        let new = vec!["new1".into(), "new2".into(), "new3".into()];
        let ops = vec!['D', 'A', 'A', 'A'];
        let rows = align_lines(&old, &new, &ops);
        assert_eq!(rows.len(), 3);
        // First row: modified pair
        assert_eq!(rows[0].left.as_ref().unwrap().change_type, ChangeType::Modified);
        assert_eq!(rows[0].right.as_ref().unwrap().change_type, ChangeType::Modified);
        // Second row: add only
        assert!(rows[1].left.is_none());
        assert_eq!(rows[1].right.as_ref().unwrap().change_type, ChangeType::Added);
        // Third row: add only
        assert!(rows[2].left.is_none());
        assert_eq!(rows[2].right.as_ref().unwrap().change_type, ChangeType::Added);
    }

    #[test]
    fn align_modification_with_extra_deletes() {
        // 3 deletes + 1 add: first pair = modified, remaining 2 = deleted
        let old = vec!["old1".into(), "old2".into(), "old3".into()];
        let new = vec!["new".into()];
        let ops = vec!['D', 'D', 'D', 'A'];
        let rows = align_lines(&old, &new, &ops);
        assert_eq!(rows.len(), 3);
        // First row: modified pair
        assert_eq!(rows[0].left.as_ref().unwrap().change_type, ChangeType::Modified);
        assert_eq!(rows[0].right.as_ref().unwrap().change_type, ChangeType::Modified);
        // Second row: delete only
        assert_eq!(rows[1].left.as_ref().unwrap().change_type, ChangeType::Deleted);
        assert!(rows[1].right.is_none());
        // Third row: delete only
        assert_eq!(rows[2].left.as_ref().unwrap().change_type, ChangeType::Deleted);
        assert!(rows[2].right.is_none());
    }

    #[test]
    fn align_mixed_context_and_changes() {
        // Context, then delete+add (modified), then context
        let old = vec!["ctx1".into(), "old".into(), "ctx2".into()];
        let new = vec!["ctx1".into(), "new".into(), "ctx2".into()];
        let ops = vec!['C', 'D', 'A', 'C'];
        let rows = align_lines(&old, &new, &ops);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].left.as_ref().unwrap().change_type, ChangeType::Context);
        assert_eq!(rows[1].left.as_ref().unwrap().change_type, ChangeType::Modified);
        assert_eq!(rows[1].right.as_ref().unwrap().change_type, ChangeType::Modified);
        assert_eq!(rows[2].left.as_ref().unwrap().change_type, ChangeType::Context);
    }

    #[test]
    fn align_context_add_context() {
        // Context, then pure add, then context
        let old = vec!["ctx1".into(), "ctx2".into()];
        let new = vec!["ctx1".into(), "added".into(), "ctx2".into()];
        let ops = vec!['C', 'A', 'C'];
        let rows = align_lines(&old, &new, &ops);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].left.as_ref().unwrap().change_type, ChangeType::Context);
        assert!(rows[1].left.is_none());
        assert_eq!(rows[1].right.as_ref().unwrap().change_type, ChangeType::Added);
        assert_eq!(rows[2].left.as_ref().unwrap().change_type, ChangeType::Context);
    }

    #[test]
    fn align_context_delete_context() {
        // Context, then pure delete, then context
        let old = vec!["ctx1".into(), "deleted".into(), "ctx2".into()];
        let new = vec!["ctx1".into(), "ctx2".into()];
        let ops = vec!['C', 'D', 'C'];
        let rows = align_lines(&old, &new, &ops);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].left.as_ref().unwrap().change_type, ChangeType::Context);
        assert_eq!(rows[1].left.as_ref().unwrap().change_type, ChangeType::Deleted);
        assert!(rows[1].right.is_none());
        assert_eq!(rows[2].left.as_ref().unwrap().change_type, ChangeType::Context);
    }

    #[test]
    fn align_line_numbers_with_adds() {
        // Context + add + context: old line nums skip add, new line nums continuous
        let old = vec!["line1".into(), "line2".into()];
        let new = vec!["line1".into(), "inserted".into(), "line2".into()];
        let ops = vec!['C', 'A', 'C'];
        let rows = align_lines(&old, &new, &ops);
        assert_eq!(rows[0].left.as_ref().unwrap().line_number, Some(1));
        assert_eq!(rows[0].right.as_ref().unwrap().line_number, Some(1));
        // Add row: no left line number
        assert!(rows[1].left.is_none());
        assert_eq!(rows[1].right.as_ref().unwrap().line_number, Some(2));
        // Second context
        assert_eq!(rows[2].left.as_ref().unwrap().line_number, Some(2));
        assert_eq!(rows[2].right.as_ref().unwrap().line_number, Some(3));
    }

    #[test]
    fn align_empty_ops() {
        let rows = align_lines(&[], &[], &[]);
        assert!(rows.is_empty());
    }

    // -- compute_diff tests with real git repos --

    #[test]
    fn diff_unstaged_modification() {
        let (dir, repo) = setup_test_repo();
        make_initial_commit(dir.path(), &repo);

        // Modify the file
        std::fs::write(dir.path().join("initial.txt"), "modified").unwrap();

        let diff = compute_diff(&repo, Path::new("initial.txt"), false, false).unwrap();
        assert_eq!(diff.path, "initial.txt");
        assert_eq!(diff.diff_type, DiffType::Modified);
        assert!(!diff.hunks.is_empty());

        // Should have at least one row
        let total_rows: usize = diff.hunks.iter().map(|h| h.rows.len()).sum();
        assert!(total_rows > 0);
    }

    #[test]
    fn diff_staged_modification() {
        let (dir, repo) = setup_test_repo();
        make_initial_commit(dir.path(), &repo);

        // Modify and stage
        std::fs::write(dir.path().join("initial.txt"), "modified content").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("initial.txt")).unwrap();
        index.write().unwrap();

        let diff = compute_diff(&repo, Path::new("initial.txt"), true, false).unwrap();
        assert_eq!(diff.diff_type, DiffType::Modified);
        assert!(!diff.hunks.is_empty());
    }

    #[test]
    fn diff_staged_new_file() {
        let (dir, repo) = setup_test_repo();
        make_initial_commit(dir.path(), &repo);

        std::fs::write(dir.path().join("new.txt"), "new content\nline2\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("new.txt")).unwrap();
        index.write().unwrap();

        let diff = compute_diff(&repo, Path::new("new.txt"), true, false).unwrap();
        assert_eq!(diff.diff_type, DiffType::Added);
    }

    #[test]
    fn diff_staged_deleted_file() {
        let (dir, repo) = setup_test_repo();
        make_initial_commit(dir.path(), &repo);

        // Stage deletion
        std::fs::remove_file(dir.path().join("initial.txt")).unwrap();
        let mut index = repo.index().unwrap();
        index.remove_path(Path::new("initial.txt")).unwrap();
        index.write().unwrap();

        let diff = compute_diff(&repo, Path::new("initial.txt"), true, false).unwrap();
        assert_eq!(diff.diff_type, DiffType::Deleted);
    }

    #[test]
    fn diff_untracked_file() {
        let (dir, repo) = setup_test_repo();

        std::fs::write(dir.path().join("untracked.txt"), "line1\nline2\nline3").unwrap();

        let diff = compute_diff(&repo, Path::new("untracked.txt"), false, true).unwrap();
        assert_eq!(diff.diff_type, DiffType::Added);
        assert_eq!(diff.hunks.len(), 1);
        assert_eq!(diff.hunks[0].rows.len(), 3);
        // All rows should be add-only
        for row in &diff.hunks[0].rows {
            assert!(row.left.is_none());
            assert_eq!(row.right.as_ref().unwrap().change_type, ChangeType::Added);
        }
    }

    #[test]
    fn diff_untracked_file_line_numbers() {
        let (dir, repo) = setup_test_repo();

        std::fs::write(dir.path().join("new.txt"), "a\nb\nc").unwrap();

        let diff = compute_diff(&repo, Path::new("new.txt"), false, true).unwrap();
        let rows = &diff.hunks[0].rows;
        assert_eq!(rows[0].right.as_ref().unwrap().line_number, Some(1));
        assert_eq!(rows[1].right.as_ref().unwrap().line_number, Some(2));
        assert_eq!(rows[2].right.as_ref().unwrap().line_number, Some(3));
    }

    #[test]
    fn diff_modification_has_correct_content() {
        let (dir, repo) = setup_test_repo();

        // Create a file with known content
        std::fs::write(dir.path().join("test.txt"), "line1\nline2\nline3\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "add test.txt", &tree, &[])
            .unwrap();

        // Modify line2 to line2_modified
        std::fs::write(dir.path().join("test.txt"), "line1\nline2_modified\nline3\n").unwrap();

        let diff = compute_diff(&repo, Path::new("test.txt"), false, false).unwrap();
        assert_eq!(diff.diff_type, DiffType::Modified);

        // Find the modified row
        let modified_rows: Vec<&AlignedRow> = diff
            .hunks
            .iter()
            .flat_map(|h| h.rows.iter())
            .filter(|r| {
                r.left
                    .as_ref()
                    .map(|l| l.change_type == ChangeType::Modified)
                    .unwrap_or(false)
            })
            .collect();

        assert!(!modified_rows.is_empty());
        let modified = &modified_rows[0];
        assert_eq!(modified.left.as_ref().unwrap().content, "line2");
        assert_eq!(modified.right.as_ref().unwrap().content, "line2_modified");
    }

    #[test]
    fn diff_hunk_header_format() {
        let (dir, repo) = setup_test_repo();

        std::fs::write(dir.path().join("file.txt"), "a\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("file.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "commit", &tree, &[])
            .unwrap();

        std::fs::write(dir.path().join("file.txt"), "b\n").unwrap();

        let diff = compute_diff(&repo, Path::new("file.txt"), false, false).unwrap();
        assert!(!diff.hunks.is_empty());
        // Hunk header should start with @@
        assert!(diff.hunks[0].header.starts_with("@@"));
    }

    #[test]
    fn diff_empty_file() {
        let (dir, repo) = setup_test_repo();

        std::fs::write(dir.path().join("empty.txt"), "").unwrap();

        let diff = compute_diff(&repo, Path::new("empty.txt"), false, true).unwrap();
        assert_eq!(diff.diff_type, DiffType::Added);
        assert_eq!(diff.hunks.len(), 1);
        assert_eq!(diff.hunks[0].rows.len(), 0);
    }

    #[test]
    fn diff_multiline_modification() {
        let (dir, repo) = setup_test_repo();

        // Create a file with 5 lines
        std::fs::write(
            dir.path().join("multi.txt"),
            "line1\nline2\nline3\nline4\nline5\n",
        )
        .unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("multi.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "commit", &tree, &[])
            .unwrap();

        // Modify line3, add a new line after line4
        std::fs::write(
            dir.path().join("multi.txt"),
            "line1\nline2\nline3_changed\nline4\nnew_line\nline5\n",
        )
        .unwrap();

        let diff = compute_diff(&repo, Path::new("multi.txt"), false, false).unwrap();
        let total_rows: usize = diff.hunks.iter().map(|h| h.rows.len()).sum();
        assert!(total_rows > 0);
    }
}

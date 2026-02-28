// Interactive hunk staging/unstaging via git2.
// Allows staging or unstaging individual hunks from a multi-hunk diff.

use git2::{ApplyLocation, Diff, DiffFormat, DiffOptions, Repository};
use std::path::Path;

/// Stage a single hunk from an unstaged file.
///
/// Computes the diff for the file (index vs workdir), extracts the specified hunk
/// as a standalone patch, and applies it to the index.
pub fn stage_hunk(
    repo: &Repository,
    file_path: &Path,
    hunk_index: usize,
) -> Result<(), git2::Error> {
    let path_str = file_path.to_string_lossy().to_string();

    // Get the unstaged diff (index vs workdir)
    let mut diff_opts = DiffOptions::new();
    diff_opts.pathspec(&path_str);
    let diff = repo.diff_index_to_workdir(None, Some(&mut diff_opts))?;

    // Extract the patch text for just the target hunk
    let patch_text = extract_hunk_patch(&diff, hunk_index, false)?;

    // Apply the patch to the index
    let patch_diff = Diff::from_buffer(patch_text.as_bytes())?;
    repo.apply(&patch_diff, ApplyLocation::Index, None)?;

    Ok(())
}

/// Unstage a single hunk from a staged file.
///
/// Computes the diff for the file (HEAD vs index), extracts the specified hunk,
/// reverses it, and applies it to the index.
pub fn unstage_hunk(
    repo: &Repository,
    file_path: &Path,
    hunk_index: usize,
) -> Result<(), git2::Error> {
    let path_str = file_path.to_string_lossy().to_string();

    // Get the staged diff (HEAD vs index)
    let head = repo.head()?.peel_to_tree()?;
    let mut diff_opts = DiffOptions::new();
    diff_opts.pathspec(&path_str);
    let index = repo.index()?;
    let diff = repo.diff_tree_to_index(Some(&head), Some(&index), Some(&mut diff_opts))?;

    // Extract the patch text for just the target hunk, reversed
    let patch_text = extract_hunk_patch(&diff, hunk_index, true)?;

    // Apply the reversed patch to the index
    let patch_diff = Diff::from_buffer(patch_text.as_bytes())?;
    repo.apply(&patch_diff, ApplyLocation::Index, None)?;

    Ok(())
}

/// Extract a single hunk from a diff as a complete patch string.
///
/// If `reverse` is true, swaps +/- lines and adjusts the header for unstaging.
fn extract_hunk_patch(
    diff: &Diff<'_>,
    target_hunk: usize,
    reverse: bool,
) -> Result<String, git2::Error> {
    // Collect the full patch using proper origin handling:
    // 'F' = file header line (content IS the line)
    // 'H' = hunk header line (content IS the @@ line)
    // ' ', '+', '-' = context/add/del (origin is the prefix)
    let mut file_header = String::new();
    let mut hunks: Vec<String> = Vec::new();
    let mut current_hunk = String::new();

    diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
        if let Ok(content) = std::str::from_utf8(line.content()) {
            match line.origin() {
                'F' => {
                    // File header lines (diff --git, index, --- a/, +++ b/)
                    file_header.push_str(content);
                }
                'H' => {
                    // Hunk header — start of a new hunk
                    if !current_hunk.is_empty() {
                        hunks.push(current_hunk.clone());
                        current_hunk.clear();
                    }
                    current_hunk.push_str(content);
                }
                ' ' | '+' | '-' => {
                    current_hunk.push(line.origin());
                    current_hunk.push_str(content);
                }
                _ => {}
            }
        }
        true
    })?;

    if !current_hunk.is_empty() {
        hunks.push(current_hunk);
    }

    if target_hunk >= hunks.len() {
        return Err(git2::Error::from_str(&format!(
            "Hunk index {} out of range (file has {} hunks)",
            target_hunk,
            hunks.len()
        )));
    }

    let hunk_text = &hunks[target_hunk];

    if reverse {
        let reversed_header = reverse_file_header(&file_header);
        let reversed_hunk = reverse_hunk(hunk_text);
        Ok(format!("{}{}", reversed_header, reversed_hunk))
    } else {
        Ok(format!("{}{}", file_header, hunk_text))
    }
}

/// For reverse-apply, the file header stays the same — only the hunk
/// body and header are reversed. git2::apply interprets a/ as source
/// and b/ as destination regardless.
fn reverse_file_header(header: &str) -> String {
    header.to_string()
}

/// Reverse a hunk: swap + and - lines, swap old/new in @@ header.
fn reverse_hunk(hunk: &str) -> String {
    let mut result = String::new();
    for line in hunk.lines() {
        if line.starts_with("@@") {
            result.push_str(&reverse_hunk_header(line));
        } else if line.starts_with('+') {
            result.push('-');
            result.push_str(&line[1..]);
        } else if line.starts_with('-') {
            result.push('+');
            result.push_str(&line[1..]);
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }
    result
}

/// Reverse the @@ header: swap old/new ranges.
/// e.g., "@@ -10,3 +12,5 @@" becomes "@@ -12,5 +10,3 @@"
fn reverse_hunk_header(header: &str) -> String {
    // Parse: @@ -old_start,old_count +new_start,new_count @@ [context]
    // Find the range between the first @@ and the second @@
    let trimmed = header.trim();
    if !trimmed.starts_with("@@") {
        return header.to_string();
    }

    // Find the closing @@
    let after_first = &trimmed[2..].trim_start();
    let closing_idx = match after_first.find("@@") {
        Some(idx) => idx,
        None => return header.to_string(),
    };

    let range_part = after_first[..closing_idx].trim();
    let after_closing = after_first[closing_idx + 2..].to_string();

    // Parse range_part: "-old_start,old_count +new_start,new_count"
    let parts: Vec<&str> = range_part.split_whitespace().collect();
    if parts.len() < 2 {
        return header.to_string();
    }

    let old_range = parts[0]; // -old_start,old_count
    let new_range = parts[1]; // +new_start,new_count

    // Swap: old becomes new and vice versa
    let reversed_old = format!("-{}", new_range.trim_start_matches('+'));
    let reversed_new = format!("+{}", old_range.trim_start_matches('-'));

    if after_closing.trim().is_empty() {
        format!("@@ {} {} @@", reversed_old, reversed_new)
    } else {
        format!("@@ {} {} @@{}", reversed_old, reversed_new, after_closing)
    }
}

/// Count the number of hunks in a diff for a given file.
pub fn count_hunks(
    repo: &Repository,
    file_path: &Path,
    staged: bool,
) -> Result<usize, git2::Error> {
    let path_str = file_path.to_string_lossy().to_string();
    let mut diff_opts = DiffOptions::new();
    diff_opts.pathspec(&path_str);

    let diff = if staged {
        let head = repo.head()?.peel_to_tree()?;
        let index = repo.index()?;
        repo.diff_tree_to_index(Some(&head), Some(&index), Some(&mut diff_opts))?
    } else {
        repo.diff_index_to_workdir(None, Some(&mut diff_opts))?
    };

    let stats = diff.stats()?;
    // Hunk count from diff stats
    let mut hunk_count = 0usize;
    diff.print(DiffFormat::Patch, |_delta, hunk, line| {
        if line.origin() == 'H' && hunk.is_some() {
            hunk_count += 1;
        }
        true
    })?;

    drop(stats);
    Ok(hunk_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git_review::status::GitStatus;

    fn setup_test_repo() -> (tempfile::TempDir, Repository) {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();
        (dir, repo)
    }

    fn make_commit(dir: &Path, repo: &Repository, files: &[(&str, &str)], msg: &str) {
        for (name, content) in files {
            std::fs::write(dir.join(name), content).unwrap();
        }
        let mut index = repo.index().unwrap();
        for (name, _) in files {
            index.add_path(Path::new(name)).unwrap();
        }
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();

        let parents = match repo.head() {
            Ok(reference) => {
                let parent = reference.peel_to_commit().unwrap();
                vec![parent]
            }
            Err(_) => vec![],
        };
        let parent_refs: Vec<&git2::Commit<'_>> = parents.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &parent_refs)
            .unwrap();
    }

    // -- reverse_hunk_header tests --

    #[test]
    fn reverse_hunk_header_basic() {
        let result = reverse_hunk_header("@@ -10,3 +12,5 @@");
        assert_eq!(result, "@@ -12,5 +10,3 @@");
    }

    #[test]
    fn reverse_hunk_header_with_context() {
        let result = reverse_hunk_header("@@ -1,4 +1,6 @@ fn main()");
        assert_eq!(result, "@@ -1,6 +1,4 @@ fn main()");
    }

    #[test]
    fn reverse_hunk_header_single_line() {
        let result = reverse_hunk_header("@@ -5,1 +5,1 @@");
        assert_eq!(result, "@@ -5,1 +5,1 @@");
    }

    // -- reverse_hunk tests --

    #[test]
    fn reverse_hunk_swaps_plus_minus() {
        let hunk = "@@ -1,2 +1,2 @@\n context\n-old\n+new\n";
        let reversed = reverse_hunk(hunk);
        assert!(reversed.contains("+old"));
        assert!(reversed.contains("-new"));
        assert!(reversed.contains(" context"));
    }

    // -- count_hunks tests --

    #[test]
    fn count_hunks_single_change() {
        let (dir, repo) = setup_test_repo();
        make_commit(
            dir.path(),
            &repo,
            &[("file.txt", "line1\nline2\nline3\n")],
            "initial",
        );
        std::fs::write(dir.path().join("file.txt"), "line1\nmodified\nline3\n").unwrap();

        let count = count_hunks(&repo, Path::new("file.txt"), false).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn count_hunks_multiple_changes() {
        let (dir, repo) = setup_test_repo();
        // Create a file with many lines so changes are in separate hunks
        let mut content = String::new();
        for i in 1..=30 {
            content.push_str(&format!("line{}\n", i));
        }
        make_commit(dir.path(), &repo, &[("file.txt", &content)], "initial");

        // Modify line 3 and line 28 (far apart = separate hunks)
        let mut modified = String::new();
        for i in 1..=30 {
            if i == 3 {
                modified.push_str("modified3\n");
            } else if i == 28 {
                modified.push_str("modified28\n");
            } else {
                modified.push_str(&format!("line{}\n", i));
            }
        }
        std::fs::write(dir.path().join("file.txt"), &modified).unwrap();

        let count = count_hunks(&repo, Path::new("file.txt"), false).unwrap();
        assert_eq!(count, 2);
    }

    // -- stage_hunk tests --

    #[test]
    fn stage_single_hunk_from_multi_hunk_file() {
        let (dir, repo) = setup_test_repo();
        // Create a file with many lines
        let mut content = String::new();
        for i in 1..=30 {
            content.push_str(&format!("line{}\n", i));
        }
        make_commit(dir.path(), &repo, &[("file.txt", &content)], "initial");

        // Modify line 3 and line 28 (far apart = separate hunks)
        let mut modified = String::new();
        for i in 1..=30 {
            if i == 3 {
                modified.push_str("CHANGED_3\n");
            } else if i == 28 {
                modified.push_str("CHANGED_28\n");
            } else {
                modified.push_str(&format!("line{}\n", i));
            }
        }
        std::fs::write(dir.path().join("file.txt"), &modified).unwrap();

        // Verify 2 hunks before staging
        let count = count_hunks(&repo, Path::new("file.txt"), false).unwrap();
        assert_eq!(count, 2);

        // Stage only hunk 0
        stage_hunk(&repo, Path::new("file.txt"), 0).unwrap();

        // Should have 1 hunk staged and 1 unstaged
        let staged_count = count_hunks(&repo, Path::new("file.txt"), true).unwrap();
        let unstaged_count = count_hunks(&repo, Path::new("file.txt"), false).unwrap();
        assert_eq!(staged_count, 1);
        assert_eq!(unstaged_count, 1);
    }

    #[test]
    fn stage_hunk_out_of_range() {
        let (dir, repo) = setup_test_repo();
        make_commit(
            dir.path(),
            &repo,
            &[("file.txt", "line1\nline2\n")],
            "initial",
        );
        std::fs::write(dir.path().join("file.txt"), "line1\nmodified\n").unwrap();

        let result = stage_hunk(&repo, Path::new("file.txt"), 5);
        assert!(result.is_err());
    }

    // -- unstage_hunk tests --

    #[test]
    fn unstage_single_hunk() {
        let (dir, repo) = setup_test_repo();
        // Create a file with many lines
        let mut content = String::new();
        for i in 1..=30 {
            content.push_str(&format!("line{}\n", i));
        }
        make_commit(dir.path(), &repo, &[("file.txt", &content)], "initial");

        // Modify line 3 and line 28
        let mut modified = String::new();
        for i in 1..=30 {
            if i == 3 {
                modified.push_str("CHANGED_3\n");
            } else if i == 28 {
                modified.push_str("CHANGED_28\n");
            } else {
                modified.push_str(&format!("line{}\n", i));
            }
        }
        std::fs::write(dir.path().join("file.txt"), &modified).unwrap();

        // Stage all changes for this file
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("file.txt")).unwrap();
        index.write().unwrap();

        // Verify 2 staged hunks
        let staged_count = count_hunks(&repo, Path::new("file.txt"), true).unwrap();
        assert_eq!(staged_count, 2);

        // Unstage hunk 0
        unstage_hunk(&repo, Path::new("file.txt"), 0).unwrap();

        // Should have 1 staged, 1 unstaged
        let staged_after = count_hunks(&repo, Path::new("file.txt"), true).unwrap();
        let unstaged_after = count_hunks(&repo, Path::new("file.txt"), false).unwrap();
        assert_eq!(staged_after, 1);
        assert_eq!(unstaged_after, 1);
    }

    // -- Renamed file header formatting --

    #[test]
    fn renamed_file_header_formatting() {
        use crate::git_review::diff_view::renamed_file_header;
        let header = renamed_file_header("old/path.rs", "new/path.rs");
        assert_eq!(header, "old/path.rs \u{2192} new/path.rs");
    }

    // -- reverse_file_header --

    #[test]
    fn reverse_file_header_unchanged() {
        let header = "diff --git a/file.txt b/file.txt\nindex abc..def 100644\n--- a/file.txt\n+++ b/file.txt\n";
        let reversed = reverse_file_header(header);
        // File header stays the same for reverse-apply
        assert_eq!(reversed, header);
    }
}

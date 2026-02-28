// Git staging, unstaging, discard, and commit operations via git2.

use git2::{IndexAddOption, Repository};
use std::path::Path;

/// Stage a single file (git add).
pub fn stage_file(repo: &Repository, path: &Path) -> Result<(), git2::Error> {
    let mut index = repo.index()?;
    index.add_path(path)?;
    index.write()?;
    Ok(())
}

/// Unstage a single file (git reset HEAD -- <file>).
/// Restores the index entry to match HEAD (or removes it if not in HEAD).
pub fn unstage_file(repo: &Repository, path: &Path) -> Result<(), git2::Error> {
    let head = repo.head();
    match head {
        Ok(reference) => {
            let commit = reference.peel_to_commit()?;
            let tree = commit.tree()?;
            let mut index = repo.index()?;
            match tree.get_path(path) {
                Ok(entry) => {
                    // File exists in HEAD — restore index entry from HEAD tree
                    let blob = repo.find_blob(entry.id())?;
                    let idx_entry = git2::IndexEntry {
                        ctime: git2::IndexTime::new(0, 0),
                        mtime: git2::IndexTime::new(0, 0),
                        dev: 0,
                        ino: 0,
                        mode: entry.filemode() as u32,
                        uid: 0,
                        gid: 0,
                        file_size: blob.content().len() as u32,
                        id: entry.id(),
                        flags: 0,
                        flags_extended: 0,
                        path: path.to_string_lossy().as_bytes().to_vec(),
                    };
                    index.add(&idx_entry)?;
                    index.write()?;
                }
                Err(_) => {
                    // File not in HEAD — remove from index entirely
                    index.remove_path(path)?;
                    index.write()?;
                }
            }
            Ok(())
        }
        Err(_) => {
            // No HEAD (initial commit) — remove from index
            let mut index = repo.index()?;
            index.remove_path(path)?;
            index.write()?;
            Ok(())
        }
    }
}

/// Discard working directory changes for a tracked file (git checkout -- <file>).
pub fn discard_file(repo: &Repository, path: &Path) -> Result<(), git2::Error> {
    let mut checkout_builder = git2::build::CheckoutBuilder::new();
    checkout_builder.path(path).force();
    repo.checkout_index(None, Some(&mut checkout_builder))
}

/// Stage all changes (git add -A equivalent).
pub fn stage_all(repo: &Repository) -> Result<(), git2::Error> {
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;

    // Also handle deletions: remove index entries for files that no longer exist
    let statuses = repo.statuses(None)?;
    for entry in statuses.iter() {
        if entry.status().contains(git2::Status::WT_DELETED) {
            if let Some(p) = entry.path() {
                index.remove_path(Path::new(p))?;
            }
        }
    }

    index.write()?;
    Ok(())
}

/// Unstage all files (git reset HEAD equivalent).
pub fn unstage_all(repo: &Repository) -> Result<(), git2::Error> {
    let head = repo.head();
    match head {
        Ok(reference) => {
            let commit = reference.peel_to_commit()?;
            let tree = commit.tree()?;
            // Read HEAD tree into index, replacing all staged changes
            let mut index = repo.index()?;
            index.read_tree(&tree)?;
            index.write()?;
            Ok(())
        }
        Err(_) => {
            // No HEAD — clear the entire index
            let mut index = repo.index()?;
            index.clear()?;
            index.write()?;
            Ok(())
        }
    }
}

/// Create a commit with the current index (git commit -m).
pub fn commit(repo: &Repository, message: &str) -> Result<git2::Oid, git2::Error> {
    let mut index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let sig = repo.signature()?;

    let parents = match repo.head() {
        Ok(reference) => {
            let parent = reference.peel_to_commit()?;
            vec![parent]
        }
        Err(_) => vec![],
    };
    let parent_refs: Vec<&git2::Commit<'_>> = parents.iter().collect();

    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git_review::status::{FileStatus, GitStatus};
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

    // -- stage_file tests --

    #[test]
    fn stage_file_adds_to_index() {
        let (dir, repo) = setup_test_repo();
        std::fs::write(dir.path().join("new.txt"), "content").unwrap();

        stage_file(&repo, Path::new("new.txt")).unwrap();

        let status = GitStatus::from_repo(&repo).unwrap();
        assert_eq!(status.staged.len(), 1);
        assert_eq!(status.staged[0].status, FileStatus::Added);
        assert!(status.untracked.is_empty());
    }

    #[test]
    fn stage_file_modified() {
        let (dir, repo) = setup_test_repo();
        make_initial_commit(dir.path(), &repo);

        std::fs::write(dir.path().join("initial.txt"), "modified").unwrap();
        stage_file(&repo, Path::new("initial.txt")).unwrap();

        let status = GitStatus::from_repo(&repo).unwrap();
        assert_eq!(status.staged.len(), 1);
        assert_eq!(status.staged[0].status, FileStatus::Modified);
        assert!(status.changed.is_empty());
    }

    // -- unstage_file tests --

    #[test]
    fn unstage_file_new_removes_from_index() {
        let (dir, repo) = setup_test_repo();
        std::fs::write(dir.path().join("new.txt"), "content").unwrap();
        stage_file(&repo, Path::new("new.txt")).unwrap();

        unstage_file(&repo, Path::new("new.txt")).unwrap();

        let status = GitStatus::from_repo(&repo).unwrap();
        assert!(status.staged.is_empty());
        assert_eq!(status.untracked.len(), 1);
    }

    #[test]
    fn unstage_file_modified_restores_head() {
        let (dir, repo) = setup_test_repo();
        make_initial_commit(dir.path(), &repo);

        // Modify and stage
        std::fs::write(dir.path().join("initial.txt"), "modified").unwrap();
        stage_file(&repo, Path::new("initial.txt")).unwrap();

        // Unstage
        unstage_file(&repo, Path::new("initial.txt")).unwrap();

        let status = GitStatus::from_repo(&repo).unwrap();
        assert!(status.staged.is_empty());
        assert_eq!(status.changed.len(), 1);
        assert_eq!(status.changed[0].status, FileStatus::Modified);
    }

    // -- discard_file tests --

    #[test]
    fn discard_file_restores_working_copy() {
        let (dir, repo) = setup_test_repo();
        make_initial_commit(dir.path(), &repo);

        // Modify tracked file
        std::fs::write(dir.path().join("initial.txt"), "modified").unwrap();
        let status = GitStatus::from_repo(&repo).unwrap();
        assert_eq!(status.changed.len(), 1);

        // Discard
        discard_file(&repo, Path::new("initial.txt")).unwrap();

        let status = GitStatus::from_repo(&repo).unwrap();
        assert!(status.changed.is_empty());
        let content = std::fs::read_to_string(dir.path().join("initial.txt")).unwrap();
        assert_eq!(content, "initial");
    }

    // -- stage_all tests --

    #[test]
    fn stage_all_stages_everything() {
        let (dir, repo) = setup_test_repo();
        make_initial_commit(dir.path(), &repo);

        std::fs::write(dir.path().join("initial.txt"), "changed").unwrap();
        std::fs::write(dir.path().join("new1.txt"), "new").unwrap();
        std::fs::write(dir.path().join("new2.txt"), "new").unwrap();

        stage_all(&repo).unwrap();

        let status = GitStatus::from_repo(&repo).unwrap();
        assert_eq!(status.staged.len(), 3);
        assert!(status.changed.is_empty());
        assert!(status.untracked.is_empty());
    }

    #[test]
    fn stage_all_handles_deletions() {
        let (dir, repo) = setup_test_repo();
        make_initial_commit(dir.path(), &repo);

        std::fs::remove_file(dir.path().join("initial.txt")).unwrap();

        stage_all(&repo).unwrap();

        let status = GitStatus::from_repo(&repo).unwrap();
        assert_eq!(status.staged.len(), 1);
        assert_eq!(status.staged[0].status, FileStatus::Deleted);
        assert!(status.changed.is_empty());
    }

    // -- unstage_all tests --

    #[test]
    fn unstage_all_clears_staged() {
        let (dir, repo) = setup_test_repo();
        make_initial_commit(dir.path(), &repo);

        std::fs::write(dir.path().join("initial.txt"), "changed").unwrap();
        std::fs::write(dir.path().join("new.txt"), "new").unwrap();
        stage_all(&repo).unwrap();
        assert!(!GitStatus::from_repo(&repo).unwrap().staged.is_empty());

        unstage_all(&repo).unwrap();

        let status = GitStatus::from_repo(&repo).unwrap();
        assert!(status.staged.is_empty());
        assert!(!status.changed.is_empty() || !status.untracked.is_empty());
    }

    #[test]
    fn unstage_all_no_head() {
        let (dir, repo) = setup_test_repo();
        std::fs::write(dir.path().join("new.txt"), "content").unwrap();
        stage_file(&repo, Path::new("new.txt")).unwrap();

        unstage_all(&repo).unwrap();

        let status = GitStatus::from_repo(&repo).unwrap();
        assert!(status.staged.is_empty());
        assert_eq!(status.untracked.len(), 1);
    }

    // -- commit tests --

    #[test]
    fn commit_creates_commit() {
        let (dir, repo) = setup_test_repo();
        std::fs::write(dir.path().join("file.txt"), "hello").unwrap();
        stage_file(&repo, Path::new("file.txt")).unwrap();

        let oid = commit(&repo, "test commit").unwrap();

        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(head.id(), oid);
        assert_eq!(head.message().unwrap(), "test commit");
    }

    #[test]
    fn commit_clears_staged() {
        let (dir, repo) = setup_test_repo();
        std::fs::write(dir.path().join("file.txt"), "hello").unwrap();
        stage_file(&repo, Path::new("file.txt")).unwrap();

        commit(&repo, "first commit").unwrap();

        let status = GitStatus::from_repo(&repo).unwrap();
        assert!(status.is_empty());
    }

    #[test]
    fn commit_with_parent() {
        let (dir, repo) = setup_test_repo();
        make_initial_commit(dir.path(), &repo);

        std::fs::write(dir.path().join("second.txt"), "content").unwrap();
        stage_file(&repo, Path::new("second.txt")).unwrap();

        let oid = commit(&repo, "second commit").unwrap();

        let new_commit = repo.find_commit(oid).unwrap();
        assert_eq!(new_commit.parent_count(), 1);
        assert_eq!(new_commit.message().unwrap(), "second commit");
    }

    // -- Combined workflow tests --

    #[test]
    fn full_workflow_stage_commit_verify() {
        let (dir, repo) = setup_test_repo();
        // Create and stage files
        std::fs::write(dir.path().join("a.txt"), "a").unwrap();
        std::fs::write(dir.path().join("b.txt"), "b").unwrap();
        stage_all(&repo).unwrap();

        // Commit
        commit(&repo, "add files").unwrap();

        // Modify and stage one
        std::fs::write(dir.path().join("a.txt"), "a_modified").unwrap();
        stage_file(&repo, Path::new("a.txt")).unwrap();

        let status = GitStatus::from_repo(&repo).unwrap();
        assert_eq!(status.staged.len(), 1);
        assert!(status.changed.is_empty());

        // Unstage
        unstage_file(&repo, Path::new("a.txt")).unwrap();
        let status = GitStatus::from_repo(&repo).unwrap();
        assert!(status.staged.is_empty());
        assert_eq!(status.changed.len(), 1);

        // Discard
        discard_file(&repo, Path::new("a.txt")).unwrap();
        let status = GitStatus::from_repo(&repo).unwrap();
        assert!(status.is_empty());
    }
}

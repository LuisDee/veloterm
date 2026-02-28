// File system watcher for the file browser — watches expanded directories
// and notifies the main thread when files change.

use crossbeam_channel::{Receiver, Sender};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Debounce interval for filesystem events.
const DEBOUNCE_MS: u64 = 200;

/// Events sent from the watcher to the main thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsEvent {
    /// A directory's contents changed — refresh it in the tree.
    DirectoryChanged(PathBuf),
}

/// Manages filesystem watches on expanded directories.
/// Uses notify crate with debouncing to avoid flooding the UI.
pub struct FileBrowserWatcher {
    watcher: RecommendedWatcher,
    watched_dirs: HashSet<PathBuf>,
    rx: Receiver<FsEvent>,
}

impl FileBrowserWatcher {
    /// Create a new file browser watcher.
    /// Returns the watcher and a receiver for filesystem events.
    pub fn new() -> Result<Self, notify::Error> {
        let (tx, rx) = crossbeam_channel::unbounded::<FsEvent>();

        let debounce_state = std::sync::Arc::new(std::sync::Mutex::new(
            DebounceState::new(),
        ));

        let watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            let event = match res {
                Ok(ev) => ev,
                Err(e) => {
                    log::warn!("File browser watcher error: {e}");
                    return;
                }
            };

            use notify::EventKind;
            match event.kind {
                EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(_) => {}
                _ => return,
            }

            // Extract affected directories
            for path in &event.paths {
                let dir = if path.is_dir() {
                    path.clone()
                } else {
                    match path.parent() {
                        Some(p) => p.to_path_buf(),
                        None => continue,
                    }
                };

                let mut state = debounce_state.lock().unwrap();
                if state.should_emit(&dir) {
                    let _ = tx.send(FsEvent::DirectoryChanged(dir));
                }
            }
        })?;

        Ok(Self {
            watcher,
            watched_dirs: HashSet::new(),
            rx,
        })
    }

    /// Start watching a directory (non-recursive).
    pub fn watch(&mut self, path: &Path) -> Result<(), notify::Error> {
        if self.watched_dirs.contains(path) {
            return Ok(());
        }
        self.watcher.watch(path, RecursiveMode::NonRecursive)?;
        self.watched_dirs.insert(path.to_path_buf());
        Ok(())
    }

    /// Stop watching a directory.
    pub fn unwatch(&mut self, path: &Path) -> Result<(), notify::Error> {
        if self.watched_dirs.remove(path) {
            self.watcher.unwatch(path)?;
        }
        Ok(())
    }

    /// Stop watching all directories.
    pub fn unwatch_all(&mut self) {
        let dirs: Vec<PathBuf> = self.watched_dirs.drain().collect();
        for dir in dirs {
            let _ = self.watcher.unwatch(&dir);
        }
    }

    /// Drain any pending filesystem events (non-blocking).
    pub fn poll_events(&self) -> Vec<FsEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.rx.try_recv() {
            events.push(event);
        }
        events
    }

    /// Get the set of currently watched directories.
    pub fn watched_dirs(&self) -> &HashSet<PathBuf> {
        &self.watched_dirs
    }
}

/// Debounce state to prevent flooding the UI with events.
struct DebounceState {
    last_emit: std::collections::HashMap<PathBuf, Instant>,
}

impl DebounceState {
    fn new() -> Self {
        Self {
            last_emit: std::collections::HashMap::new(),
        }
    }

    /// Check if enough time has passed since the last emit for this directory.
    fn should_emit(&mut self, path: &Path) -> bool {
        let now = Instant::now();
        let debounce = Duration::from_millis(DEBOUNCE_MS);

        if let Some(last) = self.last_emit.get(path) {
            if now.duration_since(*last) < debounce {
                return false;
            }
        }
        self.last_emit.insert(path.to_path_buf(), now);
        true
    }
}

/// Check if a changed path is relevant to a set of expanded directories.
pub fn is_path_in_watched_set(path: &Path, watched: &HashSet<PathBuf>) -> bool {
    // Check if path itself is watched, or its parent is watched
    if watched.contains(path) {
        return true;
    }
    if let Some(parent) = path.parent() {
        return watched.contains(parent);
    }
    false
}

/// Find the tree node index for a directory path, if it exists in the tree.
pub fn find_node_for_path(tree: &super::tree::FileTree, path: &Path) -> Option<usize> {
    for i in 0..tree.len() {
        if let Some(node) = tree.get(i) {
            if node.path == path {
                return Some(i);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debounce_first_emit_allowed() {
        let mut state = DebounceState::new();
        let path = PathBuf::from("/tmp/test");
        assert!(state.should_emit(&path));
    }

    #[test]
    fn debounce_rapid_second_blocked() {
        let mut state = DebounceState::new();
        let path = PathBuf::from("/tmp/test");
        assert!(state.should_emit(&path));
        // Immediate second call should be blocked
        assert!(!state.should_emit(&path));
    }

    #[test]
    fn debounce_different_paths_independent() {
        let mut state = DebounceState::new();
        let path1 = PathBuf::from("/tmp/test1");
        let path2 = PathBuf::from("/tmp/test2");
        assert!(state.should_emit(&path1));
        assert!(state.should_emit(&path2));
    }

    #[test]
    fn debounce_after_delay_allowed() {
        let mut state = DebounceState::new();
        let path = PathBuf::from("/tmp/test");
        assert!(state.should_emit(&path));

        // Manually set last_emit to 300ms ago
        state.last_emit.insert(
            path.clone(),
            Instant::now() - Duration::from_millis(300),
        );
        assert!(state.should_emit(&path));
    }

    #[test]
    fn path_in_watched_set_direct() {
        let mut watched = HashSet::new();
        watched.insert(PathBuf::from("/tmp/project/src"));
        assert!(is_path_in_watched_set(Path::new("/tmp/project/src"), &watched));
    }

    #[test]
    fn path_in_watched_set_child() {
        let mut watched = HashSet::new();
        watched.insert(PathBuf::from("/tmp/project/src"));
        // A file inside the watched dir
        assert!(is_path_in_watched_set(Path::new("/tmp/project/src/main.rs"), &watched));
    }

    #[test]
    fn path_not_in_watched_set() {
        let mut watched = HashSet::new();
        watched.insert(PathBuf::from("/tmp/project/src"));
        assert!(!is_path_in_watched_set(Path::new("/tmp/other/file.rs"), &watched));
    }

    #[test]
    fn find_node_for_path_found() {
        let mut tree = super::super::tree::FileTree::new(PathBuf::from("/tmp/project"));
        let src = tree.add_child(0, "src", super::super::tree::NodeType::Directory);
        let found = find_node_for_path(&tree, Path::new("/tmp/project/src"));
        assert_eq!(found, Some(src));
    }

    #[test]
    fn find_node_for_path_not_found() {
        let tree = super::super::tree::FileTree::new(PathBuf::from("/tmp/project"));
        assert_eq!(find_node_for_path(&tree, Path::new("/tmp/other")), None);
    }

    #[test]
    fn watcher_creation_succeeds() {
        let watcher = FileBrowserWatcher::new();
        assert!(watcher.is_ok());
    }

    #[test]
    fn watcher_watch_and_unwatch() {
        let dir = tempfile::tempdir().unwrap();
        let mut watcher = FileBrowserWatcher::new().unwrap();

        watcher.watch(dir.path()).unwrap();
        assert!(watcher.watched_dirs().contains(dir.path()));

        watcher.unwatch(dir.path()).unwrap();
        assert!(!watcher.watched_dirs().contains(dir.path()));
    }

    #[test]
    fn watcher_watch_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let mut watcher = FileBrowserWatcher::new().unwrap();

        watcher.watch(dir.path()).unwrap();
        watcher.watch(dir.path()).unwrap(); // second call is no-op
        assert_eq!(watcher.watched_dirs().len(), 1);
    }

    #[test]
    fn watcher_unwatch_all() {
        let dir1 = tempfile::tempdir().unwrap();
        let dir2 = tempfile::tempdir().unwrap();
        let mut watcher = FileBrowserWatcher::new().unwrap();

        watcher.watch(dir1.path()).unwrap();
        watcher.watch(dir2.path()).unwrap();
        assert_eq!(watcher.watched_dirs().len(), 2);

        watcher.unwatch_all();
        assert!(watcher.watched_dirs().is_empty());
    }

    #[test]
    fn watcher_poll_events_empty_initially() {
        let watcher = FileBrowserWatcher::new().unwrap();
        let events = watcher.poll_events();
        assert!(events.is_empty());
    }

    #[test]
    fn watcher_detects_file_creation() {
        let dir = tempfile::tempdir().unwrap();
        let mut watcher = FileBrowserWatcher::new().unwrap();
        watcher.watch(dir.path()).unwrap();

        // Give the watcher time to set up
        std::thread::sleep(Duration::from_millis(200));

        // Create a file
        std::fs::write(dir.path().join("new_file.txt"), "hello").unwrap();

        // Wait for the event to propagate
        std::thread::sleep(Duration::from_millis(500));

        let events = watcher.poll_events();
        assert!(!events.is_empty(), "Should detect file creation");
        assert!(events.iter().any(|e| matches!(e, FsEvent::DirectoryChanged(_))));
    }

    #[test]
    fn fs_event_equality() {
        let e1 = FsEvent::DirectoryChanged(PathBuf::from("/tmp/a"));
        let e2 = FsEvent::DirectoryChanged(PathBuf::from("/tmp/a"));
        let e3 = FsEvent::DirectoryChanged(PathBuf::from("/tmp/b"));
        assert_eq!(e1, e2);
        assert_ne!(e1, e3);
    }
}

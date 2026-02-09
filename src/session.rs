// Session persistence: save and restore terminal layout across restarts.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::pane::{PaneId, PaneNode, SplitDirection};
use crate::tab::TabManager;

/// Serializable snapshot of the entire session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionState {
    pub tabs: Vec<SessionTab>,
    pub active_tab: usize,
}

/// Serializable snapshot of a single tab.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionTab {
    pub title: String,
    pub pane_tree: SessionPaneNode,
}

/// Serializable snapshot of a pane tree node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum SessionPaneNode {
    Leaf {
        cwd: Option<String>,
    },
    Split {
        direction: String,
        ratio: f32,
        first: Box<SessionPaneNode>,
        second: Box<SessionPaneNode>,
    },
}

/// Per-pane data needed for session capture (CWD).
pub struct PaneCwdInfo {
    pub cwd: Option<String>,
}

impl SessionState {
    /// Capture the current session state from tab manager and per-pane CWD info.
    pub fn capture(
        tab_manager: &TabManager,
        pane_cwds: &HashMap<PaneId, PaneCwdInfo>,
    ) -> Self {
        let tabs = tab_manager
            .tabs()
            .iter()
            .map(|tab| SessionTab {
                title: tab.title.clone(),
                pane_tree: capture_pane_node(&tab.pane_tree.root(), pane_cwds),
            })
            .collect();

        Self {
            tabs,
            active_tab: tab_manager.active_index(),
        }
    }

    /// Save session state to a JSON file.
    pub fn save(&self, path: &Path) -> Result<(), SessionError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| SessionError::Serialize(e.to_string()))?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| SessionError::Io(e.to_string()))?;
        }

        std::fs::write(path, json).map_err(|e| SessionError::Io(e.to_string()))?;
        Ok(())
    }

    /// Load session state from a JSON file.
    pub fn load(path: &Path) -> Result<Self, SessionError> {
        let contents =
            std::fs::read_to_string(path).map_err(|e| SessionError::Io(e.to_string()))?;
        let state: Self = serde_json::from_str(&contents)
            .map_err(|e| SessionError::Deserialize(e.to_string()))?;
        if state.tabs.is_empty() {
            return Err(SessionError::Deserialize(
                "session has no tabs".to_string(),
            ));
        }
        Ok(state)
    }

    /// Get the default session file path (~/.config/veloterm/session.json).
    pub fn default_path() -> PathBuf {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        home.join(".config").join("veloterm").join("session.json")
    }

    /// Count the total number of panes across all tabs.
    pub fn total_panes(&self) -> usize {
        self.tabs.iter().map(|t| count_panes(&t.pane_tree)).sum()
    }
}

/// Errors that can occur during session save/restore.
#[derive(Debug)]
pub enum SessionError {
    Io(String),
    Serialize(String),
    Deserialize(String),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionError::Io(e) => write!(f, "session I/O error: {e}"),
            SessionError::Serialize(e) => write!(f, "session serialize error: {e}"),
            SessionError::Deserialize(e) => write!(f, "session deserialize error: {e}"),
        }
    }
}

impl std::error::Error for SessionError {}

/// Recursively capture a PaneNode into a serializable SessionPaneNode.
fn capture_pane_node(
    node: &PaneNode,
    pane_cwds: &HashMap<PaneId, PaneCwdInfo>,
) -> SessionPaneNode {
    match node {
        PaneNode::Leaf { id } => {
            let cwd = pane_cwds
                .get(id)
                .and_then(|info| info.cwd.clone());
            SessionPaneNode::Leaf { cwd }
        }
        PaneNode::Split {
            direction,
            ratio,
            first,
            second,
        } => SessionPaneNode::Split {
            direction: match direction {
                SplitDirection::Horizontal => "horizontal".to_string(),
                SplitDirection::Vertical => "vertical".to_string(),
            },
            ratio: *ratio,
            first: Box::new(capture_pane_node(first, pane_cwds)),
            second: Box::new(capture_pane_node(second, pane_cwds)),
        },
    }
}

/// Count panes in a session pane node tree.
fn count_panes(node: &SessionPaneNode) -> usize {
    match node {
        SessionPaneNode::Leaf { .. } => 1,
        SessionPaneNode::Split { first, second, .. } => {
            count_panes(first) + count_panes(second)
        }
    }
}

/// Restore a PaneTree from a session pane node, returning the new PaneIds for spawning.
/// Returns (root PaneNode, list of (PaneId, cwd) pairs for spawning PTYs).
pub fn restore_pane_tree(
    session_node: &SessionPaneNode,
) -> (PaneNode, Vec<(PaneId, Option<String>)>) {
    let mut pane_spawns = Vec::new();
    let root = restore_pane_node(session_node, &mut pane_spawns);
    (root, pane_spawns)
}

fn restore_pane_node(
    session_node: &SessionPaneNode,
    pane_spawns: &mut Vec<(PaneId, Option<String>)>,
) -> PaneNode {
    match session_node {
        SessionPaneNode::Leaf { cwd } => {
            let id = PaneId::next();
            // Validate CWD exists, fallback to None (which uses $HOME)
            let valid_cwd = cwd.as_ref().and_then(|path| {
                if Path::new(path).is_dir() {
                    Some(path.clone())
                } else {
                    log::warn!("Session CWD no longer exists: {path}, using default");
                    None
                }
            });
            pane_spawns.push((id, valid_cwd));
            PaneNode::leaf(id)
        }
        SessionPaneNode::Split {
            direction,
            ratio,
            first,
            second,
        } => {
            let dir = match direction.as_str() {
                "horizontal" => SplitDirection::Horizontal,
                _ => SplitDirection::Vertical,
            };
            let first_node = restore_pane_node(first, pane_spawns);
            let second_node = restore_pane_node(second, pane_spawns);
            PaneNode::split(dir, *ratio, first_node, second_node)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_session() -> SessionState {
        SessionState {
            tabs: vec![SessionTab {
                title: "Shell".to_string(),
                pane_tree: SessionPaneNode::Leaf {
                    cwd: Some("/home/user".to_string()),
                },
            }],
            active_tab: 0,
        }
    }

    fn split_session() -> SessionState {
        SessionState {
            tabs: vec![SessionTab {
                title: "Work".to_string(),
                pane_tree: SessionPaneNode::Split {
                    direction: "vertical".to_string(),
                    ratio: 0.5,
                    first: Box::new(SessionPaneNode::Leaf {
                        cwd: Some("/home/user/project".to_string()),
                    }),
                    second: Box::new(SessionPaneNode::Leaf {
                        cwd: Some("/home/user/logs".to_string()),
                    }),
                },
            }],
            active_tab: 0,
        }
    }

    fn multi_tab_session() -> SessionState {
        SessionState {
            tabs: vec![
                SessionTab {
                    title: "Tab 1".to_string(),
                    pane_tree: SessionPaneNode::Leaf { cwd: None },
                },
                SessionTab {
                    title: "Tab 2".to_string(),
                    pane_tree: SessionPaneNode::Leaf {
                        cwd: Some("/tmp".to_string()),
                    },
                },
            ],
            active_tab: 1,
        }
    }

    // ── Serialization round-trip tests ──────────────────────────

    #[test]
    fn serialize_simple_session() {
        let session = simple_session();
        let json = serde_json::to_string(&session).unwrap();
        let deserialized: SessionState = serde_json::from_str(&json).unwrap();
        assert_eq!(session, deserialized);
    }

    #[test]
    fn serialize_split_session() {
        let session = split_session();
        let json = serde_json::to_string(&session).unwrap();
        let deserialized: SessionState = serde_json::from_str(&json).unwrap();
        assert_eq!(session, deserialized);
    }

    #[test]
    fn serialize_multi_tab_session() {
        let session = multi_tab_session();
        let json = serde_json::to_string(&session).unwrap();
        let deserialized: SessionState = serde_json::from_str(&json).unwrap();
        assert_eq!(session, deserialized);
    }

    #[test]
    fn serialize_null_cwd() {
        let session = SessionState {
            tabs: vec![SessionTab {
                title: "Shell".to_string(),
                pane_tree: SessionPaneNode::Leaf { cwd: None },
            }],
            active_tab: 0,
        };
        let json = serde_json::to_string(&session).unwrap();
        assert!(json.contains("\"cwd\":null"));
        let deserialized: SessionState = serde_json::from_str(&json).unwrap();
        assert_eq!(session, deserialized);
    }

    // ── File save/load tests ────────────────────────────────────

    #[test]
    fn save_and_load_session() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        let session = split_session();
        session.save(&path).unwrap();
        let loaded = SessionState::load(&path).unwrap();
        assert_eq!(session, loaded);
    }

    #[test]
    fn save_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("dir").join("session.json");
        let session = simple_session();
        session.save(&path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn load_nonexistent_file_returns_error() {
        let result = SessionState::load(Path::new("/tmp/nonexistent_session.json"));
        assert!(result.is_err());
    }

    #[test]
    fn load_empty_tabs_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.json");
        std::fs::write(&path, r#"{"tabs":[],"active_tab":0}"#).unwrap();
        let result = SessionState::load(&path);
        assert!(result.is_err());
    }

    #[test]
    fn load_malformed_json_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.json");
        std::fs::write(&path, "not json").unwrap();
        let result = SessionState::load(&path);
        assert!(result.is_err());
    }

    // ── Capture tests ───────────────────────────────────────────

    #[test]
    fn capture_single_pane() {
        let tab_manager = TabManager::new();
        let pane_id = tab_manager.active_tab().pane_tree.focused_pane_id();
        let mut cwds = HashMap::new();
        cwds.insert(
            pane_id,
            PaneCwdInfo {
                cwd: Some("/home/user".to_string()),
            },
        );

        let session = SessionState::capture(&tab_manager, &cwds);
        assert_eq!(session.tabs.len(), 1);
        assert_eq!(session.active_tab, 0);
        match &session.tabs[0].pane_tree {
            SessionPaneNode::Leaf { cwd } => {
                assert_eq!(cwd.as_deref(), Some("/home/user"));
            }
            _ => panic!("Expected Leaf"),
        }
    }

    #[test]
    fn capture_missing_cwd_is_none() {
        let tab_manager = TabManager::new();
        let cwds = HashMap::new(); // no CWD info

        let session = SessionState::capture(&tab_manager, &cwds);
        match &session.tabs[0].pane_tree {
            SessionPaneNode::Leaf { cwd } => {
                assert!(cwd.is_none());
            }
            _ => panic!("Expected Leaf"),
        }
    }

    // ── Pane counting tests ──────────────────────────────────────

    #[test]
    fn total_panes_single() {
        assert_eq!(simple_session().total_panes(), 1);
    }

    #[test]
    fn total_panes_split() {
        assert_eq!(split_session().total_panes(), 2);
    }

    #[test]
    fn total_panes_multi_tab() {
        assert_eq!(multi_tab_session().total_panes(), 2);
    }

    // ── Restore pane tree tests ─────────────────────────────────

    #[test]
    fn restore_single_leaf() {
        let node = SessionPaneNode::Leaf {
            cwd: Some("/tmp".to_string()),
        };
        let (pane_node, spawns) = restore_pane_tree(&node);
        assert_eq!(spawns.len(), 1);
        assert_eq!(spawns[0].1.as_deref(), Some("/tmp"));
        assert!(matches!(pane_node, PaneNode::Leaf { .. }));
    }

    #[test]
    fn restore_split_creates_two_panes() {
        let node = SessionPaneNode::Split {
            direction: "vertical".to_string(),
            ratio: 0.5,
            first: Box::new(SessionPaneNode::Leaf { cwd: None }),
            second: Box::new(SessionPaneNode::Leaf {
                cwd: Some("/tmp".to_string()),
            }),
        };
        let (pane_node, spawns) = restore_pane_tree(&node);
        assert_eq!(spawns.len(), 2);
        assert!(spawns[0].1.is_none());
        assert_eq!(spawns[1].1.as_deref(), Some("/tmp"));
        assert!(matches!(pane_node, PaneNode::Split { .. }));
    }

    #[test]
    fn restore_stale_cwd_falls_back_to_none() {
        let node = SessionPaneNode::Leaf {
            cwd: Some("/nonexistent/path/that/does/not/exist".to_string()),
        };
        let (_pane_node, spawns) = restore_pane_tree(&node);
        assert_eq!(spawns.len(), 1);
        // Stale CWD should be None
        assert!(spawns[0].1.is_none());
    }

    // ── Default path test ───────────────────────────────────────

    #[test]
    fn default_path_contains_session_json() {
        let path = SessionState::default_path();
        assert!(path.to_string_lossy().contains("session.json"));
    }

    // ── SessionError display test ───────────────────────────────

    #[test]
    fn session_error_display() {
        let err = SessionError::Io("file not found".to_string());
        assert!(format!("{err}").contains("file not found"));
    }
}

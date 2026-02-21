// Markdown preview overlay: parses a markdown file and renders it in a scrollable overlay.

use iced_widget::markdown;

/// State for the markdown preview overlay in a pane.
pub struct MarkdownPreviewState {
    pub file_path: String,
    pub items: Vec<markdown::Item>,
}

impl MarkdownPreviewState {
    /// Open and parse a markdown file, returning the preview state.
    pub fn open(path: &str) -> Result<Self, std::io::Error> {
        let raw = std::fs::read_to_string(path)?;
        let items: Vec<markdown::Item> = markdown::parse(&raw).collect();
        Ok(Self {
            file_path: path.to_string(),
            items,
        })
    }

    /// Get the parsed markdown items for rendering.
    pub fn items(&self) -> &[markdown::Item] {
        &self.items
    }

    /// Get the file name (last path component) for display.
    pub fn file_name(&self) -> &str {
        std::path::Path::new(&self.file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&self.file_path)
    }
}

/// Expand `~` prefix to the user's home directory, and resolve relative paths
/// against the given pane CWD (from shell integration OSC 7).
pub fn expand_path(path: &str, pane_cwd: Option<&str>) -> String {
    if path.starts_with("~/") || path == "~" {
        if let Some(home) = std::env::var_os("HOME") {
            let home = home.to_string_lossy();
            if path == "~" {
                return home.to_string();
            }
            return format!("{}{}", home, &path[1..]);
        }
    }
    if !std::path::Path::new(path).is_absolute() {
        if let Some(cwd) = pane_cwd {
            return std::path::Path::new(cwd)
                .join(path)
                .to_string_lossy()
                .into_owned();
        }
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_valid_file_returns_ok_with_items() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.md");
        std::fs::write(&path, "# Hello\n\nSome **bold** text.").unwrap();
        let state = MarkdownPreviewState::open(path.to_str().unwrap()).unwrap();
        assert!(!state.items.is_empty());
        assert_eq!(state.file_name(), "test.md");
    }

    #[test]
    fn open_nonexistent_file_returns_err() {
        let result = MarkdownPreviewState::open("/tmp/nonexistent_veloterm_test.md");
        assert!(result.is_err());
    }

    #[test]
    fn file_name_extracts_basename() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("README.md");
        std::fs::write(&path, "# Readme").unwrap();
        let state = MarkdownPreviewState::open(path.to_str().unwrap()).unwrap();
        assert_eq!(state.file_name(), "README.md");
    }

    #[test]
    fn empty_file_produces_no_items() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.md");
        std::fs::write(&path, "").unwrap();
        let state = MarkdownPreviewState::open(path.to_str().unwrap()).unwrap();
        assert!(state.items.is_empty());
    }

    #[test]
    fn expand_path_tilde_prefix() {
        let home = std::env::var("HOME").unwrap();
        assert_eq!(expand_path("~/foo.md", None), format!("{}/foo.md", home));
    }

    #[test]
    fn expand_path_tilde_alone() {
        let home = std::env::var("HOME").unwrap();
        assert_eq!(expand_path("~", None), home);
    }

    #[test]
    fn expand_path_absolute_unchanged() {
        assert_eq!(expand_path("/absolute/path.md", None), "/absolute/path.md");
    }

    #[test]
    fn expand_path_relative_unchanged() {
        assert_eq!(expand_path("relative.md", None), "relative.md");
    }

    #[test]
    fn expand_path_relative_with_cwd() {
        assert_eq!(
            expand_path("foo.md", Some("/home/user")),
            "/home/user/foo.md"
        );
    }

    #[test]
    fn expand_path_relative_subdir_with_cwd() {
        assert_eq!(
            expand_path("sub/file.md", Some("/tmp")),
            "/tmp/sub/file.md"
        );
    }

    #[test]
    fn expand_path_tilde_ignores_cwd() {
        let home = std::env::var("HOME").unwrap();
        assert_eq!(
            expand_path("~/foo.md", Some("/tmp")),
            format!("{}/foo.md", home)
        );
    }

    #[test]
    fn expand_path_absolute_ignores_cwd() {
        assert_eq!(
            expand_path("/absolute/path.md", Some("/tmp")),
            "/absolute/path.md"
        );
    }
}

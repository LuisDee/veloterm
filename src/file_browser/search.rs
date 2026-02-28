// Fuzzy search for the file browser — uses nucleo-matcher for fast fuzzy matching.

use crate::file_browser::tree::{FileNode, NodeType};
use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use std::path::Path;

/// A single fuzzy search result.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Index of the FileNode in the tree's node array.
    pub node_index: usize,
    /// Fuzzy match score (higher is better).
    pub score: u32,
    /// Character indices in the relative path string that matched the query.
    pub match_indices: Vec<u32>,
    /// Relative path from the tree root, used for display.
    pub relative_path: String,
}

/// Wrapper around nucleo's Matcher for fuzzy file searching.
pub struct FuzzyMatcher {
    matcher: Matcher,
}

impl FuzzyMatcher {
    pub fn new() -> Self {
        Self {
            matcher: Matcher::new(Config::DEFAULT),
        }
    }

    /// Search files in the tree matching the given query.
    ///
    /// Returns results sorted by score (best match first).
    /// Only searches file nodes, not directories.
    /// Empty query returns empty results.
    pub fn search_files(
        &mut self,
        query: &str,
        nodes: &[FileNode],
        root: &Path,
    ) -> Vec<SearchResult> {
        if query.is_empty() {
            return Vec::new();
        }

        let pattern = Pattern::new(
            query,
            CaseMatching::Smart,
            Normalization::Smart,
            AtomKind::Fuzzy,
        );

        let mut results = Vec::new();
        let mut indices_buf = Vec::new();

        for (idx, node) in nodes.iter().enumerate() {
            // Only search files, not directories
            if matches!(node.node_type, NodeType::Directory) {
                continue;
            }

            let relative = node
                .path
                .strip_prefix(root)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| node.name.clone());

            // Convert to UTF-32 for nucleo
            let chars: Vec<char> = relative.chars().collect();
            let haystack = Utf32Str::Unicode(&chars);

            indices_buf.clear();
            if let Some(score) = pattern.indices(haystack, &mut self.matcher, &mut indices_buf) {
                results.push(SearchResult {
                    node_index: idx,
                    score,
                    match_indices: indices_buf.clone(),
                    relative_path: relative,
                });
            }
        }

        // Sort by score descending (best match first)
        results.sort_by(|a, b| b.score.cmp(&a.score));
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_browser::tree::NodeType;
    use std::path::PathBuf;

    fn make_file_node(name: &str, path: &str, index: usize) -> FileNode {
        FileNode {
            name: name.to_string(),
            path: PathBuf::from(path),
            node_type: NodeType::File {
                extension: PathBuf::from(name)
                    .extension()
                    .map(|e| e.to_string_lossy().to_string()),
                size: 100,
            },
            depth: 1,
            parent: Some(0),
            children: None,
            expanded: false,
        }
    }

    fn make_dir_node(name: &str, path: &str, index: usize) -> FileNode {
        FileNode {
            name: name.to_string(),
            path: PathBuf::from(path),
            node_type: NodeType::Directory,
            depth: 1,
            parent: Some(0),
            children: None,
            expanded: false,
        }
    }

    fn sample_nodes() -> Vec<FileNode> {
        let root = "/project";
        vec![
            FileNode {
                name: "project".to_string(),
                path: PathBuf::from(root),
                node_type: NodeType::Directory,
                depth: 0,
                parent: None,
                children: Some(vec![1, 2, 3, 4, 5]),
                expanded: true,
            },
            make_file_node("main.rs", "/project/src/main.rs", 1),
            make_file_node("lib.rs", "/project/src/lib.rs", 2),
            make_file_node("config.toml", "/project/config.toml", 3),
            make_dir_node("src", "/project/src", 4),
            make_file_node("README.md", "/project/README.md", 5),
        ]
    }

    #[test]
    fn empty_query_returns_empty_results() {
        let mut matcher = FuzzyMatcher::new();
        let nodes = sample_nodes();
        let results = matcher.search_files("", &nodes, Path::new("/project"));
        assert!(results.is_empty());
    }

    #[test]
    fn search_finds_matching_files() {
        let mut matcher = FuzzyMatcher::new();
        let nodes = sample_nodes();
        let results = matcher.search_files("main", &nodes, Path::new("/project"));
        assert!(!results.is_empty());
        // Should find main.rs
        assert!(results.iter().any(|r| r.relative_path.contains("main")));
    }

    #[test]
    fn search_excludes_directories() {
        let mut matcher = FuzzyMatcher::new();
        let nodes = sample_nodes();
        let results = matcher.search_files("src", &nodes, Path::new("/project"));
        // "src" directory (index 4) should NOT appear
        for result in &results {
            assert_ne!(result.node_index, 4, "directory should not be in results");
        }
    }

    #[test]
    fn results_sorted_by_score_descending() {
        let mut matcher = FuzzyMatcher::new();
        let nodes = sample_nodes();
        let results = matcher.search_files("rs", &nodes, Path::new("/project"));
        // All results should be in descending score order
        for w in results.windows(2) {
            assert!(w[0].score >= w[1].score, "results should be sorted by score descending");
        }
    }

    #[test]
    fn search_returns_match_indices() {
        let mut matcher = FuzzyMatcher::new();
        let nodes = sample_nodes();
        let results = matcher.search_files("main", &nodes, Path::new("/project"));
        let main_result = results.iter().find(|r| r.relative_path.contains("main")).unwrap();
        // match_indices should not be empty for a match
        assert!(!main_result.match_indices.is_empty());
    }

    #[test]
    fn search_uses_relative_paths() {
        let mut matcher = FuzzyMatcher::new();
        let nodes = sample_nodes();
        let results = matcher.search_files("main", &nodes, Path::new("/project"));
        let main_result = results.iter().find(|r| r.relative_path.contains("main")).unwrap();
        // Path should be relative (no leading /project)
        assert!(!main_result.relative_path.starts_with("/project"));
        assert!(main_result.relative_path.contains("main.rs"));
    }

    #[test]
    fn fuzzy_match_partial_name() {
        let mut matcher = FuzzyMatcher::new();
        let nodes = sample_nodes();
        // "cfg" should fuzzy-match "config.toml"
        let results = matcher.search_files("cfg", &nodes, Path::new("/project"));
        assert!(results.iter().any(|r| r.relative_path.contains("config")));
    }

    #[test]
    fn no_match_returns_empty() {
        let mut matcher = FuzzyMatcher::new();
        let nodes = sample_nodes();
        let results = matcher.search_files("zzzznotfound", &nodes, Path::new("/project"));
        assert!(results.is_empty());
    }

    #[test]
    fn search_with_path_separator() {
        let mut matcher = FuzzyMatcher::new();
        let nodes = sample_nodes();
        // "src/main" should match src/main.rs
        let results = matcher.search_files("src/main", &nodes, Path::new("/project"));
        assert!(results.iter().any(|r| r.relative_path.contains("main.rs")));
    }

    #[test]
    fn search_special_characters_in_filename() {
        let mut matcher = FuzzyMatcher::new();
        let mut nodes = sample_nodes();
        nodes.push(make_file_node(
            "my-file_v2.0.txt",
            "/project/my-file_v2.0.txt",
            6,
        ));
        let results = matcher.search_files("my-file", &nodes, Path::new("/project"));
        assert!(results.iter().any(|r| r.relative_path.contains("my-file")));
    }

    #[test]
    fn search_case_insensitive_by_default() {
        let mut matcher = FuzzyMatcher::new();
        let nodes = sample_nodes();
        // "readme" (lowercase) should match "README.md"
        let results = matcher.search_files("readme", &nodes, Path::new("/project"));
        assert!(results.iter().any(|r| r.relative_path.contains("README")));
    }

    #[test]
    fn search_node_index_is_correct() {
        let mut matcher = FuzzyMatcher::new();
        let nodes = sample_nodes();
        let results = matcher.search_files("lib", &nodes, Path::new("/project"));
        let lib_result = results.iter().find(|r| r.relative_path.contains("lib")).unwrap();
        // lib.rs is at index 2 in our sample_nodes
        assert_eq!(lib_result.node_index, 2);
    }

    #[test]
    fn search_score_is_nonzero_for_matches() {
        let mut matcher = FuzzyMatcher::new();
        let nodes = sample_nodes();
        let results = matcher.search_files("main", &nodes, Path::new("/project"));
        for result in &results {
            assert!(result.score > 0, "matched results should have positive score");
        }
    }

    #[test]
    fn search_empty_nodes_returns_empty() {
        let mut matcher = FuzzyMatcher::new();
        let nodes: Vec<FileNode> = vec![];
        let results = matcher.search_files("test", &nodes, Path::new("/project"));
        assert!(results.is_empty());
    }
}

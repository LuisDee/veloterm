pub mod detector;
pub mod opener;

/// The kind of detected link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkKind {
    Url,
    FilePath,
}

/// A detected link in the terminal grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedLink {
    pub kind: LinkKind,
    pub start: (usize, usize), // (row, col)
    pub end: (usize, usize),   // (row, col) — inclusive
    pub text: String,
}

impl DetectedLink {
    /// Returns true if the given grid position (row, col) falls within this link.
    pub fn contains(&self, row: usize, col: usize) -> bool {
        if self.start.0 == self.end.0 {
            // Single-line link
            row == self.start.0 && col >= self.start.1 && col <= self.end.1
        } else if row == self.start.0 {
            col >= self.start.1
        } else if row == self.end.0 {
            col <= self.end.1
        } else {
            row > self.start.0 && row < self.end.0
        }
    }
}

/// Scans terminal content and caches detected links.
pub struct LinkDetector {
    links: Vec<DetectedLink>,
    generation: u64,
}

impl LinkDetector {
    pub fn new() -> Self {
        Self {
            links: Vec::new(),
            generation: 0,
        }
    }

    /// Scan the provided lines for URLs and file paths.
    /// Each line corresponds to a terminal row.
    pub fn scan(&mut self, lines: &[String]) {
        self.generation += 1;
        self.links.clear();
        self.links.extend(detector::detect_urls(lines));
        self.links.extend(detector::detect_paths(lines));
    }

    /// Returns the link at the given grid position, if any.
    pub fn link_at(&self, row: usize, col: usize) -> Option<&DetectedLink> {
        self.links.iter().find(|link| link.contains(row, col))
    }

    /// Returns all detected links.
    pub fn links(&self) -> &[DetectedLink] {
        &self.links
    }

    /// Returns the current generation counter.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Clears all detected links (e.g., when link highlight should be removed).
    pub fn clear(&mut self) {
        self.links.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detected_link_url_creation() {
        let link = DetectedLink {
            kind: LinkKind::Url,
            start: (0, 5),
            end: (0, 25),
            text: "https://example.com".to_string(),
        };
        assert_eq!(link.kind, LinkKind::Url);
        assert_eq!(link.start, (0, 5));
        assert_eq!(link.end, (0, 25));
        assert_eq!(link.text, "https://example.com");
    }

    #[test]
    fn detected_link_filepath_creation() {
        let link = DetectedLink {
            kind: LinkKind::FilePath,
            start: (2, 0),
            end: (2, 15),
            text: "/usr/bin/cargo".to_string(),
        };
        assert_eq!(link.kind, LinkKind::FilePath);
    }

    #[test]
    fn contains_single_line_link() {
        let link = DetectedLink {
            kind: LinkKind::Url,
            start: (1, 5),
            end: (1, 20),
            text: "https://example.com".to_string(),
        };
        // Within range
        assert!(link.contains(1, 5));
        assert!(link.contains(1, 12));
        assert!(link.contains(1, 20));
        // Outside range
        assert!(!link.contains(1, 4));
        assert!(!link.contains(1, 21));
        assert!(!link.contains(0, 10));
        assert!(!link.contains(2, 10));
    }

    #[test]
    fn contains_multi_line_link() {
        let link = DetectedLink {
            kind: LinkKind::Url,
            start: (1, 70),
            end: (3, 10),
            text: "https://very-long-url.example.com/path".to_string(),
        };
        // Start row: col >= start.col
        assert!(link.contains(1, 70));
        assert!(link.contains(1, 79));
        assert!(!link.contains(1, 69));
        // Middle row: any column
        assert!(link.contains(2, 0));
        assert!(link.contains(2, 50));
        // End row: col <= end.col
        assert!(link.contains(3, 0));
        assert!(link.contains(3, 10));
        assert!(!link.contains(3, 11));
        // Outside rows
        assert!(!link.contains(0, 70));
        assert!(!link.contains(4, 5));
    }

    #[test]
    fn link_detector_new_empty() {
        let detector = LinkDetector::new();
        assert!(detector.links().is_empty());
        assert_eq!(detector.generation(), 0);
    }

    #[test]
    fn link_detector_scan_increments_generation() {
        let mut detector = LinkDetector::new();
        detector.scan(&[]);
        assert_eq!(detector.generation(), 1);
        detector.scan(&[]);
        assert_eq!(detector.generation(), 2);
    }

    #[test]
    fn link_detector_link_at_returns_none_for_empty() {
        let detector = LinkDetector::new();
        assert!(detector.link_at(0, 0).is_none());
    }

    #[test]
    fn link_detector_scan_finds_urls_and_paths() {
        let mut detector = LinkDetector::new();
        let lines = vec![
            "Visit https://example.com for info".to_string(),
            "File at /usr/bin/cargo here".to_string(),
        ];
        detector.scan(&lines);
        assert_eq!(detector.links().len(), 2);
        assert_eq!(detector.links()[0].kind, LinkKind::Url);
        assert_eq!(detector.links()[1].kind, LinkKind::FilePath);
    }

    #[test]
    fn link_detector_link_at_returns_correct_link() {
        let mut detector = LinkDetector::new();
        let lines = vec!["abc https://x.com end".to_string()];
        detector.scan(&lines);
        // 'h' of https starts at col 4
        let link = detector.link_at(0, 4);
        assert!(link.is_some());
        assert_eq!(link.unwrap().kind, LinkKind::Url);
        assert_eq!(link.unwrap().text, "https://x.com");
    }

    #[test]
    fn link_detector_link_at_returns_none_outside_link() {
        let mut detector = LinkDetector::new();
        let lines = vec!["abc https://x.com end".to_string()];
        detector.scan(&lines);
        // col 0 is 'a', not in the URL
        assert!(detector.link_at(0, 0).is_none());
        // row 1 doesn't exist
        assert!(detector.link_at(1, 4).is_none());
    }

    #[test]
    fn link_detector_rescan_replaces_old_links() {
        let mut detector = LinkDetector::new();
        detector.scan(&vec!["https://first.com".to_string()]);
        assert_eq!(detector.links().len(), 1);
        assert_eq!(detector.generation(), 1);

        detector.scan(&vec!["https://second.com and https://third.com".to_string()]);
        assert_eq!(detector.links().len(), 2);
        assert_eq!(detector.generation(), 2);
        assert_eq!(detector.links()[0].text, "https://second.com");
    }
}

use linkify::{LinkFinder, LinkKind as LinkifyKind};

use super::{DetectedLink, LinkKind};

/// Detect URLs in terminal content lines using the linkify crate.
pub fn detect_urls(lines: &[String]) -> Vec<DetectedLink> {
    let mut finder = LinkFinder::new();
    finder.kinds(&[LinkifyKind::Url]);

    let mut links = Vec::new();
    for (row, line) in lines.iter().enumerate() {
        for link in finder.links(line) {
            let start_col = link.start();
            let end_col = link.end().saturating_sub(1); // inclusive end
            links.push(DetectedLink {
                kind: LinkKind::Url,
                start: (row, start_col),
                end: (row, end_col),
                text: link.as_str().to_string(),
            });
        }
    }
    links
}

/// Detect absolute file paths in terminal content lines.
pub fn detect_paths(lines: &[String]) -> Vec<DetectedLink> {
    let mut links = Vec::new();
    for (row, line) in lines.iter().enumerate() {
        links.extend(find_paths_in_line(row, line));
    }
    links
}

fn find_paths_in_line(row: usize, line: &str) -> Vec<DetectedLink> {
    let mut results = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Look for path starts: '/' or '~/'
        let is_absolute = chars[i] == '/';
        let is_home = chars[i] == '~' && i + 1 < len && chars[i + 1] == '/';

        if !is_absolute && !is_home {
            i += 1;
            continue;
        }

        // Check that path start is at beginning of line or preceded by whitespace/delimiter
        if i > 0 && !is_path_delimiter(chars[i - 1]) {
            i += 1;
            continue;
        }

        let start_col = i;
        // Advance past the prefix
        if is_home {
            i += 2; // skip ~/
        } else {
            i += 1; // skip /
        }

        // Collect path characters
        let mut has_separator_after_prefix = false;
        while i < len && is_path_char(chars[i]) {
            if chars[i] == '/' {
                has_separator_after_prefix = true;
            }
            i += 1;
        }

        // Trim trailing punctuation that's likely not part of the path
        let mut end = i;
        while end > start_col + 1 && is_trailing_punctuation(chars[end - 1]) {
            end -= 1;
        }

        let path_text: String = chars[start_col..end].iter().collect();

        // Validate: must have content after prefix, skip single '/'
        if path_text == "/" || path_text == "~/" {
            continue;
        }

        // For absolute paths starting with /, require at least one more / or meaningful content
        if is_absolute && !has_separator_after_prefix && path_text.len() < 3 {
            continue;
        }

        // Skip known false positives
        if is_false_positive(&path_text) {
            continue;
        }

        results.push(DetectedLink {
            kind: LinkKind::FilePath,
            start: (row, start_col),
            end: (row, end.saturating_sub(1)),
            text: path_text,
        });
    }

    results
}

fn is_path_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '/' | '.' | '_' | '-' | '+' | '@' | ':' | ',' | '=' | '%')
}

fn is_path_delimiter(c: char) -> bool {
    c.is_whitespace() || matches!(c, '"' | '\'' | '(' | '[' | '{' | '<' | '`' | ';' | '|' | '&')
}

fn is_trailing_punctuation(c: char) -> bool {
    matches!(c, '.' | ',' | ':' | ';')
}

fn is_false_positive(path: &str) -> bool {
    matches!(
        path,
        "/dev/null"
            | "/dev/zero"
            | "/dev/random"
            | "/dev/urandom"
            | "/dev/stdin"
            | "/dev/stdout"
            | "/dev/stderr"
            | "/dev/tty"
            | "/dev/fd"
    ) || path.starts_with("/proc/")
        || path.starts_with("/sys/")
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- URL detection tests ---

    #[test]
    fn detect_http_url() {
        let lines = vec!["Visit http://example.com for info".to_string()];
        let links = detect_urls(&lines);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].kind, LinkKind::Url);
        assert_eq!(links[0].text, "http://example.com");
        assert_eq!(links[0].start.0, 0);
    }

    #[test]
    fn detect_https_url_with_path_and_query() {
        let lines = vec!["See https://example.com/path?q=1&b=2#frag here".to_string()];
        let links = detect_urls(&lines);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "https://example.com/path?q=1&b=2#frag");
        assert_eq!(links[0].kind, LinkKind::Url);
    }

    #[test]
    fn detect_url_stops_at_closing_paren() {
        let lines = vec!["(see https://example.com) for details".to_string()];
        let links = detect_urls(&lines);
        assert_eq!(links.len(), 1);
        // linkify should not include the trailing )
        assert!(!links[0].text.ends_with(')'));
    }

    #[test]
    fn detect_url_stops_at_angle_bracket() {
        let lines = vec!["<https://example.com> is the link".to_string()];
        let links = detect_urls(&lines);
        assert_eq!(links.len(), 1);
        assert!(!links[0].text.ends_with('>'));
    }

    #[test]
    fn no_false_positive_on_plain_text() {
        let lines = vec![
            "Hello world, this is just text.".to_string(),
            "No URLs here at all.".to_string(),
            "foo bar baz 12345".to_string(),
        ];
        let links = detect_urls(&lines);
        assert!(links.is_empty());
    }

    #[test]
    fn detect_multiple_urls_same_line() {
        let lines = vec!["http://a.com and https://b.com here".to_string()];
        let links = detect_urls(&lines);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].text, "http://a.com");
        assert_eq!(links[1].text, "https://b.com");
    }

    #[test]
    fn detect_urls_across_multiple_lines() {
        let lines = vec![
            "line0 https://first.com".to_string(),
            "no url".to_string(),
            "line2 https://second.com end".to_string(),
        ];
        let links = detect_urls(&lines);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].start.0, 0); // row 0
        assert_eq!(links[1].start.0, 2); // row 2
    }

    #[test]
    fn url_column_positions_correct() {
        let lines = vec!["abc https://x.com end".to_string()];
        let links = detect_urls(&lines);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].start, (0, 4)); // 'h' at col 4
        let expected_end = 4 + "https://x.com".len() - 1;
        assert_eq!(links[0].end, (0, expected_end));
    }

    // --- File path detection tests ---

    #[test]
    fn detect_absolute_unix_path() {
        let lines = vec!["file at /usr/bin/cargo here".to_string()];
        let links = detect_paths(&lines);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].kind, LinkKind::FilePath);
        assert_eq!(links[0].text, "/usr/bin/cargo");
    }

    #[test]
    fn detect_home_relative_path() {
        let lines = vec!["config at ~/Documents/file.txt ok".to_string()];
        let links = detect_paths(&lines);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].kind, LinkKind::FilePath);
        assert_eq!(links[0].text, "~/Documents/file.txt");
    }

    #[test]
    fn path_stops_at_whitespace() {
        let lines = vec!["/foo/bar baz".to_string()];
        let links = detect_paths(&lines);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "/foo/bar");
    }

    #[test]
    fn path_stops_at_quotes() {
        let lines = vec!["'/foo/bar' here".to_string()];
        let links = detect_paths(&lines);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "/foo/bar");
    }

    #[test]
    fn path_stops_at_parens() {
        let lines = vec!["(/foo/bar) here".to_string()];
        let links = detect_paths(&lines);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "/foo/bar");
    }

    #[test]
    fn ignore_dev_null() {
        let lines = vec!["redirect to /dev/null end".to_string()];
        let links = detect_paths(&lines);
        assert!(links.is_empty());
    }

    #[test]
    fn ignore_proc_paths() {
        let lines = vec!["reading /proc/cpuinfo data".to_string()];
        let links = detect_paths(&lines);
        assert!(links.is_empty());
    }

    #[test]
    fn ignore_single_slash() {
        let lines = vec!["just a / slash".to_string()];
        let links = detect_paths(&lines);
        assert!(links.is_empty());
    }

    #[test]
    fn path_with_dots_and_extension() {
        let lines = vec!["/foo/bar.rs and /tmp/file.log here".to_string()];
        let links = detect_paths(&lines);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].text, "/foo/bar.rs");
        assert_eq!(links[1].text, "/tmp/file.log");
    }

    #[test]
    fn path_at_start_of_line() {
        let lines = vec!["/usr/local/bin/thing".to_string()];
        let links = detect_paths(&lines);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "/usr/local/bin/thing");
        assert_eq!(links[0].start, (0, 0));
    }

    #[test]
    fn path_column_positions_correct() {
        let lines = vec!["abc /foo/bar end".to_string()];
        let links = detect_paths(&lines);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].start, (0, 4));
        assert_eq!(links[0].end, (0, 11)); // /foo/bar is 8 chars, col 4..11
    }

    #[test]
    fn no_path_in_plain_text() {
        let lines = vec![
            "Hello world".to_string(),
            "no paths 123".to_string(),
        ];
        let links = detect_paths(&lines);
        assert!(links.is_empty());
    }

    #[test]
    fn path_trailing_period_trimmed() {
        let lines = vec!["See /foo/bar.".to_string()];
        let links = detect_paths(&lines);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "/foo/bar");
    }
}

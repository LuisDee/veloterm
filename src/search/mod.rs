pub mod highlight;
pub mod overlay;

/// A single match in the scrollback buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMatch {
    /// Row index (0 = first row of scrollback, positive = toward screen bottom).
    pub row: i32,
    /// Starting column (inclusive).
    pub start_col: usize,
    /// Ending column (exclusive).
    pub end_col: usize,
}

/// Result of a search operation.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// All match positions found.
    pub matches: Vec<SearchMatch>,
    /// Total number of matches.
    pub total_count: usize,
    /// Error message if the regex was invalid.
    pub error: Option<String>,
}

/// Regex search engine over terminal content lines.
pub struct SearchEngine;

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchEngine {
    pub fn new() -> Self {
        Self
    }

    /// Search for `query` across `lines`. Each element of `lines` is one row of text.
    /// Returns `SearchResult` with all matches found. Uses case-insensitive regex by default.
    pub fn search(&self, query: &str, lines: &[String]) -> SearchResult {
        if query.is_empty() {
            return SearchResult {
                matches: Vec::new(),
                total_count: 0,
                error: None,
            };
        }

        let pattern = format!("(?i){}", query);
        let re = match regex::Regex::new(&pattern) {
            Ok(re) => re,
            Err(e) => {
                return SearchResult {
                    matches: Vec::new(),
                    total_count: 0,
                    error: Some(e.to_string()),
                };
            }
        };

        let mut matches = Vec::new();
        for (row_idx, line) in lines.iter().enumerate() {
            for m in re.find_iter(line) {
                matches.push(SearchMatch {
                    row: row_idx as i32,
                    start_col: m.start(),
                    end_col: m.end(),
                });
            }
        }

        let total_count = matches.len();
        SearchResult {
            matches,
            total_count,
            error: None,
        }
    }
}

/// Manages search state: query, matches, navigation, and active status.
pub struct SearchState {
    pub query: String,
    pub matches: Vec<SearchMatch>,
    pub current_index: usize,
    pub is_active: bool,
    pub error: Option<String>,
    engine: SearchEngine,
}

impl Default for SearchState {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            matches: Vec::new(),
            current_index: 0,
            is_active: false,
            error: None,
            engine: SearchEngine::new(),
        }
    }

    /// Update the search query and re-run search against provided lines.
    /// Resets current_index to 0.
    pub fn set_query(&mut self, query: &str, lines: &[String]) {
        self.query = query.to_string();
        self.current_index = 0;
        let result = self.engine.search(query, lines);
        self.matches = result.matches;
        self.error = result.error;
    }

    /// Advance to the next match. Wraps from last → 0.
    pub fn next_match(&mut self) {
        if self.matches.is_empty() {
            return;
        }
        self.current_index = (self.current_index + 1) % self.matches.len();
    }

    /// Go to the previous match. Wraps from 0 → last.
    pub fn prev_match(&mut self) {
        if self.matches.is_empty() {
            return;
        }
        if self.current_index == 0 {
            self.current_index = self.matches.len() - 1;
        } else {
            self.current_index -= 1;
        }
    }

    /// Returns the current active match, if any.
    pub fn current_match(&self) -> Option<&SearchMatch> {
        self.matches.get(self.current_index)
    }

    /// Returns matches visible in the given viewport range (± buffer rows).
    pub fn visible_matches(&self, viewport_start: i32, viewport_end: i32, buffer: i32) -> Vec<&SearchMatch> {
        let start = viewport_start - buffer;
        let end = viewport_end + buffer;
        self.matches
            .iter()
            .filter(|m| m.row >= start && m.row <= end)
            .collect()
    }

    /// Returns the row of the current match for scroll-to-match.
    pub fn scroll_target(&self) -> Option<i32> {
        self.current_match().map(|m| m.row)
    }

    /// Total number of matches.
    pub fn total_count(&self) -> usize {
        self.matches.len()
    }
}

/// Compute the display_offset needed to bring `match_row` into the visible viewport.
///
/// Returns `None` if the match is already visible (no scroll needed).
/// Returns `Some(new_offset)` if scrolling is required.
///
/// - `match_row`: the row index of the match (0 = top of screen, negative = scrollback)
/// - `viewport_rows`: number of visible rows in the terminal
/// - `current_offset`: current display_offset (0 = bottom, positive = scrolled up)
/// - `max_offset`: maximum display_offset (total scrollback lines)
pub fn compute_scroll_offset(
    match_row: i32,
    viewport_rows: usize,
    current_offset: usize,
    max_offset: usize,
) -> Option<usize> {
    let offset = current_offset as i32;
    let vp = viewport_rows as i32;
    // Visible row range: [-offset, -offset + viewport - 1]
    let visible_top = -offset;
    let visible_bottom = -offset + vp - 1;

    if match_row >= visible_top && match_row <= visible_bottom {
        return None; // already visible
    }

    // Scroll so match_row is at the top of the viewport
    let new_offset = (-match_row).max(0) as usize;
    Some(new_offset.min(max_offset))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(strs: &[&str]) -> Vec<String> {
        strs.iter().map(|s| s.to_string()).collect()
    }

    // ── 1.1.2 Basic literal search ─────────────────────────────────

    #[test]
    fn literal_search_finds_single_match() {
        let engine = SearchEngine::new();
        let content = lines(&["hello world"]);
        let result = engine.search("world", &content);
        assert_eq!(result.total_count, 1);
        assert_eq!(
            result.matches[0],
            SearchMatch { row: 0, start_col: 6, end_col: 11 }
        );
        assert!(result.error.is_none());
    }

    #[test]
    fn literal_search_finds_multiple_matches_in_one_row() {
        let engine = SearchEngine::new();
        let content = lines(&["foo bar foo baz foo"]);
        let result = engine.search("foo", &content);
        assert_eq!(result.total_count, 3);
        assert_eq!(result.matches[0], SearchMatch { row: 0, start_col: 0, end_col: 3 });
        assert_eq!(result.matches[1], SearchMatch { row: 0, start_col: 8, end_col: 11 });
        assert_eq!(result.matches[2], SearchMatch { row: 0, start_col: 16, end_col: 19 });
    }

    #[test]
    fn literal_search_no_match_returns_empty() {
        let engine = SearchEngine::new();
        let content = lines(&["hello world"]);
        let result = engine.search("xyz", &content);
        assert_eq!(result.total_count, 0);
        assert!(result.matches.is_empty());
        assert!(result.error.is_none());
    }

    // ── 1.1.3 Regex search ─────────────────────────────────────────

    #[test]
    fn regex_search_digit_pattern() {
        let engine = SearchEngine::new();
        let content = lines(&["abc 123 def 456"]);
        let result = engine.search(r"\d+", &content);
        assert_eq!(result.total_count, 2);
        assert_eq!(result.matches[0], SearchMatch { row: 0, start_col: 4, end_col: 7 });
        assert_eq!(result.matches[1], SearchMatch { row: 0, start_col: 12, end_col: 15 });
    }

    #[test]
    fn regex_search_url_pattern() {
        let engine = SearchEngine::new();
        let content = lines(&["visit https://example.com or http://test.org"]);
        let result = engine.search(r"https?://\S+", &content);
        assert_eq!(result.total_count, 2);
        assert_eq!(result.matches[0].start_col, 6);
        assert_eq!(result.matches[1].start_col, 29);
    }

    // ── 1.1.4 Case-insensitive search ──────────────────────────────

    #[test]
    fn search_is_case_insensitive_by_default() {
        let engine = SearchEngine::new();
        let content = lines(&["Hello WORLD hello World"]);
        let result = engine.search("hello", &content);
        assert_eq!(result.total_count, 2);
    }

    #[test]
    fn case_insensitive_regex_pattern() {
        let engine = SearchEngine::new();
        let content = lines(&["Error ERROR error"]);
        let result = engine.search("error", &content);
        assert_eq!(result.total_count, 3);
    }

    // ── 1.1.5 Invalid regex returns error ──────────────────────────

    #[test]
    fn invalid_regex_returns_error() {
        let engine = SearchEngine::new();
        let content = lines(&["some text"]);
        let result = engine.search("[invalid", &content);
        assert!(result.error.is_some());
        assert!(result.matches.is_empty());
        assert_eq!(result.total_count, 0);
    }

    #[test]
    fn invalid_regex_does_not_panic() {
        let engine = SearchEngine::new();
        let content = lines(&["text"]);
        // Multiple forms of invalid regex
        let _ = engine.search("(unclosed", &content);
        let _ = engine.search("*bad", &content);
        let _ = engine.search("[z-a]", &content);
    }

    // ── 1.1.6 Empty query returns no matches ───────────────────────

    #[test]
    fn empty_query_returns_no_matches() {
        let engine = SearchEngine::new();
        let content = lines(&["some text here"]);
        let result = engine.search("", &content);
        assert_eq!(result.total_count, 0);
        assert!(result.matches.is_empty());
        assert!(result.error.is_none());
    }

    // ── 1.1.7 Multi-line content ───────────────────────────────────

    #[test]
    fn search_across_multiple_rows() {
        let engine = SearchEngine::new();
        let content = lines(&[
            "first line with foo",
            "second line",
            "third line with foo and foo",
        ]);
        let result = engine.search("foo", &content);
        assert_eq!(result.total_count, 3);
        assert_eq!(result.matches[0].row, 0);
        assert_eq!(result.matches[1].row, 2);
        assert_eq!(result.matches[2].row, 2);
    }

    #[test]
    fn search_no_match_in_middle_rows() {
        let engine = SearchEngine::new();
        let content = lines(&[
            "match here",
            "nothing",
            "nothing",
            "match here too",
        ]);
        let result = engine.search("match", &content);
        assert_eq!(result.total_count, 2);
        assert_eq!(result.matches[0].row, 0);
        assert_eq!(result.matches[1].row, 3);
    }

    // ── 1.1.8 Match count across full buffer ───────────────────────

    #[test]
    fn match_count_large_buffer() {
        let engine = SearchEngine::new();
        // 100 lines, each containing "test"
        let content: Vec<String> = (0..100).map(|i| format!("line {} test data", i)).collect();
        let result = engine.search("test", &content);
        assert_eq!(result.total_count, 100);
        assert_eq!(result.matches.len(), 100);
    }

    #[test]
    fn total_count_equals_matches_len() {
        let engine = SearchEngine::new();
        let content = lines(&["aaa bbb aaa", "ccc aaa"]);
        let result = engine.search("aaa", &content);
        assert_eq!(result.total_count, result.matches.len());
    }

    // ── 1.3.1 SearchState tracks query, index, count ───────────────

    #[test]
    fn search_state_initial_values() {
        let state = SearchState::new();
        assert_eq!(state.query, "");
        assert_eq!(state.current_index, 0);
        assert!(state.matches.is_empty());
        assert!(!state.is_active);
        assert!(state.error.is_none());
    }

    #[test]
    fn search_state_tracks_query_after_set() {
        let mut state = SearchState::new();
        let content = lines(&["hello world"]);
        state.set_query("hello", &content);
        assert_eq!(state.query, "hello");
        assert_eq!(state.total_count(), 1);
    }

    // ── 1.3.2 next_match wraps ─────────────────────────────────────

    #[test]
    fn next_match_advances_index() {
        let mut state = SearchState::new();
        let content = lines(&["aa bb aa cc aa"]);
        state.set_query("aa", &content);
        assert_eq!(state.current_index, 0);
        state.next_match();
        assert_eq!(state.current_index, 1);
        state.next_match();
        assert_eq!(state.current_index, 2);
    }

    #[test]
    fn next_match_wraps_to_zero() {
        let mut state = SearchState::new();
        let content = lines(&["aa bb aa"]);
        state.set_query("aa", &content);
        assert_eq!(state.total_count(), 2);
        state.next_match(); // 0 → 1
        state.next_match(); // 1 → 0 (wrap)
        assert_eq!(state.current_index, 0);
    }

    #[test]
    fn next_match_no_matches_does_nothing() {
        let mut state = SearchState::new();
        let content = lines(&["hello"]);
        state.set_query("xyz", &content);
        state.next_match();
        assert_eq!(state.current_index, 0);
    }

    // ── 1.3.3 prev_match wraps ─────────────────────────────────────

    #[test]
    fn prev_match_decrements_index() {
        let mut state = SearchState::new();
        let content = lines(&["aa bb aa cc aa"]);
        state.set_query("aa", &content);
        state.next_match(); // 0 → 1
        state.next_match(); // 1 → 2
        state.prev_match(); // 2 → 1
        assert_eq!(state.current_index, 1);
    }

    #[test]
    fn prev_match_wraps_to_last() {
        let mut state = SearchState::new();
        let content = lines(&["aa bb aa cc aa"]);
        state.set_query("aa", &content);
        assert_eq!(state.current_index, 0);
        state.prev_match(); // 0 → 2 (wrap)
        assert_eq!(state.current_index, 2);
    }

    #[test]
    fn prev_match_no_matches_does_nothing() {
        let mut state = SearchState::new();
        let content = lines(&["hello"]);
        state.set_query("xyz", &content);
        state.prev_match();
        assert_eq!(state.current_index, 0);
    }

    // ── 1.3.4 set_query resets index ───────────────────────────────

    #[test]
    fn set_query_resets_index_to_zero() {
        let mut state = SearchState::new();
        let content = lines(&["aa bb aa cc aa"]);
        state.set_query("aa", &content);
        state.next_match(); // 0 → 1
        state.next_match(); // 1 → 2
        assert_eq!(state.current_index, 2);
        state.set_query("bb", &content);
        assert_eq!(state.current_index, 0);
    }

    #[test]
    fn set_query_updates_matches() {
        let mut state = SearchState::new();
        let content = lines(&["aa bb cc"]);
        state.set_query("aa", &content);
        assert_eq!(state.total_count(), 1);
        state.set_query("bb", &content);
        assert_eq!(state.total_count(), 1);
        state.set_query("dd", &content);
        assert_eq!(state.total_count(), 0);
    }

    // ── 1.3.5 current_match returns active match ───────────────────

    #[test]
    fn current_match_returns_first_match() {
        let mut state = SearchState::new();
        let content = lines(&["hello world hello"]);
        state.set_query("hello", &content);
        let m = state.current_match().unwrap();
        assert_eq!(m.row, 0);
        assert_eq!(m.start_col, 0);
        assert_eq!(m.end_col, 5);
    }

    #[test]
    fn current_match_after_navigation() {
        let mut state = SearchState::new();
        let content = lines(&["hello world hello"]);
        state.set_query("hello", &content);
        state.next_match();
        let m = state.current_match().unwrap();
        assert_eq!(m.start_col, 12);
    }

    #[test]
    fn current_match_none_when_no_matches() {
        let mut state = SearchState::new();
        let content = lines(&["hello"]);
        state.set_query("xyz", &content);
        assert!(state.current_match().is_none());
    }

    #[test]
    fn current_match_none_when_empty_query() {
        let state = SearchState::new();
        assert!(state.current_match().is_none());
    }

    // ── 1.3.6 Visible match filtering ──────────────────────────────

    #[test]
    fn visible_matches_filters_by_viewport() {
        let mut state = SearchState::new();
        let content = lines(&[
            "XFIND row 0",    // row 0
            "XFIND row 1",    // row 1
            "nothing here",   // row 2
            "XFIND row 3",    // row 3
            "XFIND row 4",    // row 4
            "nothing here",   // row 5
            "XFIND row 6",    // row 6
        ]);
        state.set_query("XFIND", &content);
        // viewport rows 2..4, buffer=0
        let visible = state.visible_matches(2, 4, 0);
        assert_eq!(visible.len(), 2); // rows 3 and 4
        assert_eq!(visible[0].row, 3);
        assert_eq!(visible[1].row, 4);
    }

    #[test]
    fn visible_matches_includes_buffer() {
        let mut state = SearchState::new();
        let content = lines(&[
            "XFIND row 0",
            "XFIND row 1",
            "nothing here",
            "XFIND row 3",
            "XFIND row 4",
            "XFIND row 5",
            "XFIND row 6",
        ]);
        state.set_query("XFIND", &content);
        // viewport rows 3..4, buffer=1 → effective range 2..5
        let visible = state.visible_matches(3, 4, 1);
        // rows 3, 4, 5 are in range 2..5 inclusive
        assert!(visible.iter().all(|m| m.row >= 2 && m.row <= 5));
    }

    #[test]
    fn visible_matches_empty_when_no_matches_in_range() {
        let mut state = SearchState::new();
        let content = lines(&[
            "XFIND row 0",
            "nothing here",
            "nothing here",
            "nothing here",
            "XFIND row 4",
        ]);
        state.set_query("XFIND", &content);
        let visible = state.visible_matches(1, 3, 0);
        assert!(visible.is_empty());
    }

    // ── scroll_target ──────────────────────────────────────────────

    #[test]
    fn scroll_target_returns_current_match_row() {
        let mut state = SearchState::new();
        let content = lines(&["nothing here", "XFIND here", "nothing here"]);
        state.set_query("XFIND", &content);
        assert_eq!(state.scroll_target(), Some(1));
    }

    #[test]
    fn scroll_target_none_when_no_matches() {
        let mut state = SearchState::new();
        let content = lines(&["hello"]);
        state.set_query("xyz", &content);
        assert_eq!(state.scroll_target(), None);
    }

    #[test]
    fn scroll_target_updates_after_navigation() {
        let mut state = SearchState::new();
        let content = lines(&["XFIND row 0", "nothing here", "XFIND row 2"]);
        state.set_query("XFIND", &content);
        assert_eq!(state.scroll_target(), Some(0));
        state.next_match();
        assert_eq!(state.scroll_target(), Some(2));
    }

    // ── 3.3.1 compute_scroll_offset — match outside viewport ────────

    #[test]
    fn scroll_offset_for_match_in_scrollback() {
        // Match at row -5 (in scrollback), viewport=24 rows, currently at offset 0
        // Need to scroll up to show row -5: offset = 5
        let result = compute_scroll_offset(-5, 24, 0, 100);
        assert_eq!(result, Some(5));
    }

    #[test]
    fn scroll_offset_for_match_below_viewport() {
        // Match at row 10, viewport=5 rows, currently scrolled up offset=20
        // Visible rows with offset 20: rows -20 to -16 (approx.)
        // Match at row 10 is well below → need to scroll to bottom and beyond
        // With offset=0, visible rows are 0..4. Row 10 still not visible.
        // Actually, rows are relative: row 0..viewport_rows-1 are visible when offset=0
        // So row 10 with viewport_rows=5 means we need offset such that row 10 is visible
        // visible range = [-(offset) .. viewport_rows-1-(offset)]
        // For row 10 to be visible: 10 >= -offset → offset <= -10... that doesn't work.
        // Let me reconsider: in the terminal, row indices from extract_grid_cells are 0..rows-1
        // display_offset shifts which rows are shown. If match_row is 10 and viewport is 5,
        // that means match is on screen row 10 which is beyond the viewport.
        // We should scroll so match row is centered or at least visible.
        // Since match_row=10 > viewport_rows-1=4, we can't show it without scrolling down,
        // but display_offset can only scroll UP (into scrollback). Row 10 in positive
        // territory means it's below the current viewport.
        // Actually, offset 0 = bottom of scrollback. With offset=20, we see 20 rows back.
        // Row indices from search are absolute grid positions.
        // If match is at row 0 and we're at offset 10, visible rows are -10 to 13 (viewport=24).
        // So match at row 0 is visible when offset ≤ viewport - 1.
        // For offset=10, visible range is roughly [-10, 13] for viewport=24.
        // match_row=0 falls in [-10, 13] → no scroll needed.
        //
        // Let me simplify: visible rows = [-offset, -offset + viewport - 1]
        // match visible if: match_row >= -offset AND match_row <= -offset + viewport - 1
        // i.e.: -offset <= match_row <= -offset + viewport - 1
        // i.e.: offset >= -(match_row) AND offset <= -(match_row) + viewport - 1
        // i.e.: offset >= -match_row AND offset <= viewport - 1 - match_row
        // For match inside viewport: no scroll needed.
        // For match above viewport (match_row < -offset): need offset = -match_row
        // For match below viewport (match_row > -offset + viewport - 1): offset = -(match_row - viewport + 1)
        // But offset can't be negative (can't scroll past bottom).
        let result = compute_scroll_offset(10, 24, 20, 100);
        // Match at row 10, viewport=24. Visible range with offset=20: [-20, 3].
        // Row 10 > 3 → need to scroll down. New offset = -(10 - 24 + 1) = -(10-23) = 13? No...
        // offset so row 10 is visible: -offset + viewport - 1 >= 10 → offset <= viewport - 1 - 10 = 13
        // So offset=0 shows rows [0, 23], offset=13 shows rows [-13, 10]
        // With offset=13, row 10 is at the bottom edge. That works.
        // But we want row somewhat centered. Let's just put it at offset = max(0, -match_row + viewport/2)
        // Hmm this is getting complex. Let me just use: offset = max(0, -match_row)
        // which makes the match at row 0 of the viewport (top).
        // For match_row=10: offset = max(0, -10) = 0 → shows rows [0, 23], row 10 is visible. ✓
        // For match_row=-5: offset = max(0, 5) = 5 → shows rows [-5, 18], row -5 is visible. ✓
        // For match_row=10, current offset=20: visible [-20, 3]. Row 10 not visible.
        //   New offset = max(0, -10) = 0. Shows [0, 23]. Row 10 visible. ✓
        assert!(result.is_some());
        let new_offset = result.unwrap();
        // Verify match is now visible: -new_offset <= 10 <= -new_offset + 23
        assert!(10 >= -(new_offset as i32));
        assert!(10 <= -(new_offset as i32) + 23);
    }

    // ── 3.3.2 match already in viewport — no scroll ─────────────────

    #[test]
    fn no_scroll_when_match_in_viewport() {
        // Match at row 5, viewport=24, offset=0 → visible range [0, 23]
        let result = compute_scroll_offset(5, 24, 0, 100);
        assert_eq!(result, None);
    }

    #[test]
    fn no_scroll_when_match_at_viewport_edge() {
        // Match at row -3, viewport=24, offset=3 → visible range [-3, 20]
        let result = compute_scroll_offset(-3, 24, 3, 100);
        assert_eq!(result, None);
    }

    // ── 3.3.3 wrapping navigation scrolls correctly ─────────────────

    #[test]
    fn scroll_offset_clamped_to_max() {
        // Match at row -200, viewport=24, max_offset=100
        let result = compute_scroll_offset(-200, 24, 0, 100);
        assert!(result.is_some());
        assert!(result.unwrap() <= 100);
    }

    #[test]
    fn scroll_offset_clamped_to_zero_minimum() {
        // Match at row 20, viewport=24, offset=50 → match below visible area?
        // Visible range: [-50, -27]. Row 20 is way below.
        // Need offset = max(0, -20) = 0
        let result = compute_scroll_offset(20, 24, 50, 100);
        assert!(result.is_some());
        assert!(result.unwrap() == 0);
    }
}

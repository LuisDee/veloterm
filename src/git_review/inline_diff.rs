// Word-level (intra-line) diff highlighting using the `similar` crate.
// For modification pairs in side-by-side diffs, this computes exactly which
// words/tokens changed within a line.

use similar::{ChangeTag, TextDiff};

/// A span of text with its change classification.
#[derive(Debug, Clone, PartialEq)]
pub struct InlineSpan {
    pub text: String,
    pub tag: InlineTag,
}

/// Classification for an inline diff span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineTag {
    /// Unchanged text.
    Equal,
    /// Added text (green highlight, appears on right/new side).
    Insert,
    /// Removed text (red highlight, appears on left/old side).
    Delete,
}

/// Compute word-level diff between an old line and a new line.
///
/// Returns `(left_spans, right_spans)` where:
/// - `left_spans` contains Equal + Delete spans (for the old/left pane)
/// - `right_spans` contains Equal + Insert spans (for the new/right pane)
pub fn inline_diff(old_line: &str, new_line: &str) -> (Vec<InlineSpan>, Vec<InlineSpan>) {
    let diff = TextDiff::from_words(old_line, new_line);
    let mut left_spans = Vec::new();
    let mut right_spans = Vec::new();

    for change in diff.iter_all_changes() {
        let text = change.value().to_string();
        match change.tag() {
            ChangeTag::Equal => {
                left_spans.push(InlineSpan {
                    text: text.clone(),
                    tag: InlineTag::Equal,
                });
                right_spans.push(InlineSpan {
                    text,
                    tag: InlineTag::Equal,
                });
            }
            ChangeTag::Delete => {
                left_spans.push(InlineSpan {
                    text,
                    tag: InlineTag::Delete,
                });
            }
            ChangeTag::Insert => {
                right_spans.push(InlineSpan {
                    text,
                    tag: InlineTag::Insert,
                });
            }
        }
    }

    (left_spans, right_spans)
}

/// Concatenate spans back into a string (for testing/verification).
pub fn spans_to_string(spans: &[InlineSpan]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- InlineTag equality --

    #[test]
    fn inline_tag_equality() {
        assert_eq!(InlineTag::Equal, InlineTag::Equal);
        assert_eq!(InlineTag::Insert, InlineTag::Insert);
        assert_eq!(InlineTag::Delete, InlineTag::Delete);
        assert_ne!(InlineTag::Insert, InlineTag::Delete);
        assert_ne!(InlineTag::Equal, InlineTag::Insert);
    }

    // -- Single word change --

    #[test]
    fn single_word_change() {
        let (left, right) = inline_diff("hello world", "hello universe");
        // Left should have "hello " equal + "world" deleted
        assert_eq!(spans_to_string(&left), "hello world");
        assert_eq!(spans_to_string(&right), "hello universe");

        // Check tags
        let left_tags: Vec<InlineTag> = left.iter().map(|s| s.tag).collect();
        assert!(left_tags.contains(&InlineTag::Equal));
        assert!(left_tags.contains(&InlineTag::Delete));
        assert!(!left_tags.contains(&InlineTag::Insert));

        let right_tags: Vec<InlineTag> = right.iter().map(|s| s.tag).collect();
        assert!(right_tags.contains(&InlineTag::Equal));
        assert!(right_tags.contains(&InlineTag::Insert));
        assert!(!right_tags.contains(&InlineTag::Delete));
    }

    // -- Multiple word changes --

    #[test]
    fn multiple_word_changes() {
        let (left, right) = inline_diff("the quick brown fox", "the slow red fox");
        assert_eq!(spans_to_string(&left), "the quick brown fox");
        assert_eq!(spans_to_string(&right), "the slow red fox");

        // "the" and "fox" should be equal on both sides
        let left_equal: Vec<&InlineSpan> =
            left.iter().filter(|s| s.tag == InlineTag::Equal).collect();
        let right_equal: Vec<&InlineSpan> =
            right.iter().filter(|s| s.tag == InlineTag::Equal).collect();

        let left_equal_text: String = left_equal.iter().map(|s| s.text.as_str()).collect();
        let right_equal_text: String = right_equal.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(left_equal_text, right_equal_text);
    }

    // -- Whitespace-only changes --

    #[test]
    fn whitespace_only_change() {
        let (left, right) = inline_diff("a  b", "a b");
        assert_eq!(spans_to_string(&left), "a  b");
        assert_eq!(spans_to_string(&right), "a b");
    }

    // -- Empty line edge cases --

    #[test]
    fn empty_old_line() {
        let (left, right) = inline_diff("", "added text");
        assert_eq!(spans_to_string(&left), "");
        assert_eq!(spans_to_string(&right), "added text");
        assert!(left.is_empty());
        assert!(!right.is_empty());
        assert!(right.iter().all(|s| s.tag == InlineTag::Insert));
    }

    #[test]
    fn empty_new_line() {
        let (left, right) = inline_diff("removed text", "");
        assert_eq!(spans_to_string(&left), "removed text");
        assert_eq!(spans_to_string(&right), "");
        assert!(!left.is_empty());
        assert!(right.is_empty());
        assert!(left.iter().all(|s| s.tag == InlineTag::Delete));
    }

    #[test]
    fn both_empty() {
        let (left, right) = inline_diff("", "");
        assert!(left.is_empty());
        assert!(right.is_empty());
    }

    // -- Identical lines --

    #[test]
    fn identical_lines() {
        let (left, right) = inline_diff("no change here", "no change here");
        assert_eq!(spans_to_string(&left), "no change here");
        assert_eq!(spans_to_string(&right), "no change here");
        assert!(left.iter().all(|s| s.tag == InlineTag::Equal));
        assert!(right.iter().all(|s| s.tag == InlineTag::Equal));
    }

    // -- Entire line replacement --

    #[test]
    fn entire_line_replacement() {
        let (left, right) = inline_diff("completely old", "totally new");
        assert_eq!(spans_to_string(&left), "completely old");
        assert_eq!(spans_to_string(&right), "totally new");
        // Left should have some deletes, right some inserts
        assert!(left.iter().any(|s| s.tag == InlineTag::Delete));
        assert!(right.iter().any(|s| s.tag == InlineTag::Insert));
    }

    // -- Partial word changes (code-like) --

    #[test]
    fn function_name_change() {
        let (left, right) =
            inline_diff("    old_function();", "    new_function();");
        assert_eq!(spans_to_string(&left), "    old_function();");
        assert_eq!(spans_to_string(&right), "    new_function();");

        // The "    " prefix and "();" suffix should be equal
        let left_has_equal = left.iter().any(|s| s.tag == InlineTag::Equal);
        let left_has_delete = left.iter().any(|s| s.tag == InlineTag::Delete);
        assert!(left_has_equal);
        assert!(left_has_delete);
    }

    #[test]
    fn code_value_change() {
        let (left, right) = inline_diff("let x = 42;", "let x = 99;");
        assert_eq!(spans_to_string(&left), "let x = 42;");
        assert_eq!(spans_to_string(&right), "let x = 99;");

        // "let x = " and ";" should be equal
        let left_equal_text: String = left
            .iter()
            .filter(|s| s.tag == InlineTag::Equal)
            .map(|s| s.text.as_str())
            .collect();
        assert!(left_equal_text.contains("let"));
        assert!(left_equal_text.contains("x"));
    }

    // -- Spans reconstruct original text --

    #[test]
    fn spans_reconstruct_left() {
        let (left, _) = inline_diff("fn foo(x: i32)", "fn bar(x: u64)");
        assert_eq!(spans_to_string(&left), "fn foo(x: i32)");
    }

    #[test]
    fn spans_reconstruct_right() {
        let (_, right) = inline_diff("fn foo(x: i32)", "fn bar(x: u64)");
        assert_eq!(spans_to_string(&right), "fn bar(x: u64)");
    }

    // -- Addition at end --

    #[test]
    fn addition_at_end() {
        let (left, right) = inline_diff("base", "base extra");
        assert_eq!(spans_to_string(&left), "base");
        assert_eq!(spans_to_string(&right), "base extra");
    }

    // -- Addition at beginning --

    #[test]
    fn addition_at_beginning() {
        let (left, right) = inline_diff("end", "start end");
        assert_eq!(spans_to_string(&left), "end");
        assert_eq!(spans_to_string(&right), "start end");
    }
}

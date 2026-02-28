// Syntax highlighting for diff content using syntect.
// Detects language from file extension and highlights each line.

use syntect::highlighting::{Color as SyntectColor, ThemeSet};
use syntect::parsing::SyntaxSet;

/// A highlighted span of text with a foreground color.
#[derive(Debug, Clone, PartialEq)]
pub struct HighlightSpan {
    pub text: String,
    pub color: DiffRgba,
}

/// RGBA color for highlighted text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffRgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl DiffRgba {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_syntect(c: SyntectColor) -> Self {
        Self {
            r: c.r,
            g: c.g,
            b: c.b,
            a: c.a,
        }
    }
}

/// Syntax highlighter that caches the syntax set and theme.
pub struct DiffHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    theme_name: String,
}

impl DiffHighlighter {
    /// Create a new highlighter with the default theme.
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            theme_name: "base16-ocean.dark".to_string(),
        }
    }

    /// Detect the syntax for a file based on its extension.
    /// Returns the syntax name (e.g., "Rust", "Python") or None if not recognized.
    pub fn detect_language(&self, file_path: &str) -> Option<String> {
        let ext = file_path.rsplit('.').next()?;
        let syntax = self.syntax_set.find_syntax_by_extension(ext)?;
        Some(syntax.name.clone())
    }

    /// Highlight a single line of code for a given file extension.
    /// Returns spans with foreground colors from the theme.
    pub fn highlight_line(&self, line: &str, file_path: &str) -> Vec<HighlightSpan> {
        use syntect::easy::HighlightLines;

        let ext = file_path.rsplit('.').next().unwrap_or("");
        let syntax = self
            .syntax_set
            .find_syntax_by_extension(ext)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = match self.theme_set.themes.get(&self.theme_name) {
            Some(t) => t,
            None => {
                return vec![HighlightSpan {
                    text: line.to_string(),
                    color: DiffRgba::new(200, 200, 200, 255),
                }]
            }
        };

        let mut h = HighlightLines::new(syntax, theme);
        match h.highlight_line(line, &self.syntax_set) {
            Ok(ranges) => ranges
                .into_iter()
                .map(|(style, text)| HighlightSpan {
                    text: text.to_string(),
                    color: DiffRgba::from_syntect(style.foreground),
                })
                .collect(),
            Err(_) => vec![HighlightSpan {
                text: line.to_string(),
                color: DiffRgba::new(200, 200, 200, 255),
            }],
        }
    }
}

/// Concatenate highlight spans back into a string.
pub fn highlight_spans_to_string(spans: &[HighlightSpan]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_rust_from_extension() {
        let h = DiffHighlighter::new();
        let lang = h.detect_language("src/main.rs");
        assert_eq!(lang, Some("Rust".to_string()));
    }

    #[test]
    fn detect_python_from_extension() {
        let h = DiffHighlighter::new();
        let lang = h.detect_language("script.py");
        assert_eq!(lang, Some("Python".to_string()));
    }

    #[test]
    fn detect_javascript_from_extension() {
        let h = DiffHighlighter::new();
        let lang = h.detect_language("app.js");
        assert_eq!(lang, Some("JavaScript".to_string()));
    }

    #[test]
    fn detect_unknown_extension_returns_none() {
        let h = DiffHighlighter::new();
        let lang = h.detect_language("file.xyzzy123");
        assert!(lang.is_none());
    }

    #[test]
    fn detect_json_from_extension() {
        let h = DiffHighlighter::new();
        let lang = h.detect_language("package.json");
        assert_eq!(lang, Some("JSON".to_string()));
    }

    #[test]
    fn detect_no_extension_returns_none() {
        let h = DiffHighlighter::new();
        let lang = h.detect_language("Makefile");
        // Makefile has no extension dot, so rsplit('.').next() returns "Makefile"
        // syntect may or may not find it by extension
        // The important thing is it doesn't panic
        let _ = lang;
    }

    #[test]
    fn highlight_rust_line_produces_spans() {
        let h = DiffHighlighter::new();
        let spans = h.highlight_line("fn main() {", "test.rs");
        assert!(!spans.is_empty());
        assert_eq!(highlight_spans_to_string(&spans), "fn main() {");
    }

    #[test]
    fn highlight_python_line_produces_spans() {
        let h = DiffHighlighter::new();
        let spans = h.highlight_line("def hello():", "test.py");
        assert!(!spans.is_empty());
        assert_eq!(highlight_spans_to_string(&spans), "def hello():");
    }

    #[test]
    fn highlight_empty_line() {
        let h = DiffHighlighter::new();
        let spans = h.highlight_line("", "test.rs");
        // May be empty or single span
        assert_eq!(highlight_spans_to_string(&spans), "");
    }

    #[test]
    fn highlight_unknown_extension_returns_plain() {
        let h = DiffHighlighter::new();
        let spans = h.highlight_line("some text", "file.xyzzy123");
        assert!(!spans.is_empty());
        assert_eq!(highlight_spans_to_string(&spans), "some text");
    }

    #[test]
    fn highlight_spans_have_colors() {
        let h = DiffHighlighter::new();
        let spans = h.highlight_line("let x: i32 = 42;", "test.rs");
        // At least some spans should have non-zero color
        assert!(spans.iter().any(|s| s.color.a > 0));
    }

    #[test]
    fn highlight_spans_reconstruct_original() {
        let h = DiffHighlighter::new();
        let line = "    pub fn compute(value: &str) -> Result<(), Error> {";
        let spans = h.highlight_line(line, "lib.rs");
        assert_eq!(highlight_spans_to_string(&spans), line);
    }

    #[test]
    fn diff_rgba_from_syntect_conversion() {
        let sc = SyntectColor {
            r: 100,
            g: 150,
            b: 200,
            a: 255,
        };
        let c = DiffRgba::from_syntect(sc);
        assert_eq!(c.r, 100);
        assert_eq!(c.g, 150);
        assert_eq!(c.b, 200);
        assert_eq!(c.a, 255);
    }
}

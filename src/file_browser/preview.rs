// File preview — content loading, type detection, and syntax highlighting
// for the file browser's right panel.

use std::path::{Path, PathBuf};
use std::time::SystemTime;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;

/// Maximum file size for full text loading (1 MB).
pub const LARGE_FILE_THRESHOLD: u64 = 1_048_576;

/// Maximum number of lines to load from a large file.
pub const LARGE_FILE_MAX_LINES: usize = 5_000;

/// Number of bytes to sample for binary detection.
const BINARY_DETECT_BYTES: usize = 8192;

/// Row height for the preview's virtual scrolling.
pub const PREVIEW_ROW_HEIGHT: f32 = 20.0;

/// What kind of content a file contains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileContentType {
    /// Displayable text (code, config, prose).
    Text,
    /// Image (png, jpg, gif, svg, webp).
    Image,
    /// Binary data that cannot be previewed as text.
    Binary,
}

/// Metadata about a previewed file.
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub file_name: String,
    pub path: PathBuf,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

/// A single span of styled text within a highlighted line.
#[derive(Debug, Clone)]
pub struct StyledSpan {
    pub text: String,
    pub style: Style,
}

/// A line of syntax-highlighted text.
#[derive(Debug, Clone)]
pub struct HighlightedLine {
    pub spans: Vec<StyledSpan>,
}

/// State for the file preview panel.
#[derive(Debug, Clone)]
pub struct FilePreview {
    pub content_type: FileContentType,
    pub metadata: FileMetadata,
    /// Raw text lines (for text files).
    pub lines: Vec<String>,
    /// Syntax-highlighted lines (parallel to `lines`).
    pub highlighted_lines: Vec<HighlightedLine>,
    /// Whether the file was truncated because it exceeded the size threshold.
    pub truncated: bool,
    /// Syntax name detected (e.g. "Rust", "Python").
    pub syntax_name: Option<String>,
    /// Image dimensions (width, height) if it's an image file.
    pub image_dimensions: Option<(u32, u32)>,
}

/// Detect the content type of a file from its extension.
pub fn detect_content_type(path: &Path) -> FileContentType {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match ext.as_deref() {
        Some("png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "bmp" | "ico" | "tiff" | "tif") => {
            FileContentType::Image
        }
        Some("exe" | "dll" | "so" | "dylib" | "o" | "a" | "lib"
            | "bin" | "dat" | "db" | "sqlite" | "sqlite3"
            | "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar"
            | "wasm" | "class" | "pyc" | "pyo") => {
            FileContentType::Binary
        }
        _ => FileContentType::Text,
    }
}

/// Check if raw bytes look like binary content (contains null bytes in the sample).
pub fn is_binary_content(data: &[u8]) -> bool {
    let sample = &data[..data.len().min(BINARY_DETECT_BYTES)];
    sample.contains(&0)
}

/// Detect a syntax definition for a file path using syntect.
pub fn detect_syntax_name(ss: &SyntaxSet, path: &Path) -> Option<String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let syntax = ss.find_syntax_by_extension(ext);
    syntax.map(|s| s.name.clone())
}

/// Format a file size for human display.
pub fn format_file_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Format a line number for gutter display, right-aligned to a given width.
pub fn format_line_number(line: usize, max_line: usize) -> String {
    let width = max_line.max(1).to_string().len();
    format!("{line:>width$}")
}

/// Compute visible line range for preview virtual scrolling.
/// Returns (start, end) exclusive end.
pub fn preview_visible_range(
    scroll_offset: f32,
    viewport_height: f32,
    total_lines: usize,
) -> (usize, usize) {
    if total_lines == 0 || viewport_height <= 0.0 {
        return (0, 0);
    }
    let start = (scroll_offset / PREVIEW_ROW_HEIGHT).floor() as usize;
    let visible_count = (viewport_height / PREVIEW_ROW_HEIGHT).ceil() as usize + 1;
    let end = (start + visible_count).min(total_lines);
    let start = start.min(total_lines);
    (start, end)
}

/// A text selection range within the preview content.
#[derive(Debug, Clone, PartialEq)]
pub struct TextSelection {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

impl TextSelection {
    /// Returns (start_line, start_col, end_line, end_col) with start always before end.
    pub fn normalized(&self) -> (usize, usize, usize, usize) {
        if self.start_line < self.end_line
            || (self.start_line == self.end_line && self.start_col <= self.end_col)
        {
            (self.start_line, self.start_col, self.end_line, self.end_col)
        } else {
            (self.end_line, self.end_col, self.start_line, self.start_col)
        }
    }

    /// Extract selected text from line data.
    pub fn extract_text(&self, lines: &[String]) -> String {
        let (sl, sc, el, ec) = self.normalized();
        if lines.is_empty() || sl >= lines.len() {
            return String::new();
        }
        let el = el.min(lines.len() - 1);

        if sl == el {
            // Single line selection
            let line = &lines[sl];
            let sc = sc.min(line.len());
            let ec = ec.min(line.len());
            line[sc..ec].to_string()
        } else {
            // Multi-line selection
            let mut result = String::new();
            // First line: from start_col to end
            let first = &lines[sl];
            let sc = sc.min(first.len());
            result.push_str(&first[sc..]);
            // Middle lines: full content
            for line in &lines[sl + 1..el] {
                result.push('\n');
                result.push_str(line);
            }
            // Last line: from start to end_col
            result.push('\n');
            let last = &lines[el];
            let ec = ec.min(last.len());
            result.push_str(&last[..ec]);
            result
        }
    }

    /// Does selection touch this line?
    pub fn contains_line(&self, line: usize) -> bool {
        let (sl, _, el, _) = self.normalized();
        line >= sl && line <= el
    }

    /// Column range selected on this line, if any.
    pub fn line_selection_range(&self, line: usize, line_len: usize) -> Option<(usize, usize)> {
        let (sl, sc, el, ec) = self.normalized();
        if line < sl || line > el {
            return None;
        }
        if sl == el {
            // Single line selection
            Some((sc.min(line_len), ec.min(line_len)))
        } else if line == sl {
            Some((sc.min(line_len), line_len))
        } else if line == el {
            Some((0, ec.min(line_len)))
        } else {
            // Middle line: fully selected
            Some((0, line_len))
        }
    }

    /// Same start and end position.
    pub fn is_empty(&self) -> bool {
        self.start_line == self.end_line && self.start_col == self.end_col
    }
}

/// Convert pixel coordinates to a (line, col) text position in the preview.
pub fn pixel_to_text_position(
    click_x: f32,
    click_y: f32,
    scroll_offset: f32,
    cell_width: f32,
    cell_height: f32,
    gutter_width: f32,
    total_lines: usize,
) -> (usize, usize) {
    let line = ((click_y + scroll_offset) / cell_height).floor() as usize;
    let line = if total_lines == 0 { 0 } else { line.min(total_lines - 1) };
    let col = ((click_x - gutter_width).max(0.0) / cell_width).floor() as usize;
    (line, col)
}

/// State for the preview panel's scroll and word-wrap toggle.
#[derive(Debug, Clone)]
pub struct PreviewViewState {
    pub scroll_offset: f32,
    pub word_wrap: bool,
    pub selection: Option<TextSelection>,
    pub mouse_down: bool,
}

impl PreviewViewState {
    pub fn new() -> Self {
        Self {
            scroll_offset: 0.0,
            word_wrap: false,
            selection: None,
            mouse_down: false,
        }
    }

    /// Scroll by delta, clamped to valid range.
    pub fn scroll_by(&mut self, delta: f32, total_lines: usize, viewport_height: f32) {
        let total_height = total_lines as f32 * PREVIEW_ROW_HEIGHT;
        let max_offset = (total_height - viewport_height).max(0.0);
        self.scroll_offset = (self.scroll_offset + delta).clamp(0.0, max_offset);
    }

    /// Start a new selection at the given line and column.
    pub fn handle_mouse_down(&mut self, line: usize, col: usize) {
        self.mouse_down = true;
        self.selection = Some(TextSelection {
            start_line: line,
            start_col: col,
            end_line: line,
            end_col: col,
        });
    }

    /// Extend the current selection to the given line and column.
    pub fn handle_mouse_drag(&mut self, line: usize, col: usize) {
        if self.mouse_down {
            if let Some(ref mut sel) = self.selection {
                sel.end_line = line;
                sel.end_col = col;
            }
        }
    }

    /// Stop dragging (selection persists).
    pub fn handle_mouse_up(&mut self) {
        self.mouse_down = false;
    }

    /// Clear selection and stop dragging.
    pub fn clear_selection(&mut self) {
        self.selection = None;
        self.mouse_down = false;
    }

    /// Copy the selected text from lines, if any non-empty selection exists.
    pub fn copy_selection(&self, lines: &[String]) -> Option<String> {
        self.selection.as_ref().and_then(|sel| {
            if sel.is_empty() {
                None
            } else {
                Some(sel.extract_text(lines))
            }
        })
    }

    /// Select all text in the preview.
    pub fn select_all(&mut self, lines: &[String]) {
        if lines.is_empty() {
            self.selection = None;
            return;
        }
        let last_line = lines.len() - 1;
        let last_col = lines[last_line].len();
        self.selection = Some(TextSelection {
            start_line: 0,
            start_col: 0,
            end_line: last_line,
            end_col: last_col,
        });
        self.mouse_down = false;
    }
}

impl FilePreview {
    /// Load a file preview from the given path.
    /// Detects file type, reads content, applies syntax highlighting for text files.
    pub fn load(path: &Path, ss: &SyntaxSet, ts: &ThemeSet) -> std::io::Result<Self> {
        let metadata_fs = std::fs::metadata(path)?;
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let meta = FileMetadata {
            file_name,
            path: path.to_path_buf(),
            size: metadata_fs.len(),
            modified: metadata_fs.modified().ok(),
        };

        let content_type = detect_content_type(path);

        match content_type {
            FileContentType::Image => {
                // Try to read image dimensions
                let dims = image_dimensions(path);
                Ok(FilePreview {
                    content_type: FileContentType::Image,
                    metadata: meta,
                    lines: Vec::new(),
                    highlighted_lines: Vec::new(),
                    truncated: false,
                    syntax_name: None,
                    image_dimensions: dims,
                })
            }
            FileContentType::Binary => Ok(FilePreview {
                content_type: FileContentType::Binary,
                metadata: meta,
                lines: Vec::new(),
                highlighted_lines: Vec::new(),
                truncated: false,
                syntax_name: None,
                image_dimensions: None,
            }),
            FileContentType::Text => {
                // Check if file is too large
                let truncated = metadata_fs.len() > LARGE_FILE_THRESHOLD;

                // Read file content
                let raw = if truncated {
                    // Read only the first portion
                    read_lines_limited(path, LARGE_FILE_MAX_LINES)?
                } else {
                    std::fs::read(path)?
                };

                // Check for binary content in text files
                if is_binary_content(&raw) {
                    return Ok(FilePreview {
                        content_type: FileContentType::Binary,
                        metadata: meta,
                        lines: Vec::new(),
                        highlighted_lines: Vec::new(),
                        truncated: false,
                        syntax_name: None,
                        image_dimensions: None,
                    });
                }

                let text = String::from_utf8_lossy(&raw);
                let lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();

                // Syntax highlighting
                let syntax_name = detect_syntax_name(ss, path);
                let highlighted_lines = highlight_lines(ss, ts, path, &lines);

                Ok(FilePreview {
                    content_type: FileContentType::Text,
                    metadata: meta,
                    lines,
                    highlighted_lines,
                    truncated,
                    syntax_name,
                    image_dimensions: None,
                })
            }
        }
    }
}

/// Read a limited number of lines from a file.
fn read_lines_limited(path: &Path, max_lines: usize) -> std::io::Result<Vec<u8>> {
    use std::io::{BufRead, BufReader};
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut result = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        if i >= max_lines {
            break;
        }
        let line = line?;
        result.extend_from_slice(line.as_bytes());
        result.push(b'\n');
    }
    Ok(result)
}

/// Try to read image dimensions without loading the full image.
fn image_dimensions(path: &Path) -> Option<(u32, u32)> {
    image::image_dimensions(path).ok()
}

/// Highlight lines using syntect.
fn highlight_lines(
    ss: &SyntaxSet,
    ts: &ThemeSet,
    path: &Path,
    lines: &[String],
) -> Vec<HighlightedLine> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let syntax = ss
        .find_syntax_by_extension(ext)
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    let theme = ts
        .themes
        .get("base16-ocean.dark")
        .or_else(|| ts.themes.values().next())
        .expect("syntect must have at least one theme");

    let mut highlighter = syntect::easy::HighlightLines::new(syntax, theme);
    let mut result = Vec::with_capacity(lines.len());

    for line in lines {
        let ranges = highlighter
            .highlight_line(line, ss)
            .unwrap_or_default();

        let spans = ranges
            .into_iter()
            .map(|(style, text)| StyledSpan {
                text: text.to_string(),
                style,
            })
            .collect();

        result.push(HighlightedLine { spans });
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // --- File type detection ---

    #[test]
    fn detect_text_file_rs() {
        assert_eq!(
            detect_content_type(Path::new("main.rs")),
            FileContentType::Text
        );
    }

    #[test]
    fn detect_text_file_py() {
        assert_eq!(
            detect_content_type(Path::new("script.py")),
            FileContentType::Text
        );
    }

    #[test]
    fn detect_text_file_toml() {
        assert_eq!(
            detect_content_type(Path::new("Cargo.toml")),
            FileContentType::Text
        );
    }

    #[test]
    fn detect_text_file_no_extension() {
        assert_eq!(
            detect_content_type(Path::new("Makefile")),
            FileContentType::Text
        );
    }

    #[test]
    fn detect_image_png() {
        assert_eq!(
            detect_content_type(Path::new("photo.png")),
            FileContentType::Image
        );
    }

    #[test]
    fn detect_image_jpg() {
        assert_eq!(
            detect_content_type(Path::new("photo.jpg")),
            FileContentType::Image
        );
    }

    #[test]
    fn detect_image_svg() {
        assert_eq!(
            detect_content_type(Path::new("icon.svg")),
            FileContentType::Image
        );
    }

    #[test]
    fn detect_image_webp() {
        assert_eq!(
            detect_content_type(Path::new("image.webp")),
            FileContentType::Image
        );
    }

    #[test]
    fn detect_binary_exe() {
        assert_eq!(
            detect_content_type(Path::new("program.exe")),
            FileContentType::Binary
        );
    }

    #[test]
    fn detect_binary_zip() {
        assert_eq!(
            detect_content_type(Path::new("archive.zip")),
            FileContentType::Binary
        );
    }

    #[test]
    fn detect_binary_wasm() {
        assert_eq!(
            detect_content_type(Path::new("module.wasm")),
            FileContentType::Binary
        );
    }

    #[test]
    fn detect_binary_so() {
        assert_eq!(
            detect_content_type(Path::new("libfoo.so")),
            FileContentType::Binary
        );
    }

    #[test]
    fn detect_image_case_insensitive() {
        assert_eq!(
            detect_content_type(Path::new("photo.PNG")),
            FileContentType::Image
        );
        assert_eq!(
            detect_content_type(Path::new("photo.Jpg")),
            FileContentType::Image
        );
    }

    // --- Binary content detection ---

    #[test]
    fn binary_detection_text_content() {
        assert!(!is_binary_content(b"Hello, world!\n"));
    }

    #[test]
    fn binary_detection_null_bytes() {
        assert!(is_binary_content(b"Hello\x00world"));
    }

    #[test]
    fn binary_detection_empty() {
        assert!(!is_binary_content(b""));
    }

    #[test]
    fn binary_detection_utf8() {
        assert!(!is_binary_content("Rsti pls!".as_bytes()));
    }

    // --- Syntax detection ---

    #[test]
    fn syntax_detection_rust() {
        let ss = SyntaxSet::load_defaults_newlines();
        let name = detect_syntax_name(&ss, Path::new("main.rs"));
        assert_eq!(name.as_deref(), Some("Rust"));
    }

    #[test]
    fn syntax_detection_python() {
        let ss = SyntaxSet::load_defaults_newlines();
        let name = detect_syntax_name(&ss, Path::new("script.py"));
        assert_eq!(name.as_deref(), Some("Python"));
    }

    #[test]
    fn syntax_detection_json() {
        let ss = SyntaxSet::load_defaults_newlines();
        let name = detect_syntax_name(&ss, Path::new("data.json"));
        assert_eq!(name.as_deref(), Some("JSON"));
    }

    #[test]
    fn syntax_detection_unknown_ext() {
        let ss = SyntaxSet::load_defaults_newlines();
        let name = detect_syntax_name(&ss, Path::new("file.xyzabc"));
        assert!(name.is_none());
    }

    #[test]
    fn syntax_detection_no_ext() {
        let ss = SyntaxSet::load_defaults_newlines();
        let name = detect_syntax_name(&ss, Path::new("Makefile"));
        // syntect may or may not recognize Makefile — just check it doesn't crash
        let _ = name;
    }

    // --- File size formatting ---

    #[test]
    fn format_size_bytes() {
        assert_eq!(format_file_size(0), "0 B");
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(1023), "1023 B");
    }

    #[test]
    fn format_size_kb() {
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(1536), "1.5 KB");
    }

    #[test]
    fn format_size_mb() {
        assert_eq!(format_file_size(1_048_576), "1.0 MB");
        assert_eq!(format_file_size(2_621_440), "2.5 MB");
    }

    #[test]
    fn format_size_gb() {
        assert_eq!(format_file_size(1_073_741_824), "1.0 GB");
    }

    // --- Line number formatting ---

    #[test]
    fn line_number_single_digit() {
        assert_eq!(format_line_number(1, 9), "1");
        assert_eq!(format_line_number(9, 9), "9");
    }

    #[test]
    fn line_number_padded() {
        assert_eq!(format_line_number(1, 100), "  1");
        assert_eq!(format_line_number(99, 100), " 99");
        assert_eq!(format_line_number(100, 100), "100");
    }

    #[test]
    fn line_number_thousands() {
        assert_eq!(format_line_number(1, 9999), "   1");
        assert_eq!(format_line_number(9999, 9999), "9999");
    }

    // --- Preview visible range ---

    #[test]
    fn preview_visible_range_from_top() {
        let (start, end) = preview_visible_range(0.0, 200.0, 100);
        assert_eq!(start, 0);
        assert_eq!(end, 11); // ceil(200/20) + 1 = 11
    }

    #[test]
    fn preview_visible_range_scrolled() {
        let (start, end) = preview_visible_range(40.0, 200.0, 100);
        assert_eq!(start, 2); // floor(40/20)
        assert_eq!(end, 13); // 2 + 11
    }

    #[test]
    fn preview_visible_range_clamped() {
        let (start, end) = preview_visible_range(0.0, 1000.0, 5);
        assert_eq!(start, 0);
        assert_eq!(end, 5);
    }

    #[test]
    fn preview_visible_range_empty() {
        let (start, end) = preview_visible_range(0.0, 200.0, 0);
        assert_eq!(start, 0);
        assert_eq!(end, 0);
    }

    // --- Preview view state ---

    #[test]
    fn preview_view_state_defaults() {
        let state = PreviewViewState::new();
        assert!((state.scroll_offset - 0.0).abs() < f32::EPSILON);
        assert!(!state.word_wrap);
    }

    #[test]
    fn preview_scroll_clamps_to_zero() {
        let mut state = PreviewViewState::new();
        state.scroll_by(-100.0, 10, 200.0);
        assert!((state.scroll_offset - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn preview_scroll_clamps_to_max() {
        let mut state = PreviewViewState::new();
        // 10 lines * 20px = 200px total, viewport 100px, max = 100
        state.scroll_by(500.0, 10, 100.0);
        assert!((state.scroll_offset - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn preview_scroll_when_content_fits() {
        let mut state = PreviewViewState::new();
        // 5 lines * 20px = 100px total, viewport 200px, max = 0
        state.scroll_by(100.0, 5, 200.0);
        assert!((state.scroll_offset - 0.0).abs() < f32::EPSILON);
    }

    // --- File loading (integration tests with temp files) ---

    #[test]
    fn load_text_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("hello.rs");
        std::fs::write(&file_path, "fn main() {\n    println!(\"Hello\");\n}\n").unwrap();

        let ss = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        let preview = FilePreview::load(&file_path, &ss, &ts).unwrap();

        assert_eq!(preview.content_type, FileContentType::Text);
        assert_eq!(preview.lines.len(), 3);
        assert_eq!(preview.highlighted_lines.len(), 3);
        assert!(!preview.truncated);
        assert_eq!(preview.syntax_name.as_deref(), Some("Rust"));
        assert_eq!(preview.metadata.file_name, "hello.rs");
    }

    #[test]
    fn load_detects_binary_content() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("data.txt");
        let mut content = vec![0u8; 100];
        content[0] = b'H';
        content[1] = b'i';
        content[50] = 0; // null byte makes it binary
        std::fs::write(&file_path, &content).unwrap();

        let ss = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        let preview = FilePreview::load(&file_path, &ss, &ts).unwrap();

        assert_eq!(preview.content_type, FileContentType::Binary);
    }

    #[test]
    fn load_image_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.png");
        // Write a minimal valid PNG (1x1 pixel)
        let img = image::RgbaImage::new(1, 1);
        img.save(&file_path).unwrap();

        let ss = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        let preview = FilePreview::load(&file_path, &ss, &ts).unwrap();

        assert_eq!(preview.content_type, FileContentType::Image);
        assert_eq!(preview.image_dimensions, Some((1, 1)));
        assert!(preview.lines.is_empty());
    }

    #[test]
    fn load_binary_by_extension() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("app.exe");
        std::fs::write(&file_path, b"MZ\x00\x00").unwrap();

        let ss = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        let preview = FilePreview::load(&file_path, &ss, &ts).unwrap();

        assert_eq!(preview.content_type, FileContentType::Binary);
    }

    #[test]
    fn load_nonexistent_file_errors() {
        let ss = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        let result = FilePreview::load(Path::new("/nonexistent/file.rs"), &ss, &ts);
        assert!(result.is_err());
    }

    #[test]
    fn load_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("empty.txt");
        std::fs::write(&file_path, "").unwrap();

        let ss = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        let preview = FilePreview::load(&file_path, &ss, &ts).unwrap();

        assert_eq!(preview.content_type, FileContentType::Text);
        assert!(preview.lines.is_empty());
        assert!(!preview.truncated);
    }

    #[test]
    fn highlighted_lines_have_spans() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.rs");
        std::fs::write(&file_path, "let x = 42;\n").unwrap();

        let ss = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        let preview = FilePreview::load(&file_path, &ss, &ts).unwrap();

        assert!(!preview.highlighted_lines.is_empty());
        // Each line should have at least one span
        for hl in &preview.highlighted_lines {
            assert!(!hl.spans.is_empty());
        }
    }

    #[test]
    fn file_metadata_captures_size() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("sized.txt");
        std::fs::write(&file_path, "12345").unwrap();

        let ss = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        let preview = FilePreview::load(&file_path, &ss, &ts).unwrap();

        assert_eq!(preview.metadata.size, 5);
        assert!(preview.metadata.modified.is_some());
    }

    #[test]
    fn file_metadata_path_matches() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("meta.rs");
        std::fs::write(&file_path, "// test").unwrap();

        let ss = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        let preview = FilePreview::load(&file_path, &ss, &ts).unwrap();

        assert_eq!(preview.metadata.path, file_path);
    }

    #[test]
    fn large_file_constant_is_1mb() {
        assert_eq!(LARGE_FILE_THRESHOLD, 1_048_576);
    }

    #[test]
    fn image_detection_gif() {
        assert_eq!(
            detect_content_type(Path::new("anim.gif")),
            FileContentType::Image
        );
    }

    #[test]
    fn image_detection_bmp() {
        assert_eq!(
            detect_content_type(Path::new("old.bmp")),
            FileContentType::Image
        );
    }

    #[test]
    fn text_detection_md() {
        assert_eq!(
            detect_content_type(Path::new("README.md")),
            FileContentType::Text
        );
    }

    #[test]
    fn text_detection_html() {
        assert_eq!(
            detect_content_type(Path::new("page.html")),
            FileContentType::Text
        );
    }

    #[test]
    fn text_detection_css() {
        assert_eq!(
            detect_content_type(Path::new("style.css")),
            FileContentType::Text
        );
    }

    #[test]
    fn text_detection_yaml() {
        assert_eq!(
            detect_content_type(Path::new("config.yaml")),
            FileContentType::Text
        );
    }

    #[test]
    fn binary_detection_dylib() {
        assert_eq!(
            detect_content_type(Path::new("lib.dylib")),
            FileContentType::Binary
        );
    }

    #[test]
    fn syntax_detection_js() {
        let ss = SyntaxSet::load_defaults_newlines();
        let name = detect_syntax_name(&ss, Path::new("app.js"));
        assert_eq!(name.as_deref(), Some("JavaScript"));
    }

    #[test]
    fn syntax_detection_toml() {
        let ss = SyntaxSet::load_defaults_newlines();
        let name = detect_syntax_name(&ss, Path::new("config.toml"));
        // syntect's default set may not include TOML — just verify no crash
        let _ = name;
    }

    // --- Word wrap toggle ---

    #[test]
    fn preview_word_wrap_toggle() {
        let mut state = PreviewViewState::new();
        assert!(!state.word_wrap);
        state.word_wrap = true;
        assert!(state.word_wrap);
        state.word_wrap = false;
        assert!(!state.word_wrap);
    }

    // --- Text selection ---

    #[test]
    fn test_text_selection_extract_single_line() {
        let sel = TextSelection { start_line: 0, start_col: 5, end_line: 0, end_col: 10 };
        let lines = vec!["Hello, World!".to_string()];
        assert_eq!(sel.extract_text(&lines), ", Wor");
    }

    #[test]
    fn test_text_selection_extract_multi_line() {
        let sel = TextSelection { start_line: 0, start_col: 5, end_line: 2, end_col: 3 };
        let lines = vec!["Hello, World!".into(), "Line two".into(), "Line three".into()];
        assert_eq!(sel.extract_text(&lines), ", World!\nLine two\nLin");
    }

    #[test]
    fn test_text_selection_normalized_forward() {
        let sel = TextSelection { start_line: 0, start_col: 5, end_line: 2, end_col: 3 };
        assert_eq!(sel.normalized(), (0, 5, 2, 3));
    }

    #[test]
    fn test_text_selection_normalized_backward() {
        let sel = TextSelection { start_line: 2, start_col: 3, end_line: 0, end_col: 5 };
        assert_eq!(sel.normalized(), (0, 5, 2, 3));
    }

    #[test]
    fn test_text_selection_contains_line() {
        let sel = TextSelection { start_line: 2, start_col: 0, end_line: 5, end_col: 10 };
        assert!(!sel.contains_line(1));
        assert!(sel.contains_line(2));
        assert!(sel.contains_line(3));
        assert!(sel.contains_line(5));
        assert!(!sel.contains_line(6));
    }

    #[test]
    fn test_text_selection_line_range_middle() {
        let sel = TextSelection { start_line: 1, start_col: 5, end_line: 3, end_col: 8 };
        // Middle line (line 2) is fully selected
        assert_eq!(sel.line_selection_range(2, 20), Some((0, 20)));
    }

    #[test]
    fn test_text_selection_line_range_start_line() {
        let sel = TextSelection { start_line: 1, start_col: 5, end_line: 3, end_col: 8 };
        assert_eq!(sel.line_selection_range(1, 20), Some((5, 20)));
    }

    #[test]
    fn test_text_selection_line_range_end_line() {
        let sel = TextSelection { start_line: 1, start_col: 5, end_line: 3, end_col: 8 };
        assert_eq!(sel.line_selection_range(3, 20), Some((0, 8)));
    }

    #[test]
    fn test_text_selection_line_range_single_line() {
        let sel = TextSelection { start_line: 2, start_col: 5, end_line: 2, end_col: 10 };
        assert_eq!(sel.line_selection_range(2, 20), Some((5, 10)));
    }

    #[test]
    fn test_text_selection_is_empty() {
        let sel = TextSelection { start_line: 2, start_col: 5, end_line: 2, end_col: 5 };
        assert!(sel.is_empty());
        let sel2 = TextSelection { start_line: 2, start_col: 5, end_line: 2, end_col: 6 };
        assert!(!sel2.is_empty());
    }

    #[test]
    fn test_pixel_to_text_position_basic() {
        let (line, col) = pixel_to_text_position(
            100.0, 45.0, // click position
            0.0,         // no scroll
            8.0, 20.0,   // cell_width, cell_height
            40.0,        // gutter_width
            100,         // total_lines
        );
        assert_eq!(line, 2);  // 45.0 / 20.0 = 2.25 -> 2
        assert_eq!(col, 7);   // (100.0 - 40.0) / 8.0 = 7.5 -> 7
    }

    #[test]
    fn test_pixel_to_text_position_with_scroll() {
        let (line, _) = pixel_to_text_position(
            50.0, 10.0,
            40.0,        // scrolled down 2 lines
            8.0, 20.0,
            40.0,
            100,
        );
        assert_eq!(line, 2);  // (10.0 + 40.0) / 20.0 = 2.5 -> 2
    }

    #[test]
    fn test_pixel_to_text_position_clamps() {
        let (line, col) = pixel_to_text_position(
            5.0, 9999.0,  // far below, in gutter
            0.0,
            8.0, 20.0,
            40.0,
            10,            // only 10 lines
        );
        assert_eq!(line, 9);  // clamped to total_lines - 1
        assert_eq!(col, 0);   // click_x < gutter_width -> 0
    }

    #[test]
    fn test_preview_mouse_down_starts_selection() {
        let mut view = PreviewViewState::new();
        view.handle_mouse_down(5, 10);
        assert!(view.mouse_down);
        assert_eq!(view.selection, Some(TextSelection { start_line: 5, start_col: 10, end_line: 5, end_col: 10 }));
    }

    #[test]
    fn test_preview_mouse_drag_extends_selection() {
        let mut view = PreviewViewState::new();
        view.handle_mouse_down(5, 10);
        view.handle_mouse_drag(7, 3);
        assert_eq!(view.selection.as_ref().unwrap().end_line, 7);
        assert_eq!(view.selection.as_ref().unwrap().end_col, 3);
    }

    #[test]
    fn test_preview_mouse_up_stops_dragging() {
        let mut view = PreviewViewState::new();
        view.handle_mouse_down(5, 10);
        view.handle_mouse_up();
        assert!(!view.mouse_down);
        assert!(view.selection.is_some()); // selection persists
    }

    #[test]
    fn test_copy_selection_returns_text() {
        let mut view = PreviewViewState::new();
        view.handle_mouse_down(0, 0);
        view.handle_mouse_drag(0, 5);
        view.handle_mouse_up();
        let lines = vec!["Hello, World!".to_string()];
        assert_eq!(view.copy_selection(&lines), Some("Hello".to_string()));
    }

    #[test]
    fn test_copy_empty_selection_returns_none() {
        let view = PreviewViewState::new();
        let lines = vec!["Hello".to_string()];
        assert_eq!(view.copy_selection(&lines), None);
    }

    #[test]
    fn test_clear_selection() {
        let mut view = PreviewViewState::new();
        view.handle_mouse_down(0, 0);
        view.handle_mouse_drag(5, 10);
        view.clear_selection();
        assert!(view.selection.is_none());
        assert!(!view.mouse_down);
    }

    #[test]
    fn test_select_all() {
        let mut view = PreviewViewState::new();
        let lines = vec![
            "First line".to_string(),
            "Second line".to_string(),
            "Third line".to_string(),
        ];
        view.select_all(&lines);
        assert_eq!(
            view.selection,
            Some(TextSelection {
                start_line: 0,
                start_col: 0,
                end_line: 2,
                end_col: 10, // "Third line".len()
            })
        );
        assert!(!view.mouse_down);
        assert_eq!(view.copy_selection(&lines), Some("First line\nSecond line\nThird line".to_string()));
    }

    #[test]
    fn test_select_all_empty_lines() {
        let mut view = PreviewViewState::new();
        let lines: Vec<String> = Vec::new();
        view.select_all(&lines);
        assert!(view.selection.is_none());
    }

    #[test]
    fn test_selection_cleared_on_new_preview() {
        let mut view = PreviewViewState::new();
        view.handle_mouse_down(0, 0);
        view.handle_mouse_drag(5, 10);
        assert!(view.selection.is_some());
        // Simulating what load_preview does: reset to new()
        view = PreviewViewState::new();
        assert!(view.selection.is_none());
        assert!(!view.mouse_down);
    }
}

# Track 30: File Browser — Viewer

## Source Requirements

From `new_feature` spec, Phase 3:
- Implement text file preview with syntax highlighting
- Implement image preview
- Handle binary files gracefully
- Add empty state
- Success: Click any file, see a beautiful syntax-highlighted preview instantly

## Cross-Cutting Constraints

- TDD workflow, all cross-cutting v1 constraints
- iced 0.14 widget system patterns
- Performance: async file reading, incremental syntax highlighting for large files
- Virtual scrolling for text content

## Dependencies

- Track 28 (Overlay Foundation) — provides SplitPanel right panel slot
- Track 29 (File Browser Nav) — provides file selection events

## What This Track Delivers

The right panel of the File Browser overlay: a file preview that renders syntax-highlighted text files, images, and graceful handling of binary/large files. Includes line numbers, scroll, word wrap toggle, and file metadata display.

## Scope IN

- Empty state: "Select a file to preview" centered dimmed text
- Text file preview with syntect syntax highlighting
- Line numbers in left gutter (dimmed, right-aligned)
- Monospace font (JetBrains Mono)
- Horizontal and vertical scrolling
- Word wrap toggle in panel header
- File name/path header at top
- File metadata: size, last modified, encoding
- Image preview: centered, dimensions + file size shown
- Binary file handling: "Binary file — cannot preview" message
- Large file handling: warning + partial load for files >1MB
- Virtual scrolling for text content (only render visible lines)
- Async file reading on background thread

## Scope OUT

- File editing (read-only viewer)
- Image zoom/pan (Track 35 polish)
- Context menus on file content (Track 31)

## Key Design Decisions

1. **syntect integration**: Load all syntax definitions at startup vs lazy-load per file type? syntect's SyntaxSet can be expensive to construct. Options: (a) load default set once at overlay open, (b) lazy-load per extension, (c) ship a pre-compiled binary syntax set.

2. **Rendering approach**: iced text widgets vs Canvas-based rendering for code? Canvas gives more control over line number gutters and intra-line styling. Text widgets are simpler but harder to customize.

3. **Large file strategy**: Stream first N lines vs mmap vs full read with pagination? Need to handle files from 1 line to millions of lines without blocking.

4. **Image rendering**: iced Image widget vs Canvas with texture? iced has an Image widget but integration with wgpu textures needs verification.

## Test Strategy

- Unit tests for syntax highlighting: correct language detection from extension
- Unit tests for line number formatting
- Unit tests for file size detection and large file threshold
- Unit tests for binary file detection
- Unit tests for file metadata extraction
- Unit tests for word wrap logic
- Unit tests for virtual scrolling (visible line range calculation)
- Integration test: selecting file triggers preview load
- Framework: `cargo test --lib`

## Complexity

M (Medium)

## Estimated Phases

3 phases:
1. Text file preview with syntax highlighting + line numbers
2. Image preview + binary file handling + large file warning
3. Virtual scrolling + word wrap + file metadata

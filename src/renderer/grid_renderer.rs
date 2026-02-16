// Terminal grid to instanced quad rendering.

use crate::config::theme::{Color, color_new};
use crate::renderer::glyph_atlas::GlyphAtlas;
use crate::renderer::gpu::CellInstance;

/// Cell attribute flags (bits 4+ to avoid conflict with renderer-internal flags 0-3).
pub const CELL_FLAG_UNDERLINE: u32 = 0x10; // bit 4
pub const CELL_FLAG_STRIKETHROUGH: u32 = 0x20; // bit 5
pub const CELL_FLAG_SELECTED: u32 = 0x40; // bit 6
pub const CELL_FLAG_VI_CURSOR: u32 = 0x80; // bit 7
pub const CELL_FLAG_BOLD: u32 = 0x100; // bit 8

/// A single cell in the terminal grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridCell {
    /// Character to display (space for empty cells).
    pub ch: char,
    /// Foreground color.
    pub fg: Color,
    /// Background color.
    pub bg: Color,
    /// Cell attribute flags (underline, strikethrough, etc.).
    pub flags: u32,
}

impl GridCell {
    /// Create a new grid cell.
    pub fn new(ch: char, fg: Color, bg: Color) -> Self {
        Self {
            ch,
            fg,
            bg,
            flags: 0,
        }
    }

    /// Create an empty cell with the given background color.
    pub fn empty(bg: Color) -> Self {
        Self {
            ch: ' ',
            fg: color_new(1.0, 1.0, 1.0, 1.0),
            bg,
            flags: 0,
        }
    }
}

/// Grid dimensions and cell sizing computed from window size and font metrics.
#[derive(Debug, Clone)]
pub struct GridDimensions {
    /// Number of columns that fit in the window.
    pub columns: u32,
    /// Number of rows that fit in the window.
    pub rows: u32,
    /// Cell width in physical pixels.
    pub cell_width: f32,
    /// Cell height in physical pixels.
    pub cell_height: f32,
    /// Window width in physical pixels.
    pub window_width: u32,
    /// Window height in physical pixels.
    pub window_height: u32,
}

impl GridDimensions {
    /// Calculate grid dimensions from window size and cell size.
    ///
    /// `window_width` and `window_height` are in physical pixels.
    /// `cell_width` and `cell_height` are in physical pixels (already DPI-scaled).
    pub fn new(window_width: u32, window_height: u32, cell_width: f32, cell_height: f32) -> Self {
        let columns = (window_width as f32 / cell_width).floor() as u32;
        let rows = (window_height as f32 / cell_height).floor() as u32;

        Self {
            columns: columns.max(1),
            rows: rows.max(1),
            cell_width,
            cell_height,
            window_width,
            window_height,
        }
    }

    /// Calculate grid dimensions from a pane rect (in physical pixels) and cell size.
    ///
    /// Uses float division (not u32 truncation) to match `grid_dims_for_rect`
    /// in window.rs. This ensures the renderer and PTY agree on column/row counts.
    pub fn from_pane_rect(rect: &crate::pane::Rect, cell_width: f32, cell_height: f32) -> Self {
        let columns = (rect.width / cell_width).floor().max(1.0) as u32;
        let rows = (rect.height / cell_height).floor().max(1.0) as u32;
        Self {
            columns,
            rows,
            cell_width,
            cell_height,
            window_width: rect.width as u32,
            window_height: rect.height as u32,
        }
    }

    /// Recalculate grid dimensions for a new window size, keeping the same cell size.
    pub fn resize(&mut self, window_width: u32, window_height: u32) {
        self.window_width = window_width;
        self.window_height = window_height;
        self.columns = (window_width as f32 / self.cell_width).floor().max(1.0) as u32;
        self.rows = (window_height as f32 / self.cell_height).floor().max(1.0) as u32;
    }

    /// Cell size in NDC units for the grid shader uniform.
    /// NDC x range is [-1, 1] = width 2.0, y range is [-1, 1] = width 2.0.
    pub fn cell_size_ndc(&self) -> [f32; 2] {
        [2.0 / self.columns as f32, 2.0 / self.rows as f32]
    }

    /// Grid size as [columns, rows] for the grid shader uniform.
    pub fn grid_size(&self) -> [f32; 2] {
        [self.columns as f32, self.rows as f32]
    }

    /// Total number of cells in the grid.
    pub fn total_cells(&self) -> u32 {
        self.columns * self.rows
    }
}

/// Byte offset into the instance buffer for a given row.
/// Each cell is one CellInstance (72 bytes), and each row has `cols` cells.
pub fn row_byte_offset(row: usize, cols: usize) -> u64 {
    (row * cols * std::mem::size_of::<CellInstance>()) as u64
}

/// Generate CellInstance data for a single row of the grid.
///
/// `cells` should have `grid.columns * grid.rows` entries, in row-major order.
/// Returns a Vec of CellInstance for the specified row only.
pub fn generate_row_instances(
    grid: &GridDimensions,
    cells: &[GridCell],
    atlas: &GlyphAtlas,
    row: u32,
) -> Vec<CellInstance> {
    let cols = grid.columns as usize;
    let start = row as usize * cols;
    let mut instances = Vec::with_capacity(cols);

    for col in 0..cols {
        let i = start + col;
        let cell = cells
            .get(i)
            .copied()
            .unwrap_or(GridCell::empty(color_new(0.0, 0.0, 0.0, 1.0)));

        let is_bold = cell.flags & CELL_FLAG_BOLD != 0;
        let (atlas_uv, has_glyph) = if cell.ch != ' ' {
            if let Some(info) = atlas.glyph_info(cell.ch, is_bold) {
                (info.uv, true)
            } else {
                ([0.0, 0.0, 0.0, 0.0], false)
            }
        } else {
            ([0.0, 0.0, 0.0, 0.0], false)
        };

        instances.push(CellInstance {
            position: [col as f32, row as f32],
            atlas_uv,
            fg_color: [cell.fg.r, cell.fg.g, cell.fg.b, cell.fg.a],
            bg_color: [cell.bg.r, cell.bg.g, cell.bg.b, cell.bg.a],
            flags: (if has_glyph { 1 } else { 0 }) | cell.flags,
            _padding: [0; 3],
        });
    }

    instances
}

/// Generate CellInstance data from a grid of cells and a glyph atlas.
///
/// `cells` should have `grid.columns * grid.rows` entries, in row-major order.
/// Returns a Vec of CellInstance ready for GPU upload.
pub fn generate_instances(
    grid: &GridDimensions,
    cells: &[GridCell],
    atlas: &GlyphAtlas,
) -> Vec<CellInstance> {
    let total = grid.total_cells() as usize;
    let mut instances = Vec::with_capacity(total);

    for i in 0..total {
        let col = (i as u32) % grid.columns;
        let row = (i as u32) / grid.columns;

        let cell = cells
            .get(i)
            .copied()
            .unwrap_or(GridCell::empty(color_new(0.0, 0.0, 0.0, 1.0)));

        let is_bold = cell.flags & CELL_FLAG_BOLD != 0;
        let (atlas_uv, has_glyph) = if cell.ch != ' ' {
            if let Some(info) = atlas.glyph_info(cell.ch, is_bold) {
                (info.uv, true)
            } else {
                ([0.0, 0.0, 0.0, 0.0], false)
            }
        } else {
            ([0.0, 0.0, 0.0, 0.0], false)
        };

        instances.push(CellInstance {
            position: [col as f32, row as f32],
            atlas_uv,
            fg_color: [cell.fg.r, cell.fg.g, cell.fg.b, cell.fg.a],
            bg_color: [cell.bg.r, cell.bg.g, cell.bg.b, cell.bg.a],
            flags: (if has_glyph { 1 } else { 0 }) | cell.flags,
            _padding: [0; 3],
        });
    }

    instances
}

/// Generate a test pattern grid for visual verification.
///
/// Layout:
/// - Row 0: "VeloTerm v0.1.0" in accent color
/// - Row 1: Empty
/// - Row 2: Full ASCII printable range (0x20-0x7E)
/// - Row 3: "claude@anthropic ~ $" with prompt colors
/// - Remaining: Alternating characters for cell alignment validation
pub fn generate_test_pattern(
    grid: &GridDimensions,
    theme: &crate::config::theme::TerminalTheme,
) -> Vec<GridCell> {
    let total = grid.total_cells() as usize;
    let cols = grid.columns as usize;
    let mut cells = vec![GridCell::empty(theme.bg_deep); total];

    // Row 0: VeloTerm header in accent color
    let header = "VeloTerm v0.1.0";
    for (i, ch) in header.chars().enumerate() {
        if i < cols {
            cells[i] = GridCell::new(ch, theme.accent_orange, theme.bg_deep);
        }
    }

    // Row 1: empty (already filled with empty cells)

    // Row 2: ASCII printable range
    if grid.rows > 2 {
        let row_start = 2 * cols;
        for (i, byte) in (0x20u8..=0x7Eu8).enumerate() {
            if i < cols {
                cells[row_start + i] =
                    GridCell::new(byte as char, theme.text_primary, theme.bg_deep);
            }
        }
    }

    // Row 3: prompt line
    if grid.rows > 3 {
        let row_start = 3 * cols;
        let prompt = "claude@anthropic ~ $";
        for (i, ch) in prompt.chars().enumerate() {
            if i < cols {
                cells[row_start + i] = GridCell::new(ch, theme.accent_orange, theme.bg_deep);
            }
        }
    }

    // Remaining rows: alternating characters
    for row in 4..grid.rows as usize {
        let row_start = row * cols;
        for col in 0..cols {
            let ch = if (row + col) % 2 == 0 { '#' } else { '.' };
            cells[row_start + col] = GridCell::new(ch, theme.text_secondary, theme.bg_deep);
        }
    }

    cells
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::glyph_atlas::GlyphAtlas;

    fn test_atlas() -> GlyphAtlas {
        GlyphAtlas::new(13.0, 1.0, "JetBrains Mono", 1.5)
    }

    fn test_fg() -> Color {
        crate::config::theme::from_hex("#E8E5DF")
    }

    fn test_bg() -> Color {
        crate::config::theme::from_hex("#1A1816")
    }

    // ── Grid dimension calculation ──────────────────────────────────

    #[test]
    fn grid_columns_from_window_width_and_cell_width() {
        // 1280px window / 16px cell = 80 columns
        let grid = GridDimensions::new(1280, 720, 16.0, 42.0);
        assert_eq!(grid.columns, 80);
    }

    #[test]
    fn grid_rows_from_window_height_and_cell_height() {
        // 720px window / 42px cell = 17.14... = 17 rows
        let grid = GridDimensions::new(1280, 720, 16.0, 42.0);
        assert_eq!(grid.rows, 17);
    }

    #[test]
    fn grid_floor_partial_columns() {
        // 1290px / 16px = 80.625 → floor = 80
        let grid = GridDimensions::new(1290, 720, 16.0, 42.0);
        assert_eq!(grid.columns, 80);
    }

    #[test]
    fn grid_floor_partial_rows() {
        // 730px / 42px = 17.38... → floor = 17
        let grid = GridDimensions::new(1280, 730, 16.0, 42.0);
        assert_eq!(grid.rows, 17);
    }

    #[test]
    fn grid_minimum_one_column() {
        // Very narrow window — must have at least 1 column
        let grid = GridDimensions::new(5, 720, 16.0, 42.0);
        assert_eq!(grid.columns, 1);
    }

    #[test]
    fn grid_minimum_one_row() {
        // Very short window — must have at least 1 row
        let grid = GridDimensions::new(1280, 10, 16.0, 42.0);
        assert_eq!(grid.rows, 1);
    }

    // ── Grid recalculation on resize ────────────────────────────────

    #[test]
    fn grid_resize_updates_columns() {
        let mut grid = GridDimensions::new(1280, 720, 16.0, 42.0);
        assert_eq!(grid.columns, 80);
        grid.resize(1920, 720);
        assert_eq!(grid.columns, 120);
    }

    #[test]
    fn grid_resize_updates_rows() {
        let mut grid = GridDimensions::new(1280, 720, 16.0, 42.0);
        assert_eq!(grid.rows, 17);
        grid.resize(1280, 1080);
        assert_eq!(grid.rows, 25); // 1080 / 42 = 25.71 → 25
    }

    #[test]
    fn grid_resize_preserves_cell_size() {
        let mut grid = GridDimensions::new(1280, 720, 16.0, 42.0);
        grid.resize(1920, 1080);
        assert_eq!(grid.cell_width, 16.0);
        assert_eq!(grid.cell_height, 42.0);
    }

    #[test]
    fn grid_resize_updates_window_dimensions() {
        let mut grid = GridDimensions::new(1280, 720, 16.0, 42.0);
        grid.resize(1920, 1080);
        assert_eq!(grid.window_width, 1920);
        assert_eq!(grid.window_height, 1080);
    }

    // ── DPI-scaled cell sizing ──────────────────────────────────────

    #[test]
    fn grid_with_retina_cell_size() {
        // Retina: 2560x1440 physical, cell 32x84 (doubled from 16x42)
        let grid = GridDimensions::new(2560, 1440, 32.0, 84.0);
        // Same logical grid as 1280x720 at 1x
        assert_eq!(grid.columns, 80);
        assert_eq!(grid.rows, 17);
    }

    #[test]
    fn grid_stores_cell_dimensions() {
        let grid = GridDimensions::new(1280, 720, 16.0, 42.0);
        assert_eq!(grid.cell_width, 16.0);
        assert_eq!(grid.cell_height, 42.0);
    }

    // ── NDC conversion ──────────────────────────────────────────────

    #[test]
    fn cell_size_ndc_covers_viewport() {
        let grid = GridDimensions::new(1280, 720, 16.0, 42.0);
        let ndc = grid.cell_size_ndc();
        // 80 columns * cell_ndc_x should equal 2.0 (full NDC width)
        let total_x = grid.columns as f32 * ndc[0];
        let total_y = grid.rows as f32 * ndc[1];
        assert!((total_x - 2.0).abs() < 1e-6, "NDC x total: {}", total_x);
        assert!((total_y - 2.0).abs() < 1e-6, "NDC y total: {}", total_y);
    }

    #[test]
    fn grid_size_matches_columns_rows() {
        let grid = GridDimensions::new(1280, 720, 16.0, 42.0);
        let size = grid.grid_size();
        assert_eq!(size[0], 80.0);
        assert_eq!(size[1], 17.0);
    }

    #[test]
    fn total_cells_is_columns_times_rows() {
        let grid = GridDimensions::new(1280, 720, 16.0, 42.0);
        assert_eq!(grid.total_cells(), 80 * 17);
    }

    // ── Cell instance buffer generation ─────────────────────────────

    // Use integer cell sizes to avoid float truncation issues in tests
    fn test_grid(cols: u32, rows: u32) -> GridDimensions {
        GridDimensions::new(cols * 10, rows * 20, 10.0, 20.0)
    }

    #[test]
    fn generate_instances_returns_correct_count() {
        let atlas = test_atlas();
        let grid = test_grid(4, 3);
        let cells: Vec<GridCell> = (0..grid.total_cells())
            .map(|_| GridCell::empty(test_bg()))
            .collect();
        let instances = generate_instances(&grid, &cells, &atlas);
        assert_eq!(instances.len(), 12); // 4 * 3
    }

    #[test]
    fn generate_instances_position_is_col_row() {
        let atlas = test_atlas();
        let grid = test_grid(3, 2);
        let cells: Vec<GridCell> = (0..grid.total_cells())
            .map(|_| GridCell::empty(test_bg()))
            .collect();
        let instances = generate_instances(&grid, &cells, &atlas);
        // First row: (0,0), (1,0), (2,0)
        assert_eq!(instances[0].position, [0.0, 0.0]);
        assert_eq!(instances[1].position, [1.0, 0.0]);
        assert_eq!(instances[2].position, [2.0, 0.0]);
        // Second row: (0,1), (1,1), (2,1)
        assert_eq!(instances[3].position, [0.0, 1.0]);
    }

    #[test]
    fn generate_instances_glyph_sets_has_glyph_flag() {
        let atlas = test_atlas();
        let grid = test_grid(2, 1);
        let cells = vec![
            GridCell::new('A', test_fg(), test_bg()),
            GridCell::empty(test_bg()),
        ];
        let instances = generate_instances(&grid, &cells, &atlas);
        assert_eq!(instances[0].flags & 1, 1, "'A' should have has_glyph flag");
        assert_eq!(instances[1].flags & 1, 0, "space should not have has_glyph");
    }

    #[test]
    fn generate_instances_uv_from_atlas() {
        let atlas = test_atlas();
        let grid = test_grid(1, 1);
        let cells = vec![GridCell::new('A', test_fg(), test_bg())];
        let instances = generate_instances(&grid, &cells, &atlas);
        let expected_uv = atlas.glyph_info('A', false).unwrap().uv;
        assert_eq!(instances[0].atlas_uv, expected_uv);
    }

    #[test]
    fn generate_instances_space_has_zero_uv() {
        let atlas = test_atlas();
        let grid = test_grid(1, 1);
        let cells = vec![GridCell::empty(test_bg())];
        let instances = generate_instances(&grid, &cells, &atlas);
        assert_eq!(instances[0].atlas_uv, [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn generate_instances_fg_color_from_cell() {
        let atlas = test_atlas();
        let fg = crate::config::theme::from_hex("#FF0000");
        let grid = test_grid(1, 1);
        let cells = vec![GridCell::new('X', fg, test_bg())];
        let instances = generate_instances(&grid, &cells, &atlas);
        assert_eq!(instances[0].fg_color, [fg.r, fg.g, fg.b, fg.a]);
    }

    #[test]
    fn generate_instances_bg_color_from_cell() {
        let atlas = test_atlas();
        let bg = crate::config::theme::from_hex("#00FF00");
        let grid = test_grid(1, 1);
        let cells = vec![GridCell::new('X', test_fg(), bg)];
        let instances = generate_instances(&grid, &cells, &atlas);
        assert_eq!(instances[0].bg_color, [bg.r, bg.g, bg.b, bg.a]);
    }

    #[test]
    fn generate_instances_unknown_char_no_glyph() {
        let atlas = test_atlas();
        let grid = test_grid(1, 1);
        // Control char not in atlas
        let cells = vec![GridCell::new('\x01', test_fg(), test_bg())];
        let instances = generate_instances(&grid, &cells, &atlas);
        assert_eq!(instances[0].flags & 1, 0);
        assert_eq!(instances[0].atlas_uv, [0.0, 0.0, 0.0, 0.0]);
    }

    // ── Cell flag propagation ─────────────────────────────────────

    #[test]
    fn generate_instances_propagates_underline_flag() {
        let atlas = test_atlas();
        let grid = test_grid(1, 1);
        let mut cell = GridCell::new('X', test_fg(), test_bg());
        cell.flags = CELL_FLAG_UNDERLINE;
        let instances = generate_instances(&grid, &[cell], &atlas);
        assert_ne!(instances[0].flags & CELL_FLAG_UNDERLINE, 0);
    }

    #[test]
    fn generate_instances_propagates_bold_flag() {
        let atlas = test_atlas();
        let grid = test_grid(1, 1);
        let mut cell = GridCell::new('X', test_fg(), test_bg());
        cell.flags = CELL_FLAG_BOLD;
        let instances = generate_instances(&grid, &[cell], &atlas);
        assert_ne!(instances[0].flags & CELL_FLAG_BOLD, 0);
    }

    #[test]
    fn generate_instances_bold_uses_different_uv() {
        let atlas = test_atlas();
        let grid = test_grid(2, 1);
        let mut regular = GridCell::new('A', test_fg(), test_bg());
        regular.flags = 0;
        let mut bold = GridCell::new('A', test_fg(), test_bg());
        bold.flags = CELL_FLAG_BOLD;
        let instances = generate_instances(&grid, &[regular, bold], &atlas);
        // Bold 'A' should use a different atlas UV than regular 'A'
        assert_ne!(
            instances[0].atlas_uv, instances[1].atlas_uv,
            "bold 'A' should have different atlas UV than regular 'A'"
        );
    }

    #[test]
    fn generate_instances_propagates_strikethrough_flag() {
        let atlas = test_atlas();
        let grid = test_grid(1, 1);
        let mut cell = GridCell::new('X', test_fg(), test_bg());
        cell.flags = CELL_FLAG_STRIKETHROUGH;
        let instances = generate_instances(&grid, &[cell], &atlas);
        assert_ne!(instances[0].flags & CELL_FLAG_STRIKETHROUGH, 0);
    }

    // ── Test pattern generation ─────────────────────────────────────

    fn test_theme() -> crate::config::theme::TerminalTheme {
        crate::config::theme::TerminalTheme::warm_dark()
    }

    #[test]
    fn test_pattern_returns_correct_cell_count() {
        let grid = test_grid(80, 24);
        let theme = test_theme();
        let cells = generate_test_pattern(&grid, &theme);
        assert_eq!(cells.len(), 80 * 24);
    }

    #[test]
    fn test_pattern_row0_has_veloterm_header() {
        let grid = test_grid(80, 24);
        let theme = test_theme();
        let cells = generate_test_pattern(&grid, &theme);
        let header: String = cells[..15].iter().map(|c| c.ch).collect();
        assert_eq!(header, "VeloTerm v0.1.0");
    }

    #[test]
    fn test_pattern_row0_header_uses_accent_color() {
        let grid = test_grid(80, 24);
        let theme = test_theme();
        let cells = generate_test_pattern(&grid, &theme);
        // 'V' in row 0 should use accent color
        assert_eq!(cells[0].fg, theme.accent_orange);
    }

    #[test]
    fn test_pattern_row1_is_empty() {
        let grid = test_grid(80, 24);
        let theme = test_theme();
        let cells = generate_test_pattern(&grid, &theme);
        // Row 1 should all be spaces
        for i in 80..160 {
            assert_eq!(cells[i].ch, ' ', "row 1, col {} should be space", i - 80);
        }
    }

    #[test]
    fn test_pattern_row2_has_ascii_range() {
        let grid = test_grid(80, 24);
        let theme = test_theme();
        let cells = generate_test_pattern(&grid, &theme);
        let row2_start = 2 * 80;
        // First char is space (0x20), then '!' (0x21), etc.
        assert_eq!(cells[row2_start].ch, ' ');
        assert_eq!(cells[row2_start + 1].ch, '!');
        assert_eq!(cells[row2_start + 33].ch, 'A'); // 0x41 - 0x20 = 33
    }

    #[test]
    fn test_pattern_row2_uses_text_primary() {
        let grid = test_grid(80, 24);
        let theme = test_theme();
        let cells = generate_test_pattern(&grid, &theme);
        let row2_start = 2 * 80;
        assert_eq!(cells[row2_start + 1].fg, theme.text_primary);
    }

    #[test]
    fn test_pattern_row3_has_prompt() {
        let grid = test_grid(80, 24);
        let theme = test_theme();
        let cells = generate_test_pattern(&grid, &theme);
        let row3_start = 3 * 80;
        let prompt: String = cells[row3_start..row3_start + 20]
            .iter()
            .map(|c| c.ch)
            .collect();
        assert_eq!(prompt, "claude@anthropic ~ $");
    }

    #[test]
    fn test_pattern_row3_uses_prompt_color() {
        let grid = test_grid(80, 24);
        let theme = test_theme();
        let cells = generate_test_pattern(&grid, &theme);
        let row3_start = 3 * 80;
        assert_eq!(cells[row3_start].fg, theme.accent_orange);
    }

    #[test]
    fn test_pattern_remaining_rows_alternate() {
        let grid = test_grid(80, 24);
        let theme = test_theme();
        let cells = generate_test_pattern(&grid, &theme);
        // Row 4, col 0: (4+0)%2 == 0 → '#'
        let row4_start = 4 * 80;
        assert_eq!(cells[row4_start].ch, '#');
        assert_eq!(cells[row4_start + 1].ch, '.');
    }

    #[test]
    fn test_pattern_remaining_uses_text_muted() {
        let grid = test_grid(80, 24);
        let theme = test_theme();
        let cells = generate_test_pattern(&grid, &theme);
        let row4_start = 4 * 80;
        assert_eq!(cells[row4_start].fg, theme.text_secondary);
    }

    // ── Row byte offset and partial instance generation ──────────────

    #[test]
    fn row_byte_offset_for_row_zero_is_zero() {
        assert_eq!(row_byte_offset(0, 80), 0);
    }

    #[test]
    fn row_byte_offset_for_row_n_is_n_times_cols_times_72() {
        // row 5, 80 columns: 5 * 80 * 72 = 28800
        assert_eq!(row_byte_offset(5, 80), 28800);
    }

    #[test]
    fn row_byte_offset_with_different_cols() {
        // row 1, 4 columns: 1 * 4 * 72 = 288
        assert_eq!(row_byte_offset(1, 4), 288);
    }

    #[test]
    fn generate_row_instances_returns_cols_instances() {
        let atlas = test_atlas();
        let grid = test_grid(4, 3);
        let cells: Vec<GridCell> = (0..grid.total_cells())
            .map(|_| GridCell::empty(test_bg()))
            .collect();
        let row_instances = generate_row_instances(&grid, &cells, &atlas, 0);
        assert_eq!(row_instances.len(), 4);
    }

    #[test]
    fn generate_row_instances_has_correct_positions() {
        let atlas = test_atlas();
        let grid = test_grid(3, 2);
        let cells: Vec<GridCell> = (0..grid.total_cells())
            .map(|_| GridCell::empty(test_bg()))
            .collect();
        let row1 = generate_row_instances(&grid, &cells, &atlas, 1);
        assert_eq!(row1[0].position, [0.0, 1.0]);
        assert_eq!(row1[1].position, [1.0, 1.0]);
        assert_eq!(row1[2].position, [2.0, 1.0]);
    }

    #[test]
    fn generate_row_instances_matches_full_generate() {
        let atlas = test_atlas();
        let grid = test_grid(4, 3);
        let cells = vec![
            GridCell::new('A', test_fg(), test_bg()),
            GridCell::new('B', test_fg(), test_bg()),
            GridCell::new('C', test_fg(), test_bg()),
            GridCell::new('D', test_fg(), test_bg()),
            GridCell::new('E', test_fg(), test_bg()),
            GridCell::new('F', test_fg(), test_bg()),
            GridCell::new('G', test_fg(), test_bg()),
            GridCell::new('H', test_fg(), test_bg()),
            GridCell::new('I', test_fg(), test_bg()),
            GridCell::new('J', test_fg(), test_bg()),
            GridCell::new('K', test_fg(), test_bg()),
            GridCell::new('L', test_fg(), test_bg()),
        ];
        let full = generate_instances(&grid, &cells, &atlas);
        for row in 0..3 {
            let row_insts = generate_row_instances(&grid, &cells, &atlas, row as u32);
            let start = row * 4;
            assert_eq!(
                row_insts,
                &full[start..start + 4],
                "row {} instances should match full generate",
                row
            );
        }
    }

    // ── Pane grid dimension tests ──────────────────────────────────────

    #[test]
    fn pane_grid_dimensions_from_pane_rect() {
        let rect = crate::pane::Rect::new(0.0, 0.0, 640.0, 360.0);
        let grid = GridDimensions::from_pane_rect(&rect, 10.0, 20.0);
        assert_eq!(grid.columns, 64); // 640 / 10
        assert_eq!(grid.rows, 18); // 360 / 20
    }

    #[test]
    fn per_pane_instances_have_correct_positions() {
        let atlas = test_atlas();
        let rect = crate::pane::Rect::new(100.0, 200.0, 30.0, 40.0);
        let grid = GridDimensions::from_pane_rect(&rect, 10.0, 20.0);
        assert_eq!(grid.columns, 3);
        assert_eq!(grid.rows, 2);
        let cells: Vec<GridCell> = (0..grid.total_cells())
            .map(|_| GridCell::empty(test_bg()))
            .collect();
        let instances = generate_instances(&grid, &cells, &atlas);
        // Positions are in grid-local coordinates (0-based col/row)
        assert_eq!(instances[0].position, [0.0, 0.0]);
        assert_eq!(instances[1].position, [1.0, 0.0]);
        assert_eq!(instances[2].position, [2.0, 0.0]);
        assert_eq!(instances[3].position, [0.0, 1.0]);
    }

    #[test]
    fn generate_instances_coordinates_within_bounds() {
        let atlas = test_atlas();
        let grid = test_grid(80, 24);
        let theme = crate::config::theme::TerminalTheme::warm_dark();
        let cells = generate_test_pattern(&grid, &theme);
        let instances = generate_instances(&grid, &cells, &atlas);

        for (i, inst) in instances.iter().enumerate() {
            let col = inst.position[0];
            let row = inst.position[1];

            assert!(
                col >= 0.0 && col < grid.columns as f32,
                "cell {} col {} out of bounds (max {})",
                i,
                col,
                grid.columns
            );
            assert!(
                row >= 0.0 && row < grid.rows as f32,
                "cell {} row {} out of bounds (max {})",
                i,
                row,
                grid.rows
            );

            // UV within [0, 1] if glyph present
            if inst.flags & 1 != 0 {
                let [u, v, w, h] = inst.atlas_uv;
                assert!(
                    u >= 0.0 && u + w <= 1.0 + 1e-6,
                    "cell {} UV u={} w={} out of bounds",
                    i,
                    u,
                    w
                );
                assert!(
                    v >= 0.0 && v + h <= 1.0 + 1e-6,
                    "cell {} UV v={} h={} out of bounds",
                    i,
                    v,
                    h
                );
            }
        }

        // No duplicate positions
        let mut positions: Vec<(u32, u32)> = instances
            .iter()
            .map(|inst| (inst.position[0] as u32, inst.position[1] as u32))
            .collect();
        let before = positions.len();
        positions.sort();
        positions.dedup();
        assert_eq!(before, positions.len(), "duplicate cell positions found");
    }

    #[test]
    fn cell_size_ndc_fills_viewport_exactly() {
        let grid = test_grid(80, 24);
        let [ndc_w, ndc_h] = grid.cell_size_ndc();

        let total_w = ndc_w * grid.columns as f32;
        let total_h = ndc_h * grid.rows as f32;

        assert!(
            (total_w - 2.0).abs() < 1e-5,
            "NDC width {} * {} cols = {} (expected 2.0)",
            ndc_w,
            grid.columns,
            total_w
        );
        assert!(
            (total_h - 2.0).abs() < 1e-5,
            "NDC height {} * {} rows = {} (expected 2.0)",
            ndc_h,
            grid.rows,
            total_h
        );
    }

    #[test]
    fn two_panes_produce_separate_instance_vecs() {
        let atlas = test_atlas();
        let rect_a = crate::pane::Rect::new(0.0, 0.0, 30.0, 20.0);
        let rect_b = crate::pane::Rect::new(30.0, 0.0, 20.0, 20.0);
        let grid_a = GridDimensions::from_pane_rect(&rect_a, 10.0, 20.0);
        let grid_b = GridDimensions::from_pane_rect(&rect_b, 10.0, 20.0);

        let cells_a: Vec<GridCell> = (0..grid_a.total_cells())
            .map(|_| GridCell::new('A', test_fg(), test_bg()))
            .collect();
        let cells_b: Vec<GridCell> = (0..grid_b.total_cells())
            .map(|_| GridCell::new('B', test_fg(), test_bg()))
            .collect();

        let inst_a = generate_instances(&grid_a, &cells_a, &atlas);
        let inst_b = generate_instances(&grid_b, &cells_b, &atlas);

        assert_eq!(inst_a.len(), 3); // 3 cols x 1 row
        assert_eq!(inst_b.len(), 2); // 2 cols x 1 row
                                     // They should be independent
        assert_ne!(inst_a.len(), inst_b.len());
    }
}

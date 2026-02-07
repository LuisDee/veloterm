// Terminal grid to instanced quad rendering.

use crate::config::theme::Color;
use crate::renderer::glyph_atlas::GlyphAtlas;
use crate::renderer::gpu::CellInstance;

/// A single cell in the terminal grid.
#[derive(Debug, Clone, Copy)]
pub struct GridCell {
    /// Character to display (space for empty cells).
    pub ch: char,
    /// Foreground color.
    pub fg: Color,
    /// Background color.
    pub bg: Color,
}

impl GridCell {
    /// Create a new grid cell.
    pub fn new(ch: char, fg: Color, bg: Color) -> Self {
        Self { ch, fg, bg }
    }

    /// Create an empty cell with the given background color.
    pub fn empty(bg: Color) -> Self {
        Self {
            ch: ' ',
            fg: Color::new(1.0, 1.0, 1.0, 1.0),
            bg,
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
            .unwrap_or(GridCell::empty(Color::new(0.0, 0.0, 0.0, 1.0)));

        let (atlas_uv, has_glyph) = if cell.ch != ' ' {
            if let Some(info) = atlas.glyph_info(cell.ch) {
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
            flags: if has_glyph { 1 } else { 0 },
            _padding: [0; 3],
        });
    }

    instances
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::glyph_atlas::GlyphAtlas;

    fn test_atlas() -> GlyphAtlas {
        GlyphAtlas::new(13.0, 1.0)
    }

    fn test_fg() -> Color {
        Color::from_hex("#E8E5DF")
    }

    fn test_bg() -> Color {
        Color::from_hex("#1A1816")
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
        let expected_uv = atlas.glyph_info('A').unwrap().uv;
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
        let fg = Color::from_hex("#FF0000");
        let grid = test_grid(1, 1);
        let cells = vec![GridCell::new('X', fg, test_bg())];
        let instances = generate_instances(&grid, &cells, &atlas);
        assert_eq!(instances[0].fg_color, [fg.r, fg.g, fg.b, fg.a]);
    }

    #[test]
    fn generate_instances_bg_color_from_cell() {
        let atlas = test_atlas();
        let bg = Color::from_hex("#00FF00");
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
}

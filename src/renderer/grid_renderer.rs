// Terminal grid to instanced quad rendering.

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

#[cfg(test)]
mod tests {
    use super::*;

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
}

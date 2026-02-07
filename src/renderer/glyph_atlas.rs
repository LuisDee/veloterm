// Glyph rasterization and GPU texture atlas.

use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache};
use std::collections::HashMap;

/// Metadata for a single glyph in the atlas.
#[derive(Debug, Clone)]
pub struct GlyphInfo {
    /// UV rect in atlas: [u, v, width, height], normalized to [0, 1].
    pub uv: [f32; 4],
}

/// Rasterized glyph atlas for GPU text rendering.
///
/// Contains an R8 texture with all ASCII printable glyphs (0x20..=0x7E)
/// pre-rendered in cell-sized slots, ready for GPU upload.
pub struct GlyphAtlas {
    /// R8 pixel data for the atlas texture (one byte per pixel, glyph mask).
    pub atlas_data: Vec<u8>,
    /// Atlas texture width in pixels (power of two).
    pub atlas_width: u32,
    /// Atlas texture height in pixels (power of two).
    pub atlas_height: u32,
    /// Cell width in pixels (monospace advance width at the given scale).
    pub cell_width: f32,
    /// Cell height in pixels (line height at the given scale).
    pub cell_height: f32,
    glyphs: HashMap<char, GlyphInfo>,
}

impl GlyphAtlas {
    /// Rasterize ASCII printable glyphs into a texture atlas.
    ///
    /// `font_size` is the base font size in points (e.g., 13.0).
    /// `scale_factor` is the DPI scale (e.g., 2.0 for Retina).
    pub fn new(font_size: f32, scale_factor: f32) -> Self {
        let scaled_size = font_size * scale_factor;
        let line_height = (scaled_size * 1.6).ceil();

        let mut font_system = FontSystem::new();
        let mut swash_cache = SwashCache::new();
        let metrics = Metrics::new(scaled_size, line_height);
        let attrs = Attrs::new().family(Family::Monospace);

        // Determine cell width from font metrics by measuring a reference glyph
        let mut buffer = Buffer::new(&mut font_system, metrics);
        buffer.set_text(&mut font_system, "M", attrs, Shaping::Advanced);
        buffer.set_size(
            &mut font_system,
            Some(scaled_size * 4.0),
            Some(line_height * 2.0),
        );
        buffer.shape_until_scroll(&mut font_system, true);

        let cell_width = buffer
            .layout_runs()
            .next()
            .and_then(|run| run.glyphs.first())
            .map(|g| g.w)
            .unwrap_or(scaled_size * 0.6);

        let cell_height = line_height;
        let slot_w = cell_width.ceil() as u32;
        let slot_h = cell_height.ceil() as u32;

        // Atlas layout: 16 glyphs per row, ceil(95/16) = 6 rows
        let glyph_count = 95u32;
        let cols = 16u32;
        let rows = glyph_count.div_ceil(cols);
        let atlas_width = (cols * slot_w).next_power_of_two().max(512);
        let atlas_height = (rows * slot_h).next_power_of_two().max(512);

        let mut atlas_data = vec![0u8; (atlas_width * atlas_height) as usize];
        let mut glyphs = HashMap::with_capacity(glyph_count as usize);

        let white = cosmic_text::Color::rgb(0xFF, 0xFF, 0xFF);

        for (i, byte) in (0x20u8..=0x7Eu8).enumerate() {
            let c = byte as char;
            let col = (i as u32) % cols;
            let row = (i as u32) / cols;
            let slot_x = col * slot_w;
            let slot_y = row * slot_h;

            buffer.set_text(&mut font_system, &c.to_string(), attrs, Shaping::Advanced);
            buffer.set_size(
                &mut font_system,
                Some(cell_width * 2.0),
                Some(cell_height * 2.0),
            );
            buffer.shape_until_scroll(&mut font_system, true);

            let aw = atlas_width;
            let sw = slot_w;
            let sh = slot_h;
            buffer.draw(
                &mut font_system,
                &mut swash_cache,
                white,
                |x, y, _w, _h, color| {
                    if x >= 0 && y >= 0 {
                        let xu = x as u32;
                        let yu = y as u32;
                        // Clamp to slot boundaries to prevent bleed into adjacent glyphs
                        if xu < sw && yu < sh {
                            let ax = slot_x + xu;
                            let ay = slot_y + yu;
                            let idx = (ay * aw + ax) as usize;
                            // Coverage/alpha is in the alpha channel, not red
                            atlas_data[idx] = atlas_data[idx].max(color.a());
                        }
                    }
                },
            );

            glyphs.insert(
                c,
                GlyphInfo {
                    uv: [
                        slot_x as f32 / atlas_width as f32,
                        slot_y as f32 / atlas_height as f32,
                        slot_w as f32 / atlas_width as f32,
                        slot_h as f32 / atlas_height as f32,
                    ],
                },
            );
        }

        Self {
            atlas_data,
            atlas_width,
            atlas_height,
            cell_width,
            cell_height,
            glyphs,
        }
    }

    /// Look up glyph metadata for a character.
    pub fn glyph_info(&self, c: char) -> Option<&GlyphInfo> {
        self.glyphs.get(&c)
    }

    /// Dump atlas as PGM (grayscale) image for debugging.
    pub fn dump_pgm(&self, path: &str) -> std::io::Result<()> {
        use std::io::Write;
        let mut f = std::fs::File::create(path)?;
        write!(f, "P5\n{} {}\n255\n", self.atlas_width, self.atlas_height)?;
        f.write_all(&self.atlas_data)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_atlas() -> GlyphAtlas {
        GlyphAtlas::new(13.0, 2.0)
    }

    // ── Font loading ────────────────────────────────────────────────

    #[test]
    fn atlas_creates_successfully() {
        let atlas = create_test_atlas();
        assert!(atlas.atlas_width > 0);
        assert!(atlas.atlas_height > 0);
    }

    // ── Glyph metrics ───────────────────────────────────────────────

    #[test]
    fn atlas_cell_width_is_reasonable() {
        let atlas = create_test_atlas();
        // At 26px (13 * 2.0), monospace cell width is typically 15-20px
        assert!(
            atlas.cell_width > 5.0,
            "cell_width {} too small",
            atlas.cell_width
        );
        assert!(
            atlas.cell_width < 50.0,
            "cell_width {} too large",
            atlas.cell_width
        );
    }

    #[test]
    fn atlas_cell_height_is_reasonable() {
        let atlas = create_test_atlas();
        // line_height = ceil(26 * 1.6) = 42
        assert!(
            atlas.cell_height > 10.0,
            "cell_height {} too small",
            atlas.cell_height
        );
        assert!(
            atlas.cell_height < 80.0,
            "cell_height {} too large",
            atlas.cell_height
        );
    }

    #[test]
    fn atlas_cell_height_greater_than_width() {
        let atlas = create_test_atlas();
        assert!(
            atlas.cell_height > atlas.cell_width,
            "cell_height {} should be greater than cell_width {}",
            atlas.cell_height,
            atlas.cell_width
        );
    }

    // ── Atlas texture dimensions ────────────────────────────────────

    #[test]
    fn atlas_dimensions_are_power_of_two() {
        let atlas = create_test_atlas();
        assert!(
            atlas.atlas_width.is_power_of_two(),
            "atlas_width {} is not power of two",
            atlas.atlas_width
        );
        assert!(
            atlas.atlas_height.is_power_of_two(),
            "atlas_height {} is not power of two",
            atlas.atlas_height
        );
    }

    #[test]
    fn atlas_has_data() {
        let atlas = create_test_atlas();
        assert!(!atlas.atlas_data.is_empty());
    }

    #[test]
    fn atlas_data_matches_dimensions() {
        let atlas = create_test_atlas();
        assert_eq!(
            atlas.atlas_data.len(),
            (atlas.atlas_width * atlas.atlas_height) as usize,
        );
    }

    // ── UV coordinate calculation ───────────────────────────────────

    #[test]
    fn atlas_uv_within_bounds() {
        let atlas = create_test_atlas();
        for byte in 0x20u8..=0x7Eu8 {
            let c = byte as char;
            if let Some(info) = atlas.glyph_info(c) {
                let [u, v, w, h] = info.uv;
                assert!(u >= 0.0 && u <= 1.0, "u out of bounds for '{}': {}", c, u);
                assert!(v >= 0.0 && v <= 1.0, "v out of bounds for '{}': {}", c, v);
                assert!(
                    w > 0.0 && u + w <= 1.0 + 1e-6,
                    "w out of bounds for '{}'",
                    c
                );
                assert!(
                    h > 0.0 && v + h <= 1.0 + 1e-6,
                    "h out of bounds for '{}'",
                    c
                );
            }
        }
    }

    // ── ASCII range coverage ────────────────────────────────────────

    #[test]
    fn atlas_has_glyph_for_a() {
        let atlas = create_test_atlas();
        assert!(atlas.glyph_info('A').is_some());
    }

    #[test]
    fn atlas_has_all_ascii_printable() {
        let atlas = create_test_atlas();
        for byte in 0x20u8..=0x7Eu8 {
            let c = byte as char;
            assert!(atlas.glyph_info(c).is_some(), "missing glyph for '{}'", c);
        }
    }

    #[test]
    fn atlas_covers_95_ascii_glyphs() {
        let atlas = create_test_atlas();
        let count = (0x20u8..=0x7Eu8)
            .filter(|&b| atlas.glyph_info(b as char).is_some())
            .count();
        assert_eq!(count, 95);
    }

    #[test]
    fn atlas_visible_glyph_has_nonzero_pixels() {
        let atlas = create_test_atlas();
        let info = atlas.glyph_info('A').unwrap();
        let [u, v, uw, vh] = info.uv;
        let x0 = (u * atlas.atlas_width as f32) as u32;
        let y0 = (v * atlas.atlas_height as f32) as u32;
        let x1 = ((u + uw) * atlas.atlas_width as f32) as u32;
        let y1 = ((v + vh) * atlas.atlas_height as f32) as u32;

        let mut has_nonzero = false;
        for y in y0..y1 {
            for x in x0..x1 {
                let idx = (y * atlas.atlas_width + x) as usize;
                if atlas.atlas_data[idx] > 0 {
                    has_nonzero = true;
                    break;
                }
            }
        }
        assert!(has_nonzero, "Glyph 'A' should have non-zero pixels");
    }

    #[test]
    fn atlas_no_glyph_for_control_char() {
        let atlas = create_test_atlas();
        assert!(atlas.glyph_info('\x00').is_none());
        assert!(atlas.glyph_info('\n').is_none());
    }
}

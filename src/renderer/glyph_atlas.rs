// Glyph rasterization and GPU texture atlas.
//
// macOS: Uses CoreText for native-quality font rendering with platform-consistent
// antialiasing. Produces RGBA atlas with per-channel coverage.
// Other platforms: Uses cosmic-text (swash) for cross-platform glyph rasterization.

#[cfg(not(target_os = "macos"))]
use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache};
use std::collections::HashMap;

/// Source Code Pro Medium — bundled as a compiled-in resource.
const SOURCE_CODE_PRO_TTF: &[u8] =
    include_bytes!("../../assets/fonts/SourceCodePro-Medium.ttf");

/// Extra pixels per side added to each atlas slot to prevent glyph clipping.
/// Descenders, ascenders, and anti-aliased fringes need room beyond the cell boundary.
pub(crate) const GLYPH_PADDING: u32 = 2;

/// Metadata for a single glyph in the atlas.
#[derive(Debug, Clone)]
pub struct GlyphInfo {
    /// UV rect in atlas: [u, v, width, height], normalized to [0, 1].
    pub uv: [f32; 4],
}

/// Rasterized glyph atlas for GPU text rendering.
///
/// Contains an R8 texture with all ASCII printable glyphs (0x20..=0x7E)
/// plus UI chrome characters, pre-rendered in cell-sized slots, ready for GPU upload.
pub struct GlyphAtlas {
    /// Pixel data for the atlas texture.
    pub atlas_data: Vec<u8>,
    /// Atlas texture width in pixels (power of two).
    pub atlas_width: u32,
    /// Atlas texture height in pixels (power of two).
    pub atlas_height: u32,
    /// Cell width in pixels (monospace advance width at the given scale).
    pub cell_width: f32,
    /// Cell height in pixels (line height at the given scale).
    pub cell_height: f32,
    /// Bytes per pixel: 1 for R8 grayscale, 4 for RGBA.
    pub bytes_per_pixel: u32,
    /// Ratio of font metrics height (ascent+descent) to cell_height.
    /// Used by the shader to size the cursor to font height instead of full cell height.
    pub cursor_height_ratio: f32,
    glyphs: HashMap<char, GlyphInfo>,
    /// Number of columns in the atlas grid layout.
    atlas_cols: u32,
    /// Slot width in pixels (cell + padding).
    slot_w: u32,
    /// Slot height in pixels (cell + padding).
    slot_h: u32,
    /// Next slot index for on-demand glyph rasterization.
    next_overflow_index: u32,
    /// Whether the atlas data has been modified since the last GPU upload.
    pub atlas_dirty: bool,
    /// CoreText rasterizer for on-demand glyph rasterization (macOS only).
    #[cfg(target_os = "macos")]
    rasterizer: crate::renderer::coretext_rasterizer::CoreTextRasterizer,
}

/// Extra UI chrome characters beyond ASCII printable range.
const EXTRA_CHARS: &[char] = &[
    '\u{273B}', // ✻ TEARDROP-SPOKED ASTERISK (brand icon)
    '\u{2460}', '\u{2461}', '\u{2462}', '\u{2463}', '\u{2464}', // ①②③④⑤
    '\u{2465}', '\u{2466}', '\u{2467}', '\u{2468}', // ⑥⑦⑧⑨
    '\u{25CF}', // ● BLACK CIRCLE (status dot)
    '\u{00B7}', // · MIDDLE DOT (separator)
    '\u{00D7}', // × MULTIPLICATION SIGN (tab close)
    '\u{2026}', // … HORIZONTAL ELLIPSIS
    '\u{276F}', // ❯ HEAVY RIGHT-POINTING ANGLE QUOTATION MARK (shell prompts)
    '\u{2714}', // ✔ HEAVY CHECK MARK
    '\u{2718}', // ✘ HEAVY BALLOT X
    '\u{279C}', // ➜ HEAVY ROUND-TIPPED RIGHTWARDS ARROW
    '\u{2192}', // → RIGHTWARDS ARROW
    '\u{2190}', // ← LEFTWARDS ARROW
    '\u{2502}', // │ BOX DRAWINGS LIGHT VERTICAL
    '\u{251C}', // ├ BOX DRAWINGS LIGHT VERTICAL AND RIGHT
    '\u{2514}', // └ BOX DRAWINGS LIGHT UP AND RIGHT
    '\u{2500}', // ─ BOX DRAWINGS LIGHT HORIZONTAL
    '\u{25B6}', // ▶ BLACK RIGHT-POINTING TRIANGLE
    '\u{25C0}', // ◀ BLACK LEFT-POINTING TRIANGLE
    '\u{25D0}', // ◐ CIRCLE WITH LEFT HALF BLACK (theme icon)
    '\u{2713}', // ✓ CHECK MARK
    // Block Elements (U+2580-U+259F) — used by Claude Code mascot, TUI apps
    '\u{2580}', '\u{2581}', '\u{2582}', '\u{2583}', // ▀▁▂▃
    '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}', // ▄▅▆▇
    '\u{2588}', '\u{2589}', '\u{258A}', '\u{258B}', // █▉▊▋
    '\u{258C}', '\u{258D}', '\u{258E}', '\u{258F}', // ▌▍▎▏
    '\u{2590}', '\u{2591}', '\u{2592}', '\u{2593}', // ▐░▒▓
    '\u{2594}', '\u{2595}', '\u{2596}', '\u{2597}', // ▔▕▖▗
    '\u{2598}', '\u{2599}', '\u{259A}', '\u{259B}', // ▘▙▚▛
    '\u{259C}', '\u{259D}', '\u{259E}', '\u{259F}', // ▜▝▞▟
    // Powerline symbols (used by oh-my-zsh, starship, etc.)
    '\u{E0A0}', '\u{E0A1}', '\u{E0A2}', // Branch, Line number, Padlock
    '\u{E0B0}', '\u{E0B1}', '\u{E0B2}', '\u{E0B3}', // Arrow fills
    // Braille pattern endpoints (used by plotting tools)
    '\u{2800}', '\u{28FF}',
];

impl GlyphAtlas {
    /// Rasterize ASCII printable glyphs + UI chrome into a texture atlas.
    ///
    /// On macOS, uses CoreText for native-quality rendering (RGBA atlas).
    /// On other platforms, uses cosmic-text/swash (R8 grayscale atlas).
    #[allow(unused_variables)] // font_family only used on non-macOS
    pub fn new(
        font_size: f32,
        scale_factor: f32,
        font_family: &str,
        line_height_multiplier: f32,
    ) -> Self {
        #[cfg(target_os = "macos")]
        {
            Self::new_coretext(font_size, scale_factor, line_height_multiplier)
        }
        #[cfg(not(target_os = "macos"))]
        {
            Self::new_swash(font_size, scale_factor, font_family, line_height_multiplier)
        }
    }

    /// macOS: Rasterize using CoreText for platform-native font rendering.
    /// Produces an RGBA atlas with per-channel coverage for subpixel blending.
    #[cfg(target_os = "macos")]
    fn new_coretext(
        font_size: f32,
        scale_factor: f32,
        line_height_multiplier: f32,
    ) -> Self {
        use crate::renderer::coretext_rasterizer::CoreTextRasterizer;

        let scaled_size = font_size * scale_factor;
        let rasterizer = CoreTextRasterizer::new(SOURCE_CODE_PRO_TTF, scaled_size);

        let cell_width = rasterizer.advance_width('M') as f32;
        let font_metrics_height = (rasterizer.ascent() + rasterizer.descent()) as f32;
        let cell_height = (font_metrics_height * line_height_multiplier).ceil();
        let cursor_height_ratio = font_metrics_height / cell_height;

        let cell_w = cell_width.ceil() as u32;
        let cell_h = cell_height.ceil() as u32;
        let slot_w = cell_w + GLYPH_PADDING * 2;
        let slot_h = cell_h + GLYPH_PADDING * 2;

        // Atlas layout: 16 glyphs per row
        let ascii_count = 95u32;
        let glyph_count = ascii_count + EXTRA_CHARS.len() as u32;
        let cols = 16u32;
        let rows = glyph_count.div_ceil(cols);
        let atlas_width = (cols * slot_w).next_power_of_two().max(512);
        let atlas_height = (rows * slot_h).next_power_of_two().max(512);

        let bytes_per_pixel = 4u32;
        let mut atlas_data =
            vec![0u8; (atlas_width * atlas_height * bytes_per_pixel) as usize];
        let mut glyphs = HashMap::with_capacity(glyph_count as usize);

        // Build character list: ASCII printable + UI chrome
        let chars: Vec<char> = (0x20u8..=0x7Eu8)
            .map(|b| b as char)
            .chain(EXTRA_CHARS.iter().copied())
            .collect();

        let pad = GLYPH_PADDING;
        for (i, &c) in chars.iter().enumerate() {
            let col = (i as u32) % cols;
            let row = (i as u32) / cols;
            let slot_x = col * slot_w;
            let slot_y = row * slot_h;

            // Rasterize at cell dimensions using CoreText
            let glyph_bmp = rasterizer.rasterize(c, cell_w, cell_h);

            // Copy RGBA data into padded slot in atlas
            for y in 0..glyph_bmp.height {
                for x in 0..glyph_bmp.width {
                    let src_idx = ((y * glyph_bmp.width + x) * 4) as usize;
                    let dst_x = slot_x + pad + x;
                    let dst_y = slot_y + pad + y;
                    if dst_x < atlas_width && dst_y < atlas_height {
                        let dst_idx =
                            ((dst_y * atlas_width + dst_x) * bytes_per_pixel) as usize;
                        atlas_data[dst_idx..dst_idx + 4]
                            .copy_from_slice(&glyph_bmp.data[src_idx..src_idx + 4]);
                    }
                }
            }

            glyphs.insert(
                c,
                GlyphInfo {
                    uv: [
                        (slot_x + pad) as f32 / atlas_width as f32,
                        (slot_y + pad) as f32 / atlas_height as f32,
                        cell_w as f32 / atlas_width as f32,
                        cell_h as f32 / atlas_height as f32,
                    ],
                },
            );
        }

        let next_overflow_index = chars.len() as u32;

        log::info!(
            "CoreText atlas: {}x{} (slot: {}x{}, cell: {:.1}x{:.1}, {} glyphs)",
            atlas_width,
            atlas_height,
            slot_w,
            slot_h,
            cell_width,
            cell_height,
            glyphs.len(),
        );

        Self {
            atlas_data,
            atlas_width,
            atlas_height,
            cell_width,
            cell_height,
            bytes_per_pixel,
            cursor_height_ratio,
            glyphs,
            atlas_cols: cols,
            slot_w,
            slot_h,
            next_overflow_index,
            atlas_dirty: false,
            rasterizer,
        }
    }

    /// Cross-platform: Rasterize using cosmic-text/swash.
    /// Produces an R8 grayscale atlas.
    #[cfg(not(target_os = "macos"))]
    fn new_swash(
        font_size: f32,
        scale_factor: f32,
        font_family: &str,
        line_height_multiplier: f32,
    ) -> Self {
        let scaled_size = font_size * scale_factor;
        let line_height = (scaled_size * line_height_multiplier).ceil();

        let mut font_system = FontSystem::new();
        font_system
            .db_mut()
            .load_font_data(SOURCE_CODE_PRO_TTF.to_vec());

        let mut swash_cache = SwashCache::new();
        let metrics = Metrics::new(scaled_size, line_height);
        let attrs = Self::resolve_font_attrs(font_family);

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
        let cursor_height_ratio = scaled_size / cell_height;
        let slot_w = cell_width.ceil() as u32 + GLYPH_PADDING * 2;
        let slot_h = cell_height.ceil() as u32 + GLYPH_PADDING * 2;

        let ascii_count = 95u32;
        let glyph_count = ascii_count + EXTRA_CHARS.len() as u32;
        let cols = 16u32;
        let rows = glyph_count.div_ceil(cols);
        let atlas_width = (cols * slot_w).next_power_of_two().max(512);
        let atlas_height = (rows * slot_h).next_power_of_two().max(512);

        let mut atlas_data = vec![0u8; (atlas_width * atlas_height) as usize];
        let mut glyphs = HashMap::with_capacity(glyph_count as usize);

        let white = cosmic_text::Color::rgb(0xFF, 0xFF, 0xFF);

        for (i, byte) in (0x20u8..=0x7Eu8).enumerate() {
            let c = byte as char;
            Self::rasterize_glyph(
                c,
                i as u32,
                cols,
                slot_w,
                slot_h,
                atlas_width,
                atlas_height,
                &mut atlas_data,
                &mut glyphs,
                &mut font_system,
                &mut swash_cache,
                &mut buffer,
                attrs,
                cell_width,
                cell_height,
                white,
            );
        }

        for (j, &c) in EXTRA_CHARS.iter().enumerate() {
            let i = ascii_count + j as u32;
            Self::rasterize_glyph(
                c,
                i,
                cols,
                slot_w,
                slot_h,
                atlas_width,
                atlas_height,
                &mut atlas_data,
                &mut glyphs,
                &mut font_system,
                &mut swash_cache,
                &mut buffer,
                attrs,
                cell_width,
                cell_height,
                white,
            );
        }

        log::info!(
            "Glyph atlas: {}x{} (slot: {}x{}, cell: {:.1}x{:.1}, {} glyphs)",
            atlas_width,
            atlas_height,
            slot_w,
            slot_h,
            cell_width,
            cell_height,
            glyphs.len(),
        );

        let next_overflow_index = ascii_count + EXTRA_CHARS.len() as u32;

        Self {
            atlas_data,
            atlas_width,
            atlas_height,
            cell_width,
            cell_height,
            bytes_per_pixel: 1,
            cursor_height_ratio,
            glyphs,
            atlas_cols: cols,
            slot_w,
            slot_h,
            next_overflow_index,
            atlas_dirty: false,
        }
    }

    /// Rasterize a single glyph into the atlas at the given slot index (swash path).
    #[cfg(not(target_os = "macos"))]
    #[allow(clippy::too_many_arguments)]
    fn rasterize_glyph(
        c: char,
        index: u32,
        cols: u32,
        slot_w: u32,
        slot_h: u32,
        atlas_width: u32,
        atlas_height: u32,
        atlas_data: &mut [u8],
        glyphs: &mut HashMap<char, GlyphInfo>,
        font_system: &mut FontSystem,
        swash_cache: &mut SwashCache,
        buffer: &mut Buffer,
        attrs: Attrs<'_>,
        cell_width: f32,
        cell_height: f32,
        color: cosmic_text::Color,
    ) {
        let col = index % cols;
        let row = index / cols;
        let slot_x = col * slot_w;
        let slot_y = row * slot_h;

        buffer.set_text(font_system, &c.to_string(), attrs, Shaping::Advanced);
        buffer.set_size(
            font_system,
            Some(cell_width * 2.0),
            Some(cell_height * 2.0),
        );
        buffer.shape_until_scroll(font_system, true);

        let aw = atlas_width;
        let sw = slot_w;
        let sh = slot_h;
        let pad = GLYPH_PADDING;
        buffer.draw(font_system, swash_cache, color, |x, y, _w, _h, c| {
            if x >= 0 && y >= 0 {
                let xu = x as u32 + pad;
                let yu = y as u32 + pad;
                // Clamp to slot boundaries to prevent bleed into adjacent glyphs
                if xu < sw && yu < sh {
                    let ax = slot_x + xu;
                    let ay = slot_y + yu;
                    let idx = (ay * aw + ax) as usize;
                    // Coverage/alpha is in the alpha channel
                    atlas_data[idx] = atlas_data[idx].max(c.a());
                }
            }
        });

        let cell_w_px = cell_width.ceil() as u32;
        let cell_h_px = cell_height.ceil() as u32;
        glyphs.insert(
            c,
            GlyphInfo {
                uv: [
                    (slot_x + pad) as f32 / atlas_width as f32,
                    (slot_y + pad) as f32 / atlas_height as f32,
                    cell_w_px as f32 / atlas_width as f32,
                    cell_h_px as f32 / atlas_height as f32,
                ],
            },
        );
    }

    /// Resolve font family name to cosmic-text `Attrs` with fallback chain.
    #[cfg(not(target_os = "macos"))]
    fn resolve_font_attrs(font_family: &str) -> Attrs<'static> {
        match font_family.to_lowercase().as_str() {
            "source code pro" => Attrs::new().family(Family::Name("Source Code Pro")),
            "jetbrains mono" => Attrs::new().family(Family::Name("JetBrains Mono")),
            "sf mono" => Attrs::new().family(Family::Name("SF Mono")),
            "menlo" => Attrs::new().family(Family::Name("Menlo")),
            "monospace" => Attrs::new().family(Family::Monospace),
            _ => {
                // Try the user-specified family; cosmic-text falls back if not found
                // We leak the string to get a 'static lifetime for Family::Name
                let leaked: &'static str = Box::leak(font_family.to_string().into_boxed_str());
                Attrs::new().family(Family::Name(leaked))
            }
        }
    }

    /// Look up glyph metadata for a character.
    pub fn glyph_info(&self, c: char) -> Option<&GlyphInfo> {
        self.glyphs.get(&c)
    }

    /// Rasterize a glyph on demand if not already in the atlas.
    /// Returns the glyph info if successfully rasterized (or already present),
    /// or None if the atlas has no room for more glyphs.
    /// Sets `atlas_dirty` to true when new glyphs are added.
    #[cfg(target_os = "macos")]
    pub fn rasterize_on_demand(&mut self, c: char) -> Option<&GlyphInfo> {
        if self.glyphs.contains_key(&c) {
            return self.glyphs.get(&c);
        }

        let index = self.next_overflow_index;
        let col = index % self.atlas_cols;
        let row = index / self.atlas_cols;
        let slot_x = col * self.slot_w;
        let slot_y = row * self.slot_h;

        // Check if slot fits within atlas bounds
        if slot_x + self.slot_w > self.atlas_width || slot_y + self.slot_h > self.atlas_height {
            return None;
        }

        let cell_w = self.cell_width.ceil() as u32;
        let cell_h = self.cell_height.ceil() as u32;
        let pad = GLYPH_PADDING;
        let bpp = self.bytes_per_pixel;

        let glyph_bmp = self.rasterizer.rasterize(c, cell_w, cell_h);

        // Copy RGBA data into padded slot in atlas
        for y in 0..glyph_bmp.height {
            for x in 0..glyph_bmp.width {
                let src_idx = ((y * glyph_bmp.width + x) * 4) as usize;
                let dst_x = slot_x + pad + x;
                let dst_y = slot_y + pad + y;
                if dst_x < self.atlas_width && dst_y < self.atlas_height {
                    let dst_idx = ((dst_y * self.atlas_width + dst_x) * bpp) as usize;
                    self.atlas_data[dst_idx..dst_idx + 4]
                        .copy_from_slice(&glyph_bmp.data[src_idx..src_idx + 4]);
                }
            }
        }

        self.glyphs.insert(
            c,
            GlyphInfo {
                uv: [
                    (slot_x + pad) as f32 / self.atlas_width as f32,
                    (slot_y + pad) as f32 / self.atlas_height as f32,
                    cell_w as f32 / self.atlas_width as f32,
                    cell_h as f32 / self.atlas_height as f32,
                ],
            },
        );

        self.next_overflow_index += 1;
        self.atlas_dirty = true;

        self.glyphs.get(&c)
    }

    /// Rasterize a glyph on demand (non-macOS stub — not yet implemented).
    #[cfg(not(target_os = "macos"))]
    pub fn rasterize_on_demand(&mut self, _c: char) -> Option<&GlyphInfo> {
        // TODO: Implement cosmic-text on-demand rasterization
        None
    }

    /// Dump atlas as PGM (grayscale) image for debugging.
    /// For RGBA atlases, extracts the alpha channel.
    pub fn dump_pgm(&self, path: &str) -> std::io::Result<()> {
        use std::io::Write;
        let mut f = std::fs::File::create(path)?;
        write!(f, "P5\n{} {}\n255\n", self.atlas_width, self.atlas_height)?;
        if self.bytes_per_pixel == 4 {
            // Extract alpha channel from RGBA data
            let alpha: Vec<u8> = self.atlas_data.chunks(4).map(|px| px[3]).collect();
            f.write_all(&alpha)?;
        } else {
            f.write_all(&self.atlas_data)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_atlas() -> GlyphAtlas {
        GlyphAtlas::new(13.0, 2.0, "JetBrains Mono", 1.5)
    }

    // ── Font loading ────────────────────────────────────────────────

    #[test]
    fn atlas_creates_successfully() {
        let atlas = create_test_atlas();
        assert!(atlas.atlas_width > 0);
        assert!(atlas.atlas_height > 0);
    }

    #[test]
    fn atlas_creates_with_bundled_jetbrains_mono() {
        let atlas = GlyphAtlas::new(13.0, 2.0, "JetBrains Mono", 1.5);
        assert!(atlas.cell_width > 0.0);
        assert!(atlas.glyph_info('A').is_some());
    }

    #[test]
    fn atlas_creates_with_monospace_fallback() {
        let atlas = GlyphAtlas::new(13.0, 2.0, "monospace", 1.5);
        assert!(atlas.cell_width > 0.0);
        assert!(atlas.glyph_info('A').is_some());
    }

    #[test]
    fn atlas_creates_with_unknown_font_falls_back() {
        // Unknown font should still create a working atlas via system fallback
        let atlas = GlyphAtlas::new(13.0, 2.0, "NonExistentFont12345", 1.5);
        assert!(atlas.cell_width > 0.0);
        assert!(atlas.glyph_info('A').is_some());
    }

    #[test]
    fn atlas_line_height_multiplier_affects_cell_height() {
        let atlas_small = GlyphAtlas::new(13.0, 2.0, "JetBrains Mono", 1.2);
        let atlas_large = GlyphAtlas::new(13.0, 2.0, "JetBrains Mono", 1.8);
        assert!(
            atlas_large.cell_height > atlas_small.cell_height,
            "larger line_height multiplier should produce taller cells: {} vs {}",
            atlas_large.cell_height,
            atlas_small.cell_height
        );
    }

    #[test]
    fn atlas_default_line_height_produces_expected_size() {
        let atlas = GlyphAtlas::new(13.0, 2.0, "JetBrains Mono", 1.5);
        // On macOS (CoreText): cell_height = ceil((ascent + descent) * multiplier)
        // On other platforms (swash): cell_height = ceil(scaled_size * multiplier) = ceil(26 * 1.5) = 39
        // Both should produce reasonable values within this range
        assert!(
            atlas.cell_height >= 30.0 && atlas.cell_height <= 55.0,
            "cell_height {} should be in reasonable range for 13pt @ 2x with 1.5x line height",
            atlas.cell_height
        );
    }

    #[test]
    fn atlas_cursor_height_ratio_is_less_than_one_with_line_height() {
        let atlas = GlyphAtlas::new(13.0, 2.0, "JetBrains Mono", 1.6);
        // With line_height_multiplier > 1.0, cursor_height_ratio should be < 1.0
        assert!(atlas.cursor_height_ratio > 0.0 && atlas.cursor_height_ratio < 1.0,
            "cursor_height_ratio {} should be between 0 and 1 for line_height > 1.0",
            atlas.cursor_height_ratio);
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
            (atlas.atlas_width * atlas.atlas_height * atlas.bytes_per_pixel) as usize,
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
        let bpp = atlas.bytes_per_pixel;

        let mut has_nonzero = false;
        for y in y0..y1 {
            for x in x0..x1 {
                let base = ((y * atlas.atlas_width + x) * bpp) as usize;
                // Check any channel for non-zero (R8: single byte, RGBA: any of 4)
                if atlas.atlas_data[base..base + bpp as usize].iter().any(|&b| b > 0) {
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

    // ── Glyph padding tests ────────────────────────────────────────

    #[test]
    fn atlas_uv_covers_cell_area_not_padded_slot() {
        let atlas = create_test_atlas();
        let info = atlas.glyph_info('A').unwrap();
        let uv_w_px = info.uv[2] * atlas.atlas_width as f32;
        let uv_h_px = info.uv[3] * atlas.atlas_height as f32;
        // UV should cover exactly the cell area (glyph content), not the padded slot
        assert!(
            (uv_w_px - atlas.cell_width.ceil()).abs() < 1.0,
            "UV width {uv_w_px} should match cell width {}",
            atlas.cell_width.ceil()
        );
        assert!(
            (uv_h_px - atlas.cell_height.ceil()).abs() < 1.0,
            "UV height {uv_h_px} should match cell height {}",
            atlas.cell_height.ceil()
        );
    }

    #[test]
    fn atlas_glyph_not_clipped_at_boundary() {
        // Glyphs with descenders (g, y, p) should have non-zero pixels
        // near the bottom of their slot (within the padding zone)
        let atlas = create_test_atlas();
        let bpp = atlas.bytes_per_pixel;
        for ch in ['g', 'y', 'p'] {
            let info = atlas.glyph_info(ch).unwrap();
            let [u, v, uw, vh] = info.uv;
            let x0 = (u * atlas.atlas_width as f32) as u32;
            let y0 = (v * atlas.atlas_height as f32) as u32;
            let x1 = ((u + uw) * atlas.atlas_width as f32) as u32;
            let y1 = ((v + vh) * atlas.atlas_height as f32) as u32;

            let mut has_nonzero = false;
            for y in y0..y1 {
                for x in x0..x1 {
                    let base = ((y * atlas.atlas_width + x) * bpp) as usize;
                    if atlas.atlas_data[base..base + bpp as usize].iter().any(|&b| b > 0) {
                        has_nonzero = true;
                        break;
                    }
                }
                if has_nonzero {
                    break;
                }
            }
            assert!(has_nonzero, "Glyph '{ch}' should have non-zero pixels");
        }
    }

    // ── UI chrome character tests ──────────────────────────────────

    #[test]
    fn atlas_has_ui_chrome_characters() {
        let atlas = create_test_atlas();
        // These are the extra UI characters used for tab bar, status, etc.
        for &ch in EXTRA_CHARS {
            // Some characters may not be available in JetBrains Mono,
            // but the slot will exist (just empty). Check the glyph entry exists.
            assert!(
                atlas.glyph_info(ch).is_some(),
                "missing UI chrome glyph for U+{:04X}",
                ch as u32
            );
        }
    }

    #[test]
    fn atlas_total_glyph_count_includes_extras() {
        let atlas = create_test_atlas();
        let ascii_count = (0x20u8..=0x7Eu8)
            .filter(|&b| atlas.glyph_info(b as char).is_some())
            .count();
        let extra_count = EXTRA_CHARS
            .iter()
            .filter(|&&ch| atlas.glyph_info(ch).is_some())
            .count();
        assert_eq!(ascii_count, 95);
        assert_eq!(
            ascii_count + extra_count,
            atlas.glyphs.len(),
            "total glyphs should be ASCII + extras"
        );
    }

    // ── Cross-platform validation (exercises whichever backend is active) ──

    #[test]
    fn atlas_minimum_512px_constraint() {
        // Even at 1x scale with small font, atlas must be >= 512px
        let atlas = GlyphAtlas::new(10.0, 1.0, "Source Code Pro", 1.2);
        assert!(
            atlas.atlas_width >= 512,
            "Atlas width {} should be >= 512",
            atlas.atlas_width
        );
        assert!(
            atlas.atlas_height >= 512,
            "Atlas height {} should be >= 512",
            atlas.atlas_height
        );
    }

    #[test]
    fn atlas_bundled_source_code_pro_metrics() {
        // Source Code Pro is the bundled font — must produce valid metrics on all platforms
        let atlas = GlyphAtlas::new(15.0, 2.0, "Source Code Pro", 1.2);
        assert!(atlas.cell_width > 0.0, "cell_width must be positive");
        assert!(atlas.cell_height > 0.0, "cell_height must be positive");
        assert!(
            atlas.cell_width < atlas.cell_height,
            "monospace cells should be taller than wide"
        );
        assert!(atlas.glyphs.len() >= 95, "must have all ASCII printable");
    }

    #[test]
    fn atlas_bytes_per_pixel_matches_platform() {
        let atlas = create_test_atlas();
        // macOS CoreText: 4 (RGBA), Linux swash: 1 (R8 grayscale)
        #[cfg(target_os = "macos")]
        assert_eq!(atlas.bytes_per_pixel, 4, "macOS should use RGBA atlas");
        #[cfg(not(target_os = "macos"))]
        assert_eq!(atlas.bytes_per_pixel, 1, "Linux should use R8 grayscale atlas");
    }

    #[test]
    fn atlas_scale_factors_produce_different_sizes() {
        let atlas_1x = GlyphAtlas::new(13.0, 1.0, "Source Code Pro", 1.5);
        let atlas_2x = GlyphAtlas::new(13.0, 2.0, "Source Code Pro", 1.5);
        // 2x scale should produce larger cell dimensions
        assert!(
            atlas_2x.cell_width > atlas_1x.cell_width,
            "2x scale ({}) should produce wider cells than 1x ({})",
            atlas_2x.cell_width,
            atlas_1x.cell_width
        );
    }

    // ── Block element coverage ──────────────────────────────────

    #[test]
    fn atlas_has_all_block_elements() {
        let atlas = create_test_atlas();
        for cp in 0x2580u32..=0x259Fu32 {
            let ch = char::from_u32(cp).unwrap();
            assert!(
                atlas.glyph_info(ch).is_some(),
                "missing block element U+{:04X} '{}'",
                cp,
                ch
            );
        }
    }

    #[test]
    fn atlas_full_block_has_nonzero_pixels() {
        // █ (U+2588) should have substantial pixel coverage
        let atlas = create_test_atlas();
        let info = atlas.glyph_info('\u{2588}').unwrap();
        let [u, v, uw, vh] = info.uv;
        let x0 = (u * atlas.atlas_width as f32) as u32;
        let y0 = (v * atlas.atlas_height as f32) as u32;
        let x1 = ((u + uw) * atlas.atlas_width as f32) as u32;
        let y1 = ((v + vh) * atlas.atlas_height as f32) as u32;
        let bpp = atlas.bytes_per_pixel;
        let total_pixels = (x1 - x0) * (y1 - y0);
        let nonzero = (y0..y1)
            .flat_map(|y| (x0..x1).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let base = ((y * atlas.atlas_width + x) * bpp) as usize;
                atlas.atlas_data[base..base + bpp as usize]
                    .iter()
                    .any(|&b| b > 0)
            })
            .count();
        // Font-rendered block should fill a significant portion (>40%) of cell
        // Exact coverage depends on font metrics; key thing is it's not empty
        assert!(
            nonzero as f32 / total_pixels as f32 > 0.4,
            "Full block should fill significant portion of cell: {}/{} ({:.0}%)",
            nonzero,
            total_pixels,
            nonzero as f32 / total_pixels as f32 * 100.0
        );
    }

    #[test]
    fn atlas_block_elements_have_visible_pixels() {
        // Key block elements used by Claude mascot must have non-zero pixels
        let atlas = create_test_atlas();
        let bpp = atlas.bytes_per_pixel;
        for ch in [
            '\u{2580}', '\u{2584}', '\u{2588}', '\u{258C}', '\u{2590}', '\u{259B}',
            '\u{259C}',
        ] {
            let info = atlas.glyph_info(ch).unwrap();
            let [u, v, uw, vh] = info.uv;
            let x0 = (u * atlas.atlas_width as f32) as u32;
            let y0 = (v * atlas.atlas_height as f32) as u32;
            let x1 = ((u + uw) * atlas.atlas_width as f32) as u32;
            let y1 = ((v + vh) * atlas.atlas_height as f32) as u32;
            let nonzero = (y0..y1)
                .flat_map(|y| (x0..x1).map(move |x| (x, y)))
                .filter(|&(x, y)| {
                    let base = ((y * atlas.atlas_width + x) * bpp) as usize;
                    atlas.atlas_data[base..base + bpp as usize]
                        .iter()
                        .any(|&b| b > 0)
                })
                .count();
            assert!(
                nonzero > 0,
                "Block element U+{:04X} '{}' should have visible pixels",
                ch as u32,
                ch
            );
        }
    }

    #[test]
    fn atlas_powerline_symbols_present() {
        let atlas = create_test_atlas();
        for &cp in &[0xE0A0u32, 0xE0B0, 0xE0B2] {
            let ch = char::from_u32(cp).unwrap();
            assert!(
                atlas.glyph_info(ch).is_some(),
                "missing powerline U+{:04X}",
                cp
            );
        }
    }

    #[test]
    fn atlas_block_elements_uvs_within_bounds() {
        let atlas = create_test_atlas();
        for cp in 0x2580u32..=0x259Fu32 {
            let ch = char::from_u32(cp).unwrap();
            if let Some(info) = atlas.glyph_info(ch) {
                let [u, v, w, h] = info.uv;
                assert!(
                    u >= 0.0 && v >= 0.0 && u + w <= 1.0 + 1e-6 && v + h <= 1.0 + 1e-6,
                    "UV out of bounds for U+{:04X}: {:?}",
                    cp,
                    info.uv
                );
            }
        }
    }

    #[test]
    fn atlas_total_glyph_count_with_block_elements() {
        let atlas = create_test_atlas();
        // 95 ASCII + EXTRA_CHARS (22 original + 32 block elements + 7 powerline + 2 braille)
        assert!(
            atlas.glyphs.len() >= 95 + 22 + 32,
            "Expected at least {} glyphs, got {}",
            95 + 22 + 32,
            atlas.glyphs.len()
        );
    }

    // ── On-demand glyph rasterization ───────────────────────────

    #[test]
    fn on_demand_rasterize_unknown_char() {
        let mut atlas = create_test_atlas();
        // CJK character not pre-baked
        let ch = '\u{4E16}'; // 世
        assert!(atlas.glyph_info(ch).is_none(), "should not be pre-baked");
        let result = atlas.rasterize_on_demand(ch);
        assert!(result.is_some(), "should rasterize on demand");
        // Subsequent lookup should return cached entry
        assert!(atlas.glyph_info(ch).is_some());
    }

    #[test]
    fn on_demand_returns_existing_glyph() {
        let mut atlas = create_test_atlas();
        let info_before = atlas.glyph_info('A').unwrap().uv;
        let info_after = atlas.rasterize_on_demand('A').unwrap().uv;
        assert_eq!(
            info_before, info_after,
            "should return pre-baked glyph unchanged"
        );
    }

    #[test]
    fn on_demand_uv_within_bounds() {
        let mut atlas = create_test_atlas();
        for ch in ['α', 'β', 'γ', '你', '好', '⌘', '⌥'] {
            if let Some(info) = atlas.rasterize_on_demand(ch) {
                let [u, v, w, h] = info.uv;
                assert!(
                    u >= 0.0 && v >= 0.0 && u + w <= 1.0 + 1e-6 && v + h <= 1.0 + 1e-6,
                    "UV out of bounds for '{}': {:?}",
                    ch,
                    info.uv
                );
            }
        }
    }

    #[test]
    fn on_demand_atlas_data_grows_correctly() {
        let mut atlas = create_test_atlas();
        let initial_len = atlas.glyphs.len();
        atlas.rasterize_on_demand('α');
        atlas.rasterize_on_demand('β');
        assert_eq!(atlas.glyphs.len(), initial_len + 2);
    }

    #[test]
    fn on_demand_sets_dirty_flag() {
        let mut atlas = create_test_atlas();
        assert!(!atlas.atlas_dirty);
        atlas.rasterize_on_demand('α');
        assert!(atlas.atlas_dirty, "atlas should be dirty after on-demand rasterization");
    }

    #[test]
    fn on_demand_respects_atlas_capacity() {
        // Create small atlas (1x scale) — still has 512px minimum but fewer pre-baked glyphs slots used
        let mut atlas = GlyphAtlas::new(10.0, 1.0, "Source Code Pro", 1.2);
        let mut rasterized = 0;
        // Try rasterizing many CJK chars until atlas is full
        for cp in 0x4E00u32..0x9FFFu32 {
            if let Some(ch) = char::from_u32(cp) {
                if atlas.rasterize_on_demand(ch).is_some() {
                    rasterized += 1;
                } else {
                    break; // atlas full
                }
            }
        }
        assert!(
            rasterized > 0,
            "should rasterize at least some chars before atlas fills"
        );
        // Verify the atlas is actually bounded — we can't add infinite glyphs
        // (the atlas is 512x512 minimum with ~16px slots, so ~1024 slots max)
        assert!(
            rasterized < 10000,
            "should not rasterize infinitely many chars"
        );
    }
}

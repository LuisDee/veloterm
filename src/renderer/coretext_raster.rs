// CoreText-based glyph rasterizer for macOS.
//
// Provides native font hinting, stem darkening, and grid-fitting that
// produces crisp text even at low DPI (e.g. 108 PPI external monitors).

use super::glyph_atlas::{GlyphAtlas, GlyphInfo, GLYPH_PADDING};
use core_graphics::base::kCGImageAlphaPremultipliedLast;
use core_graphics::color_space::CGColorSpace;
use core_graphics::context::CGContext;
use core_graphics::data_provider::CGDataProvider;
use core_graphics::font::CGFont;
use core_graphics::geometry::{CGPoint, CGSize};
use core_text::font as ct_font;
use core_text::font::CTFont;
use core_text::font_descriptor::kCTFontOrientationHorizontal;
use std::collections::HashMap;
use std::sync::Arc;

/// JetBrains Mono Regular — bundled as a compiled-in resource.
const JETBRAINS_MONO_TTF: &[u8] =
    include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf");

/// Load a CTFont at the given size.
///
/// Always uses the bundled JetBrains Mono to ensure consistent metrics.
/// System-installed fonts (even with the same name) may be Nerd Font variants
/// or different versions with wider advance widths, causing spacing issues.
fn load_font(_font_family: &str, scaled_size: f64) -> CTFont {
    load_bundled_font(scaled_size)
}

/// Load the bundled JetBrains Mono via CGDataProvider → CGFont → CTFont.
fn load_bundled_font(scaled_size: f64) -> CTFont {
    let provider = CGDataProvider::from_buffer(Arc::new(JETBRAINS_MONO_TTF));
    let cg_font = CGFont::from_data_provider(provider)
        .expect("Failed to create CGFont from bundled JetBrains Mono");
    let font = ct_font::new_from_CGFont(&cg_font, scaled_size);
    log::info!("CoreText: loaded bundled JetBrains Mono at {scaled_size:.1}px");
    font
}

/// Rasterize all ASCII printable glyphs using CoreText into a GlyphAtlas.
pub fn rasterize_atlas(
    _font_data: &[u8],
    font_size: f32,
    scale_factor: f32,
    font_family: &str,
    line_height_multiplier: f32,
) -> GlyphAtlas {
    let scaled_size = (font_size * scale_factor) as f64;
    let line_height = (scaled_size as f32 * line_height_multiplier).ceil();

    let ct_font = load_font(font_family, scaled_size);

    // Cell width: measure advance of 'M'
    let cell_width = measure_advance(&ct_font, 'M');
    let cell_height = line_height;

    let slot_w = cell_width.ceil() as u32 + GLYPH_PADDING * 2;
    let slot_h = cell_height.ceil() as u32 + GLYPH_PADDING * 2;

    // Atlas layout: 16 glyphs per row, ceil(95/16) = 6 rows
    let glyph_count = 95u32;
    let cols = 16u32;
    let rows = glyph_count.div_ceil(cols);
    let atlas_width = (cols * slot_w).next_power_of_two().max(512);
    let atlas_height = (rows * slot_h).next_power_of_two().max(512);

    log::info!(
        "CoreText atlas: {}x{} (slot: {}x{}, cell: {:.1}x{:.1})",
        atlas_width,
        atlas_height,
        slot_w,
        slot_h,
        cell_width,
        cell_height,
    );

    // Create CGBitmapContext — RGBA, 8-bit per component.
    // CTFontDrawGlyphs requires an RGBA context; DeviceGray doesn't work.
    // We render white glyphs on black and extract max(R,G,B) as coverage.
    let color_space = CGColorSpace::create_device_rgb();
    let mut cg_ctx = CGContext::create_bitmap_context(
        None,
        atlas_width as usize,
        atlas_height as usize,
        8,                          // bits per component
        atlas_width as usize * 4,   // bytes per row (RGBA)
        &color_space,
        kCGImageAlphaPremultipliedLast,
    );

    // Enable antialiasing AND font smoothing.
    // Font smoothing provides stem darkening that makes glyphs look proper weight.
    // Without it, glyphs appear thin/spindly at low DPI.
    // We extract max(R,G,B) as coverage to capture full stroke weight
    // while avoiding color fringing from subpixel rendering.
    cg_ctx.set_allows_antialiasing(true);
    cg_ctx.set_should_antialias(true);
    cg_ctx.set_allows_font_smoothing(true);
    cg_ctx.set_should_smooth_fonts(true);

    // Set fill color to white for glyph rendering
    cg_ctx.set_rgb_fill_color(1.0, 1.0, 1.0, 1.0);

    // Font metrics for baseline positioning
    let ascent = ct_font.ascent() as f32;

    let mut glyphs = HashMap::with_capacity(glyph_count as usize);

    for (i, byte) in (0x20u8..=0x7Eu8).enumerate() {
        let c = byte as char;
        let col = (i as u32) % cols;
        let row = (i as u32) / cols;
        let slot_x = col * slot_w;
        let slot_y = row * slot_h;

        // Get glyph index
        let characters: [u16; 1] = [byte as u16];
        let mut glyph_indices: [u16; 1] = [0];
        unsafe {
            ct_font.get_glyphs_for_characters(
                characters.as_ptr(),
                glyph_indices.as_mut_ptr(),
                1,
            );
        }

        // Compute position in CGContext coordinate space (Y-up, origin at bottom-left).
        // We want the glyph baseline at: slot_y + GLYPH_PADDING + ascent from top.
        // In CG Y-up coords: y = atlas_height - (slot_y + GLYPH_PADDING + ascent)
        let pos_x = (slot_x + GLYPH_PADDING) as f64;
        let pos_y = (atlas_height as f32
            - (slot_y + GLYPH_PADDING) as f32
            - ascent) as f64;

        let positions = [CGPoint::new(pos_x, pos_y)];
        ct_font.draw_glyphs(&glyph_indices, &positions, cg_ctx.clone());

        // UV coordinates use the same top-down layout as the swash backend.
        // The pixel data Y-flip (during extraction below) already converts CG's
        // bottom-up storage to top-down. Slot row 0 ends up at the top of the
        // atlas, so v = slot_y / atlas_height maps correctly. Do NOT also flip
        // the UV — that would double-flip and produce upside-down glyphs.
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

    // Extract grayscale coverage from the RGBA bitmap.
    // With font smoothing enabled, CoreText distributes energy across R/G/B
    // channels (subpixel rendering). Taking max(R,G,B) captures the full
    // stroke weight including stem darkening, while avoiding color fringing.
    // Must call bytes_per_row() before data() to avoid borrow conflict.
    let actual_bpr = cg_ctx.bytes_per_row();
    let cg_data = cg_ctx.data();
    let h = atlas_height as usize;
    let w = atlas_width as usize;
    let mut atlas_data = vec![0u8; w * h];

    for y in 0..h {
        for x in 0..w {
            let src_idx = y * actual_bpr + x * 4;
            let r = cg_data[src_idx];
            let g = cg_data[src_idx + 1];
            let b = cg_data[src_idx + 2];
            let dst_idx = y * w + x;
            atlas_data[dst_idx] = r.max(g).max(b);
        }
    }

    GlyphAtlas::from_parts(atlas_data, atlas_width, atlas_height, cell_width, cell_height, glyphs)
}

/// Measure the horizontal advance of a single character using CTFont.
fn measure_advance(font: &CTFont, ch: char) -> f32 {
    let characters: [u16; 1] = [ch as u16];
    let mut glyph_indices: [u16; 1] = [0];
    unsafe {
        font.get_glyphs_for_characters(
            characters.as_ptr(),
            glyph_indices.as_mut_ptr(),
            1,
        );
    }

    let mut advances = [CGSize::new(0.0, 0.0)];
    unsafe {
        font.get_advances_for_glyphs(
            kCTFontOrientationHorizontal,
            glyph_indices.as_ptr(),
            advances.as_mut_ptr(),
            1,
        );
    }

    advances[0].width as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coretext_loads_bundled_font() {
        let font = load_bundled_font(26.0);
        assert!(font.ascent() > 0.0, "ascent should be positive");
        assert!(font.descent() > 0.0, "descent should be positive");
    }

    #[test]
    fn coretext_metrics_reasonable() {
        let font = load_font("JetBrains Mono", 26.0);
        let advance = measure_advance(&font, 'M');
        assert!(advance > 5.0, "advance {advance} too small");
        assert!(advance < 40.0, "advance {advance} too large");
        assert!(font.ascent() > 0.0);
    }

    #[test]
    fn coretext_atlas_has_ink() {
        let atlas = rasterize_atlas(JETBRAINS_MONO_TTF, 13.0, 2.0, "JetBrains Mono", 1.5);

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
        assert!(has_nonzero, "CoreText 'A' should have non-zero pixels");
    }

    #[test]
    fn coretext_system_font_fallback() {
        // SF Mono should be available on macOS
        let font = load_font("SF Mono", 16.0);
        let advance = measure_advance(&font, 'A');
        assert!(advance > 0.0, "SF Mono should have positive advance");
    }

    #[test]
    fn coretext_unknown_font_falls_back() {
        let font = load_font("NonExistentFont999", 16.0);
        let advance = measure_advance(&font, 'A');
        assert!(advance > 0.0, "Fallback font should have positive advance");
    }

    #[test]
    fn coretext_descender_glyphs_have_ink() {
        let atlas = rasterize_atlas(JETBRAINS_MONO_TTF, 13.0, 2.0, "JetBrains Mono", 1.5);
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
                    let idx = (y * atlas.atlas_width + x) as usize;
                    if atlas.atlas_data[idx] > 0 {
                        has_nonzero = true;
                        break;
                    }
                }
            }
            assert!(has_nonzero, "CoreText '{ch}' should have non-zero pixels");
        }
    }

    #[test]
    fn coretext_atlas_covers_all_ascii() {
        let atlas = rasterize_atlas(JETBRAINS_MONO_TTF, 13.0, 1.0, "JetBrains Mono", 1.5);
        let count = (0x20u8..=0x7Eu8)
            .filter(|&b| atlas.glyph_info(b as char).is_some())
            .count();
        assert_eq!(count, 95);
    }

    #[test]
    fn coretext_atlas_dimensions_power_of_two() {
        let atlas = rasterize_atlas(JETBRAINS_MONO_TTF, 13.0, 2.0, "JetBrains Mono", 1.5);
        assert!(atlas.atlas_width.is_power_of_two());
        assert!(atlas.atlas_height.is_power_of_two());
    }

    #[test]
    fn coretext_cell_height_matches_swash_formula() {
        let font_size = 13.0_f32;
        let scale_factor = 2.0_f32;
        let line_height_multiplier = 1.5_f32;
        let expected = (font_size * scale_factor * line_height_multiplier).ceil();
        let atlas = rasterize_atlas(
            JETBRAINS_MONO_TTF,
            font_size,
            scale_factor,
            "JetBrains Mono",
            line_height_multiplier,
        );
        assert_eq!(atlas.cell_height, expected);
    }
}

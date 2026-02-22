// macOS CoreText-based glyph rasterizer.
//
// Uses native CoreText font rendering for system-consistent antialiasing,
// subpixel rendering, and font hinting. Produces RGBA glyph bitmaps where
// each channel carries independent coverage for subpixel AA blending.

use core_foundation::base::{CFRange, TCFType};
use core_foundation::string::CFString;
use core_graphics::base::kCGImageAlphaPremultipliedLast;
use core_graphics::color_space::CGColorSpace;
use core_graphics::context::CGContext;
use core_graphics::geometry::{CGPoint, CGSize};
use core_text::font::{CTFont, CTFontRef};
use core_text::font_descriptor::kCTFontOrientationDefault;
use core_text::font_manager::create_font_descriptor;

extern "C" {
    /// Returns a font that can render the given string range, falling back to
    /// system fonts when the current font lacks the required glyphs.
    fn CTFontCreateForString(
        currentFont: *const std::os::raw::c_void,
        string: *const std::os::raw::c_void,
        range: CFRange,
    ) -> *mut std::os::raw::c_void;
}

/// A rasterized glyph bitmap with per-channel subpixel coverage.
pub struct RasterizedGlyph {
    /// RGBA pixel data (4 bytes per pixel, premultiplied alpha).
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// CoreText-based glyph rasterizer for macOS.
pub struct CoreTextRasterizer {
    font: CTFont,
}

impl CoreTextRasterizer {
    /// Create a rasterizer from bundled font data at the given pixel size.
    pub fn new(font_data: &[u8], pixel_size: f32) -> Self {
        let descriptor = create_font_descriptor(font_data)
            .expect("Failed to create CTFontDescriptor from font data");
        let font = core_text::font::new_from_descriptor(&descriptor, pixel_size as f64);
        Self { font }
    }

    /// Get the font ascent in pixels.
    pub fn ascent(&self) -> f64 {
        self.font.ascent()
    }

    /// Get the font descent in pixels (positive value).
    pub fn descent(&self) -> f64 {
        self.font.descent()
    }

    /// Get the advance width of a glyph in pixels.
    pub fn advance_width(&self, ch: char) -> f64 {
        let chars = [ch as u16];
        let mut glyphs = [0u16; 1];
        unsafe {
            self.font.get_glyphs_for_characters(
                chars.as_ptr(),
                glyphs.as_mut_ptr(),
                1,
            );
            let mut advances = [CGSize::new(0.0, 0.0)];
            self.font.get_advances_for_glyphs(
                kCTFontOrientationDefault,
                glyphs.as_ptr(),
                advances.as_mut_ptr(),
                1,
            )
        }
    }

    /// Rasterize a single character into an RGBA bitmap of the given cell dimensions.
    ///
    /// The glyph is drawn white-on-transparent so each RGB channel carries
    /// independent subpixel coverage. The shader uses per-channel blending:
    ///   color.r = mix(bg.r, fg.r, glyph.r)
    pub fn rasterize(&self, ch: char, cell_width: u32, cell_height: u32) -> RasterizedGlyph {
        let w = cell_width as usize;
        let h = cell_height as usize;
        let bytes_per_row = w * 4;

        let color_space = CGColorSpace::create_device_rgb();
        let mut ctx = CGContext::create_bitmap_context(
            None,
            w,
            h,
            8,
            bytes_per_row,
            &color_space,
            kCGImageAlphaPremultipliedLast,
        );

        // Enable font smoothing for system-consistent rendering weight.
        // macOS Retina displays use grayscale smoothing (not subpixel AA) which
        // adds slight stem weight without color fringes. Disabling smoothing
        // produces noticeably thinner text than native terminal emulators.
        ctx.set_allows_font_smoothing(true);
        ctx.set_should_smooth_fonts(true);
        ctx.set_should_antialias(true);

        // Draw white glyph — RGB channels carry subpixel coverage
        ctx.set_rgb_fill_color(1.0, 1.0, 1.0, 1.0);

        // Get glyph ID
        let chars = [ch as u16];
        let mut glyphs = [0u16; 1];
        let found = unsafe {
            self.font.get_glyphs_for_characters(
                chars.as_ptr(),
                glyphs.as_mut_ptr(),
                1,
            )
        };

        if found && glyphs[0] != 0 {
            // Position glyph at baseline: x=0, y=descent (CGContext has Y-up)
            let descent = self.font.descent();
            let positions = [CGPoint::new(0.0, descent)];
            self.font.draw_glyphs(&glyphs, &positions, ctx.clone());
        } else {
            // System font fallback for characters not in the primary font
            // (e.g. ❯ U+276F from starship prompt, Nerd Font icons, etc.)
            unsafe {
                let ch_str = CFString::new(&ch.to_string());
                let range = CFRange { location: 0, length: 1 };
                let fallback_ptr = CTFontCreateForString(
                    self.font.as_concrete_TypeRef() as *const _,
                    ch_str.as_concrete_TypeRef() as *const _,
                    range,
                );
                if !fallback_ptr.is_null() {
                    let fallback = CTFont::wrap_under_create_rule(fallback_ptr as CTFontRef);
                    let mut fb_glyphs = [0u16; 1];
                    let fb_found = fallback.get_glyphs_for_characters(
                        chars.as_ptr(),
                        fb_glyphs.as_mut_ptr(),
                        1,
                    );
                    if fb_found && fb_glyphs[0] != 0 {
                        let descent = fallback.descent();
                        let positions = [CGPoint::new(0.0, descent)];
                        fallback.draw_glyphs(&fb_glyphs, &positions, ctx.clone());
                    }
                }
            }
        }

        // Extract pixel data (no vertical flip).
        // CGBitmapContextGetData() returns pixels in standard raster order
        // (row 0 = top of image), despite CGContext using Y-up for drawing.
        // CGContext may use a larger internal row stride than bytes_per_row
        // for alignment — read with actual stride, write with expected stride.
        let actual_bpr = ctx.bytes_per_row();
        let data = ctx.data();
        let expected_bpr = w * 4;

        let mut pixels = vec![0u8; h * expected_bpr];
        for row in 0..h {
            let src_start = row * actual_bpr;
            let dst_start = row * expected_bpr;
            pixels[dst_start..dst_start + expected_bpr]
                .copy_from_slice(&data[src_start..src_start + expected_bpr]);
        }

        RasterizedGlyph {
            data: pixels,
            width: cell_width,
            height: cell_height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const JETBRAINS_MONO_TTF: &[u8] =
        include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf");

    #[test]
    fn coretext_rasterizer_creates_successfully() {
        let rast = CoreTextRasterizer::new(JETBRAINS_MONO_TTF, 26.0);
        assert!(rast.ascent() > 0.0);
        assert!(rast.descent() > 0.0);
    }

    #[test]
    fn coretext_rasterizer_advance_width_positive() {
        let rast = CoreTextRasterizer::new(JETBRAINS_MONO_TTF, 26.0);
        let advance = rast.advance_width('M');
        assert!(advance > 0.0, "advance should be positive: {advance}");
    }

    #[test]
    fn coretext_rasterize_produces_nonempty_bitmap() {
        let rast = CoreTextRasterizer::new(JETBRAINS_MONO_TTF, 26.0);
        let glyph = rast.rasterize('A', 16, 42);
        assert_eq!(glyph.width, 16);
        assert_eq!(glyph.height, 42);
        assert_eq!(glyph.data.len(), 16 * 42 * 4);
        // At least some pixels should be non-zero
        let nonzero = glyph.data.iter().filter(|&&b| b > 0).count();
        assert!(nonzero > 0, "glyph 'A' should have visible pixels");
    }

    #[test]
    fn coretext_rasterize_space_is_blank() {
        let rast = CoreTextRasterizer::new(JETBRAINS_MONO_TTF, 26.0);
        let glyph = rast.rasterize(' ', 16, 42);
        let nonzero = glyph.data.iter().filter(|&&b| b > 0).count();
        assert_eq!(nonzero, 0, "space should have no visible pixels");
    }

    #[test]
    fn coretext_rasterize_data_size_matches_dimensions() {
        let rast = CoreTextRasterizer::new(JETBRAINS_MONO_TTF, 26.0);
        for (w, h) in [(16, 42), (15, 39), (17, 45), (13, 33)] {
            let glyph = rast.rasterize('A', w, h);
            assert_eq!(
                glyph.data.len(),
                (w * h * 4) as usize,
                "data size mismatch for {}x{}: got {} expected {}",
                w, h, glyph.data.len(), w * h * 4
            );
        }
    }

    #[test]
    fn coretext_rasterize_vertical_bar_no_diagonal_shear() {
        let rast = CoreTextRasterizer::new(JETBRAINS_MONO_TTF, 26.0);
        let glyph = rast.rasterize('|', 16, 42);
        let w = glyph.width as usize;

        let mut centers: Vec<f64> = Vec::new();
        for row in 0..glyph.height as usize {
            let mut sum_x = 0.0_f64;
            let mut sum_w = 0.0_f64;
            for col in 0..w {
                let idx = (row * w + col) * 4;
                let alpha = glyph.data[idx + 3] as f64;
                if alpha > 0.0 {
                    sum_x += col as f64 * alpha;
                    sum_w += alpha;
                }
            }
            if sum_w > 0.0 {
                centers.push(sum_x / sum_w);
            }
        }

        assert!(!centers.is_empty(), "'|' should have visible rows");
        let first = centers[0];
        for (i, &c) in centers.iter().enumerate() {
            assert!(
                (c - first).abs() <= 1.0,
                "row {} center {:.1} deviates from first {:.1} — diagonal shear detected",
                i, c, first
            );
        }
    }

    #[test]
    fn coretext_rasterize_channels_consistent_grayscale() {
        let rast = CoreTextRasterizer::new(JETBRAINS_MONO_TTF, 26.0);
        let glyph = rast.rasterize('A', 16, 42);
        for (i, pixel) in glyph.data.chunks(4).enumerate() {
            let a = pixel[3];
            if a > 20 {
                let r = pixel[0] as i16;
                let g = pixel[1] as i16;
                let b = pixel[2] as i16;
                assert!(
                    (r - g).abs() <= 10 && (g - b).abs() <= 10 && (r - b).abs() <= 10,
                    "pixel {} has color fringe: R={} G={} B={} A={} — subpixel AA leak",
                    i, pixel[0], pixel[1], pixel[2], a
                );
            }
        }
    }

    #[test]
    fn coretext_rasterize_glyph_centered_in_cell() {
        let rast = CoreTextRasterizer::new(JETBRAINS_MONO_TTF, 26.0);
        let glyph = rast.rasterize('M', 16, 42);
        let w = glyph.width as usize;
        let third = w / 3;

        let mut mid_coverage = 0u64;
        for row in 0..glyph.height as usize {
            for col in third..(w - third) {
                let idx = (row * w + col) * 4;
                mid_coverage += glyph.data[idx + 3] as u64;
            }
        }

        assert!(
            mid_coverage > 0,
            "'M' should have non-zero pixels in the middle third of the cell"
        );
    }

    #[test]
    fn coretext_rasterize_t_crossbar_at_top() {
        let rast = CoreTextRasterizer::new(JETBRAINS_MONO_TTF, 26.0);
        let glyph = rast.rasterize('T', 16, 42);
        let w = glyph.width as usize;
        let h = glyph.height as usize;

        // Find glyph bounding box — check within it, not the full cell.
        // CGBitmapContext returns top-to-bottom data; the glyph sits near
        // the bottom of the cell because CGContext draws at y=descent.
        let mut first_row = h;
        let mut last_row = 0;
        for row in 0..h {
            for col in 0..w {
                let idx = (row * w + col) * 4;
                if glyph.data[idx + 3] > 0 {
                    if row < first_row { first_row = row; }
                    if row > last_row { last_row = row; }
                }
            }
        }
        assert!(last_row > first_row, "'T' glyph should have visible rows");

        // Within the glyph bounding box, top quarter should have more
        // coverage than bottom quarter (crossbar vs stem only).
        let glyph_h = last_row - first_row + 1;
        let quarter = glyph_h / 4;

        let top_coverage: u64 = (first_row..first_row + quarter)
            .flat_map(|row| (0..w).map(move |col| (row, col)))
            .map(|(row, col)| glyph.data[(row * w + col) * 4 + 3] as u64)
            .sum();
        let bottom_coverage: u64 = (last_row + 1 - quarter..=last_row)
            .flat_map(|row| (0..w).map(move |col| (row, col)))
            .map(|(row, col)| glyph.data[(row * w + col) * 4 + 3] as u64)
            .sum();

        assert!(
            top_coverage > bottom_coverage,
            "T crossbar should be at top of glyph bbox: top_coverage={top_coverage}, bottom={bottom_coverage} — glyphs may be vertically flipped"
        );
    }

    #[test]
    fn coretext_rasterize_l_stroke_on_left() {
        let rast = CoreTextRasterizer::new(JETBRAINS_MONO_TTF, 26.0);
        let glyph = rast.rasterize('L', 16, 42);
        let w = glyph.width as usize;
        let h = glyph.height as usize;

        // 'L' has a vertical stroke on the left side.
        // Sum coverage in left third vs right third of the upper half
        // (exclude bottom quarter where the horizontal bar is)
        let upper_rows = h * 3 / 4;
        let third = w / 3;
        let mut left_coverage: u64 = 0;
        let mut right_coverage: u64 = 0;
        for row in 0..upper_rows {
            for col in 0..third {
                let idx = (row * w + col) * 4;
                left_coverage += glyph.data[idx + 3] as u64;
            }
            for col in (w - third)..w {
                let idx = (row * w + col) * 4;
                right_coverage += glyph.data[idx + 3] as u64;
            }
        }

        assert!(
            left_coverage > right_coverage,
            "L vertical stroke should be on left: left_coverage={left_coverage}, right={right_coverage} — glyphs may be horizontally flipped"
        );
    }

    const JETBRAINS_MONO_BOLD_TTF: &[u8] =
        include_bytes!("../../assets/fonts/JetBrainsMono-Bold.ttf");

    #[test]
    fn coretext_rasterize_bold_has_more_coverage() {
        let regular = CoreTextRasterizer::new(JETBRAINS_MONO_TTF, 26.0);
        let bold = CoreTextRasterizer::new(JETBRAINS_MONO_BOLD_TTF, 26.0);

        let reg_glyph = regular.rasterize('A', 16, 42);
        let bold_glyph = bold.rasterize('A', 16, 42);

        let reg_coverage: u64 = reg_glyph.data.iter().map(|&b| b as u64).sum();
        let bold_coverage: u64 = bold_glyph.data.iter().map(|&b| b as u64).sum();

        assert!(
            bold_coverage >= reg_coverage,
            "bold coverage {} should be >= regular coverage {}",
            bold_coverage, reg_coverage
        );
    }

    #[test]
    fn coretext_rasterize_fallback_for_missing_glyph() {
        let rast = CoreTextRasterizer::new(JETBRAINS_MONO_TTF, 26.0);
        // U+276F (❯) may not be in JetBrains Mono — system font fallback should render it
        let glyph = rast.rasterize('\u{276F}', 16, 42);
        let nonzero = glyph.data.iter().filter(|&&b| b > 0).count();
        assert!(nonzero > 0, "fallback glyph '❯' (U+276F) should have visible pixels");
    }
}

// Claude-themed color definitions for VeloTerm.
// See conductor/ui-guide.md for full specification.

/// RGBA color represented as f32 components in [0.0, 1.0] range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// Complete theme definition for VeloTerm.
/// Each field maps to a UI element category from the UI guide.
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: &'static str,
    pub background: Color,
    pub pane_background: Color,
    pub border: Color,
    pub text_primary: Color,
    pub text_muted: Color,
    pub accent: Color,
    pub accent_hover: Color,
    pub prompt: Color,
    pub success: Color,
    pub error: Color,
    pub selection: Color,
    pub search_match: Color,
    pub search_match_active: Color,
}

impl Color {
    /// Create a Color from f32 RGBA components.
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Convert a hex color string (e.g. "#1A1816") to an RGBA Color with alpha 1.0.
    ///
    /// Accepts 6-digit hex with leading '#'. Case-insensitive.
    /// Panics on invalid format.
    pub fn from_hex(hex: &str) -> Self {
        let hex = hex
            .strip_prefix('#')
            .expect("hex color must start with '#'");
        assert!(hex.len() == 6, "hex color must be 6 digits");
        let r = u8::from_str_radix(&hex[0..2], 16).expect("invalid red hex");
        let g = u8::from_str_radix(&hex[2..4], 16).expect("invalid green hex");
        let b = u8::from_str_radix(&hex[4..6], 16).expect("invalid blue hex");
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }
}

impl Theme {
    /// Look up a theme by config name (e.g., "claude_dark").
    /// Returns None if the name is unknown.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "claude_dark" => Some(Self::claude_dark()),
            "claude_light" => Some(Self::claude_light()),
            "claude_warm" => Some(Self::claude_warm()),
            _ => None,
        }
    }

    /// Claude Dark theme — warm dark background with high contrast text.
    pub fn claude_dark() -> Self {
        Self {
            name: "Claude Dark",
            background: Color::from_hex("#1A1816"),
            pane_background: Color::from_hex("#252320"),
            border: Color::from_hex("#3D3833"),
            text_primary: Color::from_hex("#E8E5DF"),
            text_muted: Color::from_hex("#9B9389"),
            accent: Color::from_hex("#E89171"),
            accent_hover: Color::from_hex("#F5A488"),
            prompt: Color::from_hex("#E89171"),
            success: Color::from_hex("#6BCF9B"),
            error: Color::from_hex("#F57878"),
            selection: Color::from_hex("#3D2E23"),
            search_match: Color::from_hex("#5C4A1E"),
            search_match_active: Color::from_hex("#8B6914"),
        }
    }

    /// Claude Light theme — bright warm background with dark text.
    pub fn claude_light() -> Self {
        Self {
            name: "Claude Light",
            background: Color::from_hex("#FAFAF8"),
            pane_background: Color::from_hex("#FFFFFF"),
            border: Color::from_hex("#E5E3DE"),
            text_primary: Color::from_hex("#1A1816"),
            text_muted: Color::from_hex("#6B6662"),
            accent: Color::from_hex("#CC785C"),
            accent_hover: Color::from_hex("#B3654A"),
            prompt: Color::from_hex("#CC785C"),
            success: Color::from_hex("#2D7A4F"),
            error: Color::from_hex("#C44242"),
            selection: Color::from_hex("#FFE8DC"),
            search_match: Color::from_hex("#FFF0C8"),
            search_match_active: Color::from_hex("#FFD966"),
        }
    }

    /// Claude Warm theme — warmer dark variant with softened contrast.
    pub fn claude_warm() -> Self {
        Self {
            name: "Claude Warm",
            background: Color::from_hex("#2B2824"),
            pane_background: Color::from_hex("#353230"),
            border: Color::from_hex("#4A453F"),
            text_primary: Color::from_hex("#E8E3D8"),
            text_muted: Color::from_hex("#A39A8D"),
            accent: Color::from_hex("#E89171"),
            accent_hover: Color::from_hex("#F5A488"),
            prompt: Color::from_hex("#E89171"),
            success: Color::from_hex("#7FD6A6"),
            error: Color::from_hex("#F57878"),
            selection: Color::from_hex("#4A3D32"),
            search_match: Color::from_hex("#5C4A1E"),
            search_match_active: Color::from_hex("#8B6914"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helper ──────────────────────────────────────────────────────

    /// Assert two colors are equal within floating-point tolerance.
    fn assert_color_approx(actual: Color, expected: Color, label: &str) {
        let eps = 1.0 / 512.0; // < 0.5 of a u8 step (1/255 ≈ 0.0039)
        assert!(
            (actual.r - expected.r).abs() < eps
                && (actual.g - expected.g).abs() < eps
                && (actual.b - expected.b).abs() < eps
                && (actual.a - expected.a).abs() < eps,
            "{label}: expected {expected:?}, got {actual:?}"
        );
    }

    /// Build an expected Color from integer RGB (0–255).
    fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }

    // ── from_hex tests ─────────────────────────────────────────────

    #[test]
    fn hex_black() {
        assert_color_approx(Color::from_hex("#000000"), rgb(0, 0, 0), "#000000");
    }

    #[test]
    fn hex_white() {
        assert_color_approx(Color::from_hex("#FFFFFF"), rgb(255, 255, 255), "#FFFFFF");
    }

    #[test]
    fn hex_lowercase() {
        assert_color_approx(Color::from_hex("#ff8800"), rgb(255, 136, 0), "#ff8800");
    }

    #[test]
    fn hex_claude_dark_bg() {
        assert_color_approx(Color::from_hex("#1A1816"), rgb(0x1A, 0x18, 0x16), "#1A1816");
    }

    #[test]
    fn hex_accent_dark() {
        assert_color_approx(Color::from_hex("#E89171"), rgb(0xE8, 0x91, 0x71), "#E89171");
    }

    // ── Claude Dark theme tests ────────────────────────────────────

    #[test]
    fn dark_background() {
        let t = Theme::claude_dark();
        assert_color_approx(t.background, rgb(0x1A, 0x18, 0x16), "dark.background");
    }

    #[test]
    fn dark_pane_background() {
        let t = Theme::claude_dark();
        assert_color_approx(
            t.pane_background,
            rgb(0x25, 0x23, 0x20),
            "dark.pane_background",
        );
    }

    #[test]
    fn dark_border() {
        let t = Theme::claude_dark();
        assert_color_approx(t.border, rgb(0x3D, 0x38, 0x33), "dark.border");
    }

    #[test]
    fn dark_text_primary() {
        let t = Theme::claude_dark();
        assert_color_approx(t.text_primary, rgb(0xE8, 0xE5, 0xDF), "dark.text_primary");
    }

    #[test]
    fn dark_text_muted() {
        let t = Theme::claude_dark();
        assert_color_approx(t.text_muted, rgb(0x9B, 0x93, 0x89), "dark.text_muted");
    }

    #[test]
    fn dark_accent() {
        let t = Theme::claude_dark();
        assert_color_approx(t.accent, rgb(0xE8, 0x91, 0x71), "dark.accent");
    }

    #[test]
    fn dark_accent_hover() {
        let t = Theme::claude_dark();
        assert_color_approx(t.accent_hover, rgb(0xF5, 0xA4, 0x88), "dark.accent_hover");
    }

    #[test]
    fn dark_prompt() {
        let t = Theme::claude_dark();
        assert_color_approx(t.prompt, rgb(0xE8, 0x91, 0x71), "dark.prompt");
    }

    #[test]
    fn dark_success() {
        let t = Theme::claude_dark();
        assert_color_approx(t.success, rgb(0x6B, 0xCF, 0x9B), "dark.success");
    }

    #[test]
    fn dark_error() {
        let t = Theme::claude_dark();
        assert_color_approx(t.error, rgb(0xF5, 0x78, 0x78), "dark.error");
    }

    #[test]
    fn dark_selection() {
        let t = Theme::claude_dark();
        assert_color_approx(t.selection, rgb(0x3D, 0x2E, 0x23), "dark.selection");
    }

    // ── Claude Light theme tests ───────────────────────────────────

    #[test]
    fn light_background() {
        let t = Theme::claude_light();
        assert_color_approx(t.background, rgb(0xFA, 0xFA, 0xF8), "light.background");
    }

    #[test]
    fn light_pane_background() {
        let t = Theme::claude_light();
        assert_color_approx(
            t.pane_background,
            rgb(0xFF, 0xFF, 0xFF),
            "light.pane_background",
        );
    }

    #[test]
    fn light_border() {
        let t = Theme::claude_light();
        assert_color_approx(t.border, rgb(0xE5, 0xE3, 0xDE), "light.border");
    }

    #[test]
    fn light_text_primary() {
        let t = Theme::claude_light();
        assert_color_approx(t.text_primary, rgb(0x1A, 0x18, 0x16), "light.text_primary");
    }

    #[test]
    fn light_text_muted() {
        let t = Theme::claude_light();
        assert_color_approx(t.text_muted, rgb(0x6B, 0x66, 0x62), "light.text_muted");
    }

    #[test]
    fn light_accent() {
        let t = Theme::claude_light();
        assert_color_approx(t.accent, rgb(0xCC, 0x78, 0x5C), "light.accent");
    }

    #[test]
    fn light_accent_hover() {
        let t = Theme::claude_light();
        assert_color_approx(t.accent_hover, rgb(0xB3, 0x65, 0x4A), "light.accent_hover");
    }

    #[test]
    fn light_prompt() {
        let t = Theme::claude_light();
        assert_color_approx(t.prompt, rgb(0xCC, 0x78, 0x5C), "light.prompt");
    }

    #[test]
    fn light_success() {
        let t = Theme::claude_light();
        assert_color_approx(t.success, rgb(0x2D, 0x7A, 0x4F), "light.success");
    }

    #[test]
    fn light_error() {
        let t = Theme::claude_light();
        assert_color_approx(t.error, rgb(0xC4, 0x42, 0x42), "light.error");
    }

    #[test]
    fn light_selection() {
        let t = Theme::claude_light();
        assert_color_approx(t.selection, rgb(0xFF, 0xE8, 0xDC), "light.selection");
    }

    // ── Claude Warm theme tests ────────────────────────────────────

    #[test]
    fn warm_background() {
        let t = Theme::claude_warm();
        assert_color_approx(t.background, rgb(0x2B, 0x28, 0x24), "warm.background");
    }

    #[test]
    fn warm_pane_background() {
        let t = Theme::claude_warm();
        assert_color_approx(
            t.pane_background,
            rgb(0x35, 0x32, 0x30),
            "warm.pane_background",
        );
    }

    #[test]
    fn warm_border() {
        let t = Theme::claude_warm();
        assert_color_approx(t.border, rgb(0x4A, 0x45, 0x3F), "warm.border");
    }

    #[test]
    fn warm_text_primary() {
        let t = Theme::claude_warm();
        assert_color_approx(t.text_primary, rgb(0xE8, 0xE3, 0xD8), "warm.text_primary");
    }

    #[test]
    fn warm_text_muted() {
        let t = Theme::claude_warm();
        assert_color_approx(t.text_muted, rgb(0xA3, 0x9A, 0x8D), "warm.text_muted");
    }

    #[test]
    fn warm_accent() {
        let t = Theme::claude_warm();
        assert_color_approx(t.accent, rgb(0xE8, 0x91, 0x71), "warm.accent");
    }

    #[test]
    fn warm_accent_hover() {
        let t = Theme::claude_warm();
        assert_color_approx(t.accent_hover, rgb(0xF5, 0xA4, 0x88), "warm.accent_hover");
    }

    #[test]
    fn warm_prompt() {
        let t = Theme::claude_warm();
        assert_color_approx(t.prompt, rgb(0xE8, 0x91, 0x71), "warm.prompt");
    }

    #[test]
    fn warm_success() {
        let t = Theme::claude_warm();
        assert_color_approx(t.success, rgb(0x7F, 0xD6, 0xA6), "warm.success");
    }

    #[test]
    fn warm_error() {
        let t = Theme::claude_warm();
        assert_color_approx(t.error, rgb(0xF5, 0x78, 0x78), "warm.error");
    }

    #[test]
    fn warm_selection() {
        let t = Theme::claude_warm();
        assert_color_approx(t.selection, rgb(0x4A, 0x3D, 0x32), "warm.selection");
    }

    // ── Theme name tests ───────────────────────────────────────────

    #[test]
    fn theme_names() {
        assert_eq!(Theme::claude_dark().name, "Claude Dark");
        assert_eq!(Theme::claude_light().name, "Claude Light");
        assert_eq!(Theme::claude_warm().name, "Claude Warm");
    }

    // ── from_name tests ──────────────────────────────────────────

    #[test]
    fn from_name_claude_dark() {
        let theme = Theme::from_name("claude_dark").unwrap();
        assert_eq!(theme.name, "Claude Dark");
    }

    #[test]
    fn from_name_claude_light() {
        let theme = Theme::from_name("claude_light").unwrap();
        assert_eq!(theme.name, "Claude Light");
    }

    #[test]
    fn from_name_claude_warm() {
        let theme = Theme::from_name("claude_warm").unwrap();
        assert_eq!(theme.name, "Claude Warm");
    }

    #[test]
    fn from_name_unknown_returns_none() {
        assert!(Theme::from_name("nonexistent").is_none());
    }
}

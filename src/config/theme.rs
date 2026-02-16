// Anthropic brand design system — TerminalTheme definitions for VeloTerm.
//
// All colors derived from Anthropic's brand tokens.
// Replaces the old custom Color struct with iced_core::Color (same layout: r, g, b, a: f32).

/// Re-export iced_core::Color as the canonical Color type.
/// All existing `use crate::config::theme::Color` imports resolve to iced_core::Color
/// without touching import statements across the codebase.
pub type Color = iced_core::Color;

// ── Helper constructors ─────────────────────────────────────────────

/// Construct a Color from f32 RGBA components (drop-in for old Color::new).
pub const fn color_new(r: f32, g: f32, b: f32, a: f32) -> Color {
    Color { r, g, b, a }
}

/// Construct a Color from u8 RGB values (opaque).
const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}

/// Construct a Color from u8 RGB + f32 alpha.
const fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color {
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a,
    }
}

/// Parse a hex color string (e.g. "#d97757") to a Color with alpha 1.0.
/// Panics on invalid format. For non-const contexts only.
pub fn from_hex(hex: &str) -> Color {
    let hex = hex
        .strip_prefix('#')
        .expect("hex color must start with '#'");
    assert!(hex.len() == 6, "hex color must be 6 digits");
    let r = u8::from_str_radix(&hex[0..2], 16).expect("invalid red hex");
    let g = u8::from_str_radix(&hex[2..4], 16).expect("invalid green hex");
    let b = u8::from_str_radix(&hex[4..6], 16).expect("invalid blue hex");
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}

// ── Theme struct ────────────────────────────────────────────────────

/// Complete theme definition for VeloTerm.
/// Fields follow the Anthropic design system token hierarchy.
#[derive(Debug, Clone, Copy)]
pub struct TerminalTheme {
    /// Theme config name (e.g. "Warm Dark").
    pub name: &'static str,

    // ── Surface hierarchy ────────────────────────────────────
    /// Main window / terminal content background.
    pub bg_deep: Color,
    /// Title bar, status bar background.
    pub bg_surface: Color,
    /// Sidebar background.
    pub bg_raised: Color,
    /// Hover state for interactive elements.
    pub bg_hover: Color,
    /// Active / selected state (e.g. current tab).
    pub bg_active: Color,
    /// Terminal input area (may differ from bg_deep in light theme).
    pub bg_input: Color,

    // ── Text hierarchy ───────────────────────────────────────
    /// Commands, file names, primary content.
    pub text_primary: Color,
    /// Output text, secondary content.
    pub text_secondary: Color,
    /// Timestamps, metadata, line numbers.
    pub text_muted: Color,
    /// Decorative separators, disabled text, placeholder.
    pub text_ghost: Color,

    // ── Accent colors ────────────────────────────────────────
    /// Prompt marker, cursor, active tab indicator, primary CTA.
    pub accent_orange: Color,
    /// Directory names, prompt path, links.
    pub accent_blue: Color,
    /// Executables, success states, status dot.
    pub accent_green: Color,
    /// Symlink names.
    pub accent_purple: Color,
    /// Errors, stderr, deletions.
    pub accent_red: Color,
    /// Warnings, modified indicators.
    pub accent_yellow: Color,

    // ── Borders ──────────────────────────────────────────────
    /// Subtle dividers between major sections.
    pub border_subtle: Color,
    /// More prominent borders (status bar dividers).
    pub border_visible: Color,
    /// High-emphasis borders (focused input, drag handles).
    pub border_strong: Color,

    // ── Selection & search ───────────────────────────────────
    /// Text selection background.
    pub selection: Color,
    /// Inactive search match background.
    pub search_match: Color,
    /// Active search match background.
    pub search_match_active: Color,

    // ── ANSI 16-color palette ────────────────────────────────
    /// Standard terminal ANSI colors 0–15.
    pub ansi: [Color; 16],
}

impl TerminalTheme {
    /// Look up a theme by config name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "warm_dark" => Some(DARK),
            "midnight" => Some(MIDNIGHT),
            "ember" => Some(EMBER),
            "dusk" => Some(DUSK),
            "light" => Some(LIGHT),
            // Legacy aliases
            "claude_dark" | "claude_warm" => {
                log::warn!("legacy theme name '{name}', use 'warm_dark' instead");
                Some(DARK)
            }
            "claude_light" => {
                log::warn!("legacy theme name 'claude_light', use 'light' instead");
                Some(LIGHT)
            }
            _ => None,
        }
    }

    /// List of available themes: (config_name, display_name).
    pub fn available_themes() -> &'static [(&'static str, &'static str)] {
        &[
            ("warm_dark", "Warm Dark"),
            ("midnight", "Midnight"),
            ("ember", "Ember"),
            ("dusk", "Dusk"),
            ("light", "Light"),
        ]
    }

    /// Convenience: construct the default dark theme.
    pub fn warm_dark() -> Self {
        DARK
    }

    /// Convenience: construct the light theme.
    pub fn light() -> Self {
        LIGHT
    }

    /// Convenience: construct the midnight theme.
    pub fn midnight() -> Self {
        MIDNIGHT
    }

    /// Convenience: construct the ember theme.
    pub fn ember() -> Self {
        EMBER
    }

    /// Convenience: construct the dusk theme.
    pub fn dusk() -> Self {
        DUSK
    }
}

// ── Layout constants ────────────────────────────────────────────────

pub mod layout {
    pub const TITLEBAR_HEIGHT: f32 = 38.0;
    pub const STATUSBAR_HEIGHT: f32 = 28.0;
    pub const SIDEBAR_WIDTH: f32 = 200.0;
    pub const SIDEBAR_MIN_WIDTH: f32 = 160.0;
    pub const SIDEBAR_MAX_WIDTH: f32 = 300.0;
    pub const TERMINAL_PAD_TOP: f32 = 16.0;
    pub const TERMINAL_PAD_BOTTOM: f32 = 16.0;
    pub const TERMINAL_PAD_LEFT: f32 = 22.0;
    pub const TERMINAL_PAD_RIGHT: f32 = 22.0;
    pub const SIDEBAR_PAD_HORIZONTAL: f32 = 6.0;
    pub const SIDEBAR_HEADER_PAD_TOP: f32 = 14.0;
    pub const SIDEBAR_HEADER_PAD_BOTTOM: f32 = 8.0;
    pub const TAB_PAD_VERTICAL: f32 = 8.0;
    pub const TAB_PAD_HORIZONTAL: f32 = 10.0;
    pub const TAB_GAP: f32 = 2.0;
    pub const TAB_INDICATOR_SIZE: f32 = 6.0;
    pub const TAB_BORDER_RADIUS: f32 = 4.0;
    pub const SPLIT_DIVIDER_WIDTH: f32 = 1.0;
    pub const SPLIT_HIT_AREA: f32 = 11.0;
    pub const SPLIT_MIN_PANE_WIDTH: f32 = 200.0;
    pub const SPLIT_SNAP_THRESHOLD: f32 = 20.0;
    pub const SCROLLBAR_WIDTH: f32 = 6.0;
    pub const SCROLLBAR_RADIUS: f32 = 3.0;
    pub const CURSOR_WIDTH: f32 = 8.0;
    pub const CURSOR_RADIUS: f32 = 1.0;
    pub const STATUS_DOT_SIZE: f32 = 6.0;
    pub const STATUS_DIVIDER_HEIGHT: f32 = 12.0;
    pub const STATUS_ITEM_GAP: f32 = 14.0;
}

// ── Typography constants ────────────────────────────────────────────

pub mod typography {
    pub const FONT_UI: &str = "DM Sans";
    pub const FONT_MONO: &str = "JetBrains Mono";
    pub const FONT_UI_FALLBACK: &str = "Helvetica";
    pub const FONT_MONO_FALLBACK: &str = "Menlo";
    pub const SIZE_TITLEBAR: f32 = 12.5;
    pub const SIZE_SIDEBAR_HEADER: f32 = 10.0;
    pub const SIZE_SIDEBAR_TAB: f32 = 13.0;
    pub const SIZE_SIDEBAR_SHORTCUT: f32 = 10.0;
    pub const SIZE_NEW_SESSION_BTN: f32 = 12.0;
    pub const SIZE_TERMINAL: f32 = 13.0;
    pub const SIZE_STATUSBAR: f32 = 11.0;
    pub const SIZE_PROMPT_VERSION: f32 = 12.0;
    pub const TERMINAL_LINE_HEIGHT: f32 = 1.65;
}

// ── Animation constants ─────────────────────────────────────────────

pub mod animation {
    use std::time::Duration;

    pub const HOVER_DURATION: Duration = Duration::from_millis(120);
    pub const SCROLLBAR_HIDE_DELAY: Duration = Duration::from_millis(1500);
    pub const SCROLLBAR_FADE_DURATION: Duration = Duration::from_millis(300);
    pub const CURSOR_BLINK_CYCLE: Duration = Duration::from_millis(1100);
    pub const CURSOR_BLINK_RESUME_DELAY: Duration = Duration::from_millis(500);
    pub const DIVIDER_HOVER_DURATION: Duration = Duration::from_millis(100);
    pub const THEME_SWITCH_DURATION: Duration = Duration::from_millis(250);
    pub const STATUS_PULSE_CYCLE: Duration = Duration::from_millis(2000);
}

// ── DARK THEME (default) ────────────────────────────────────────────

pub const DARK: TerminalTheme = TerminalTheme {
    name: "Warm Dark",

    // Surfaces
    bg_deep:    rgb(25, 25, 24),    // #191918
    bg_surface: rgb(30, 30, 29),    // #1e1e1d
    bg_raised:  rgb(37, 37, 36),    // #252524
    bg_hover:   rgb(44, 44, 42),    // #2c2c2a
    bg_active:  rgb(51, 51, 49),    // #333331
    bg_input:   rgb(25, 25, 24),    // #191918

    // Text
    text_primary:   rgb(232, 230, 220), // #e8e6dc
    text_secondary: rgb(176, 174, 165), // #b0aea5
    text_muted:     rgb(122, 120, 111), // #7a786f
    text_ghost:     rgb(74, 73, 69),    // #4a4945

    // Accents
    accent_orange: rgb(217, 119, 87),  // #d97757
    accent_blue:   rgb(106, 155, 204), // #6a9bcc
    accent_green:  rgb(120, 140, 93),  // #788c5d
    accent_purple: rgb(196, 167, 231), // #c4a7e7
    accent_red:    rgb(196, 91, 91),   // #c45b5b
    accent_yellow: rgb(201, 168, 76),  // #c9a84c

    // Borders
    border_subtle:  rgba(250, 249, 245, 0.06),
    border_visible: rgba(250, 249, 245, 0.10),
    border_strong:  rgba(250, 249, 245, 0.15),

    // Selection & search
    selection:           rgb(61, 46, 35),   // #3D2E23 — warm dark selection
    search_match:        rgb(92, 74, 30),   // #5C4A1E — yellow-brown match
    search_match_active: rgb(139, 105, 20), // #8B6914 — bright gold active match

    // ANSI palette
    ansi: [
        rgb(25, 25, 24),      // 0  Black
        rgb(196, 91, 91),     // 1  Red
        rgb(120, 140, 93),    // 2  Green
        rgb(201, 168, 76),    // 3  Yellow
        rgb(106, 155, 204),   // 4  Blue
        rgb(196, 167, 231),   // 5  Magenta
        rgb(125, 175, 168),   // 6  Cyan
        rgb(176, 174, 165),   // 7  White (default fg)
        rgb(74, 73, 69),      // 8  Bright Black (comments)
        rgb(217, 114, 107),   // 9  Bright Red
        rgb(143, 168, 109),   // 10 Bright Green
        rgb(212, 185, 94),    // 11 Bright Yellow
        rgb(130, 176, 217),   // 12 Bright Blue
        rgb(209, 184, 238),   // 13 Bright Magenta
        rgb(143, 194, 187),   // 14 Bright Cyan
        rgb(232, 230, 220),   // 15 Bright White (bold fg)
    ],
};

// ── LIGHT THEME ─────────────────────────────────────────────────────

pub const LIGHT: TerminalTheme = TerminalTheme {
    name: "Light",

    // Surfaces
    bg_deep:    rgb(244, 243, 238),  // #f4f3ee
    bg_surface: rgb(234, 232, 224),  // #eae8e0
    bg_raised:  rgb(238, 236, 229),  // #eeece5
    bg_hover:   rgb(228, 226, 218),  // #e4e2da
    bg_active:  rgb(223, 221, 213),  // #dfddd5
    bg_input:   rgb(250, 249, 245),  // #faf9f5

    // Text
    text_primary:   rgb(26, 26, 25),    // #1a1a19
    text_secondary: rgb(74, 72, 67),    // #4a4843
    text_muted:     rgb(138, 135, 126), // #8a877e
    text_ghost:     rgb(184, 181, 172), // #b8b5ac

    // Accents (deeper for light bg contrast)
    accent_orange: rgb(193, 95, 60),   // #c15f3c
    accent_blue:   rgb(74, 125, 168),  // #4a7da8
    accent_green:  rgb(93, 122, 66),   // #5d7a42
    accent_purple: rgb(124, 94, 160),  // #7c5ea0
    accent_red:    rgb(184, 76, 63),   // #b84c3f
    accent_yellow: rgb(154, 123, 46),  // #9a7b2e

    // Borders
    border_subtle:  rgba(20, 20, 19, 0.06),
    border_visible: rgba(20, 20, 19, 0.10),
    border_strong:  rgba(20, 20, 19, 0.15),

    // Selection & search
    selection:           rgb(255, 232, 220), // #FFE8DC — warm light selection
    search_match:        rgb(255, 240, 200), // #FFF0C8 — yellow light match
    search_match_active: rgb(255, 217, 102), // #FFD966 — bright gold active match

    // ANSI palette
    ansi: [
        rgb(26, 26, 25),      // 0  Black
        rgb(184, 76, 63),     // 1  Red
        rgb(93, 122, 66),     // 2  Green
        rgb(154, 123, 46),    // 3  Yellow
        rgb(74, 125, 168),    // 4  Blue
        rgb(124, 94, 160),    // 5  Magenta
        rgb(77, 138, 131),    // 6  Cyan
        rgb(74, 72, 67),      // 7  White
        rgb(138, 135, 126),   // 8  Bright Black
        rgb(196, 91, 78),     // 9  Bright Red
        rgb(109, 138, 80),    // 10 Bright Green
        rgb(168, 136, 58),    // 11 Bright Yellow
        rgb(90, 141, 184),    // 12 Bright Blue
        rgb(140, 110, 176),   // 13 Bright Magenta
        rgb(93, 154, 147),    // 14 Bright Cyan
        rgb(26, 26, 25),      // 15 Bright White
    ],
};

// ── MIDNIGHT THEME ─────────────────────────────────────────────────

pub const MIDNIGHT: TerminalTheme = TerminalTheme {
    name: "Midnight",

    // Surfaces
    bg_deep:    rgb(0x12, 0x14, 0x1a),  // #12141a
    bg_surface: rgb(0x17, 0x1a, 0x21),  // #171a21
    bg_raised:  rgb(0x1e, 0x22, 0x30),  // #1e2230
    bg_hover:   rgb(0x26, 0x2a, 0x38),  // #262a38
    bg_active:  rgb(0x2e, 0x33, 0x40),  // #2e3340
    bg_input:   rgb(0x12, 0x14, 0x1a),  // #12141a

    // Text
    text_primary:   rgb(0xd0, 0xd4, 0xdc), // #d0d4dc
    text_secondary: rgb(0x8b, 0x95, 0xa8), // #8b95a8
    text_muted:     rgb(0x5a, 0x62, 0x78), // #5a6278
    text_ghost:     rgb(0x36, 0x3b, 0x48), // #363b48

    // Accents
    accent_orange: rgb(0xd9, 0x77, 0x57), // #d97757
    accent_blue:   rgb(0x7b, 0xaa, 0xd4), // #7baad4
    accent_green:  rgb(0x7d, 0x9b, 0x6a), // #7d9b6a
    accent_purple: rgb(0xb8, 0xa5, 0xd6), // #b8a5d6
    accent_red:    rgb(0xc4, 0x5b, 0x5b), // #c45b5b
    accent_yellow: rgb(0xc9, 0xa8, 0x4c), // #c9a84c

    // Borders
    border_subtle:  rgba(200, 210, 230, 0.06),
    border_visible: rgba(200, 210, 230, 0.10),
    border_strong:  rgba(200, 210, 230, 0.15),

    // Selection & search
    selection:           rgb(35, 40, 55),
    search_match:        rgb(70, 62, 30),
    search_match_active: rgb(110, 90, 20),

    // ANSI palette
    ansi: [
        rgb(0x12, 0x14, 0x1a), // 0  Black
        rgb(0xc4, 0x5b, 0x5b), // 1  Red
        rgb(0x7d, 0x9b, 0x6a), // 2  Green
        rgb(0xc9, 0xa8, 0x4c), // 3  Yellow
        rgb(0x7b, 0xaa, 0xd4), // 4  Blue
        rgb(0xb8, 0xa5, 0xd6), // 5  Magenta
        rgb(0x6d, 0xb3, 0xaa), // 6  Cyan
        rgb(0x8b, 0x95, 0xa8), // 7  White
        rgb(0x36, 0x3b, 0x48), // 8  Bright Black
        rgb(0xd9, 0x72, 0x6b), // 9  Bright Red
        rgb(0x8f, 0xa8, 0x6d), // 10 Bright Green
        rgb(0xd4, 0xb9, 0x5e), // 11 Bright Yellow
        rgb(0x82, 0xb0, 0xd9), // 12 Bright Blue
        rgb(0xd1, 0xb8, 0xee), // 13 Bright Magenta
        rgb(0x8f, 0xc2, 0xbb), // 14 Bright Cyan
        rgb(0xd0, 0xd4, 0xdc), // 15 Bright White
    ],
};

// ── EMBER THEME ────────────────────────────────────────────────────

pub const EMBER: TerminalTheme = TerminalTheme {
    name: "Ember",

    // Surfaces
    bg_deep:    rgb(0x1a, 0x14, 0x12),  // #1a1412
    bg_surface: rgb(0x20, 0x19, 0x16),  // #201916
    bg_raised:  rgb(0x28, 0x20, 0x1c),  // #28201c
    bg_hover:   rgb(0x33, 0x29, 0x24),  // #332924
    bg_active:  rgb(0x3d, 0x31, 0x2b),  // #3d312b
    bg_input:   rgb(0x1a, 0x14, 0x12),  // #1a1412

    // Text
    text_primary:   rgb(0xe0, 0xd8, 0xcc), // #e0d8cc
    text_secondary: rgb(0xa8, 0x90, 0x80), // #a89080
    text_muted:     rgb(0x7a, 0x6b, 0x5c), // #7a6b5c
    text_ghost:     rgb(0x4a, 0x3d, 0x34), // #4a3d34

    // Accents
    accent_orange: rgb(0xd9, 0x77, 0x57), // #d97757
    accent_blue:   rgb(0x6a, 0x9b, 0xcc), // #6a9bcc
    accent_green:  rgb(0x8a, 0x9b, 0x68), // #8a9b68
    accent_purple: rgb(0xc4, 0xa0, 0xd0), // #c4a0d0
    accent_red:    rgb(0xc4, 0x5b, 0x5b), // #c45b5b
    accent_yellow: rgb(0xc9, 0xa8, 0x4c), // #c9a84c

    // Borders
    border_subtle:  rgba(240, 220, 200, 0.06),
    border_visible: rgba(240, 220, 200, 0.10),
    border_strong:  rgba(240, 220, 200, 0.15),

    // Selection & search
    selection:           rgb(55, 38, 30),
    search_match:        rgb(80, 65, 28),
    search_match_active: rgb(120, 95, 20),

    // ANSI palette
    ansi: [
        rgb(0x1a, 0x14, 0x12), // 0  Black
        rgb(0xc4, 0x5b, 0x5b), // 1  Red
        rgb(0x8a, 0x9b, 0x68), // 2  Green
        rgb(0xc9, 0xa8, 0x4c), // 3  Yellow
        rgb(0x6a, 0x9b, 0xcc), // 4  Blue
        rgb(0xc4, 0xa0, 0xd0), // 5  Magenta
        rgb(0x7d, 0xaf, 0xa8), // 6  Cyan
        rgb(0xa8, 0x90, 0x80), // 7  White
        rgb(0x4a, 0x3d, 0x34), // 8  Bright Black
        rgb(0xd9, 0x72, 0x6b), // 9  Bright Red
        rgb(0x8f, 0xa8, 0x6d), // 10 Bright Green
        rgb(0xd4, 0xb9, 0x5e), // 11 Bright Yellow
        rgb(0x82, 0xb0, 0xd9), // 12 Bright Blue
        rgb(0xd1, 0xb8, 0xee), // 13 Bright Magenta
        rgb(0x8f, 0xc2, 0xbb), // 14 Bright Cyan
        rgb(0xe0, 0xd8, 0xcc), // 15 Bright White
    ],
};

// ── DUSK THEME ─────────────────────────────────────────────────────

pub const DUSK: TerminalTheme = TerminalTheme {
    name: "Dusk",

    // Surfaces
    bg_deep:    rgb(0x2c, 0x2b, 0x28),  // #2c2b28
    bg_surface: rgb(0x33, 0x32, 0x30),  // #333230
    bg_raised:  rgb(0x3a, 0x39, 0x37),  // #3a3937
    bg_hover:   rgb(0x44, 0x43, 0x3f),  // #44433f
    bg_active:  rgb(0x4e, 0x4d, 0x48),  // #4e4d48
    bg_input:   rgb(0x28, 0x27, 0x24),  // #282724

    // Text
    text_primary:   rgb(0xec, 0xe9, 0xe0), // #ece9e0
    text_secondary: rgb(0xb5, 0xb2, 0xa8), // #b5b2a8
    text_muted:     rgb(0x8a, 0x87, 0x7d), // #8a877d
    text_ghost:     rgb(0x5e, 0x5c, 0x55), // #5e5c55

    // Accents
    accent_orange: rgb(0xd9, 0x77, 0x57), // #d97757
    accent_blue:   rgb(0x6a, 0x9b, 0xcc), // #6a9bcc
    accent_green:  rgb(0x78, 0x8c, 0x5d), // #788c5d
    accent_purple: rgb(0xc4, 0xa7, 0xe7), // #c4a7e7
    accent_red:    rgb(0xc4, 0x5b, 0x5b), // #c45b5b
    accent_yellow: rgb(0xc9, 0xa8, 0x4c), // #c9a84c

    // Borders
    border_subtle:  rgba(250, 249, 245, 0.08),
    border_visible: rgba(250, 249, 245, 0.12),
    border_strong:  rgba(250, 249, 245, 0.18),

    // Selection & search
    selection:           rgb(70, 55, 42),
    search_match:        rgb(100, 82, 35),
    search_match_active: rgb(145, 110, 25),

    // ANSI palette
    ansi: [
        rgb(0x2c, 0x2b, 0x28), // 0  Black
        rgb(0xc4, 0x5b, 0x5b), // 1  Red
        rgb(0x78, 0x8c, 0x5d), // 2  Green
        rgb(0xc9, 0xa8, 0x4c), // 3  Yellow
        rgb(0x6a, 0x9b, 0xcc), // 4  Blue
        rgb(0xc4, 0xa7, 0xe7), // 5  Magenta
        rgb(0x7d, 0xaf, 0xa8), // 6  Cyan
        rgb(0xb5, 0xb2, 0xa8), // 7  White
        rgb(0x5e, 0x5c, 0x55), // 8  Bright Black
        rgb(0xd9, 0x72, 0x6b), // 9  Bright Red
        rgb(0x8f, 0xa8, 0x6d), // 10 Bright Green
        rgb(0xd4, 0xb9, 0x5e), // 11 Bright Yellow
        rgb(0x82, 0xb0, 0xd9), // 12 Bright Blue
        rgb(0xd1, 0xb8, 0xee), // 13 Bright Magenta
        rgb(0x8f, 0xc2, 0xbb), // 14 Bright Cyan
        rgb(0xec, 0xe9, 0xe0), // 15 Bright White
    ],
};

// ── Theme selection enum ────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    WarmDark,
    Midnight,
    Ember,
    Dusk,
    Light,
}

impl ThemeMode {
    pub fn theme(&self) -> &'static TerminalTheme {
        match self {
            ThemeMode::WarmDark => &DARK,
            ThemeMode::Midnight => &MIDNIGHT,
            ThemeMode::Ember => &EMBER,
            ThemeMode::Dusk => &DUSK,
            ThemeMode::Light => &LIGHT,
        }
    }

    pub fn toggle(&self) -> Self {
        match self {
            ThemeMode::WarmDark => ThemeMode::Midnight,
            ThemeMode::Midnight => ThemeMode::Ember,
            ThemeMode::Ember => ThemeMode::Dusk,
            ThemeMode::Dusk => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::WarmDark,
        }
    }
}

impl Default for ThemeMode {
    fn default() -> Self {
        ThemeMode::WarmDark
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_color_approx(actual: Color, expected: Color, label: &str) {
        let eps = 1.0 / 512.0;
        assert!(
            (actual.r - expected.r).abs() < eps
                && (actual.g - expected.g).abs() < eps
                && (actual.b - expected.b).abs() < eps
                && (actual.a - expected.a).abs() < eps,
            "{label}: expected {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn hex_black() {
        assert_color_approx(from_hex("#000000"), rgb(0, 0, 0), "#000000");
    }

    #[test]
    fn hex_white() {
        assert_color_approx(from_hex("#FFFFFF"), rgb(255, 255, 255), "#FFFFFF");
    }

    #[test]
    fn hex_lowercase() {
        assert_color_approx(from_hex("#ff8800"), rgb(255, 136, 0), "#ff8800");
    }

    #[test]
    fn hex_accent_orange() {
        assert_color_approx(from_hex("#d97757"), rgb(0xD9, 0x77, 0x57), "#d97757");
    }

    // ── Dark theme spot checks ──────────────────────────────────────

    #[test]
    fn dark_bg_deep() {
        assert_color_approx(DARK.bg_deep, rgb(25, 25, 24), "dark.bg_deep");
    }

    #[test]
    fn dark_bg_surface() {
        assert_color_approx(DARK.bg_surface, rgb(30, 30, 29), "dark.bg_surface");
    }

    #[test]
    fn dark_text_primary() {
        assert_color_approx(DARK.text_primary, rgb(232, 230, 220), "dark.text_primary");
    }

    #[test]
    fn dark_accent_orange() {
        assert_color_approx(DARK.accent_orange, rgb(217, 119, 87), "dark.accent_orange");
    }

    #[test]
    fn dark_accent_green() {
        assert_color_approx(DARK.accent_green, rgb(120, 140, 93), "dark.accent_green");
    }

    #[test]
    fn dark_accent_blue() {
        assert_color_approx(DARK.accent_blue, rgb(106, 155, 204), "dark.accent_blue");
    }

    #[test]
    fn dark_accent_red() {
        assert_color_approx(DARK.accent_red, rgb(196, 91, 91), "dark.accent_red");
    }

    #[test]
    fn dark_selection() {
        assert_color_approx(DARK.selection, rgb(0x3D, 0x2E, 0x23), "dark.selection");
    }

    #[test]
    fn dark_border_subtle_alpha() {
        assert!((DARK.border_subtle.a - 0.06).abs() < 0.01, "dark.border_subtle alpha should be ~0.06");
    }

    #[test]
    fn dark_border_visible_alpha() {
        assert!((DARK.border_visible.a - 0.10).abs() < 0.01, "dark.border_visible alpha should be ~0.10");
    }

    #[test]
    fn dark_ansi_palette_length() {
        assert_eq!(DARK.ansi.len(), 16);
    }

    // ── Light theme spot checks ─────────────────────────────────────

    #[test]
    fn light_bg_deep() {
        assert_color_approx(LIGHT.bg_deep, rgb(244, 243, 238), "light.bg_deep");
    }

    #[test]
    fn light_text_primary() {
        assert_color_approx(LIGHT.text_primary, rgb(26, 26, 25), "light.text_primary");
    }

    #[test]
    fn light_accent_orange() {
        assert_color_approx(LIGHT.accent_orange, rgb(193, 95, 60), "light.accent_orange");
    }

    #[test]
    fn light_selection() {
        assert_color_approx(LIGHT.selection, rgb(0xFF, 0xE8, 0xDC), "light.selection");
    }

    // ── from_name tests ─────────────────────────────────────────────

    #[test]
    fn from_name_warm_dark() {
        let theme = TerminalTheme::from_name("warm_dark").unwrap();
        assert_eq!(theme.name, "Warm Dark");
    }

    #[test]
    fn from_name_midnight() {
        let theme = TerminalTheme::from_name("midnight").unwrap();
        assert_eq!(theme.name, "Midnight");
    }

    #[test]
    fn from_name_ember() {
        let theme = TerminalTheme::from_name("ember").unwrap();
        assert_eq!(theme.name, "Ember");
    }

    #[test]
    fn from_name_dusk() {
        let theme = TerminalTheme::from_name("dusk").unwrap();
        assert_eq!(theme.name, "Dusk");
    }

    #[test]
    fn from_name_light() {
        let theme = TerminalTheme::from_name("light").unwrap();
        assert_eq!(theme.name, "Light");
    }

    #[test]
    fn from_name_legacy_claude_dark() {
        let theme = TerminalTheme::from_name("claude_dark").unwrap();
        assert_eq!(theme.name, "Warm Dark");
    }

    #[test]
    fn from_name_legacy_claude_light() {
        let theme = TerminalTheme::from_name("claude_light").unwrap();
        assert_eq!(theme.name, "Light");
    }

    #[test]
    fn from_name_unknown_returns_none() {
        assert!(TerminalTheme::from_name("nonexistent").is_none());
    }

    #[test]
    fn available_themes_returns_five() {
        let themes = TerminalTheme::available_themes();
        assert_eq!(themes.len(), 5);
        for &(config_name, display_name) in themes {
            let t = TerminalTheme::from_name(config_name)
                .unwrap_or_else(|| panic!("{} should resolve", config_name));
            assert_eq!(t.name, display_name);
        }
    }

    // ── color_new helper ────────────────────────────────────────────

    #[test]
    fn color_new_creates_correct_color() {
        let c = color_new(0.5, 0.25, 0.75, 1.0);
        assert!((c.r - 0.5).abs() < 0.001);
        assert!((c.g - 0.25).abs() < 0.001);
        assert!((c.b - 0.75).abs() < 0.001);
        assert!((c.a - 1.0).abs() < 0.001);
    }

    // ── ThemeMode ───────────────────────────────────────────────────

    #[test]
    fn theme_mode_warm_dark_returns_warm_dark() {
        let mode = ThemeMode::WarmDark;
        assert_eq!(mode.theme().name, "Warm Dark");
    }

    #[test]
    fn theme_mode_toggle_cycles_all_five() {
        let mode = ThemeMode::WarmDark;
        let mode = mode.toggle();
        assert_eq!(mode, ThemeMode::Midnight);
        let mode = mode.toggle();
        assert_eq!(mode, ThemeMode::Ember);
        let mode = mode.toggle();
        assert_eq!(mode, ThemeMode::Dusk);
        let mode = mode.toggle();
        assert_eq!(mode, ThemeMode::Light);
        let mode = mode.toggle();
        assert_eq!(mode, ThemeMode::WarmDark);
    }
}

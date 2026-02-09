// iced_wgpu integration layer — Anthropic design UI chrome
//
// Renders title bar, tab bar, pane headers with circled-digit badges,
// accent stripes, rounded pane containers with borders/shadows, and
// status bar as an iced overlay on top of the custom wgpu terminal renderer.

use crate::config::theme::Theme;
use iced_graphics::Viewport;
use iced_runtime::user_interface::{Cache, UserInterface};
use iced_wgpu::Engine;
use iced_widget::{
    button, column, container, pin, row, stack, text, MouseArea, Row, Stack,
};

/// Horizontal space filler (iced 0.14: space::horizontal, not horizontal_space).
fn hspace<'a>() -> iced_widget::Space {
    iced_widget::Space::new().width(iced_core::Length::Fill)
}

/// Messages produced by iced UI widgets, returned to the application for processing.
#[derive(Debug, Clone)]
pub enum UiMessage {
    TabSelected(usize),
    TabClosed(usize),
    NewTab,
    TabHovered(Option<usize>),
    Noop,
}

/// Tab descriptor for the iced widget tree.
#[derive(Debug, Clone)]
pub struct TabInfo {
    pub title: String,
    pub is_active: bool,
    pub has_notification: bool,
}

/// Pane descriptor for the iced widget tree.
/// Positions are in physical pixels relative to the content area origin (after header + tab bar).
#[derive(Debug, Clone)]
pub struct PaneInfo {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub is_focused: bool,
    pub index: usize,
    pub title: String,
    pub shell_name: String,
    /// Scrollbar thumb rect in physical pixels relative to content area origin.
    /// (x, y, width, height). None if no scrollbar visible.
    pub scrollbar_thumb: Option<(f32, f32, f32, f32)>,
    /// Scrollbar opacity (0.0 = hidden, 1.0 = fully visible).
    pub scrollbar_alpha: f32,
}

/// Divider between panes, for visual rendering.
#[derive(Debug, Clone)]
pub struct DividerDisplay {
    /// Physical pixel rect relative to content area origin.
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub is_hovered: bool,
}

/// State snapshot passed to the iced widget tree each frame.
pub struct UiState<'a> {
    pub tabs: Vec<TabInfo>,
    pub active_tab_index: usize,
    pub hovered_tab: Option<usize>,
    pub active_pane_index: usize,
    pub panes: Vec<PaneInfo>,
    pub pane_count: usize,
    pub is_zoomed: bool,
    pub theme: &'a Theme,
    pub window_width: f32,
    pub window_height: f32,
    pub scale_factor: f32,
    /// Search bar state.
    pub search_active: bool,
    pub search_query: String,
    pub search_current: usize,
    pub search_total: usize,
    pub search_error: bool,
    /// Dividers between panes.
    pub dividers: Vec<DividerDisplay>,
    /// Whether a visual bell flash is active.
    pub bell_flash: bool,
    /// Command palette state.
    pub palette_active: bool,
    pub palette_query: String,
    pub palette_items: Vec<(String, String, String)>, // (name, description, keybinding)
    pub palette_selected: usize,
}

/// Holds iced rendering state: renderer, viewport, event queue, and UI cache.
pub struct IcedLayer {
    renderer: iced_wgpu::Renderer,
    viewport: Viewport,
    cache: Cache,
    events: Vec<iced_core::Event>,
    cursor: iced_core::mouse::Cursor,
    format: wgpu::TextureFormat,
}

/// Convert a theme Color to an iced Color.
fn to_iced_color(c: &crate::config::theme::Color) -> iced_core::Color {
    iced_core::Color::from_rgba(c.r, c.g, c.b, c.a)
}

/// Circled digit badge for pane index (1-based).
fn pane_badge(index: usize) -> &'static str {
    match index + 1 {
        1 => "\u{2460}",
        2 => "\u{2461}",
        3 => "\u{2462}",
        4 => "\u{2463}",
        5 => "\u{2464}",
        6 => "\u{2465}",
        7 => "\u{2466}",
        8 => "\u{2467}",
        9 => "\u{2468}",
        _ => "\u{2460}",
    }
}

/// Header bar height in logical pixels.
const HEADER_BAR_HEIGHT: f32 = 46.0;
/// Tab bar height in logical pixels.
const TAB_BAR_HEIGHT: f32 = 28.0;
/// Status bar height in logical pixels.
const STATUS_BAR_HEIGHT: f32 = 36.0;
/// Pane header height in logical pixels.
const PANE_HEADER_HEIGHT: f32 = 36.0;

type IcedElement<'a> = iced_core::Element<'a, UiMessage, iced_core::Theme, iced_wgpu::Renderer>;

impl IcedLayer {
    /// Create a new iced layer sharing the existing GPU resources.
    pub fn new(
        adapter: &wgpu::Adapter,
        device: wgpu::Device,
        queue: wgpu::Queue,
        format: wgpu::TextureFormat,
        physical_width: u32,
        physical_height: u32,
        scale_factor: f32,
    ) -> Self {
        let shell = iced_graphics::Shell::headless();
        let engine = Engine::new(adapter, device, queue, format, None, shell);
        let renderer = iced_wgpu::Renderer::new(
            engine,
            iced_core::Font::default(),
            iced_core::Pixels(16.0),
        );
        let viewport = Viewport::with_physical_size(
            iced_core::Size::new(physical_width, physical_height),
            scale_factor,
        );

        Self {
            renderer,
            viewport,
            cache: Cache::new(),
            events: Vec::new(),
            cursor: iced_core::mouse::Cursor::Unavailable,
            format,
        }
    }

    /// Update the viewport after a window resize.
    pub fn resize(&mut self, physical_width: u32, physical_height: u32, scale_factor: f32) {
        self.viewport = Viewport::with_physical_size(
            iced_core::Size::new(physical_width, physical_height),
            scale_factor,
        );
    }

    /// Push a winit event, converting it to an iced event via iced_winit::conversion.
    pub fn push_event(
        &mut self,
        event: &winit::event::WindowEvent,
        scale_factor: f32,
        modifiers: winit::keyboard::ModifiersState,
    ) {
        if let Some(iced_event) =
            iced_winit::conversion::window_event(event.clone(), scale_factor, modifiers)
        {
            if let iced_core::Event::Mouse(iced_core::mouse::Event::CursorMoved { position }) =
                &iced_event
            {
                self.cursor = iced_core::mouse::Cursor::Available(*position);
            }
            self.events.push(iced_event);
        }
    }

    /// Run the iced UI lifecycle and present onto the given texture view.
    pub fn render(&mut self, view: &wgpu::TextureView, state: &UiState) -> Vec<UiMessage> {
        let bounds = self.viewport.logical_size();
        let scale = self.viewport.scale_factor() as f32;

        let widget = Self::view(state, scale);

        let mut interface = UserInterface::build(
            widget,
            bounds,
            std::mem::take(&mut self.cache),
            &mut self.renderer,
        );

        let mut messages: Vec<UiMessage> = Vec::new();
        let mut clipboard = iced_core::clipboard::Null;
        let events = std::mem::take(&mut self.events);
        let (_state, _statuses) = interface.update(
            &events,
            self.cursor,
            &mut self.renderer,
            &mut clipboard,
            &mut messages,
        );

        interface.draw(
            &mut self.renderer,
            &iced_core::Theme::Dark,
            &iced_core::renderer::Style {
                text_color: iced_core::Color::WHITE,
            },
            self.cursor,
        );

        self.cache = interface.into_cache();

        self.renderer
            .present(None, self.format, view, &self.viewport);

        messages
    }

    /// Build the full UI chrome widget tree from application state.
    fn view<'a>(state: &'a UiState, scale: f32) -> IcedElement<'a> {
        let theme = state.theme;

        let title_bar = Self::title_bar(theme, scale);
        let title_divider = Self::divider(theme, scale);
        let tab_bar = Self::tab_bar(state, scale);

        // Content area: transparent base + pane chrome overlay
        let transparent_base = container(column![])
            .width(iced_core::Length::Fill)
            .height(iced_core::Length::Fill)
            .style(|_: &iced_core::Theme| container::Style {
                background: None,
                ..Default::default()
            });

        let content: IcedElement<'a> = if state.panes.is_empty() {
            transparent_base.into()
        } else {
            let pane_chrome = Self::pane_chrome(state, scale);
            stack![transparent_base, pane_chrome]
                .width(iced_core::Length::Fill)
                .height(iced_core::Length::Fill)
                .into()
        };

        let status_divider = Self::divider(theme, scale);
        let status_bar = Self::status_bar(state, scale);

        let main_ui: IcedElement<'a> = column![title_bar, title_divider, tab_bar, content, status_divider, status_bar]
            .width(iced_core::Length::Fill)
            .height(iced_core::Length::Fill)
            .into();

        let with_flash: IcedElement<'a> = if state.bell_flash {
            let flash_overlay = container(column![])
                .width(iced_core::Length::Fill)
                .height(iced_core::Length::Fill)
                .style(|_: &iced_core::Theme| container::Style {
                    background: Some(iced_core::Background::Color(
                        iced_core::Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                    )),
                    ..Default::default()
                });

            stack![main_ui, flash_overlay]
                .width(iced_core::Length::Fill)
                .height(iced_core::Length::Fill)
                .into()
        } else {
            main_ui
        };

        // Command palette overlay (modal, centered at top)
        if state.palette_active {
            let palette_overlay = Self::command_palette(state, scale);
            let scrim = container(column![])
                .width(iced_core::Length::Fill)
                .height(iced_core::Length::Fill)
                .style(|_: &iced_core::Theme| container::Style {
                    background: Some(iced_core::Background::Color(
                        iced_core::Color::from_rgba(0.0, 0.0, 0.0, 0.4),
                    )),
                    ..Default::default()
                });

            stack![with_flash, scrim, palette_overlay]
                .width(iced_core::Length::Fill)
                .height(iced_core::Length::Fill)
                .into()
        } else {
            with_flash
        }
    }

    /// Title bar: ✻ Claude Terminal ... v0.1.0
    fn title_bar<'a>(theme: &Theme, scale: f32) -> IcedElement<'a> {
        let surface = to_iced_color(&theme.surface);
        let accent = to_iced_color(&theme.accent);
        let text_color = to_iced_color(&theme.text);
        let dim_color = to_iced_color(&theme.text_dim);

        let height = HEADER_BAR_HEIGHT / scale;
        let sparkle_size = 18.0 / scale;
        let title_size = 14.0 / scale;
        let version_size = 11.0 / scale;
        let pad_v = 14.0 / scale;
        let pad_h = 24.0 / scale;
        let spacing = 10.0 / scale;

        let content = row![
            row![
                text("\u{273B}").size(sparkle_size).color(accent),
                text("Claude Terminal").size(title_size).color(text_color),
            ]
            .spacing(spacing)
            .align_y(iced_core::Alignment::Center),
            hspace(),
            text("v0.1.0").size(version_size).color(dim_color),
        ]
        .align_y(iced_core::Alignment::Center)
        .padding(iced_core::Padding::from([pad_v, pad_h]));

        container(content)
            .width(iced_core::Length::Fill)
            .height(height)
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(surface)),
                ..Default::default()
            })
            .into()
    }

    /// 1px divider line between chrome sections.
    fn divider<'a>(theme: &Theme, scale: f32) -> IcedElement<'a> {
        let border_color = to_iced_color(&theme.border);
        container(column![])
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(border_color)),
                ..Default::default()
            })
            .width(iced_core::Length::Fill)
            .height(1.0 / scale)
            .into()
    }

    /// Tab bar: row of tab buttons with close buttons, new-tab button.
    fn tab_bar<'a>(state: &UiState, scale: f32) -> IcedElement<'a> {
        let theme = state.theme;
        let surface = to_iced_color(&theme.surface);
        let surface_raised = to_iced_color(&theme.surface_raised);
        let accent = to_iced_color(&theme.accent);
        let text_color = to_iced_color(&theme.text);
        let text_secondary = to_iced_color(&theme.text_secondary);

        let height = TAB_BAR_HEIGHT / scale;
        let font_size = 12.0 / scale;
        let max_tab_w = 200.0 / scale;
        let min_tab_w = 60.0 / scale;
        let new_tab_w = 28.0 / scale;

        let tab_count = state.tabs.len();
        let available = (state.window_width / scale - new_tab_w).max(0.0);
        let raw_w = if tab_count > 0 {
            available / tab_count as f32
        } else {
            0.0
        };
        let tw = raw_w.clamp(min_tab_w, max_tab_w);

        let mut tab_row = Row::new()
            .height(height)
            .align_y(iced_core::Alignment::Center);

        for (i, tab) in state.tabs.iter().enumerate() {
            let is_active = i == state.active_tab_index;
            let is_hovered = state.hovered_tab == Some(i);

            let fg = if is_active { text_color } else { text_secondary };
            let bg = if is_active { surface_raised } else { surface };

            // Tab title (truncated)
            let max_chars = ((tw / (font_size * 0.6)) as usize).max(3);
            let title_chars: Vec<char> = tab.title.chars().collect();
            let show_close = is_active || is_hovered;
            let usable_chars = if show_close {
                max_chars.saturating_sub(2)
            } else {
                max_chars
            };
            let display_title = if title_chars.len() > usable_chars && usable_chars > 1 {
                let mut t: String = title_chars[..usable_chars - 1].iter().collect();
                t.push('\u{2026}');
                t
            } else {
                title_chars[..title_chars.len().min(usable_chars)]
                    .iter()
                    .collect()
            };

            let mut tab_content: Row<'a, UiMessage, iced_core::Theme, iced_wgpu::Renderer> =
                Row::new()
                    .align_y(iced_core::Alignment::Center)
                    .width(tw)
                    .height(height);

            tab_content = tab_content.push(
                container(text(display_title).size(font_size).color(fg))
                    .width(iced_core::Length::Fill)
                    .center_x(iced_core::Length::Fill)
                    .center_y(iced_core::Length::Fill),
            );

            if show_close {
                let close_fg = iced_core::Color { a: 0.7, ..fg };
                let close_btn = button(text("\u{00D7}").size(font_size).color(close_fg))
                    .on_press(UiMessage::TabClosed(i))
                    .padding(0)
                    .style(move |_: &iced_core::Theme, _status| button::Style {
                        background: None,
                        text_color: close_fg,
                        border: iced_core::Border::default(),
                        shadow: iced_core::Shadow::default(),
                        snap: false,
                    });
                tab_content = tab_content.push(close_btn);
            }

            let tab_bg = bg;
            let is_active_tab = is_active;
            let accent_color = accent;
            let tab_container = container(tab_content)
                .width(tw)
                .height(height)
                .style(move |_: &iced_core::Theme| container::Style {
                    background: Some(iced_core::Background::Color(tab_bg)),
                    border: if is_active_tab {
                        iced_core::Border {
                            color: accent_color,
                            width: 0.0,
                            radius: 0.0.into(),
                        }
                    } else {
                        iced_core::Border::default()
                    },
                    ..Default::default()
                });

            let tab_widget = MouseArea::new(tab_container)
                .on_press(UiMessage::TabSelected(i))
                .on_enter(UiMessage::TabHovered(Some(i)))
                .on_exit(UiMessage::TabHovered(None));

            tab_row = tab_row.push(tab_widget);

            // Separator between tabs
            if i + 1 < tab_count {
                let sep_color = iced_core::Color {
                    a: 0.5,
                    ..to_iced_color(&theme.border)
                };
                let separator = container(column![])
                    .width(1.0 / scale)
                    .height(height - 4.0 / scale)
                    .style(move |_: &iced_core::Theme| container::Style {
                        background: Some(iced_core::Background::Color(sep_color)),
                        ..Default::default()
                    });
                tab_row = tab_row.push(separator);
            }
        }

        // Active tab accent stripe
        let mut accent_row: Row<'a, UiMessage, iced_core::Theme, iced_wgpu::Renderer> =
            Row::new()
                .width(iced_core::Length::Fill)
                .height(2.0 / scale);
        for (i, _) in state.tabs.iter().enumerate() {
            let stripe_color = if i == state.active_tab_index {
                accent
            } else {
                surface
            };
            accent_row = accent_row.push(
                container(column![])
                    .width(tw)
                    .height(2.0 / scale)
                    .style(move |_: &iced_core::Theme| container::Style {
                        background: Some(iced_core::Background::Color(stripe_color)),
                        ..Default::default()
                    }),
            );
        }

        // New tab "+" button
        let plus_btn = button(
            container(text("+").size(font_size).color(text_secondary))
                .center_x(new_tab_w)
                .center_y(height),
        )
        .on_press(UiMessage::NewTab)
        .width(new_tab_w)
        .height(height)
        .padding(0)
        .style(move |_: &iced_core::Theme, status| {
            let bg_color = match status {
                button::Status::Hovered => surface_raised,
                _ => surface,
            };
            button::Style {
                background: Some(iced_core::Background::Color(bg_color)),
                text_color: text_secondary,
                border: iced_core::Border::default(),
                shadow: iced_core::Shadow::default(),
                snap: false,
            }
        });

        tab_row = tab_row.push(plus_btn);

        let tab_bar_column = column![tab_row, accent_row];

        container(tab_bar_column)
            .width(iced_core::Length::Fill)
            .height(height)
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(surface)),
                ..Default::default()
            })
            .into()
    }

    /// Pane chrome: rounded containers with headers, accent stripes, borders, shadows,
    /// plus dividers, scrollbar thumbs, and search bar overlay.
    fn pane_chrome<'a>(state: &'a UiState, scale: f32) -> IcedElement<'a> {
        let theme = state.theme;
        let surface = to_iced_color(&theme.surface);
        let surface_raised = to_iced_color(&theme.surface_raised);
        let accent = to_iced_color(&theme.accent);
        let text_color = to_iced_color(&theme.text);
        let text_secondary = to_iced_color(&theme.text_secondary);
        let text_dim = to_iced_color(&theme.text_dim);
        let success = to_iced_color(&theme.success);
        let border_color = to_iced_color(&theme.border);
        let border_subtle = to_iced_color(&theme.border_subtle);

        let badge_size = 13.0 / scale;
        let title_size = 12.0 / scale;
        let shell_size = 10.0 / scale;
        let dot_size = 6.0 / scale;
        let header_pad_v = 10.0 / scale;
        let header_pad_h = 16.0 / scale;
        let spacing = 8.0 / scale;
        let radius = 8.0 / scale;

        let mut chrome_stack = Stack::new()
            .width(iced_core::Length::Fill)
            .height(iced_core::Length::Fill);

        for pane in &state.panes {
            let px = pane.x / scale;
            let py = pane.y / scale;
            let pw = pane.width / scale;
            let ph = pane.height / scale;

            let is_active = pane.is_focused;
            let badge_color = if is_active { accent } else { text_dim };
            let title_color = if is_active { text_color } else { text_secondary };
            let header_bg = if is_active { surface_raised } else { surface };
            let stripe_color = if is_active { accent } else { border_subtle };
            let stripe_h = if is_active { 4.0 / scale } else { 1.0 / scale };
            let pane_border = if is_active { accent } else { border_color };

            let badge = pane_badge(pane.index);

            // Header: badge + title + green status dot ... shell label
            let header_content = row![
                row![
                    text(badge).size(badge_size).color(badge_color),
                    text(&pane.title).size(title_size).color(title_color),
                    text("\u{25CF}").size(dot_size).color(success),
                ]
                .spacing(spacing)
                .align_y(iced_core::Alignment::Center),
                hspace(),
                text(&pane.shell_name).size(shell_size).color(text_dim),
            ]
            .align_y(iced_core::Alignment::Center)
            .padding(iced_core::Padding::from([header_pad_v, header_pad_h]));

            // Header container with rounded top corners
            let hdr_bg = header_bg;
            let r = radius;
            let header = container(header_content)
                .width(iced_core::Length::Fill)
                .style(move |_: &iced_core::Theme| container::Style {
                    background: Some(iced_core::Background::Color(hdr_bg)),
                    border: iced_core::Border {
                        color: iced_core::Color::TRANSPARENT,
                        width: 0.0,
                        radius: iced_core::border::Radius::from(0.0).top(r),
                    },
                    ..Default::default()
                });

            // Accent stripe (2px active, 1px inactive)
            let sc = stripe_color;
            let accent_stripe = container(column![])
                .width(iced_core::Length::Fill)
                .height(stripe_h)
                .style(move |_: &iced_core::Theme| container::Style {
                    background: Some(iced_core::Background::Color(sc)),
                    ..Default::default()
                });

            // Transparent body (terminal grid shows through from custom pipeline)
            let body = container(column![])
                .width(iced_core::Length::Fill)
                .height(iced_core::Length::Fill)
                .style(|_: &iced_core::Theme| container::Style {
                    background: None,
                    ..Default::default()
                });

            let pane_column = column![header, accent_stripe, body]
                .width(iced_core::Length::Fill)
                .spacing(0);

            // Outer container: rounded border + shadow, transparent background
            let is_active_pane = is_active;
            let pane_container = container(pane_column)
                .width(pw)
                .height(ph)
                .style(move |_: &iced_core::Theme| container::Style {
                    background: None,
                    border: iced_core::Border {
                        color: pane_border,
                        width: 1.0,
                        radius: radius.into(),
                    },
                    shadow: iced_core::Shadow {
                        color: iced_core::Color::from_rgba(
                            0.0,
                            0.0,
                            0.0,
                            if is_active_pane { 0.3 } else { 0.15 },
                        ),
                        offset: iced_core::Vector::new(0.0, 2.0 / scale),
                        blur_radius: if is_active_pane {
                            12.0 / scale
                        } else {
                            6.0 / scale
                        },
                    },
                    ..Default::default()
                });

            chrome_stack = chrome_stack.push(pin(pane_container).x(px).y(py));

            // Scrollbar thumb (overlay on right edge of pane)
            if let Some((sx, sy, sw, sh)) = pane.scrollbar_thumb {
                if pane.scrollbar_alpha > 0.0 {
                    let alpha = pane.scrollbar_alpha;
                    let scrollbar = container(column![])
                        .width(sw / scale)
                        .height(sh / scale)
                        .style(move |_: &iced_core::Theme| container::Style {
                            background: Some(iced_core::Background::Color(
                                iced_core::Color::from_rgba(1.0, 1.0, 1.0, alpha * 0.5),
                            )),
                            border: iced_core::Border {
                                color: iced_core::Color::TRANSPARENT,
                                width: 0.0,
                                radius: (2.0 / scale).into(),
                            },
                            ..Default::default()
                        });
                    chrome_stack =
                        chrome_stack.push(pin(scrollbar).x(sx / scale).y(sy / scale));
                }
            }
        }

        // Dividers between panes
        for div in &state.dividers {
            let div_color = if div.is_hovered { accent } else { border_color };
            let divider_widget = container(column![])
                .width(div.width / scale)
                .height(div.height / scale)
                .style(move |_: &iced_core::Theme| container::Style {
                    background: Some(iced_core::Background::Color(div_color)),
                    ..Default::default()
                });
            chrome_stack =
                chrome_stack.push(pin(divider_widget).x(div.x / scale).y(div.y / scale));
        }

        // Search bar overlay (positioned at top-right of focused pane)
        if state.search_active {
            if let Some(focused_pane) = state.panes.iter().find(|p| p.is_focused) {
                let search_bar =
                    Self::search_bar(state, focused_pane, scale);
                chrome_stack = chrome_stack.push(search_bar);
            }
        }

        chrome_stack.into()
    }

    /// Search bar: floating overlay at top-right of the focused pane.
    fn search_bar<'a>(
        state: &'a UiState,
        pane: &PaneInfo,
        scale: f32,
    ) -> IcedElement<'a> {
        let theme = state.theme;
        let surface = to_iced_color(&theme.surface);
        let text_color = to_iced_color(&theme.text);
        let text_dim = to_iced_color(&theme.text_dim);
        let accent = to_iced_color(&theme.accent);
        let error_color = to_iced_color(&theme.error);
        let border_color = to_iced_color(&theme.border);

        let font_size = 12.0 / scale;
        let pad_v = 6.0 / scale;
        let pad_h = 12.0 / scale;
        let bar_width = 260.0 / scale;
        let bar_height = 30.0 / scale;
        let spacing = 8.0 / scale;
        let radius = 6.0 / scale;

        // Query text (truncated if needed)
        let query_display = if state.search_query.is_empty() {
            "Find...".to_string()
        } else {
            let max_chars = 20;
            if state.search_query.len() > max_chars {
                format!("{}...", &state.search_query[..max_chars])
            } else {
                state.search_query.clone()
            }
        };

        let query_color = if state.search_error {
            error_color
        } else if state.search_query.is_empty() {
            text_dim
        } else {
            text_color
        };

        // Match count
        let match_text = if state.search_total > 0 {
            format!("{}/{}", state.search_current, state.search_total)
        } else if !state.search_query.is_empty() {
            "0/0".to_string()
        } else {
            String::new()
        };

        let bar_content = row![
            text(query_display).size(font_size).color(query_color),
            hspace(),
            text(match_text).size(font_size).color(text_dim),
        ]
        .spacing(spacing)
        .align_y(iced_core::Alignment::Center)
        .padding(iced_core::Padding::from([pad_v, pad_h]));

        let bar_bg = surface;
        let bar_border = if state.search_error { error_color } else { border_color };
        let bar_accent = accent;
        let has_query = !state.search_query.is_empty();
        let bar = container(bar_content)
            .width(bar_width)
            .height(bar_height)
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(bar_bg)),
                border: iced_core::Border {
                    color: if has_query { bar_accent } else { bar_border },
                    width: 1.0,
                    radius: radius.into(),
                },
                shadow: iced_core::Shadow {
                    color: iced_core::Color::from_rgba(0.0, 0.0, 0.0, 0.3),
                    offset: iced_core::Vector::new(0.0, 2.0 / scale),
                    blur_radius: 8.0 / scale,
                },
                ..Default::default()
            });

        // Position at top-right of focused pane (inside the pane, offset from header)
        let bar_x = (pane.x + pane.width - 260.0 - 8.0) / scale;
        let bar_y = (pane.y + PANE_HEADER_HEIGHT + 4.0) / scale;

        pin(bar).x(bar_x).y(bar_y).into()
    }

    /// Command palette: floating modal centered near top of window.
    fn command_palette<'a>(state: &'a UiState, scale: f32) -> IcedElement<'a> {
        let theme = state.theme;
        let surface = to_iced_color(&theme.surface);
        let surface_raised = to_iced_color(&theme.surface_raised);
        let text_color = to_iced_color(&theme.text);
        let text_dim = to_iced_color(&theme.text_dim);
        let text_secondary = to_iced_color(&theme.text_secondary);
        let accent = to_iced_color(&theme.accent);
        let border_color = to_iced_color(&theme.border);

        let font_size = 13.0 / scale;
        let small_size = 11.0 / scale;
        let pad_h = 14.0 / scale;
        let pad_v = 8.0 / scale;
        let palette_width = 440.0 / scale;
        let input_height = 36.0 / scale;
        let item_height = 32.0 / scale;
        let radius = 8.0 / scale;
        let spacing = 2.0 / scale;
        let max_visible = 10;

        // Input field display
        let query_display = if state.palette_query.is_empty() {
            "Type a command...".to_string()
        } else {
            state.palette_query.clone()
        };
        let query_color = if state.palette_query.is_empty() {
            text_dim
        } else {
            text_color
        };

        let input_bar = container(
            row![
                text("\u{1F50D}").size(font_size).color(text_dim),
                text(query_display).size(font_size).color(query_color),
            ]
            .spacing(8.0 / scale)
            .align_y(iced_core::Alignment::Center)
            .padding(iced_core::Padding::from([pad_v, pad_h])),
        )
        .width(palette_width)
        .height(input_height)
        .style(move |_: &iced_core::Theme| container::Style {
            background: Some(iced_core::Background::Color(surface)),
            border: iced_core::Border {
                color: border_color,
                width: 0.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        });

        // Result list
        let visible_count = state.palette_items.len().min(max_visible);
        let mut result_col = iced_widget::Column::new().spacing(spacing);

        for (i, (name, _desc, keybinding)) in state.palette_items.iter().enumerate().take(visible_count) {
            let is_selected = i == state.palette_selected;
            let item_bg = if is_selected { surface_raised } else { surface };
            let item_fg = if is_selected { text_color } else { text_secondary };
            let kb_color = text_dim;

            let item_content = row![
                text(name.as_str()).size(font_size).color(item_fg),
                hspace(),
                text(keybinding.as_str()).size(small_size).color(kb_color),
            ]
            .align_y(iced_core::Alignment::Center)
            .padding(iced_core::Padding::from([pad_v, pad_h]));

            let item_accent = accent;
            let item = container(item_content)
                .width(palette_width)
                .height(item_height)
                .style(move |_: &iced_core::Theme| container::Style {
                    background: Some(iced_core::Background::Color(item_bg)),
                    border: if is_selected {
                        iced_core::Border {
                            color: item_accent,
                            width: 0.0,
                            radius: 0.0.into(),
                        }
                    } else {
                        iced_core::Border::default()
                    },
                    ..Default::default()
                });

            result_col = result_col.push(item);
        }

        // Item count footer
        let count_text = format!("{} commands", state.palette_items.len());
        let footer = container(
            text(count_text).size(small_size).color(text_dim),
        )
        .width(palette_width)
        .padding(iced_core::Padding::from([4.0 / scale, pad_h]))
        .style(move |_: &iced_core::Theme| container::Style {
            background: Some(iced_core::Background::Color(surface)),
            ..Default::default()
        });

        // Separator
        let sep = container(column![])
            .width(palette_width)
            .height(1.0 / scale)
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(border_color)),
                ..Default::default()
            });

        let palette_box = container(
            column![input_bar, sep, result_col, footer],
        )
        .width(palette_width)
        .style(move |_: &iced_core::Theme| container::Style {
            background: Some(iced_core::Background::Color(surface)),
            border: iced_core::Border {
                color: border_color,
                width: 1.0,
                radius: radius.into(),
            },
            shadow: iced_core::Shadow {
                color: iced_core::Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                offset: iced_core::Vector::new(0.0, 4.0 / scale),
                blur_radius: 16.0 / scale,
            },
            ..Default::default()
        });

        // Center horizontally, position near top
        let palette_x = ((state.window_width / scale) - palette_width) / 2.0;
        let palette_y = 80.0 / scale;

        pin(palette_box).x(palette_x).y(palette_y).into()
    }

    /// Status bar: ✻ Claude Terminal | ● Pane N | user · UTF-8 · bash
    fn status_bar<'a>(state: &UiState, scale: f32) -> IcedElement<'a> {
        let theme = state.theme;
        let surface = to_iced_color(&theme.surface);
        let accent = to_iced_color(&theme.accent);
        let dim_color = to_iced_color(&theme.text_dim);
        let secondary = to_iced_color(&theme.text_secondary);
        let success = to_iced_color(&theme.success);
        let border_sep = to_iced_color(&theme.border);

        let height = STATUS_BAR_HEIGHT / scale;
        let brand_size = 11.0 / scale;
        let dot_size = 6.0 / scale;
        let info_size = 11.0 / scale;
        let pad_v = 10.0 / scale;
        let pad_h = 24.0 / scale;
        let small_spacing = 6.0 / scale;
        let info_spacing = 8.0 / scale;

        let user = std::env::var("USER").unwrap_or_else(|_| "user".into());

        // Left: brand
        let left = row![
            text("\u{273B}").size(brand_size).color(accent),
            text("Claude Terminal").size(brand_size).color(dim_color),
        ]
        .spacing(small_spacing)
        .align_y(iced_core::Alignment::Center);

        // Center: active pane
        let center = row![
            text("\u{25CF}").size(dot_size).color(success),
            text(format!("Pane {}", state.active_pane_index + 1))
                .size(info_size)
                .color(secondary),
        ]
        .spacing(small_spacing)
        .align_y(iced_core::Alignment::Center);

        // Right: session info with · separators
        let right = row![
            text(user).size(info_size).color(dim_color),
            text("\u{00B7}").size(info_size).color(border_sep),
            text("UTF-8").size(info_size).color(dim_color),
            text("\u{00B7}").size(info_size).color(border_sep),
            text("bash").size(info_size).color(dim_color),
        ]
        .spacing(info_spacing)
        .align_y(iced_core::Alignment::Center);

        let content = row![left, hspace(), center, hspace(), right,]
            .align_y(iced_core::Alignment::Center)
            .padding(iced_core::Padding::from([pad_v, pad_h]));

        container(content)
            .width(iced_core::Length::Fill)
            .height(height)
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(surface)),
                ..Default::default()
            })
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::theme::Theme;

    fn try_create_headless_gpu() -> Option<(wgpu::Adapter, wgpu::Device, wgpu::Queue)> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        }))
        .ok()?;
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Test Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
            },
        ))
        .ok()?;
        Some((adapter, device, queue))
    }

    fn test_ui_state(theme: &Theme) -> UiState {
        UiState {
            tabs: vec![TabInfo {
                title: "Tab 1".to_string(),
                is_active: true,
                has_notification: false,
            }],
            active_tab_index: 0,
            hovered_tab: None,
            active_pane_index: 0,
            panes: vec![PaneInfo {
                x: 12.0,
                y: 12.0,
                width: 1256.0,
                height: 600.0,
                is_focused: true,
                index: 0,
                title: "Shell".to_string(),
                shell_name: "bash".to_string(),
                scrollbar_thumb: None,
                scrollbar_alpha: 0.0,
            }],
            pane_count: 1,
            is_zoomed: false,
            theme,
            window_width: 1280.0,
            window_height: 720.0,
            scale_factor: 2.0,
            search_active: false,
            search_query: String::new(),
            search_current: 0,
            search_total: 0,
            search_error: false,
            dividers: Vec::new(),
            bell_flash: false,
            palette_active: false,
            palette_query: String::new(),
            palette_items: Vec::new(),
            palette_selected: 0,
        }
    }

    #[test]
    fn iced_layer_creates_successfully() {
        let (adapter, device, queue) = match try_create_headless_gpu() {
            Some(ctx) => ctx,
            None => return,
        };
        let _layer = IcedLayer::new(
            &adapter,
            device,
            queue,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            1280,
            720,
            2.0,
        );
    }

    #[test]
    fn iced_layer_resize_updates_viewport() {
        let (adapter, device, queue) = match try_create_headless_gpu() {
            Some(ctx) => ctx,
            None => return,
        };
        let mut layer = IcedLayer::new(
            &adapter,
            device,
            queue,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            1280,
            720,
            2.0,
        );

        layer.resize(1920, 1080, 2.0);
        let size = layer.viewport.physical_size();
        assert_eq!(size.width, 1920);
        assert_eq!(size.height, 1080);
    }

    #[test]
    fn iced_layer_viewport_logical_size_accounts_for_scale() {
        let (adapter, device, queue) = match try_create_headless_gpu() {
            Some(ctx) => ctx,
            None => return,
        };
        let layer = IcedLayer::new(
            &adapter,
            device,
            queue,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            2560,
            1440,
            2.0,
        );

        let logical = layer.viewport.logical_size();
        assert!((logical.width - 1280.0).abs() < 1.0);
        assert!((logical.height - 720.0).abs() < 1.0);
    }

    #[test]
    fn iced_layer_events_accumulate() {
        let (adapter, device, queue) = match try_create_headless_gpu() {
            Some(ctx) => ctx,
            None => return,
        };
        let mut layer = IcedLayer::new(
            &adapter,
            device,
            queue,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            1280,
            720,
            1.0,
        );

        assert_eq!(layer.events.len(), 0);

        let event =
            winit::event::WindowEvent::Resized(winit::dpi::PhysicalSize::new(1920, 1080));
        layer.push_event(&event, 1.0, winit::keyboard::ModifiersState::empty());

        assert!(
            layer.events.len() >= 1,
            "Should have at least 1 iced event after resize"
        );
    }

    #[test]
    fn iced_layer_renders_with_ui_state() {
        let (adapter, device, queue) = match try_create_headless_gpu() {
            Some(ctx) => ctx,
            None => return,
        };
        let mut layer = IcedLayer::new(
            &adapter,
            device.clone(),
            queue.clone(),
            wgpu::TextureFormat::Bgra8UnormSrgb,
            1280,
            720,
            1.0,
        );

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Test Target"),
            size: wgpu::Extent3d {
                width: 1280,
                height: 720,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let theme = Theme::claude_dark();
        let state = test_ui_state(&theme);
        let messages = layer.render(&view, &state);
        assert!(messages.is_empty(), "No interactions, no messages expected");
    }

    #[test]
    fn ui_message_variants_exist() {
        let _tab = UiMessage::TabSelected(0);
        let _close = UiMessage::TabClosed(0);
        let _new = UiMessage::NewTab;
        let _noop = UiMessage::Noop;
    }

    #[test]
    fn tab_info_constructor() {
        let tab = TabInfo {
            title: "test".to_string(),
            is_active: true,
            has_notification: false,
        };
        assert!(tab.is_active);
        assert_eq!(tab.title, "test");
    }

    #[test]
    fn to_iced_color_converts_correctly() {
        let theme_color = crate::config::theme::Color::new(0.5, 0.25, 0.75, 1.0);
        let iced_color = to_iced_color(&theme_color);
        assert!((iced_color.r - 0.5).abs() < 0.01);
        assert!((iced_color.g - 0.25).abs() < 0.01);
        assert!((iced_color.b - 0.75).abs() < 0.01);
        assert!((iced_color.a - 1.0).abs() < 0.01);
    }

    #[test]
    fn chrome_heights_match_legacy_constants() {
        assert_eq!(HEADER_BAR_HEIGHT, crate::header_bar::HEADER_BAR_HEIGHT);
        assert_eq!(TAB_BAR_HEIGHT, crate::tab::bar::TAB_BAR_HEIGHT);
        assert_eq!(STATUS_BAR_HEIGHT, crate::status_bar::STATUS_BAR_HEIGHT);
    }

    #[test]
    fn tab_bar_renders_with_multiple_tabs() {
        let (adapter, device, queue) = match try_create_headless_gpu() {
            Some(ctx) => ctx,
            None => return,
        };
        let mut layer = IcedLayer::new(
            &adapter,
            device.clone(),
            queue.clone(),
            wgpu::TextureFormat::Bgra8UnormSrgb,
            1280,
            720,
            1.0,
        );

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Test Target"),
            size: wgpu::Extent3d {
                width: 1280,
                height: 720,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let theme = Theme::claude_dark();
        let state = UiState {
            tabs: vec![
                TabInfo {
                    title: "Tab 1".to_string(),
                    is_active: true,
                    has_notification: false,
                },
                TabInfo {
                    title: "Tab 2".to_string(),
                    is_active: false,
                    has_notification: false,
                },
                TabInfo {
                    title: "Tab 3".to_string(),
                    is_active: false,
                    has_notification: true,
                },
            ],
            active_tab_index: 0,
            hovered_tab: Some(1),
            active_pane_index: 0,
            panes: vec![PaneInfo {
                x: 12.0,
                y: 12.0,
                width: 1256.0,
                height: 600.0,
                is_focused: true,
                index: 0,
                title: "Shell".to_string(),
                shell_name: "bash".to_string(),
                scrollbar_thumb: None,
                scrollbar_alpha: 0.0,
            }],
            pane_count: 1,
            is_zoomed: false,
            theme: &theme,
            window_width: 1280.0,
            window_height: 720.0,
            scale_factor: 1.0,
            search_active: false,
            search_query: String::new(),
            search_current: 0,
            search_total: 0,
            search_error: false,
            dividers: Vec::new(),
            bell_flash: false,
            palette_active: false,
            palette_query: String::new(),
            palette_items: Vec::new(),
            palette_selected: 0,
        };
        let messages = layer.render(&view, &state);
        assert!(messages.is_empty(), "No interactions, no messages expected");
    }

    #[test]
    fn pane_chrome_renders_with_multiple_panes() {
        let (adapter, device, queue) = match try_create_headless_gpu() {
            Some(ctx) => ctx,
            None => return,
        };
        let mut layer = IcedLayer::new(
            &adapter,
            device.clone(),
            queue.clone(),
            wgpu::TextureFormat::Bgra8UnormSrgb,
            1280,
            720,
            1.0,
        );

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Test Target"),
            size: wgpu::Extent3d {
                width: 1280,
                height: 720,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let theme = Theme::claude_dark();
        let state = UiState {
            tabs: vec![TabInfo {
                title: "Tab 1".to_string(),
                is_active: true,
                has_notification: false,
            }],
            active_tab_index: 0,
            hovered_tab: None,
            active_pane_index: 0,
            panes: vec![
                PaneInfo {
                    x: 12.0,
                    y: 12.0,
                    width: 620.0,
                    height: 600.0,
                    is_focused: true,
                    index: 0,
                    title: "src".to_string(),
                    shell_name: "bash".to_string(),
                    scrollbar_thumb: None,
                    scrollbar_alpha: 0.0,
                },
                PaneInfo {
                    x: 636.0,
                    y: 12.0,
                    width: 620.0,
                    height: 600.0,
                    is_focused: false,
                    index: 1,
                    title: "tests".to_string(),
                    shell_name: "bash".to_string(),
                    scrollbar_thumb: None,
                    scrollbar_alpha: 0.0,
                },
            ],
            pane_count: 2,
            is_zoomed: false,
            theme: &theme,
            window_width: 1280.0,
            window_height: 720.0,
            scale_factor: 1.0,
            search_active: false,
            search_query: String::new(),
            search_current: 0,
            search_total: 0,
            search_error: false,
            dividers: Vec::new(),
            bell_flash: false,
            palette_active: false,
            palette_query: String::new(),
            palette_items: Vec::new(),
            palette_selected: 0,
        };
        let messages = layer.render(&view, &state);
        assert!(messages.is_empty(), "No interactions, no messages expected");
    }

    #[test]
    fn pane_info_constructor() {
        let pane = PaneInfo {
            x: 10.0,
            y: 20.0,
            width: 500.0,
            height: 400.0,
            is_focused: true,
            index: 0,
            title: "home".to_string(),
            shell_name: "zsh".to_string(),
            scrollbar_thumb: None,
            scrollbar_alpha: 0.0,
        };
        assert!(pane.is_focused);
        assert_eq!(pane.title, "home");
        assert_eq!(pane.index, 0);
    }

    #[test]
    fn pane_header_height_matches_legacy() {
        assert_eq!(PANE_HEADER_HEIGHT, crate::pane::header::PANE_HEADER_HEIGHT);
    }

    #[test]
    fn ui_message_tab_hovered_variant() {
        let hover = UiMessage::TabHovered(Some(2));
        match hover {
            UiMessage::TabHovered(Some(idx)) => assert_eq!(idx, 2),
            _ => panic!("Expected TabHovered(Some(2))"),
        }
        let unhover = UiMessage::TabHovered(None);
        match unhover {
            UiMessage::TabHovered(None) => {}
            _ => panic!("Expected TabHovered(None)"),
        }
    }

    #[test]
    fn pane_badge_returns_circled_digits() {
        assert_eq!(pane_badge(0), "\u{2460}");
        assert_eq!(pane_badge(1), "\u{2461}");
        assert_eq!(pane_badge(2), "\u{2462}");
        assert_eq!(pane_badge(8), "\u{2468}");
        assert_eq!(pane_badge(9), "\u{2460}"); // wraps
    }
}

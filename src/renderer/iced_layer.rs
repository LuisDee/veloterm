// iced_wgpu integration layer — shares the existing wgpu device/queue
// and composites iced widget output onto the same TextureView.

use crate::config::theme::Theme;
use iced_graphics::Viewport;
use iced_runtime::user_interface::{Cache, UserInterface};
use iced_wgpu::Engine;
use iced_widget::{column, container, row, text, Row};

/// Messages produced by iced UI widgets, returned to the application for processing.
#[derive(Debug, Clone)]
pub enum UiMessage {
    TabSelected(usize),
    TabClosed(usize),
    NewTab,
    Noop,
}

/// Tab descriptor for the iced widget tree.
#[derive(Debug, Clone)]
pub struct TabInfo {
    pub title: String,
    pub is_active: bool,
    pub has_notification: bool,
}

/// State snapshot passed to the iced widget tree each frame.
pub struct UiState<'a> {
    pub tabs: Vec<TabInfo>,
    pub active_tab_index: usize,
    pub hovered_tab: Option<usize>,
    pub active_pane_index: usize,
    pub theme: &'a Theme,
    pub window_width: f32,
    pub window_height: f32,
    pub scale_factor: f32,
}

/// Holds iced rendering state: renderer, viewport, event queue, and UI cache.
/// Created once during Renderer::new(), updated on resize and input events.
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

/// Header bar height in logical pixels.
const HEADER_BAR_HEIGHT: f32 = 46.0;
/// Tab bar height in logical pixels.
const TAB_BAR_HEIGHT: f32 = 28.0;
/// Status bar height in logical pixels.
const STATUS_BAR_HEIGHT: f32 = 36.0;

type IcedElement<'a> = iced_core::Element<'a, UiMessage, iced_core::Theme, iced_wgpu::Renderer>;

impl IcedLayer {
    /// Create a new iced layer sharing the existing GPU resources.
    /// `device` and `queue` are cloned (wgpu 27: internally Arc'd).
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
            // Track cursor position from mouse events
            if let iced_core::Event::Mouse(iced_core::mouse::Event::CursorMoved { position }) =
                &iced_event
            {
                self.cursor = iced_core::mouse::Cursor::Available(*position);
            }
            self.events.push(iced_event);
        }
    }

    /// Run the iced UI lifecycle and present onto the given texture view.
    /// Returns any messages produced by widget interactions.
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
    fn view<'a>(state: &UiState, scale: f32) -> IcedElement<'a> {
        let theme = state.theme;

        // Header bar
        let header = Self::header_bar(theme, scale);

        // Tab bar
        let tab_bar = Self::tab_bar(state, scale);

        // Content area (transparent — grid renders underneath via custom pipeline)
        let content = container(text(""))
            .width(iced_core::Length::Fill)
            .height(iced_core::Length::Fill)
            .style(|_: &iced_core::Theme| container::Style {
                background: None,
                ..Default::default()
            });

        // Status bar
        let status = Self::status_bar(state, scale);

        column![header, tab_bar, content, status]
            .width(iced_core::Length::Fill)
            .height(iced_core::Length::Fill)
            .into()
    }

    /// Header bar widget: brand text left, version right.
    fn header_bar<'a>(theme: &Theme, scale: f32) -> IcedElement<'a> {
        let surface = to_iced_color(&theme.surface);
        let accent = to_iced_color(&theme.accent);
        let text_color = to_iced_color(&theme.text);
        let dim_color = to_iced_color(&theme.text_dim);
        let border_color = to_iced_color(&theme.border);

        let height = HEADER_BAR_HEIGHT / scale;
        let font_size = 13.0 / scale;
        let pad_h = 12.0 / scale;

        let sparkle = text("*")
            .size(font_size)
            .color(accent);

        let brand = text(" Claude Terminal")
            .size(font_size)
            .color(text_color);

        let version = text("v0.1.0")
            .size(font_size)
            .color(dim_color);

        let left = row![sparkle, brand]
            .align_y(iced_core::Alignment::Center);

        let right = container(version)
            .align_right(iced_core::Length::Fill);

        let inner: Row<'a, UiMessage, iced_core::Theme, iced_wgpu::Renderer> = row![
            left,
            right,
        ]
        .padding(iced_core::Padding::from([0.0, pad_h]))
        .align_y(iced_core::Alignment::Center)
        .width(iced_core::Length::Fill);

        container(inner)
            .width(iced_core::Length::Fill)
            .height(height)
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(surface)),
                border: iced_core::Border {
                    color: border_color,
                    width: 0.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    /// Tab bar widget: transparent placeholder until Phase 2 migrates tabs.
    /// The legacy overlay pipeline renders the actual tab bar underneath.
    fn tab_bar<'a>(_state: &UiState, scale: f32) -> IcedElement<'a> {
        let height = TAB_BAR_HEIGHT / scale;

        container(text(""))
            .width(iced_core::Length::Fill)
            .height(height)
            .style(|_: &iced_core::Theme| container::Style {
                background: None,
                ..Default::default()
            })
            .into()
    }

    /// Status bar widget: brand info left, pane indicator center, session info right.
    fn status_bar<'a>(state: &UiState, scale: f32) -> IcedElement<'a> {
        let theme = state.theme;
        let surface = to_iced_color(&theme.surface);
        let accent = to_iced_color(&theme.accent);
        let dim_color = to_iced_color(&theme.text_dim);
        let secondary = to_iced_color(&theme.text_secondary);
        let success = to_iced_color(&theme.success);
        let border_color = to_iced_color(&theme.border);

        let height = STATUS_BAR_HEIGHT / scale;
        let font_size = 12.0 / scale;
        let pad_h = 12.0 / scale;

        // Left: sparkle + brand
        let left = row![
            text("*").size(font_size).color(accent),
            text(" Claude Terminal").size(font_size).color(dim_color),
        ]
        .align_y(iced_core::Alignment::Center);

        // Center: pane indicator
        let center = row![
            text("\u{25CF} ").size(font_size).color(success),
            text(format!("Pane {}", state.active_pane_index + 1)).size(font_size).color(secondary),
        ]
        .align_y(iced_core::Alignment::Center);

        // Right: session info
        let user = std::env::var("USER").unwrap_or_else(|_| "user".into());
        let right_text = format!("{user} \u{00B7} UTF-8 \u{00B7} bash");
        let right = text(right_text)
            .size(font_size)
            .color(dim_color);

        let inner: Row<'a, UiMessage, iced_core::Theme, iced_wgpu::Renderer> = row![
            container(left).width(iced_core::Length::FillPortion(1)),
            container(center)
                .width(iced_core::Length::FillPortion(1))
                .center_x(iced_core::Length::Fill),
            container(right)
                .width(iced_core::Length::FillPortion(1))
                .align_right(iced_core::Length::Fill),
        ]
        .padding(iced_core::Padding::from([0.0, pad_h]))
        .align_y(iced_core::Alignment::Center)
        .width(iced_core::Length::Fill);

        container(inner)
            .width(iced_core::Length::Fill)
            .height(height)
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(surface)),
                border: iced_core::Border {
                    color: border_color,
                    width: 0.0,
                    radius: 0.0.into(),
                },
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
            tabs: vec![
                TabInfo {
                    title: "Tab 1".to_string(),
                    is_active: true,
                    has_notification: false,
                },
            ],
            active_tab_index: 0,
            hovered_tab: None,
            active_pane_index: 0,
            theme,
            window_width: 1280.0,
            window_height: 720.0,
            scale_factor: 2.0,
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

        let event = winit::event::WindowEvent::Resized(winit::dpi::PhysicalSize::new(1920, 1080));
        layer.push_event(&event, 1.0, winit::keyboard::ModifiersState::empty());

        assert!(layer.events.len() >= 1, "Should have at least 1 iced event after resize");
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
}

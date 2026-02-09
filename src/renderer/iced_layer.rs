// iced_wgpu integration layer — shares the existing wgpu device/queue
// and composites iced widget output onto the same TextureView.

use iced_graphics::Viewport;
use iced_runtime::user_interface::{Cache, UserInterface};
use iced_wgpu::Engine;
use iced_widget::{container, text};

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
    /// Call this AFTER the custom render pipeline has submitted, BEFORE surface present.
    pub fn render(&mut self, view: &wgpu::TextureView) {
        let bounds = self.viewport.logical_size();

        // Build widget tree — for now, an empty container (proof-of-concept).
        // Track 24 will replace this with real UI chrome widgets.
        let widget = Self::view();

        // Build the UserInterface from the widget tree
        let mut interface = UserInterface::build(
            widget,
            bounds,
            std::mem::take(&mut self.cache),
            &mut self.renderer,
        );

        // Process accumulated events
        let mut messages: Vec<()> = Vec::new();
        let mut clipboard = iced_core::clipboard::Null;
        let events = std::mem::take(&mut self.events);
        let (_state, _statuses) = interface.update(
            &events,
            self.cursor,
            &mut self.renderer,
            &mut clipboard,
            &mut messages,
        );

        // Draw the UI
        interface.draw(
            &mut self.renderer,
            &iced_core::Theme::Dark,
            &iced_core::renderer::Style {
                text_color: iced_core::Color::WHITE,
            },
            self.cursor,
        );

        // Save the cache for next frame
        self.cache = interface.into_cache();

        // Present iced's rendering onto the shared TextureView.
        // clear_color: None = transparent overlay (preserves custom renderer output underneath).
        self.renderer
            .present(None, self.format, view, &self.viewport);
    }

    /// The proof-of-concept widget tree.
    /// Returns an empty transparent container — iced renders but adds nothing visible.
    /// Track 24 will replace this with the actual UI chrome.
    fn view() -> iced_core::Element<'static, (), iced_core::Theme, iced_wgpu::Renderer> {
        container(text(""))
            .width(iced_core::Length::Fill)
            .height(iced_core::Length::Fill)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        // Push a resize event
        let event = winit::event::WindowEvent::Resized(winit::dpi::PhysicalSize::new(1920, 1080));
        layer.push_event(&event, 1.0, winit::keyboard::ModifiersState::empty());

        assert!(layer.events.len() >= 1, "Should have at least 1 iced event after resize");
    }

    #[test]
    fn iced_layer_empty_widget_tree_does_not_panic() {
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

        // Create a texture to render to
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

        // This should not panic — iced renders an empty/transparent layer
        layer.render(&view);
    }
}

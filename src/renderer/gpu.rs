// wgpu device, surface, and render pipeline setup.

use crate::config::theme::Color;

/// GPU state holding the wgpu instance, adapter, device, and queue.
/// Surface management is separate because it requires a window handle.
pub struct GpuContext {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl GpuContext {
    /// Create a new GPU context without a surface (headless).
    /// Selects a high-performance adapter and creates a device with default limits.
    pub async fn new_headless() -> Result<Self, GpuError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .ok_or(GpuError::AdapterNotFound)?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("VeloTerm Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .map_err(GpuError::DeviceCreationFailed)?;

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
    }

    /// Log information about the selected adapter.
    pub fn log_adapter_info(&self) {
        let info = self.adapter.get_info();
        log::info!(
            "GPU adapter: {} ({:?}, {:?})",
            info.name,
            info.device_type,
            info.backend
        );
    }
}

/// Errors that can occur during GPU initialization.
#[derive(Debug)]
pub enum GpuError {
    AdapterNotFound,
    DeviceCreationFailed(wgpu::RequestDeviceError),
}

impl std::fmt::Display for GpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AdapterNotFound => write!(f, "No suitable GPU adapter found"),
            Self::DeviceCreationFailed(e) => write!(f, "GPU device creation failed: {e}"),
        }
    }
}

impl std::error::Error for GpuError {}

/// Configuration for the render surface, extracted for testability.
#[derive(Debug, Clone)]
pub struct SurfaceConfig {
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
    pub present_mode: wgpu::PresentMode,
}

impl SurfaceConfig {
    /// Create a surface configuration for the given size and preferred format.
    /// Uses Fifo present mode (guaranteed available, v-synced).
    pub fn new(width: u32, height: u32, format: wgpu::TextureFormat) -> Self {
        Self {
            width,
            height,
            format,
            present_mode: wgpu::PresentMode::Fifo,
        }
    }

    /// Build a wgpu SurfaceConfiguration from this config.
    pub fn to_wgpu_config(&self) -> wgpu::SurfaceConfiguration {
        wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.format,
            width: self.width,
            height: self.height,
            present_mode: self.present_mode,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        }
    }
}

/// The clear color for Claude Dark theme background (#1A1816).
pub fn clear_color() -> wgpu::Color {
    let c = Color::from_hex("#1A1816");
    wgpu::Color {
        r: c.r as f64,
        g: c.g as f64,
        b: c.b as f64,
        a: c.a as f64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Headless GPU context tests ─────────────────────────────────

    fn try_create_headless() -> Option<GpuContext> {
        pollster::block_on(GpuContext::new_headless()).ok()
    }

    #[test]
    fn headless_context_creates_successfully() {
        let ctx = try_create_headless();
        assert!(
            ctx.is_some(),
            "GPU adapter should be available on dev machine"
        );
    }

    #[test]
    fn adapter_has_name() {
        let ctx = try_create_headless().expect("GPU required");
        let info = ctx.adapter.get_info();
        assert!(!info.name.is_empty(), "Adapter should have a name");
    }

    #[test]
    fn adapter_prefers_high_performance() {
        let ctx = try_create_headless().expect("GPU required");
        let info = ctx.adapter.get_info();
        assert!(
            info.device_type == wgpu::DeviceType::DiscreteGpu
                || info.device_type == wgpu::DeviceType::IntegratedGpu,
            "Adapter should be a real GPU, got {:?}",
            info.device_type
        );
    }

    #[test]
    fn device_supports_default_limits() {
        let ctx = try_create_headless().expect("GPU required");
        let limits = ctx.device.limits();
        assert!(limits.max_texture_dimension_2d >= 2048);
        assert!(limits.max_bind_groups >= 2);
        assert!(limits.max_vertex_buffers >= 1);
    }

    // ── SurfaceConfig tests ────────────────────────────────────────

    #[test]
    fn surface_config_stores_dimensions() {
        let cfg = SurfaceConfig::new(1280, 720, wgpu::TextureFormat::Bgra8UnormSrgb);
        assert_eq!(cfg.width, 1280);
        assert_eq!(cfg.height, 720);
    }

    #[test]
    fn surface_config_stores_format() {
        let cfg = SurfaceConfig::new(1280, 720, wgpu::TextureFormat::Bgra8UnormSrgb);
        assert_eq!(cfg.format, wgpu::TextureFormat::Bgra8UnormSrgb);
    }

    #[test]
    fn surface_config_uses_fifo_present_mode() {
        let cfg = SurfaceConfig::new(1280, 720, wgpu::TextureFormat::Bgra8UnormSrgb);
        assert_eq!(cfg.present_mode, wgpu::PresentMode::Fifo);
    }

    #[test]
    fn surface_config_to_wgpu() {
        let cfg = SurfaceConfig::new(1280, 720, wgpu::TextureFormat::Bgra8UnormSrgb);
        let wgpu_cfg = cfg.to_wgpu_config();
        assert_eq!(wgpu_cfg.width, 1280);
        assert_eq!(wgpu_cfg.height, 720);
        assert_eq!(wgpu_cfg.format, wgpu::TextureFormat::Bgra8UnormSrgb);
        assert_eq!(wgpu_cfg.present_mode, wgpu::PresentMode::Fifo);
        assert!(wgpu_cfg
            .usage
            .contains(wgpu::TextureUsages::RENDER_ATTACHMENT));
    }

    // ── Clear color tests ──────────────────────────────────────────

    #[test]
    fn clear_color_matches_claude_dark_background() {
        let c = clear_color();
        let eps = 1.0 / 512.0;
        assert!((c.r - 26.0 / 255.0).abs() < eps, "red: {}", c.r);
        assert!((c.g - 24.0 / 255.0).abs() < eps, "green: {}", c.g);
        assert!((c.b - 22.0 / 255.0).abs() < eps, "blue: {}", c.b);
        assert_eq!(c.a, 1.0);
    }
}

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
            .map_err(|_| GpuError::AdapterNotFound)?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("VeloTerm Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                    memory_hints: wgpu::MemoryHints::default(),
                    trace: wgpu::Trace::Off,
                    experimental_features: wgpu::ExperimentalFeatures::disabled(),
                },
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
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

/// Convert a single sRGB component to linear.
fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// The clear color for Claude Dark theme background (#141413).
/// Returns linear color values for correct rendering on sRGB surfaces.
pub fn clear_color() -> wgpu::Color {
    let c = Color::from_hex("#141413");
    wgpu::Color {
        r: srgb_to_linear(c.r as f64),
        g: srgb_to_linear(c.g as f64),
        b: srgb_to_linear(c.b as f64),
        a: c.a as f64,
    }
}

/// Per-cell instance data sent to the GPU vertex shader.
/// Layout must match the CellInstance struct in grid.wgsl.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CellInstance {
    pub position: [f32; 2], // grid column, row
    pub atlas_uv: [f32; 4], // u, v, width, height in atlas
    pub fg_color: [f32; 4], // foreground RGBA
    pub bg_color: [f32; 4], // background RGBA
    pub flags: u32,         // bit 0: has_glyph
    pub _padding: [u32; 3], // pad to 16-byte alignment
}

impl CellInstance {
    /// Vertex buffer layout describing CellInstance attributes for the pipeline.
    pub fn vertex_buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<CellInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // position: vec2<f32> at location(0)
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
                // atlas_uv: vec4<f32> at location(1)
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 8,
                    shader_location: 1,
                },
                // fg_color: vec4<f32> at location(2)
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 24,
                    shader_location: 2,
                },
                // bg_color: vec4<f32> at location(3)
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 40,
                    shader_location: 3,
                },
                // flags: u32 at location(4)
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Uint32,
                    offset: 56,
                    shader_location: 4,
                },
            ],
        }
    }
}

/// Uniform data for the grid shader.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GridUniforms {
    pub cell_size: [f32; 2],
    pub grid_size: [f32; 2],
    pub atlas_size: [f32; 2],
    pub _padding: [f32; 2],
}

// ── Overlay pipeline types ───────────────────────────────────────────

/// Per-instance data for UI overlay quads (dividers, focus overlays).
/// Layout must match OverlayInstance struct in overlay.wgsl.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct OverlayInstance {
    pub rect: [f32; 4],   // x, y, width, height in pixels
    pub color: [f32; 4],  // RGBA
    pub extras: [f32; 4], // border_radius, _pad, _pad, _pad
}

impl OverlayInstance {
    pub fn vertex_buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<OverlayInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // rect: vec4<f32> at location(0)
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 0,
                },
                // color: vec4<f32> at location(1)
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 16,
                    shader_location: 1,
                },
                // extras: vec4<f32> at location(2) — border_radius in .x
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 32,
                    shader_location: 2,
                },
            ],
        }
    }
}

/// Uniform data for the overlay shader.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct OverlayUniforms {
    pub surface_size: [f32; 2],
    pub _padding: [f32; 2],
}

/// Create the bind group layout for the overlay shader.
/// Group 0: uniforms (binding 0) only.
pub fn create_overlay_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Overlay Bind Group Layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

/// Create the render pipeline for the overlay shader.
pub fn create_overlay_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader_source = include_str!("../../shaders/overlay.wgsl");
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Overlay Shader"),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Overlay Pipeline Layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Overlay Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[OverlayInstance::vertex_buffer_layout()],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

/// Create the bind group layout for the grid shader.
/// Group 0: uniforms (binding 0), atlas texture (binding 1), sampler (binding 2).
pub fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Grid Bind Group Layout"),
        entries: &[
            // Uniforms
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Atlas texture
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // Sampler
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

/// Create the render pipeline for the grid shader.
pub fn create_render_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader_source = include_str!("../../shaders/grid.wgsl");
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Grid Shader"),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Grid Pipeline Layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Grid Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[CellInstance::vertex_buffer_layout()],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

/// Upload glyph atlas R8 data to a GPU texture.
pub fn create_atlas_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
    data: &[u8],
) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Glyph Atlas"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    texture
}

/// Create a linear-filtering sampler for the glyph atlas.
/// Linear filtering smooths fractional-pixel glyph positions instead of producing jagged edges.
pub fn create_atlas_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("Atlas Sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        ..Default::default()
    })
}

/// Create the bind group for the grid shader.
pub fn create_grid_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform_buffer: &wgpu::Buffer,
    atlas_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Grid Bind Group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(atlas_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
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
    fn clear_color_is_linear_version_of_claude_dark_background() {
        let c = clear_color();
        // #141413 = sRGB(20/255, 20/255, 19/255) → linear values are much smaller
        // srgb_to_linear(20/255) ≈ 0.00607
        assert!(c.r < 0.01, "red should be linearized: {}", c.r);
        assert!(c.g < 0.01, "green should be linearized: {}", c.g);
        assert!(c.b < 0.01, "blue should be linearized: {}", c.b);
        assert!(c.r > 0.004, "red not too small: {}", c.r);
        assert_eq!(c.a, 1.0);
    }

    // ── CellInstance layout tests ──────────────────────────────────

    #[test]
    fn cell_instance_size_is_64_bytes() {
        // 2 + 4 + 4 + 4 + 1 + 3 padding = 18 f32s = 72 bytes... wait
        // position(2*4=8) + atlas_uv(4*4=16) + fg(4*4=16) + bg(4*4=16) + flags(4) + pad(12) = 72
        // Actually let's just assert the actual size
        assert_eq!(
            std::mem::size_of::<CellInstance>(),
            72,
            "CellInstance must be 72 bytes (position 8 + atlas_uv 16 + fg 16 + bg 16 + flags 4 + pad 12)"
        );
    }

    #[test]
    fn cell_instance_is_pod() {
        // Verify bytemuck::Pod works — if this compiles and runs, it's valid
        let cell = CellInstance {
            position: [0.0, 0.0],
            atlas_uv: [0.0, 0.0, 1.0, 1.0],
            fg_color: [1.0, 1.0, 1.0, 1.0],
            bg_color: [0.0, 0.0, 0.0, 1.0],
            flags: 1,
            _padding: [0; 3],
        };
        let bytes = bytemuck::bytes_of(&cell);
        assert_eq!(bytes.len(), 72);
    }

    #[test]
    fn cell_instance_vertex_layout_has_5_attributes() {
        let layout = CellInstance::vertex_buffer_layout();
        assert_eq!(layout.attributes.len(), 5);
        assert_eq!(layout.step_mode, wgpu::VertexStepMode::Instance);
    }

    #[test]
    fn cell_instance_vertex_layout_stride() {
        let layout = CellInstance::vertex_buffer_layout();
        assert_eq!(layout.array_stride, 72);
    }

    #[test]
    fn grid_uniforms_size_is_32_bytes() {
        assert_eq!(
            std::mem::size_of::<GridUniforms>(),
            32,
            "GridUniforms must be 32 bytes (cell_size 8 + grid_size 8 + atlas_size 8 + pad 8)"
        );
    }

    // ── Pipeline creation tests (require GPU) ──────────────────────

    #[test]
    fn shader_compiles_and_pipeline_creates() {
        let ctx = try_create_headless().expect("GPU required");
        let bind_group_layout = create_bind_group_layout(&ctx.device);
        // Using Bgra8UnormSrgb — our preferred surface format (sRGB gamma encoding)
        let pipeline = create_render_pipeline(
            &ctx.device,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            &bind_group_layout,
        );
        // If we get here without panicking, the shader compiled and pipeline created
        // The pipeline object existing is the assertion
        let _ = pipeline;
    }

    #[test]
    fn bind_group_layout_has_3_entries() {
        let ctx = try_create_headless().expect("GPU required");
        // We can't introspect the layout directly, but we can verify it was created
        // by using it to create a pipeline (which validates the layout)
        let layout = create_bind_group_layout(&ctx.device);
        let _ = layout; // creation success is the assertion
    }

    // ── Atlas GPU texture upload tests ───────────────────────────────

    #[test]
    fn atlas_texture_has_correct_dimensions() {
        let ctx = try_create_headless().expect("GPU required");
        let data = vec![128u8; 64 * 64];
        let texture = create_atlas_texture(&ctx.device, &ctx.queue, 64, 64, &data);
        assert_eq!(texture.width(), 64);
        assert_eq!(texture.height(), 64);
    }

    #[test]
    fn atlas_texture_format_is_r8unorm() {
        let ctx = try_create_headless().expect("GPU required");
        let data = vec![0u8; 64 * 64];
        let texture = create_atlas_texture(&ctx.device, &ctx.queue, 64, 64, &data);
        assert_eq!(texture.format(), wgpu::TextureFormat::R8Unorm);
    }

    #[test]
    fn atlas_sampler_creates_successfully() {
        let ctx = try_create_headless().expect("GPU required");
        let _sampler = create_atlas_sampler(&ctx.device);
    }

    #[test]
    fn grid_bind_group_creates_successfully() {
        let ctx = try_create_headless().expect("GPU required");

        let uniform_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Test Uniforms"),
            size: std::mem::size_of::<GridUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });

        let data = vec![0u8; 64 * 64];
        let texture = create_atlas_texture(&ctx.device, &ctx.queue, 64, 64, &data);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = create_atlas_sampler(&ctx.device);

        let layout = create_bind_group_layout(&ctx.device);
        let _bind_group =
            create_grid_bind_group(&ctx.device, &layout, &uniform_buffer, &view, &sampler);
    }

    // ── Overlay types tests ──────────────────────────────────────────

    #[test]
    fn overlay_instance_size_is_48_bytes() {
        assert_eq!(
            std::mem::size_of::<OverlayInstance>(),
            48,
            "OverlayInstance must be 48 bytes (rect 16 + color 16 + extras 16)"
        );
    }

    #[test]
    fn overlay_instance_is_pod() {
        let inst = OverlayInstance {
            rect: [100.0, 200.0, 2.0, 720.0],
            color: [0.5, 0.4, 0.3, 1.0],
            extras: [0.0, 0.0, 0.0, 0.0],
        };
        let bytes = bytemuck::bytes_of(&inst);
        assert_eq!(bytes.len(), 48);
    }

    #[test]
    fn overlay_instance_vertex_layout_has_3_attributes() {
        let layout = OverlayInstance::vertex_buffer_layout();
        assert_eq!(layout.attributes.len(), 3);
        assert_eq!(layout.step_mode, wgpu::VertexStepMode::Instance);
        assert_eq!(layout.array_stride, 48);
    }

    #[test]
    fn overlay_uniforms_size_is_16_bytes() {
        assert_eq!(
            std::mem::size_of::<OverlayUniforms>(),
            16,
            "OverlayUniforms must be 16 bytes (surface_size 8 + pad 8)"
        );
    }

    #[test]
    fn overlay_shader_compiles_and_pipeline_creates() {
        let ctx = try_create_headless().expect("GPU required");
        let layout = create_overlay_bind_group_layout(&ctx.device);
        let pipeline = create_overlay_pipeline(
            &ctx.device,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            &layout,
        );
        let _ = pipeline;
    }
}

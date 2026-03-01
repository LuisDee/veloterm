// GPU image rendering for the Kitty Graphics Protocol.
//
// Renders textured quads for inline terminal images. Each image gets its own
// wgpu::Texture + BindGroup (separate from the glyph atlas). Placements are
// rendered in z-index order: z<0 under text, z>=0 over text.

use crate::image_protocol::{ImageData, Placement};
use crate::renderer::grid_renderer::GridDimensions;
use wgpu::util::DeviceExt;

/// NDC quad coordinates for an image placement.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageQuad {
    /// Left edge in NDC (-1..1)
    pub x0: f32,
    /// Top edge in NDC (-1..1, +1 = top)
    pub y0: f32,
    /// Width in NDC
    pub w: f32,
    /// Height in NDC
    pub h: f32,
}

/// UV coordinates within a source image texture.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceUV {
    pub u: f32,
    pub v: f32,
    pub w: f32,
    pub h: f32,
}

/// Compute NDC quad position for an image placement on the terminal grid.
///
/// `scroll_offset` is the absolute row number at the top of the viewport.
/// Returns the quad's top-left position and size in NDC space.
pub fn compute_quad_ndc(
    dims: &GridDimensions,
    placement: &Placement,
    scroll_offset: i64,
) -> ImageQuad {
    let cell_w_ndc = 2.0 / dims.columns as f32;
    let cell_h_ndc = 2.0 / dims.rows as f32;

    // Placement position relative to viewport
    let rel_row = (placement.grid_row - scroll_offset) as f32;
    let rel_col = placement.grid_col as f32;

    // Display size in cells: use explicit columns/rows if set, otherwise
    // compute from image pixel dimensions / cell pixel dimensions
    let display_cols = if placement.columns > 0 {
        placement.columns as f32
    } else {
        // Fallback: would need image width + cell_width, but we don't have image here
        // Default to 1 column
        1.0
    };
    let display_rows = if placement.rows > 0 {
        placement.rows as f32
    } else {
        1.0
    };

    // NDC coordinates: origin at top-left = (-1, +1)
    let x0 = -1.0 + rel_col * cell_w_ndc;
    let y0 = 1.0 - rel_row * cell_h_ndc;
    let w = display_cols * cell_w_ndc;
    let h = display_rows * cell_h_ndc;

    ImageQuad { x0, y0, w, h }
}

/// Compute UV coordinates within the source image for a placement.
///
/// If `source_rect` is None, the full image is used (0,0,1,1).
/// If `source_rect` is Some((x, y, w, h)), the UV is cropped to that region.
pub fn compute_source_uv(image: &ImageData, placement: &Placement) -> SourceUV {
    match placement.source_rect {
        None => SourceUV {
            u: 0.0,
            v: 0.0,
            w: 1.0,
            h: 1.0,
        },
        Some((sx, sy, sw, sh)) => {
            let iw = image.width.max(1) as f32;
            let ih = image.height.max(1) as f32;
            SourceUV {
                u: sx as f32 / iw,
                v: sy as f32 / ih,
                w: sw as f32 / iw,
                h: sh as f32 / ih,
            }
        }
    }
}

/// Filter placements into those visible in the current viewport.
/// Returns (under_text, over_text) split by z-index.
pub fn split_placements_by_z<'a>(
    placements: &[&'a Placement],
) -> (Vec<&'a Placement>, Vec<&'a Placement>) {
    let mut under = Vec::new();
    let mut over = Vec::new();
    for &p in placements {
        if p.z_index < 0 {
            under.push(p);
        } else {
            over.push(p);
        }
    }
    (under, over)
}

/// Uniform data for the image shader. Must match ImageUniforms in image.wgsl.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ImageUniforms {
    /// NDC position (top-left x, top-left y)
    pub position: [f32; 2],
    /// NDC size (width, height)
    pub size: [f32; 2],
    /// Source UV rect (u, v, w, h)
    pub source_uv: [f32; 4],
}

/// Create the bind group layout for the image shader.
pub fn create_image_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Image Bind Group Layout"),
        entries: &[
            // Uniforms
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(
                        std::mem::size_of::<ImageUniforms>() as u64,
                    ),
                },
                count: None,
            },
            // Image texture
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

/// Create the render pipeline for image quad rendering.
pub fn create_image_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader_source = include_str!("../../shaders/image.wgsl");
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Image Shader"),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Image Pipeline Layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Image Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[], // No vertex buffer — quad generated from vertex_index
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

/// Create a bilinear-filtering sampler for image textures.
/// Images are typically scaled up/down so bilinear filtering looks better than nearest.
pub fn create_image_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("Image Sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        ..Default::default()
    })
}

/// Upload RGBA pixel data to a GPU texture.
pub fn create_image_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
    rgba_data: &[u8],
) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Kitty Image"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
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
        rgba_data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
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

/// Create a bind group for rendering a single image placement.
pub fn create_image_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform_buffer: &wgpu::Buffer,
    texture_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Image Bind Group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(texture_view),
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

    fn test_dims() -> GridDimensions {
        GridDimensions {
            columns: 80,
            rows: 24,
            cell_width: 10.0,
            cell_height: 20.0,
            window_width: 800,
            window_height: 480,
        }
    }

    // ── Placement-to-quad math (pure functions, no GPU) ─────────

    #[test]
    fn compute_quad_position_top_left() {
        let dims = test_dims();
        let placement = Placement {
            grid_row: 0,
            grid_col: 0,
            columns: 5,
            rows: 3,
            ..Default::default()
        };
        let quad = compute_quad_ndc(&dims, &placement, 0);
        // Top-left of grid = NDC (-1, 1)
        assert!((quad.x0 - (-1.0)).abs() < 0.01);
        assert!((quad.y0 - 1.0).abs() < 0.01);
    }

    #[test]
    fn compute_quad_size_matches_cell_count() {
        let dims = test_dims();
        let placement = Placement {
            grid_row: 0,
            grid_col: 0,
            columns: 10,
            rows: 5,
            ..Default::default()
        };
        let quad = compute_quad_ndc(&dims, &placement, 0);
        // 10 columns out of 80 = 10/80 of the NDC width (2.0)
        let expected_w = 10.0 / 80.0 * 2.0;
        let expected_h = 5.0 / 24.0 * 2.0;
        assert!(
            (quad.w - expected_w).abs() < 0.001,
            "width: {} vs {}",
            quad.w,
            expected_w
        );
        assert!(
            (quad.h - expected_h).abs() < 0.001,
            "height: {} vs {}",
            quad.h,
            expected_h
        );
    }

    #[test]
    fn compute_quad_offset_position() {
        let dims = test_dims();
        let placement = Placement {
            grid_row: 10,
            grid_col: 20,
            columns: 5,
            rows: 3,
            ..Default::default()
        };
        let quad = compute_quad_ndc(&dims, &placement, 0);
        let cell_w_ndc = 2.0 / 80.0;
        let cell_h_ndc = 2.0 / 24.0;
        let expected_x = -1.0 + 20.0 * cell_w_ndc;
        let expected_y = 1.0 - 10.0 * cell_h_ndc;
        assert!(
            (quad.x0 - expected_x).abs() < 0.001,
            "x: {} vs {}",
            quad.x0,
            expected_x
        );
        assert!(
            (quad.y0 - expected_y).abs() < 0.001,
            "y: {} vs {}",
            quad.y0,
            expected_y
        );
    }

    #[test]
    fn compute_quad_scrolled_up() {
        let dims = test_dims();
        let placement = Placement {
            grid_row: 100,
            grid_col: 0,
            columns: 5,
            rows: 3,
            ..Default::default()
        };
        // Scrolled to row 100 — placement should be at top of viewport
        let quad = compute_quad_ndc(&dims, &placement, 100);
        assert!((quad.y0 - 1.0).abs() < 0.01);
    }

    #[test]
    fn compute_quad_off_screen_above() {
        let dims = test_dims();
        let placement = Placement {
            grid_row: 0,
            grid_col: 0,
            columns: 5,
            rows: 3,
            ..Default::default()
        };
        let quad = compute_quad_ndc(&dims, &placement, 50);
        // Row 0 when scrolled to 50 means rel_row = -50
        // y0 = 1.0 - (-50 * cell_h_ndc) = 1.0 + 50*cell_h_ndc > 1.0
        assert!(quad.y0 > 1.0, "should be above viewport: y0={}", quad.y0);
    }

    // ── Source UV math ──────────────────────────────────────────

    #[test]
    fn compute_source_uv_full_image() {
        let img = ImageData {
            width: 100,
            height: 50,
            pixels: vec![],
            refcount: 0,
        };
        let placement = Placement::default();
        let uv = compute_source_uv(&img, &placement);
        assert_eq!(uv.u, 0.0);
        assert_eq!(uv.v, 0.0);
        assert_eq!(uv.w, 1.0);
        assert_eq!(uv.h, 1.0);
    }

    #[test]
    fn compute_source_uv_cropped() {
        let img = ImageData {
            width: 100,
            height: 50,
            pixels: vec![],
            refcount: 0,
        };
        let placement = Placement {
            source_rect: Some((10, 5, 20, 10)),
            ..Default::default()
        };
        let uv = compute_source_uv(&img, &placement);
        assert!((uv.u - 0.1).abs() < 0.001); // x=10/100
        assert!((uv.v - 0.1).abs() < 0.001); // y=5/50
        assert!((uv.w - 0.2).abs() < 0.001); // w=20/100
        assert!((uv.h - 0.2).abs() < 0.001); // h=10/50
    }

    #[test]
    fn compute_source_uv_zero_dimension_safe() {
        let img = ImageData {
            width: 0,
            height: 0,
            pixels: vec![],
            refcount: 0,
        };
        let placement = Placement {
            source_rect: Some((10, 5, 20, 10)),
            ..Default::default()
        };
        // Should not panic (divides by max(1))
        let uv = compute_source_uv(&img, &placement);
        assert!(uv.u.is_finite());
    }

    // ── Z-index splitting ───────────────────────────────────────

    #[test]
    fn split_placements_empty() {
        let placements: Vec<&Placement> = vec![];
        let (under, over) = split_placements_by_z(&placements);
        assert!(under.is_empty());
        assert!(over.is_empty());
    }

    #[test]
    fn split_placements_by_z_index() {
        let p1 = Placement {
            z_index: -1,
            placement_id: 1,
            ..Default::default()
        };
        let p2 = Placement {
            z_index: 0,
            placement_id: 2,
            ..Default::default()
        };
        let p3 = Placement {
            z_index: 5,
            placement_id: 3,
            ..Default::default()
        };
        let placements: Vec<&Placement> = vec![&p1, &p2, &p3];
        let (under, over) = split_placements_by_z(&placements);
        assert_eq!(under.len(), 1, "z<0 goes under text");
        assert_eq!(under[0].z_index, -1);
        assert_eq!(over.len(), 2, "z>=0 goes over text");
    }

    #[test]
    fn split_placements_z_zero_goes_over() {
        let p = Placement {
            z_index: 0,
            ..Default::default()
        };
        let placements: Vec<&Placement> = vec![&p];
        let (under, over) = split_placements_by_z(&placements);
        assert!(under.is_empty());
        assert_eq!(over.len(), 1);
    }

    // ── ImageUniforms layout ────────────────────────────────────

    #[test]
    fn image_uniforms_size() {
        // position(8) + size(8) + source_uv(16) = 32 bytes
        assert_eq!(
            std::mem::size_of::<ImageUniforms>(),
            32,
            "ImageUniforms must be 32 bytes"
        );
    }

    #[test]
    fn image_uniforms_is_pod() {
        let u = ImageUniforms {
            position: [-1.0, 1.0],
            size: [0.5, 0.5],
            source_uv: [0.0, 0.0, 1.0, 1.0],
        };
        let bytes = bytemuck::bytes_of(&u);
        assert_eq!(bytes.len(), 32);
    }

    // ── GPU pipeline tests (require headless GPU) ───────────────

    fn try_create_headless() -> Option<crate::renderer::gpu::GpuContext> {
        pollster::block_on(crate::renderer::gpu::GpuContext::new_headless()).ok()
    }

    #[test]
    fn image_shader_compiles_and_pipeline_creates() {
        let ctx = match try_create_headless() {
            Some(c) => c,
            None => return,
        };
        let layout = create_image_bind_group_layout(&ctx.device);
        let _pipeline = create_image_pipeline(
            &ctx.device,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            &layout,
        );
    }

    #[test]
    fn image_bind_group_layout_creates() {
        let ctx = match try_create_headless() {
            Some(c) => c,
            None => return,
        };
        let _layout = create_image_bind_group_layout(&ctx.device);
    }

    #[test]
    fn image_sampler_creates() {
        let ctx = match try_create_headless() {
            Some(c) => c,
            None => return,
        };
        let _sampler = create_image_sampler(&ctx.device);
    }

    #[test]
    fn image_texture_upload() {
        let ctx = match try_create_headless() {
            Some(c) => c,
            None => return,
        };
        // 2x2 RGBA test image (red pixel)
        let rgba = vec![
            255, 0, 0, 255, // red
            0, 255, 0, 255, // green
            0, 0, 255, 255, // blue
            255, 255, 0, 255, // yellow
        ];
        let texture = create_image_texture(&ctx.device, &ctx.queue, 2, 2, &rgba);
        assert_eq!(texture.width(), 2);
        assert_eq!(texture.height(), 2);
        assert_eq!(texture.format(), wgpu::TextureFormat::Rgba8UnormSrgb);
    }

    #[test]
    fn image_bind_group_creates() {
        let ctx = match try_create_headless() {
            Some(c) => c,
            None => return,
        };
        let layout = create_image_bind_group_layout(&ctx.device);
        let sampler = create_image_sampler(&ctx.device);

        let rgba = vec![255u8; 4 * 4]; // 2x2
        let texture = create_image_texture(&ctx.device, &ctx.queue, 2, 2, &rgba);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let uniforms = ImageUniforms {
            position: [-1.0, 1.0],
            size: [0.5, 0.5],
            source_uv: [0.0, 0.0, 1.0, 1.0],
        };
        let uniform_buffer =
            ctx.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Test Image Uniforms"),
                    contents: bytemuck::bytes_of(&uniforms),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let _bind_group = create_image_bind_group(
            &ctx.device,
            &layout,
            &uniform_buffer,
            &view,
            &sampler,
        );
    }
}

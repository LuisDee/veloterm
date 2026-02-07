pub mod cursor;
pub mod glyph_atlas;
pub mod gpu;
pub mod grid_renderer;

use crate::config::theme::Theme;
use glyph_atlas::GlyphAtlas;
use gpu::{
    clear_color, create_atlas_sampler, create_atlas_texture, create_bind_group_layout,
    create_grid_bind_group, create_render_pipeline, GpuError, GridUniforms, SurfaceConfig,
};
use grid_renderer::{generate_instances, generate_test_pattern, GridCell, GridDimensions};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;

/// Top-level render coordinator.
/// Holds all GPU state, glyph atlas, grid, and render resources.
pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: SurfaceConfig,
    render_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    instance_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    instance_count: u32,
    grid: GridDimensions,
    atlas: GlyphAtlas,
    theme: Theme,
    _bind_group_layout: wgpu::BindGroupLayout,
    _atlas_view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
}

impl Renderer {
    /// Initialize the renderer with a window, theme, and font size.
    /// Creates GPU context, glyph atlas, grid, and all render resources.
    pub async fn new(window: Arc<Window>, theme: Theme, font_size: f32) -> Result<Self, GpuError> {
        let size = window.inner_size();
        let scale_factor = window.scale_factor() as f32;

        // GPU setup
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance
            .create_surface(window)
            .map_err(|_| GpuError::AdapterNotFound)?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .ok_or(GpuError::AdapterNotFound)?;

        let info = adapter.get_info();
        log::info!(
            "GPU adapter: {} ({:?}, {:?})",
            info.name,
            info.device_type,
            info.backend
        );

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

        // Surface configuration
        let surface_caps = surface.get_capabilities(&adapter);
        let format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = SurfaceConfig::new(size.width, size.height, format);
        surface.configure(&device, &surface_config.to_wgpu_config());

        // Glyph atlas
        let atlas = GlyphAtlas::new(font_size, scale_factor);
        log::info!(
            "Glyph atlas: {}x{} (cell: {:.1}x{:.1})",
            atlas.atlas_width,
            atlas.atlas_height,
            atlas.cell_width,
            atlas.cell_height
        );

        // Upload atlas texture
        let atlas_texture = create_atlas_texture(
            &device,
            &queue,
            atlas.atlas_width,
            atlas.atlas_height,
            &atlas.atlas_data,
        );
        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = create_atlas_sampler(&device);

        // Grid dimensions
        let grid =
            GridDimensions::new(size.width, size.height, atlas.cell_width, atlas.cell_height);
        log::info!("Grid: {}x{} cells", grid.columns, grid.rows);

        // Initial test pattern with configured theme
        let cells = generate_test_pattern(&grid, &theme);
        let instances = generate_instances(&grid, &cells, &atlas);
        let instance_count = instances.len() as u32;

        // Uniforms
        let uniforms = GridUniforms {
            cell_size: grid.cell_size_ndc(),
            grid_size: grid.grid_size(),
            atlas_size: [atlas.atlas_width as f32, atlas.atlas_height as f32],
            _padding: [0.0; 2],
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Uniforms"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Instance buffer
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cell Instances"),
            contents: bytemuck::cast_slice(&instances),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        // Render pipeline
        let bind_group_layout = create_bind_group_layout(&device);
        let render_pipeline = create_render_pipeline(&device, format, &bind_group_layout);
        let bind_group = create_grid_bind_group(
            &device,
            &bind_group_layout,
            &uniform_buffer,
            &atlas_view,
            &sampler,
        );

        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
            render_pipeline,
            bind_group,
            instance_buffer,
            uniform_buffer,
            instance_count,
            grid,
            atlas,
            theme,
            _bind_group_layout: bind_group_layout,
            _atlas_view: atlas_view,
            _sampler: sampler,
        })
    }

    /// Render a frame: acquire surface texture, draw instances, present.
    pub fn render_frame(&self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Grid Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
            render_pass.draw(0..6, 0..self.instance_count);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }

    /// Handle window resize: reconfigure surface, recalculate grid, rebuild instances.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        // Clamp to GPU's max texture dimension to prevent wgpu validation errors
        let max_dim = self.device.limits().max_texture_dimension_2d;
        let width = width.min(max_dim);
        let height = height.min(max_dim);

        // Reconfigure surface
        self.surface_config = SurfaceConfig::new(width, height, self.surface_config.format);
        self.surface
            .configure(&self.device, &self.surface_config.to_wgpu_config());

        // Recalculate grid
        self.grid.resize(width, height);
        log::debug!(
            "Grid resized: {}x{} cells",
            self.grid.columns,
            self.grid.rows
        );

        // Rebuild test pattern and instances
        let cells = generate_test_pattern(&self.grid, &self.theme);
        let instances = generate_instances(&self.grid, &cells, &self.atlas);
        self.instance_count = instances.len() as u32;

        // Update uniforms
        let uniforms = GridUniforms {
            cell_size: self.grid.cell_size_ndc(),
            grid_size: self.grid.grid_size(),
            atlas_size: [
                self.atlas.atlas_width as f32,
                self.atlas.atlas_height as f32,
            ],
            _padding: [0.0; 2],
        };
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        // Recreate instance buffer (size may have changed)
        self.instance_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Cell Instances"),
                contents: bytemuck::cast_slice(&instances),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
    }

    /// Get a reference to the grid dimensions.
    pub fn grid(&self) -> &GridDimensions {
        &self.grid
    }

    /// Update the rendered cells from external terminal state.
    pub fn update_cells(&mut self, cells: &[GridCell]) {
        let instances = generate_instances(&self.grid, cells, &self.atlas);
        self.instance_count = instances.len() as u32;

        self.instance_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Cell Instances"),
                contents: bytemuck::cast_slice(&instances),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Renderer initialization (headless) ─────────────────────────

    #[test]
    fn renderer_components_initialize() {
        // Test that all component pieces work together without a window
        let atlas = GlyphAtlas::new(13.0, 2.0);
        let grid = GridDimensions::new(1280, 720, atlas.cell_width, atlas.cell_height);
        let theme = Theme::claude_dark();
        let cells = generate_test_pattern(&grid, &theme);
        let instances = generate_instances(&grid, &cells, &atlas);

        assert!(grid.columns > 0);
        assert!(grid.rows > 0);
        assert_eq!(instances.len(), grid.total_cells() as usize);
    }

    #[test]
    fn renderer_uniforms_match_grid() {
        let atlas = GlyphAtlas::new(13.0, 2.0);
        let grid = GridDimensions::new(1280, 720, atlas.cell_width, atlas.cell_height);

        let uniforms = GridUniforms {
            cell_size: grid.cell_size_ndc(),
            grid_size: grid.grid_size(),
            atlas_size: [atlas.atlas_width as f32, atlas.atlas_height as f32],
            _padding: [0.0; 2],
        };

        assert_eq!(uniforms.grid_size[0], grid.columns as f32);
        assert_eq!(uniforms.grid_size[1], grid.rows as f32);
        assert_eq!(uniforms.atlas_size[0], atlas.atlas_width as f32);
    }

    #[test]
    fn renderer_resize_changes_grid() {
        let atlas = GlyphAtlas::new(13.0, 2.0);
        let mut grid = GridDimensions::new(1280, 720, atlas.cell_width, atlas.cell_height);
        let original_cols = grid.columns;
        let original_rows = grid.rows;

        grid.resize(1920, 1080);
        assert!(grid.columns > original_cols);
        assert!(grid.rows > original_rows);
    }

    #[test]
    fn renderer_instances_have_correct_count_after_resize() {
        let atlas = GlyphAtlas::new(13.0, 2.0);
        let mut grid = GridDimensions::new(1280, 720, atlas.cell_width, atlas.cell_height);
        let theme = Theme::claude_dark();

        grid.resize(800, 600);
        let cells = generate_test_pattern(&grid, &theme);
        let instances = generate_instances(&grid, &cells, &atlas);
        assert_eq!(instances.len(), grid.total_cells() as usize);
    }

    // ── GPU resource tests (require headless GPU) ──────────────────

    #[test]
    fn renderer_gpu_resources_create() {
        let ctx = match pollster::block_on(gpu::GpuContext::new_headless()) {
            Ok(ctx) => ctx,
            Err(_) => return, // Skip if no GPU
        };

        let atlas = GlyphAtlas::new(13.0, 1.0);
        let atlas_texture = create_atlas_texture(
            &ctx.device,
            &ctx.queue,
            atlas.atlas_width,
            atlas.atlas_height,
            &atlas.atlas_data,
        );
        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = create_atlas_sampler(&ctx.device);

        let grid = GridDimensions::new(640, 480, atlas.cell_width, atlas.cell_height);
        let theme = Theme::claude_dark();
        let cells = generate_test_pattern(&grid, &theme);
        let instances = generate_instances(&grid, &cells, &atlas);

        let uniforms = GridUniforms {
            cell_size: grid.cell_size_ndc(),
            grid_size: grid.grid_size(),
            atlas_size: [atlas.atlas_width as f32, atlas.atlas_height as f32],
            _padding: [0.0; 2],
        };
        let uniform_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Test Uniforms"),
                contents: bytemuck::bytes_of(&uniforms),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let _instance_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Test Instances"),
                contents: bytemuck::cast_slice(&instances),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let bind_group_layout = create_bind_group_layout(&ctx.device);
        let _pipeline = create_render_pipeline(
            &ctx.device,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            &bind_group_layout,
        );
        let _bind_group = create_grid_bind_group(
            &ctx.device,
            &bind_group_layout,
            &uniform_buffer,
            &atlas_view,
            &sampler,
        );

        assert!(instances.len() > 0);
    }

    // ── Config integration tests ────────────────────────────────────

    #[test]
    fn config_font_size_affects_cell_dimensions() {
        let small = GlyphAtlas::new(10.0, 1.0);
        let large = GlyphAtlas::new(20.0, 1.0);
        // Larger font → larger cells
        assert!(large.cell_width > small.cell_width);
        assert!(large.cell_height > small.cell_height);
    }

    #[test]
    fn config_larger_font_produces_fewer_grid_cells() {
        // Same window size, different font sizes → different grid dimensions
        let small_atlas = GlyphAtlas::new(10.0, 1.0);
        let large_atlas = GlyphAtlas::new(20.0, 1.0);
        let small_grid =
            GridDimensions::new(1280, 720, small_atlas.cell_width, small_atlas.cell_height);
        let large_grid =
            GridDimensions::new(1280, 720, large_atlas.cell_width, large_atlas.cell_height);
        assert!(
            large_grid.columns < small_grid.columns,
            "larger font should produce fewer columns"
        );
        assert!(
            large_grid.rows < small_grid.rows,
            "larger font should produce fewer rows"
        );
    }

    #[test]
    fn config_theme_from_name_works_in_renderer() {
        let atlas = GlyphAtlas::new(13.0, 2.0);
        let grid = GridDimensions::new(1280, 720, atlas.cell_width, atlas.cell_height);

        for name in &["claude_dark", "claude_light", "claude_warm"] {
            let theme = Theme::from_name(name).unwrap();
            let cells = generate_test_pattern(&grid, &theme);
            let instances = generate_instances(&grid, &cells, &atlas);
            assert_eq!(instances.len(), grid.total_cells() as usize);
        }
    }

    #[test]
    fn config_theme_produces_different_background_colors() {
        let atlas = GlyphAtlas::new(13.0, 1.0);
        let grid = GridDimensions::new(640, 480, atlas.cell_width, atlas.cell_height);

        let dark = Theme::from_name("claude_dark").unwrap();
        let light = Theme::from_name("claude_light").unwrap();

        let dark_cells = generate_test_pattern(&grid, &dark);
        let light_cells = generate_test_pattern(&grid, &light);

        let dark_instances = generate_instances(&grid, &dark_cells, &atlas);
        let light_instances = generate_instances(&grid, &light_cells, &atlas);

        // Background colors must differ between themes
        assert_ne!(
            dark_instances[0].bg_color, light_instances[0].bg_color,
            "dark and light themes should produce different bg colors"
        );
    }

    #[test]
    fn config_unknown_theme_falls_back_to_claude_dark() {
        let fallback = Theme::from_name("nonexistent");
        assert!(fallback.is_none());
        // Application code does: .unwrap_or_else(|| Theme::claude_dark())
        let theme = fallback.unwrap_or_else(Theme::claude_dark);
        assert_eq!(theme.name, "Claude Dark");
    }
}

pub mod cursor;
#[cfg(target_os = "macos")]
pub mod coretext_rasterizer;
pub mod damage;
pub mod glyph_atlas;
pub mod gpu;
pub mod grid_renderer;
pub mod iced_layer;

use crate::config::theme::TerminalTheme;
use crate::pane::{PaneId, Rect as PaneRect};
use damage::{DamageState, PaneDamageMap};
use glyph_atlas::GlyphAtlas;
use gpu::{
    clear_color, create_atlas_sampler, create_atlas_texture, create_bind_group_layout,
    create_grid_bind_group, create_render_pipeline, GpuError, GridUniforms, SurfaceConfig,
};
use grid_renderer::{
    generate_instances, generate_row_instances, generate_test_pattern, row_byte_offset, GridCell,
    GridDimensions,
};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;

/// Describes a single pane to be rendered in a frame.
pub struct PaneRenderDescriptor {
    /// The pane's unique ID.
    pub pane_id: PaneId,
    /// The pane's pixel rect within the window.
    pub rect: PaneRect,
    /// The pane's terminal cells (row-major).
    pub cells: Vec<GridCell>,
    /// Optional cursor overlay instance for this pane.
    pub cursor_instance: Option<gpu::CellInstance>,
}

/// Top-level render coordinator.
/// Holds all GPU state, glyph atlas, grid, and render resources.
pub struct Renderer {
    #[allow(dead_code)] // Retained for iced Engine recreation on config change
    adapter: wgpu::Adapter,
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
    theme: TerminalTheme,
    damage_state: DamageState,
    pane_damage: PaneDamageMap,
    _bind_group_layout: wgpu::BindGroupLayout,
    _atlas_view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    /// Terminal content padding in physical pixels (top, bottom, left, right).
    padding: [f32; 4],
    /// DPI scale factor used for atlas creation.
    scale_factor: f32,
    /// Minimum uniform buffer offset alignment (from device limits).
    uniform_align: u64,
    /// iced UI layer for widget rendering (composited on top of custom pipeline).
    iced: iced_layer::IcedLayer,
}

impl Renderer {
    /// Initialize the renderer with a window, theme, and font configuration.
    /// Creates GPU context, glyph atlas, grid, and all render resources.
    pub async fn new(
        window: Arc<Window>,
        theme: TerminalTheme,
        font_size: f32,
        font_family: &str,
        line_height_multiplier: f32,
    ) -> Result<Self, GpuError> {
        let size = window.inner_size();
        let winit_scale = window.scale_factor();

        // On macOS, detect the actual display scale via CoreGraphics.
        // Bare binaries (not .app bundles) get scale_factor=1.0 from winit
        // even on Retina displays, causing fonts to render at half size.
        #[cfg(target_os = "macos")]
        let scale_factor = crate::platform::macos::detect_display_scale(winit_scale) as f32;
        #[cfg(not(target_os = "macos"))]
        let scale_factor = winit_scale as f32;

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
            .map_err(|_| GpuError::AdapterNotFound)?;

        let info = adapter.get_info();
        log::info!(
            "GPU adapter: {} ({:?}, {:?})",
            info.name,
            info.device_type,
            info.backend
        );

        // Request adapter's actual limits so the device supports the full
        // display resolution (downlevel_defaults caps at 2048 which is too
        // small for Retina displays).
        let adapter_limits = adapter.limits();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("VeloTerm Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: adapter_limits,
                    memory_hints: wgpu::MemoryHints::default(),
                    trace: wgpu::Trace::Off,
                    experimental_features: wgpu::ExperimentalFeatures::disabled(),
                },
            )
            .await
            .map_err(GpuError::DeviceCreationFailed)?;

        // Surface configuration
        let surface_caps = surface.get_capabilities(&adapter);
        log::info!("Available surface formats: {:?}", surface_caps.formats);
        let format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        log::info!("Selected surface format: {:?} (is_srgb={})", format, format.is_srgb());

        // Clamp to GPU's max texture dimension to prevent wgpu validation errors
        // (macOS retina windows can exceed 2048px)
        let max_dim = device.limits().max_texture_dimension_2d;
        let clamped_width = size.width.min(max_dim).max(1);
        let clamped_height = size.height.min(max_dim).max(1);

        let surface_config = SurfaceConfig::new(clamped_width, clamped_height, format);
        surface.configure(&device, &surface_config.to_wgpu_config());

        // Glyph atlas
        let atlas = GlyphAtlas::new(font_size, scale_factor, font_family, line_height_multiplier);
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
            atlas.bytes_per_pixel,
        );
        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = create_atlas_sampler(&device);

        // Grid dimensions (use clamped size to match surface)
        let grid =
            GridDimensions::new(clamped_width, clamped_height, atlas.cell_width, atlas.cell_height);
        log::info!("Grid: {}x{} cells", grid.columns, grid.rows);

        // Initial test pattern with configured theme
        let cells = generate_test_pattern(&grid, &theme);
        let instances = generate_instances(&grid, &cells, &atlas);
        let instance_count = instances.len() as u32;

        // Uniforms — allocate space for multiple panes using dynamic offsets
        let uniform_align = device.limits().min_uniform_buffer_offset_alignment as u64;
        let max_panes: u64 = 16;
        let uniform_buffer_size = max_panes * uniform_align.max(std::mem::size_of::<GridUniforms>() as u64);
        let atlas_rgba = if atlas.bytes_per_pixel == 4 { 1.0 } else { 0.0 };
        let uniforms = GridUniforms {
            cell_size: grid.cell_size_ndc(),
            grid_size: grid.grid_size(),
            atlas_size: [atlas.atlas_width as f32, atlas.atlas_height as f32],
            flags: [atlas_rgba, atlas.cursor_height_ratio],
        };
        let mut uniform_data = vec![0u8; uniform_buffer_size as usize];
        uniform_data[..std::mem::size_of::<GridUniforms>()].copy_from_slice(bytemuck::bytes_of(&uniforms));
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Uniforms"),
            contents: &uniform_data,
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

        // Damage tracking
        let damage_state = DamageState::new(grid.columns as usize);
        let pane_damage = PaneDamageMap::new();

        // iced UI layer (shares device/queue via Clone — wgpu 27 uses internal Arc)
        let iced = iced_layer::IcedLayer::new(
            &adapter,
            device.clone(),
            queue.clone(),
            format,
            clamped_width,
            clamped_height,
            scale_factor,
        );

        Ok(Self {
            adapter,
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
            damage_state,
            pane_damage,
            _bind_group_layout: bind_group_layout,
            _atlas_view: atlas_view,
            _sampler: sampler,
            padding: [0.0; 4],
            scale_factor,
            uniform_align,
            iced,
        })
    }

    /// Set terminal content padding in physical pixels [top, bottom, left, right].
    pub fn set_padding(&mut self, top: f32, bottom: f32, left: f32, right: f32) {
        self.padding = [top, bottom, left, right];
    }

    /// Get the current padding [top, bottom, left, right] in physical pixels.
    pub fn padding(&self) -> [f32; 4] {
        self.padding
    }

    /// Get the detected DPI scale factor (may differ from winit on macOS Retina).
    pub fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    /// Rebuild the glyph atlas with new font parameters and update all dependent GPU resources.
    /// Call after font size, family, or line_height changes.
    pub fn rebuild_atlas(&mut self, font_size: f32, font_family: &str, line_height_multiplier: f32) {
        let atlas = GlyphAtlas::new(font_size, self.scale_factor, font_family, line_height_multiplier);
        log::info!(
            "Atlas rebuilt: {}x{} (cell: {:.1}x{:.1})",
            atlas.atlas_width,
            atlas.atlas_height,
            atlas.cell_width,
            atlas.cell_height,
        );

        // Upload new atlas texture
        let atlas_texture = create_atlas_texture(
            &self.device,
            &self.queue,
            atlas.atlas_width,
            atlas.atlas_height,
            &atlas.atlas_data,
            atlas.bytes_per_pixel,
        );
        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = create_atlas_sampler(&self.device);

        // Recreate bind group with new atlas texture
        let bind_group = create_grid_bind_group(
            &self.device,
            &self._bind_group_layout,
            &self.uniform_buffer,
            &atlas_view,
            &sampler,
        );

        // Update grid dimensions
        let grid = GridDimensions::new(
            self.surface_config.width,
            self.surface_config.height,
            atlas.cell_width,
            atlas.cell_height,
        );

        // Update uniform buffer
        let atlas_rgba = if atlas.bytes_per_pixel == 4 { 1.0 } else { 0.0 };
        let uniforms = GridUniforms {
            cell_size: grid.cell_size_ndc(),
            grid_size: grid.grid_size(),
            atlas_size: [atlas.atlas_width as f32, atlas.atlas_height as f32],
            flags: [atlas_rgba, self.atlas.cursor_height_ratio],
        };
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        self.atlas = atlas;
        self.grid = grid;
        self.bind_group = bind_group;
        self._atlas_view = atlas_view;
        self._sampler = sampler;
        self.pane_damage.force_full_damage_all();
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
                        load: wgpu::LoadOp::Clear(clear_color(&self.theme)),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[0]);
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

        // Reset damage state for new column count and force full repaint
        self.damage_state.resize(self.grid.columns as usize);
        self.damage_state.force_full_damage();

        // Rebuild test pattern and instances
        let cells = generate_test_pattern(&self.grid, &self.theme);
        let instances = generate_instances(&self.grid, &cells, &self.atlas);
        self.instance_count = instances.len() as u32;

        // Update uniforms
        let atlas_rgba = if self.atlas.bytes_per_pixel == 4 { 1.0 } else { 0.0 };
        let uniforms = GridUniforms {
            cell_size: self.grid.cell_size_ndc(),
            grid_size: self.grid.grid_size(),
            atlas_size: [
                self.atlas.atlas_width as f32,
                self.atlas.atlas_height as f32,
            ],
            flags: [atlas_rgba, self.atlas.cursor_height_ratio],
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

        // Update iced viewport
        self.iced.resize(width, height, self.scale_factor);
    }

    /// Get a reference to the grid dimensions.
    pub fn grid(&self) -> &GridDimensions {
        &self.grid
    }

    /// Update the rendered cells from external terminal state.
    ///
    /// Uses damage tracking to only update GPU buffer rows that changed.
    /// Falls back to full buffer write on first frame, after resize, or
    /// when force_full_damage() was called.
    pub fn update_cells(&mut self, cells: &[GridCell]) {
        let dirty = self.damage_state.process_frame(cells);
        let cols = self.grid.columns as usize;

        let any_dirty = dirty.iter().any(|&d| d);
        if !any_dirty {
            return;
        }

        let all_dirty = dirty.iter().all(|&d| d);
        if all_dirty {
            // Full update: regenerate all instances and write entire buffer
            let instances = generate_instances(&self.grid, cells, &self.atlas);
            self.instance_count = instances.len() as u32;
            self.queue
                .write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances));
        } else {
            // Partial update: only write dirty rows
            for (row, &is_dirty) in dirty.iter().enumerate() {
                if is_dirty {
                    let row_instances =
                        generate_row_instances(&self.grid, cells, &self.atlas, row as u32);
                    let offset = row_byte_offset(row, cols);
                    self.queue.write_buffer(
                        &self.instance_buffer,
                        offset,
                        bytemuck::cast_slice(&row_instances),
                    );
                }
            }
        }
    }

    /// Force a full repaint on the next update_cells() call.
    /// Call this after theme changes, font size changes, or scroll position changes.
    pub fn force_full_damage(&mut self) {
        self.damage_state.force_full_damage();
    }

    /// Get the cell width in physical pixels.
    pub fn cell_width(&self) -> f32 {
        self.atlas.cell_width
    }

    /// Get the cell height in physical pixels.
    pub fn cell_height(&self) -> f32 {
        self.atlas.cell_height
    }

    /// Get a reference to the active theme.
    pub fn theme(&self) -> &TerminalTheme {
        &self.theme
    }

    /// Replace the active theme.
    pub fn set_theme(&mut self, theme: TerminalTheme) {
        self.theme = theme;
    }

    /// Get a mutable reference to the per-pane damage map.
    pub fn pane_damage_mut(&mut self) -> &mut PaneDamageMap {
        &mut self.pane_damage
    }

    /// Remove a pane's damage state when the pane is closed.
    pub fn remove_pane_damage(&mut self, pane_id: PaneId) {
        self.pane_damage.remove(pane_id);
    }

    /// Update the DPI scale factor (e.g. when window moves between displays).
    pub fn update_scale_factor(&mut self, scale: f32) {
        if (self.scale_factor - scale).abs() < 0.01 {
            return;
        }
        log::info!("Scale factor updated: {:.2} → {:.2}", self.scale_factor, scale);
        self.scale_factor = scale;
        self.iced.update_scale(scale);
    }

    /// Get a mutable reference to the iced UI layer for event routing.
    pub fn iced_layer_mut(&mut self) -> &mut iced_layer::IcedLayer {
        &mut self.iced
    }

    /// Render a frame with multiple panes, each getting its own scissor rect.
    ///
    /// For each pane: compute grid dimensions from its rect, generate instances,
    /// apply damage tracking, write per-pane uniforms, and draw with scissor clipping.
    pub fn render_panes(
        &mut self,
        panes: &mut [PaneRenderDescriptor],
        ui_state: &iced_layer::UiState,
    ) -> Result<(wgpu::SurfaceTexture, Vec<iced_layer::UiMessage>), wgpu::SurfaceError> {
        // Process damage and prepare per-pane instance data
        struct PaneDrawData {
            rect: PaneRect,
            grid: GridDimensions,
            instances: Vec<gpu::CellInstance>,
        }

        let mut draw_data: Vec<PaneDrawData> = Vec::with_capacity(panes.len());

        let [pad_top, pad_bottom, pad_left, pad_right] = self.padding;
        let _multi_pane = panes.len() > 1;
        let cw = self.cell_width();
        let ch = self.cell_height();

        for pane in panes.iter_mut() {
            // Compute content area: pane rect inset by padding
            let content_rect = PaneRect {
                x: pane.rect.x + pad_left,
                y: pane.rect.y + pad_top,
                width: (pane.rect.width - pad_left - pad_right).max(1.0),
                height: (pane.rect.height - pad_top - pad_bottom).max(1.0),
            };
            let pane_grid = GridDimensions::from_pane_rect(
                &content_rect,
                cw,
                ch,
            );
            let cols = pane_grid.columns as usize;

            // Diagnostic: log pane dimensions and cell count mismatches
            let expected = pane_grid.columns as usize * pane_grid.rows as usize;
            if pane.cells.len() != expected {
                log::warn!(
                    "CELL MISMATCH {:?}: cells={} expected={} (grid={}x{}, \
                     content=({:.1},{:.1}), cell=({:.2},{:.2}), scale={:.2})",
                    pane.pane_id,
                    pane.cells.len(), expected,
                    pane_grid.columns, pane_grid.rows,
                    content_rect.width, content_rect.height,
                    cw, ch, self.scale_factor,
                );
            }

            // Get or create damage state for this pane
            let damage_state = self.pane_damage.get_or_create(pane.pane_id, cols);

            // If pane grid changed size, resize damage
            if damage_state.cols != cols {
                damage_state.resize(cols);
            }

            let dirty = damage_state.process_frame(&pane.cells);
            let any_dirty = dirty.iter().any(|&d| d);

            // Always regenerate instances: the render pass clears the surface every
            // frame, so we can never rely on the previous frame's content surviving.
            // Damage tracking would cause flashing (clear wipes grid, iced body is
            // transparent → terminal content disappears on "clean" frames).
            let force_redraw = true;
            let mut instances = if any_dirty || force_redraw {
                generate_instances(&pane_grid, &pane.cells, &self.atlas)
            } else {
                Vec::new()
            };

            // Append cursor overlay instance
            if let Some(cursor_inst) = &pane.cursor_instance {
                instances.push(*cursor_inst);
            }

            draw_data.push(PaneDrawData {
                rect: pane.rect,
                grid: pane_grid,
                instances,
            });
        }

        // Compute total pane instances
        let total_pane_instances: usize = draw_data.iter().map(|d| d.instances.len()).sum();

        // Get surface texture (even if nothing to render, we need it for present)
        let output = self.surface.get_current_texture()?;

        // Skip custom pipeline rendering when nothing changed and no overlays need drawing,
        // but always clear the surface and run iced for UI chrome.
        if total_pane_instances == 0 {
            let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
            // Clear the surface so we don't flash undefined swapchain content
            let mut encoder = self.device.create_command_encoder(
                &wgpu::CommandEncoderDescriptor { label: Some("Clear Encoder") },
            );
            {
                let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Clear Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(clear_color(&self.theme)),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
            }
            self.queue.submit(Some(encoder.finish()));
            let iced_messages = self.iced.render(&view, ui_state);
            return Ok((output, iced_messages));
        }

        // Build a single combined instance buffer with all pane data
        let mut all_instances: Vec<gpu::CellInstance> =
            Vec::with_capacity(total_pane_instances);
        let mut pane_ranges: Vec<(PaneRect, GridDimensions, u32, u32)> = Vec::new();

        for data in &draw_data {
            let start = all_instances.len() as u32;
            if !data.instances.is_empty() {
                all_instances.extend_from_slice(&data.instances);
            }
            let count = if !data.instances.is_empty() {
                data.instances.len() as u32
            } else {
                0
            };
            pane_ranges.push((data.rect, data.grid.clone(), start, count));
        }

        // Upload instance buffer
        if !all_instances.is_empty() {
            let buffer_size =
                (all_instances.len() * std::mem::size_of::<gpu::CellInstance>()) as u64;
            let current_size = self.instance_buffer.size();

            if buffer_size > current_size {
                self.instance_buffer =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Cell Instances"),
                            contents: bytemuck::cast_slice(&all_instances),
                            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        });
            } else {
                self.queue.write_buffer(
                    &self.instance_buffer,
                    0,
                    bytemuck::cast_slice(&all_instances),
                );
            }
            self.instance_count = all_instances.len() as u32;
        }

        // Upload all pane uniforms to the dynamic uniform buffer BEFORE encoding
        // the render pass. Each pane's uniforms go at a different aligned offset.
        // This prevents the old bug where queue.write_buffer inside the render pass
        // loop would overwrite earlier panes' data (GPU only saw the last write).
        let align = self.uniform_align;
        let atlas_rgba = if self.atlas.bytes_per_pixel == 4 { 1.0 } else { 0.0 };
        for (i, (_rect, grid, _start, _count)) in pane_ranges.iter().enumerate() {
            let uniforms = GridUniforms {
                cell_size: grid.cell_size_ndc(),
                grid_size: grid.grid_size(),
                atlas_size: [
                    self.atlas.atlas_width as f32,
                    self.atlas.atlas_height as f32,
                ],
                flags: [atlas_rgba, self.atlas.cursor_height_ratio],
            };
            let offset = i as u64 * align;
            self.queue
                .write_buffer(&self.uniform_buffer, offset, bytemuck::bytes_of(&uniforms));
        }

        // Begin render pass (output already acquired above)
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Pane Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Multi-Pane Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color(&self.theme)),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.instance_buffer.slice(..));

            for (i, (rect, grid, start, count)) in pane_ranges.iter().enumerate() {
                if *count == 0 {
                    continue;
                }

                // Use dynamic offset to select this pane's uniforms
                let dynamic_offset = (i as u64 * align) as u32;
                render_pass.set_bind_group(0, &self.bind_group, &[dynamic_offset]);

                // Set viewport and scissor to the content area (inset by padding).
                // The viewport maps NDC (-1..1) to the content rect, so cell (0,0)
                // renders at the content origin, not the surface origin.
                //
                // CRITICAL: The viewport dimensions must match the grid's exact pixel
                // dimensions (columns * cell_width, rows * cell_height), NOT the raw
                // content_rect width/height. Otherwise NDC-to-pixel mapping is wrong
                // and text appears stretched (especially visible in split panes where
                // content_rect.width is fractional).
                let cx = rect.x + pad_left;
                let cy = rect.y + pad_top;
                let content_w = (rect.width - pad_left - pad_right).max(1.0);
                let content_h = (rect.height - pad_top - pad_bottom).max(1.0);
                // Use exact grid pixel dimensions for viewport to prevent stretching
                let grid_pixel_w = grid.columns as f32 * grid.cell_width;
                let grid_pixel_h = grid.rows as f32 * grid.cell_height;
                let vp_w = grid_pixel_w.min(content_w).max(1.0);
                let vp_h = grid_pixel_h.min(content_h).max(1.0);
                let sx = cx.max(0.0) as u32;
                let sy = cy.max(0.0) as u32;
                let sw = (vp_w as u32).min(self.surface_config.width.saturating_sub(sx));
                let sh = (vp_h as u32).min(self.surface_config.height.saturating_sub(sy));
                if sw > 0 && sh > 0 {
                    render_pass.set_viewport(cx, cy, vp_w, vp_h, 0.0, 1.0);
                    render_pass.set_scissor_rect(sx, sy, sw, sh);
                    render_pass.draw(0..6, *start..*start + *count);
                }
            }

        }

        self.queue.submit(std::iter::once(encoder.finish()));

        // Phase 4: iced UI layer — composites widget output on top of the custom pipeline.
        // iced's present() creates its own render pass and submits internally.
        let iced_messages = self.iced.render(&view, ui_state);

        // Don't present yet - let caller capture screenshot if needed, then present
        Ok((output, iced_messages))
    }

    /// Capture a screenshot of the surface texture to PNG.
    /// Call this AFTER render_panes() but BEFORE calling present() on the texture.
    pub fn capture_screenshot(
        &self,
        surface_texture: &wgpu::Texture,
        path: &std::path::Path,
    ) -> anyhow::Result<()> {
        use image::{ImageBuffer, Rgba};

        let width = self.surface_config.width;
        let height = self.surface_config.height;
        let bytes_per_pixel = 4u32;

        // Row padding must be aligned to 256 bytes
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let padded_bytes_per_row = (unpadded_bytes_per_row + 255) & !255;

        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Screenshot Buffer"),
            size: (padded_bytes_per_row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Screenshot Encoder"),
            });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: surface_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &output_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        // Map and read back
        let buffer_slice = output_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).ok();
        });
        self.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).ok();
        receiver.recv()??;

        let data = buffer_slice.get_mapped_range();

        // Remove row padding and handle BGRA -> RGBA conversion if needed
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for row in 0..height {
            let start = (row * padded_bytes_per_row) as usize;
            let end = start + (width * bytes_per_pixel) as usize;
            let row_data = &data[start..end];

            // Check if we need to swap B and R channels (Bgra8Unorm -> Rgba)
            if matches!(
                self.surface_config.format,
                wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
            ) {
                for chunk in row_data.chunks_exact(4) {
                    pixels.push(chunk[2]); // R (was B)
                    pixels.push(chunk[1]); // G
                    pixels.push(chunk[0]); // B (was R)
                    pixels.push(chunk[3]); // A
                }
            } else {
                pixels.extend_from_slice(row_data);
            }
        }

        drop(data);
        output_buffer.unmap();

        // Save as PNG
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_raw(width, height, pixels)
                .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer"))?;
        img.save(path)?;

        log::info!("✓ Screenshot saved: {}", path.display());
        Ok(())
    }

    /// Compute scissor rect parameters for a pane rect's content area within the surface.
    /// Returns (x, y, width, height) clamped to surface bounds, inset by padding.
    pub fn scissor_rect_for_pane(&self, rect: &PaneRect) -> (u32, u32, u32, u32) {
        let [pad_top, _pad_bottom, pad_left, pad_right] = self.padding;
        let sx = (rect.x + pad_left).max(0.0) as u32;
        let sy = (rect.y + pad_top).max(0.0) as u32;
        let content_w = (rect.width - pad_left - pad_right).max(0.0);
        let content_h = (rect.height - self.padding[0] - self.padding[1]).max(0.0);
        let sw = (content_w as u32).min(self.surface_config.width.saturating_sub(sx));
        let sh = (content_h as u32).min(self.surface_config.height.saturating_sub(sy));
        (sx, sy, sw, sh)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Renderer initialization (headless) ─────────────────────────

    #[test]
    fn renderer_components_initialize() {
        // Test that all component pieces work together without a window
        let atlas = GlyphAtlas::new(13.0, 2.0, "JetBrains Mono", 1.5);
        let grid = GridDimensions::new(1280, 720, atlas.cell_width, atlas.cell_height);
        let theme = TerminalTheme::warm_dark();
        let cells = generate_test_pattern(&grid, &theme);
        let instances = generate_instances(&grid, &cells, &atlas);

        assert!(grid.columns > 0);
        assert!(grid.rows > 0);
        assert_eq!(instances.len(), grid.total_cells() as usize);
    }

    #[test]
    fn renderer_uniforms_match_grid() {
        let atlas = GlyphAtlas::new(13.0, 2.0, "JetBrains Mono", 1.5);
        let grid = GridDimensions::new(1280, 720, atlas.cell_width, atlas.cell_height);

        let uniforms = GridUniforms {
            cell_size: grid.cell_size_ndc(),
            grid_size: grid.grid_size(),
            atlas_size: [atlas.atlas_width as f32, atlas.atlas_height as f32],
            flags: [0.0, 0.0],
        };

        assert_eq!(uniforms.grid_size[0], grid.columns as f32);
        assert_eq!(uniforms.grid_size[1], grid.rows as f32);
        assert_eq!(uniforms.atlas_size[0], atlas.atlas_width as f32);
    }

    #[test]
    fn renderer_resize_changes_grid() {
        let atlas = GlyphAtlas::new(13.0, 2.0, "JetBrains Mono", 1.5);
        let mut grid = GridDimensions::new(1280, 720, atlas.cell_width, atlas.cell_height);
        let original_cols = grid.columns;
        let original_rows = grid.rows;

        grid.resize(1920, 1080);
        assert!(grid.columns > original_cols);
        assert!(grid.rows > original_rows);
    }

    #[test]
    fn renderer_instances_have_correct_count_after_resize() {
        let atlas = GlyphAtlas::new(13.0, 2.0, "JetBrains Mono", 1.5);
        let mut grid = GridDimensions::new(1280, 720, atlas.cell_width, atlas.cell_height);
        let theme = TerminalTheme::warm_dark();

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

        let atlas = GlyphAtlas::new(13.0, 1.0, "JetBrains Mono", 1.5);
        let atlas_texture = create_atlas_texture(
            &ctx.device,
            &ctx.queue,
            atlas.atlas_width,
            atlas.atlas_height,
            &atlas.atlas_data,
            atlas.bytes_per_pixel,
        );
        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = create_atlas_sampler(&ctx.device);

        let grid = GridDimensions::new(640, 480, atlas.cell_width, atlas.cell_height);
        let theme = TerminalTheme::warm_dark();
        let cells = generate_test_pattern(&grid, &theme);
        let instances = generate_instances(&grid, &cells, &atlas);

        let uniforms = GridUniforms {
            cell_size: grid.cell_size_ndc(),
            grid_size: grid.grid_size(),
            atlas_size: [atlas.atlas_width as f32, atlas.atlas_height as f32],
            flags: [0.0, 0.0],
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
        let small = GlyphAtlas::new(10.0, 1.0, "JetBrains Mono", 1.5);
        let large = GlyphAtlas::new(20.0, 1.0, "JetBrains Mono", 1.5);
        // Larger font → larger cells
        assert!(large.cell_width > small.cell_width);
        assert!(large.cell_height > small.cell_height);
    }

    #[test]
    fn config_larger_font_produces_fewer_grid_cells() {
        // Same window size, different font sizes → different grid dimensions
        let small_atlas = GlyphAtlas::new(10.0, 1.0, "JetBrains Mono", 1.5);
        let large_atlas = GlyphAtlas::new(20.0, 1.0, "JetBrains Mono", 1.5);
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
        let atlas = GlyphAtlas::new(13.0, 2.0, "JetBrains Mono", 1.5);
        let grid = GridDimensions::new(1280, 720, atlas.cell_width, atlas.cell_height);

        for name in &["warm_dark", "midnight", "ember", "dusk", "light", "claude_dark", "claude_light", "claude_warm"] {
            let theme = TerminalTheme::from_name(name).unwrap();
            let cells = generate_test_pattern(&grid, &theme);
            let instances = generate_instances(&grid, &cells, &atlas);
            assert_eq!(instances.len(), grid.total_cells() as usize);
        }
    }

    #[test]
    fn config_theme_produces_different_background_colors() {
        let atlas = GlyphAtlas::new(13.0, 1.0, "JetBrains Mono", 1.5);
        let grid = GridDimensions::new(640, 480, atlas.cell_width, atlas.cell_height);

        let dark = TerminalTheme::from_name("warm_dark").unwrap();
        let light = TerminalTheme::from_name("light").unwrap();

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
    fn config_unknown_theme_falls_back_to_warm_dark() {
        let fallback = TerminalTheme::from_name("nonexistent");
        assert!(fallback.is_none());
        // Application code does: .unwrap_or_else(|| TerminalTheme::warm_dark())
        let theme = fallback.unwrap_or_else(TerminalTheme::warm_dark);
        assert_eq!(theme.name, "Warm Dark");
    }

    // ── Scissor rect / multi-pane tests ──────────────────────────────

    #[test]
    fn scissor_rect_single_pane_covers_full_surface() {
        // A single pane covering the entire window
        let rect = PaneRect::new(0.0, 0.0, 1280.0, 720.0);
        let sx = rect.x.max(0.0) as u32;
        let sy = rect.y.max(0.0) as u32;
        let sw = rect.width as u32;
        let sh = rect.height as u32;
        assert_eq!((sx, sy, sw, sh), (0, 0, 1280, 720));
    }

    #[test]
    fn two_pane_layout_produces_tiling_scissor_regions() {
        // Vertical split: left 640px, right 640px
        let left = PaneRect::new(0.0, 0.0, 640.0, 720.0);
        let right = PaneRect::new(640.0, 0.0, 640.0, 720.0);

        let l_sx = left.x as u32;
        let l_sy = left.y as u32;
        let l_sw = left.width as u32;
        let l_sh = left.height as u32;

        let r_sx = right.x as u32;
        let r_sy = right.y as u32;
        let r_sw = right.width as u32;
        let r_sh = right.height as u32;

        // Left pane
        assert_eq!((l_sx, l_sy, l_sw, l_sh), (0, 0, 640, 720));
        // Right pane
        assert_eq!((r_sx, r_sy, r_sw, r_sh), (640, 0, 640, 720));
        // Together they tile the window
        assert_eq!(l_sw + r_sw, 1280);
        assert_eq!(l_sh, r_sh);
    }

    #[test]
    fn scissor_rect_matches_pane_pixel_rect() {
        let rect = PaneRect::new(100.0, 200.0, 500.0, 300.0);
        let sx = rect.x as u32;
        let sy = rect.y as u32;
        let sw = rect.width as u32;
        let sh = rect.height as u32;
        assert_eq!(sx, 100);
        assert_eq!(sy, 200);
        assert_eq!(sw, 500);
        assert_eq!(sh, 300);
    }

    // ── Text overlay instance tests ─────────────────────────────────

    #[test]
    fn text_overlay_instances_generated_from_cells() {
        use crate::config::theme::color_new;

        let atlas = GlyphAtlas::new(13.0, 1.0, "JetBrains Mono", 1.5);
        let rect = PaneRect::new(100.0, 5.0, 200.0, atlas.cell_height);
        let grid = GridDimensions::from_pane_rect(&rect, atlas.cell_width, atlas.cell_height);

        let mut cells =
            vec![GridCell::empty(color_new(0.2, 0.2, 0.2, 1.0)); grid.total_cells() as usize];
        cells[0] = GridCell::new(
            'H',
            color_new(1.0, 1.0, 1.0, 1.0),
            color_new(0.2, 0.2, 0.2, 1.0),
        );
        cells[1] = GridCell::new(
            'i',
            color_new(1.0, 1.0, 1.0, 1.0),
            color_new(0.2, 0.2, 0.2, 1.0),
        );

        let instances = generate_instances(&grid, &cells, &atlas);
        assert_eq!(instances.len(), grid.total_cells() as usize);
        assert!(instances[0].flags & 1 == 1, "H should have glyph flag");
        assert!(instances[1].flags & 1 == 1, "i should have glyph flag");
    }

    #[test]
    fn text_overlay_grid_dimensions_match_rect() {
        let atlas = GlyphAtlas::new(13.0, 1.0, "JetBrains Mono", 1.5);
        let rect = PaneRect::new(50.0, 10.0, 300.0, atlas.cell_height);
        let grid = GridDimensions::from_pane_rect(&rect, atlas.cell_width, atlas.cell_height);
        let expected_cols = (300.0 / atlas.cell_width).floor() as u32;
        assert_eq!(grid.columns, expected_cols);
        assert_eq!(grid.rows, 1);
    }

    // ── Padding tests ───────────────────────────────────────────────

    #[test]
    fn padding_reduces_grid_columns_and_rows() {
        let atlas = GlyphAtlas::new(13.0, 1.0, "JetBrains Mono", 1.5);
        let rect_no_pad = PaneRect::new(0.0, 0.0, 800.0, 600.0);
        let grid_no_pad =
            GridDimensions::from_pane_rect(&rect_no_pad, atlas.cell_width, atlas.cell_height);

        // Simulate padding by shrinking the content rect
        let pad = 24.0; // 12px each side
        let rect_padded = PaneRect::new(12.0, 12.0, 800.0 - pad, 600.0 - pad);
        let grid_padded =
            GridDimensions::from_pane_rect(&rect_padded, atlas.cell_width, atlas.cell_height);

        assert!(
            grid_padded.columns < grid_no_pad.columns,
            "padding should reduce columns: {} vs {}",
            grid_padded.columns,
            grid_no_pad.columns,
        );
        assert!(
            grid_padded.rows < grid_no_pad.rows,
            "padding should reduce rows: {} vs {}",
            grid_padded.rows,
            grid_no_pad.rows,
        );
    }

    #[test]
    fn zero_padding_does_not_change_grid() {
        let atlas = GlyphAtlas::new(13.0, 1.0, "JetBrains Mono", 1.5);
        let rect = PaneRect::new(0.0, 0.0, 800.0, 600.0);
        let grid = GridDimensions::from_pane_rect(&rect, atlas.cell_width, atlas.cell_height);

        // With zero padding, content rect equals pane rect
        let content_rect = PaneRect::new(0.0, 0.0, 800.0, 600.0);
        let content_grid =
            GridDimensions::from_pane_rect(&content_rect, atlas.cell_width, atlas.cell_height);

        assert_eq!(grid.columns, content_grid.columns);
        assert_eq!(grid.rows, content_grid.rows);
    }

    #[test]
    fn large_padding_clamps_to_minimum_one_cell() {
        let atlas = GlyphAtlas::new(13.0, 1.0, "JetBrains Mono", 1.5);
        // Padding larger than pane: content area is 1x1
        let content_rect = PaneRect::new(0.0, 0.0, 1.0, 1.0);
        let grid =
            GridDimensions::from_pane_rect(&content_rect, atlas.cell_width, atlas.cell_height);
        assert_eq!(grid.columns, 1);
        assert_eq!(grid.rows, 1);
    }
}

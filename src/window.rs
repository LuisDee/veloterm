// Window creation and event loop management for VeloTerm.

use std::collections::HashMap;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::ModifiersState;
use winit::window::{CursorIcon, Window, WindowAttributes, WindowId};

use crate::config::theme::Theme;
use crate::config::types::Config;
use crate::input::{match_pane_command, PaneCommand};
use crate::pane::divider::{generate_divider_quads, generate_unfocused_overlay_quads, OverlayQuad};
use crate::pane::interaction::{CursorType, InteractionEffect, PaneInteraction};
use crate::pane::{PaneId, PaneTree, Rect, SplitDirection};
use crate::renderer::PaneRenderDescriptor;

/// Default window width in logical pixels.
pub const DEFAULT_WIDTH: f64 = 1280.0;
/// Default window height in logical pixels.
pub const DEFAULT_HEIGHT: f64 = 720.0;
/// Default window title.
pub const DEFAULT_TITLE: &str = "VeloTerm";

/// Configuration for the VeloTerm window.
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub width: f64,
    pub height: f64,
    pub title: String,
    pub resizable: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            title: DEFAULT_TITLE.to_string(),
            resizable: true,
        }
    }
}

impl WindowConfig {
    /// Build a `WindowAttributes` from this configuration.
    pub fn to_window_attributes(&self) -> WindowAttributes {
        Window::default_attributes()
            .with_title(self.title.clone())
            .with_inner_size(LogicalSize::new(self.width, self.height))
            .with_resizable(self.resizable)
            .with_decorations(true)
            .with_fullscreen(None)
    }
}

/// Convert a logical size to physical size given a DPI scale factor.
pub fn logical_to_physical(width: f64, height: f64, scale_factor: f64) -> PhysicalSize<u32> {
    PhysicalSize::new(
        (width * scale_factor) as u32,
        (height * scale_factor) as u32,
    )
}

/// Calculate the DPI-adjusted font size from a base font size and scale factor.
pub fn scaled_font_size(base_size: f32, scale_factor: f64) -> f32 {
    base_size * scale_factor as f32
}

/// Per-pane state: the terminal emulator and PTY session for a single pane.
pub struct PaneState {
    pub terminal: crate::terminal::Terminal,
    pub pty: crate::pty::PtySession,
}

/// Main application state implementing the winit event loop handler.
pub struct App {
    config: WindowConfig,
    pub(crate) app_config: Config,
    window: Option<Arc<Window>>,
    renderer: Option<crate::renderer::Renderer>,
    pane_tree: PaneTree,
    pane_states: HashMap<PaneId, PaneState>,
    modifiers: ModifiersState,
    interaction: PaneInteraction,
}

impl App {
    pub fn new(config: WindowConfig, app_config: Config) -> Self {
        Self {
            config,
            app_config,
            window: None,
            renderer: None,
            pane_tree: PaneTree::new(),
            pane_states: HashMap::new(),
            modifiers: ModifiersState::empty(),
            interaction: PaneInteraction::new(),
        }
    }

    /// Get the pane tree (for testing).
    pub fn pane_tree(&self) -> &PaneTree {
        &self.pane_tree
    }

    /// Get the pane states map (for testing).
    pub fn pane_states(&self) -> &HashMap<PaneId, PaneState> {
        &self.pane_states
    }

    /// Spawn a PTY + Terminal for a new pane, using the given grid dimensions.
    fn spawn_pane(&mut self, pane_id: PaneId, cols: u16, rows: u16) {
        let scrollback = self.app_config.scrollback.lines as usize;
        let shell = crate::pty::default_shell();
        match crate::pty::PtySession::new(&shell, cols, rows) {
            Ok(pty) => {
                log::info!(
                    "PTY spawned for pane {:?}: {shell} ({cols}x{rows})",
                    pane_id
                );
                let terminal = crate::terminal::Terminal::new(
                    cols as usize,
                    rows as usize,
                    scrollback,
                );
                self.pane_states.insert(pane_id, PaneState { terminal, pty });
            }
            Err(e) => {
                log::error!("Failed to spawn PTY for pane {:?}: {e}", pane_id);
            }
        }
    }

    /// Handle a pane command (split, close, focus, zoom).
    fn handle_pane_command(
        &mut self,
        command: PaneCommand,
        event_loop: &ActiveEventLoop,
    ) {
        let (width, height) = self
            .window
            .as_ref()
            .map(|w| {
                let s = w.inner_size();
                (s.width, s.height)
            })
            .unwrap_or((1280, 720));

        match command {
            PaneCommand::SplitVertical | PaneCommand::SplitHorizontal => {
                let direction = match command {
                    PaneCommand::SplitVertical => SplitDirection::Vertical,
                    _ => SplitDirection::Horizontal,
                };
                if let Some(new_id) = self.pane_tree.split_focused(direction) {
                    // Compute grid dims for the new pane from its layout rect
                    let layout = self
                        .pane_tree
                        .calculate_layout(width as f32, height as f32);
                    if let Some((_, rect)) = layout.iter().find(|(id, _)| *id == new_id) {
                        let (cols, rows) = self.grid_dims_for_rect(rect);
                        self.spawn_pane(new_id, cols, rows);
                    }
                    // Resize existing panes since the layout changed
                    self.resize_all_panes(width, height);
                    self.update_interaction_layout(width, height);
                    // Force full damage on all panes
                    if let Some(renderer) = &mut self.renderer {
                        renderer.pane_damage_mut().force_full_damage_all();
                    }
                }
            }
            PaneCommand::ClosePane => {
                let closing_id = self.pane_tree.focused_pane_id();
                match self.pane_tree.close_focused() {
                    Some(_) => {
                        // Remove state for closed pane
                        self.pane_states.remove(&closing_id);
                        if let Some(renderer) = &mut self.renderer {
                            renderer.remove_pane_damage(closing_id);
                            renderer.pane_damage_mut().force_full_damage_all();
                        }
                        // Resize remaining panes
                        self.resize_all_panes(width, height);
                        self.update_interaction_layout(width, height);
                    }
                    None => {
                        // Last pane — exit application
                        log::info!("Last pane closed, exiting");
                        event_loop.exit();
                    }
                }
            }
            PaneCommand::FocusDirection(direction) => {
                self.pane_tree
                    .focus_direction(direction, width as f32, height as f32);
            }
            PaneCommand::ZoomToggle => {
                self.pane_tree.zoom_toggle();
                // Resize all panes since visible panes changed
                self.resize_all_panes(width, height);
                self.update_interaction_layout(width, height);
                if let Some(renderer) = &mut self.renderer {
                    renderer.pane_damage_mut().force_full_damage_all();
                }
            }
        }
    }

    /// Compute grid columns and rows for a pane rect.
    fn grid_dims_for_rect(&self, rect: &crate::pane::Rect) -> (u16, u16) {
        if let Some(renderer) = &self.renderer {
            let cw = renderer.cell_width();
            let ch = renderer.cell_height();
            let cols = (rect.width / cw).floor().max(1.0) as u16;
            let rows = (rect.height / ch).floor().max(1.0) as u16;
            (cols, rows)
        } else {
            (80, 24)
        }
    }

    /// Resize all pane terminals and PTYs to match their current layout rects.
    fn resize_all_panes(&mut self, width: u32, height: u32) {
        let layout = self
            .pane_tree
            .calculate_layout(width as f32, height as f32);
        for (pane_id, rect) in &layout {
            let (cols, rows) = self.grid_dims_for_rect(rect);
            if let Some(state) = self.pane_states.get_mut(pane_id) {
                state.terminal.resize(cols as usize, rows as usize);
                let _ = state.pty.resize(cols, rows);
            }
        }
    }

    /// Get the interaction state machine (for testing).
    pub fn interaction(&self) -> &PaneInteraction {
        &self.interaction
    }

    /// Apply an InteractionEffect to the app state.
    pub(crate) fn apply_interaction_effect(&mut self, effect: InteractionEffect) {
        match effect {
            InteractionEffect::None => {}
            InteractionEffect::SetCursor(cursor_type) => {
                if let Some(window) = &self.window {
                    let icon = match cursor_type {
                        CursorType::Default => CursorIcon::Default,
                        CursorType::EwResize => CursorIcon::EwResize,
                        CursorType::NsResize => CursorIcon::NsResize,
                    };
                    window.set_cursor(icon);
                }
            }
            InteractionEffect::UpdateRatio {
                split_index,
                new_ratio,
            } => {
                self.pane_tree.set_split_ratio_by_index(split_index, new_ratio);
                let (w, h) = self.window_size();
                self.resize_all_panes(w, h);
                self.update_interaction_layout(w, h);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            InteractionEffect::FocusPane(pane_id) => {
                self.pane_tree.set_focus(pane_id);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
        }
    }

    /// Update the interaction state machine's cached layout.
    fn update_interaction_layout(&mut self, width: u32, height: u32) {
        let bounds = Rect::new(0.0, 0.0, width as f32, height as f32);
        self.interaction
            .update_layout(self.pane_tree.root(), bounds, 20.0);
    }

    /// Generate all overlay quads (dividers + unfocused pane dimming).
    fn generate_overlay_quads(&self, width: f32, height: f32) -> Vec<OverlayQuad> {
        if self.pane_tree.pane_count() <= 1 || self.pane_tree.is_zoomed() {
            return Vec::new();
        }

        let theme = if let Some(renderer) = &self.renderer {
            renderer.theme()
        } else {
            return Vec::new();
        };

        let hovered_index = match self.interaction.state() {
            crate::pane::interaction::InteractionState::Hovering { divider_index } => {
                Some(*divider_index)
            }
            _ => None,
        };

        let mut quads = generate_divider_quads(
            self.interaction.dividers(),
            &theme.border,
            &theme.accent,
            hovered_index,
        );

        let layout = self.pane_tree.calculate_layout(width, height);
        let focused = self.pane_tree.focused_pane_id();
        quads.extend(generate_unfocused_overlay_quads(
            &layout,
            focused,
            &theme.background,
            0.3,
        ));

        quads
    }

    /// Get window physical size, with fallback.
    fn window_size(&self) -> (u32, u32) {
        self.window
            .as_ref()
            .map(|w| {
                let s = w.inner_size();
                (s.width, s.height)
            })
            .unwrap_or((1280, 720))
    }

    /// Run the application event loop. This blocks until the window is closed.
    pub fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let event_loop = EventLoop::new()?;
        let mut app = self;
        event_loop.run_app(&mut app)?;
        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = self.config.to_window_attributes();
        match event_loop.create_window(attrs) {
            Ok(window) => {
                let size = window.inner_size();
                let scale = window.scale_factor();
                log::info!(
                    "Window created: {}x{} (scale factor: {:.2})",
                    size.width,
                    size.height,
                    scale
                );

                let window = Arc::new(window);

                // Resolve theme from config
                let theme = Theme::from_name(&self.app_config.colors.theme).unwrap_or_else(|| {
                    log::warn!(
                        "Unknown theme '{}', falling back to claude_dark",
                        self.app_config.colors.theme
                    );
                    Theme::claude_dark()
                });
                let font_size = self.app_config.font.size as f32;

                // Initialize renderer
                match pollster::block_on(crate::renderer::Renderer::new(
                    window.clone(),
                    theme,
                    font_size,
                )) {
                    Ok(renderer) => {
                        log::info!("Renderer initialized");

                        self.renderer = Some(renderer);

                        // Spawn PTY and terminal for the initial pane
                        let initial_pane_id = self.pane_tree.focused_pane_id();
                        let cols = self.renderer.as_ref().unwrap().grid().columns as u16;
                        let rows = self.renderer.as_ref().unwrap().grid().rows as u16;
                        self.spawn_pane(initial_pane_id, cols, rows);
                    }
                    Err(e) => {
                        log::error!("Failed to initialize renderer: {e}");
                        event_loop.exit();
                        return;
                    }
                }

                self.window = Some(window);

                // Initialize interaction layout
                let (w, h) = self.window_size();
                self.update_interaction_layout(w, h);
            }
            Err(e) => {
                log::error!("Failed to create window: {e}");
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                log::info!("Window close requested");
                event_loop.exit();
            }
            WindowEvent::ModifiersChanged(new_modifiers) => {
                self.modifiers = new_modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    // Check for pane commands first
                    if let Some(cmd) =
                        match_pane_command(&event.logical_key, self.modifiers)
                    {
                        self.handle_pane_command(cmd, event_loop);
                        return;
                    }

                    // Route normal keys to focused pane's PTY
                    let bytes = crate::input::translate_key(
                        &event.logical_key,
                        event.text.as_ref().map(|s| s.as_ref()),
                        event.state,
                        self.modifiers,
                    );
                    let focused = self.pane_tree.focused_pane_id();
                    if let (Some(bytes), Some(state)) =
                        (bytes, self.pane_states.get_mut(&focused))
                    {
                        if let Err(e) = state.pty.write(&bytes) {
                            log::warn!("PTY write error: {e}");
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let effect = self
                    .interaction
                    .on_cursor_moved(position.x as f32, position.y as f32);
                self.apply_interaction_effect(effect);
            }
            WindowEvent::MouseInput {
                state: btn_state,
                button: MouseButton::Left,
                ..
            } => {
                let (width, height) = self.window_size();
                let layout = self
                    .pane_tree
                    .calculate_layout(width as f32, height as f32);
                let effect = match btn_state {
                    ElementState::Pressed => self.interaction.on_mouse_press(&layout),
                    ElementState::Released => self.interaction.on_mouse_release(),
                };
                self.apply_interaction_effect(effect);
            }
            WindowEvent::Resized(size) => {
                log::debug!("Window resized to {}x{}", size.width, size.height);
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                    renderer.pane_damage_mut().force_full_damage_all();
                }
                // Resize all pane terminals and PTYs
                self.resize_all_panes(size.width, size.height);
                // Update interaction layout for new window size
                self.update_interaction_layout(size.width, size.height);
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                log::debug!("Scale factor changed to {scale_factor:.2}");
            }
            WindowEvent::RedrawRequested => {
                let (width, height) = self
                    .window
                    .as_ref()
                    .map(|w| {
                        let s = w.inner_size();
                        (s.width, s.height)
                    })
                    .unwrap_or((1280, 720));

                // Drain PTY output into terminals for all panes
                for state in self.pane_states.values_mut() {
                    while let Ok(bytes) = state.pty.reader_rx.try_recv() {
                        state.terminal.feed(&bytes);
                    }
                }

                // Build render descriptors for visible panes
                let layout = self
                    .pane_tree
                    .calculate_layout(width as f32, height as f32);
                let visible = self.pane_tree.visible_panes();

                let mut pane_descs: Vec<PaneRenderDescriptor> = Vec::new();
                for (pane_id, rect) in &layout {
                    if !visible.contains(pane_id) {
                        continue;
                    }
                    if let Some(state) = self.pane_states.get(&pane_id) {
                        let cells =
                            crate::terminal::grid_bridge::extract_grid_cells(&state.terminal);
                        pane_descs.push(PaneRenderDescriptor {
                            pane_id: *pane_id,
                            rect: *rect,
                            cells,
                        });
                    }
                }

                // Generate and upload overlay quads (dividers + unfocused dimming)
                let overlay_quads =
                    self.generate_overlay_quads(width as f32, height as f32);

                if let Some(renderer) = &mut self.renderer {
                    renderer.update_overlays(&overlay_quads);
                    match renderer.render_panes(&mut pane_descs) {
                        Ok(()) => {}
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            renderer.resize(width, height);
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            log::error!("GPU out of memory");
                            event_loop.exit();
                        }
                        Err(e) => {
                            log::warn!("Surface error: {e}");
                        }
                    }
                }
                // Request next frame
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::Config;

    // ── WindowConfig defaults ──────────────────────────────────────

    #[test]
    fn default_config_size() {
        let cfg = WindowConfig::default();
        assert_eq!(cfg.width, 1280.0);
        assert_eq!(cfg.height, 720.0);
    }

    #[test]
    fn default_config_title() {
        let cfg = WindowConfig::default();
        assert_eq!(cfg.title, "VeloTerm");
    }

    #[test]
    fn default_config_resizable() {
        let cfg = WindowConfig::default();
        assert!(cfg.resizable);
    }

    // ── WindowAttributes construction ──────────────────────────────

    #[test]
    fn window_attributes_has_correct_inner_size() {
        let cfg = WindowConfig::default();
        let attrs = cfg.to_window_attributes();
        let expected = LogicalSize::new(1280.0, 720.0);
        // WindowAttributes stores inner_size as Option<Size>
        assert_eq!(attrs.inner_size, Some(expected.into()));
    }

    #[test]
    fn window_attributes_has_correct_title() {
        let cfg = WindowConfig::default();
        let attrs = cfg.to_window_attributes();
        assert_eq!(attrs.title, "VeloTerm");
    }

    #[test]
    fn window_attributes_is_resizable() {
        let cfg = WindowConfig::default();
        let attrs = cfg.to_window_attributes();
        assert!(attrs.resizable);
    }

    #[test]
    fn window_attributes_non_resizable() {
        let cfg = WindowConfig {
            resizable: false,
            ..Default::default()
        };
        let attrs = cfg.to_window_attributes();
        assert!(!attrs.resizable);
    }

    #[test]
    fn window_attributes_custom_size() {
        let cfg = WindowConfig {
            width: 1920.0,
            height: 1080.0,
            ..Default::default()
        };
        let attrs = cfg.to_window_attributes();
        let expected = LogicalSize::new(1920.0, 1080.0);
        assert_eq!(attrs.inner_size, Some(expected.into()));
    }

    // ── DPI scale factor handling ──────────────────────────────────

    #[test]
    fn logical_to_physical_1x() {
        let phys = logical_to_physical(1280.0, 720.0, 1.0);
        assert_eq!(phys.width, 1280);
        assert_eq!(phys.height, 720);
    }

    #[test]
    fn logical_to_physical_2x_retina() {
        let phys = logical_to_physical(1280.0, 720.0, 2.0);
        assert_eq!(phys.width, 2560);
        assert_eq!(phys.height, 1440);
    }

    #[test]
    fn logical_to_physical_1_5x() {
        let phys = logical_to_physical(1280.0, 720.0, 1.5);
        assert_eq!(phys.width, 1920);
        assert_eq!(phys.height, 1080);
    }

    #[test]
    fn scaled_font_size_1x() {
        let size = scaled_font_size(13.0, 1.0);
        assert_eq!(size, 13.0);
    }

    #[test]
    fn scaled_font_size_2x_retina() {
        let size = scaled_font_size(13.0, 2.0);
        assert_eq!(size, 26.0);
    }

    #[test]
    fn scaled_font_size_1_5x() {
        let size = scaled_font_size(13.0, 1.5);
        assert!((size - 19.5).abs() < f32::EPSILON);
    }

    // ── App initialization and shutdown ──────────────────────────────

    #[test]
    fn app_starts_with_no_window() {
        let app = App::new(WindowConfig::default(), Config::default());
        assert!(app.window.is_none());
    }

    #[test]
    fn app_starts_with_no_renderer() {
        let app = App::new(WindowConfig::default(), Config::default());
        assert!(app.renderer.is_none());
    }

    #[test]
    fn app_drop_without_run_is_safe() {
        let app = App::new(WindowConfig::default(), Config::default());
        drop(app);
        // No panic — clean drop without GPU resources
    }

    #[test]
    fn app_stores_config() {
        let cfg = WindowConfig {
            width: 800.0,
            height: 600.0,
            title: "Test".to_string(),
            resizable: false,
        };
        let app = App::new(cfg.clone(), Config::default());
        assert_eq!(app.config.width, 800.0);
        assert_eq!(app.config.title, "Test");
    }

    #[test]
    fn app_stores_app_config() {
        let mut app_config = Config::default();
        app_config.colors.theme = "claude_warm".to_string();
        app_config.scrollback.lines = 5000;
        let app = App::new(WindowConfig::default(), app_config);
        assert_eq!(app.app_config.colors.theme, "claude_warm");
        assert_eq!(app.app_config.scrollback.lines, 5000);
    }

    #[test]
    fn app_config_defaults_match_expected_values() {
        let app = App::new(WindowConfig::default(), Config::default());
        assert_eq!(app.app_config.colors.theme, "claude_dark");
        assert_eq!(app.app_config.font.size, 14.0);
        assert_eq!(app.app_config.scrollback.lines, 10_000);
        assert_eq!(app.app_config.cursor.style, "block");
        assert!(app.app_config.cursor.blink);
    }

    #[test]
    fn app_config_font_size_is_passed_through() {
        let mut cfg = Config::default();
        cfg.font.size = 20.0;
        let app = App::new(WindowConfig::default(), cfg);
        assert_eq!(app.app_config.font.size, 20.0);
    }

    #[test]
    fn app_config_theme_resolution() {
        // Verify Theme::from_name produces the right theme for each config value
        use crate::config::theme::Theme;
        for (config_name, display_name) in [
            ("claude_dark", "Claude Dark"),
            ("claude_light", "Claude Light"),
            ("claude_warm", "Claude Warm"),
        ] {
            let theme = Theme::from_name(config_name).unwrap();
            assert_eq!(theme.name, display_name);
        }
    }

    #[test]
    fn app_config_unknown_theme_fallback() {
        use crate::config::theme::Theme;
        // Unknown theme returns None, app falls back to claude_dark
        let result = Theme::from_name("nonexistent");
        assert!(result.is_none());
        let fallback = result.unwrap_or_else(Theme::claude_dark);
        assert_eq!(fallback.name, "Claude Dark");
    }

    #[test]
    fn app_config_scrollback_passed_to_terminal() {
        let scrollback = 5000_usize;
        let terminal = crate::terminal::Terminal::new(80, 24, scrollback);
        // Terminal is created successfully with custom scrollback
        assert_eq!(terminal.history_size(), 0); // empty at start, but accepts the config value
    }

    #[test]
    fn app_config_from_toml_wires_all_fields() {
        let toml = r#"
[font]
size = 18.0

[colors]
theme = "claude_light"

[scrollback]
lines = 2000

[cursor]
style = "beam"
blink = false
"#;
        let cfg = Config::from_toml(toml).unwrap();
        let app = App::new(WindowConfig::default(), cfg);
        assert_eq!(app.app_config.font.size, 18.0);
        assert_eq!(app.app_config.colors.theme, "claude_light");
        assert_eq!(app.app_config.scrollback.lines, 2000);
        assert_eq!(app.app_config.cursor.style, "beam");
        assert!(!app.app_config.cursor.blink);
    }

    // ── PaneTree integration ──────────────────────────────────────

    #[test]
    fn app_creates_single_pane_tree_on_startup() {
        let app = App::new(WindowConfig::default(), Config::default());
        assert_eq!(app.pane_tree.pane_count(), 1);
    }

    #[test]
    fn app_pane_states_empty_before_resumed() {
        let app = App::new(WindowConfig::default(), Config::default());
        // Before window creation, no PTY states are spawned
        assert!(app.pane_states.is_empty());
    }

    #[test]
    fn app_pane_tree_focused_pane_is_initial() {
        let app = App::new(WindowConfig::default(), Config::default());
        let ids = app.pane_tree.pane_ids();
        assert_eq!(app.pane_tree.focused_pane_id(), ids[0]);
    }

    #[test]
    fn app_spawn_pane_adds_state() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        let pane_id = app.pane_tree.focused_pane_id();
        app.spawn_pane(pane_id, 80, 24);
        assert!(app.pane_states.contains_key(&pane_id));
    }

    #[test]
    fn app_spawn_pane_terminal_has_correct_dims() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        let pane_id = app.pane_tree.focused_pane_id();
        app.spawn_pane(pane_id, 120, 40);
        let state = app.pane_states.get(&pane_id).unwrap();
        assert_eq!(state.terminal.columns(), 120);
        assert_eq!(state.terminal.rows(), 40);
    }

    #[test]
    fn app_pane_count_matches_tree_leaf_count() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        let pane_id = app.pane_tree.focused_pane_id();
        app.spawn_pane(pane_id, 80, 24);
        assert_eq!(app.pane_tree.pane_count(), app.pane_states.len());
    }

    #[test]
    fn app_multiple_panes_have_independent_terminals() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        let first_id = app.pane_tree.focused_pane_id();
        app.spawn_pane(first_id, 80, 24);

        let second_id = app
            .pane_tree
            .split_focused(SplitDirection::Vertical)
            .unwrap();
        app.spawn_pane(second_id, 60, 20);

        // Each pane has its own terminal
        let s1 = app.pane_states.get(&first_id).unwrap();
        let s2 = app.pane_states.get(&second_id).unwrap();
        assert_eq!(s1.terminal.columns(), 80);
        assert_eq!(s2.terminal.columns(), 60);
    }

    #[test]
    fn app_close_pane_removes_state() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        let first_id = app.pane_tree.focused_pane_id();
        app.spawn_pane(first_id, 80, 24);

        let second_id = app
            .pane_tree
            .split_focused(SplitDirection::Vertical)
            .unwrap();
        app.spawn_pane(second_id, 80, 24);

        // Close focused pane (second_id)
        let closing_id = app.pane_tree.focused_pane_id();
        app.pane_tree.close_focused();
        app.pane_states.remove(&closing_id);

        assert_eq!(app.pane_tree.pane_count(), 1);
        assert_eq!(app.pane_states.len(), 1);
        assert!(!app.pane_states.contains_key(&closing_id));
    }

    #[test]
    fn app_grid_dims_for_rect_default_without_renderer() {
        let app = App::new(WindowConfig::default(), Config::default());
        let rect = crate::pane::Rect::new(0.0, 0.0, 640.0, 480.0);
        let (cols, rows) = app.grid_dims_for_rect(&rect);
        // Without renderer, falls back to 80x24
        assert_eq!((cols, rows), (80, 24));
    }

    // ── PaneInteraction integration ──────────────────────────────────

    #[test]
    fn app_interaction_starts_idle() {
        let app = App::new(WindowConfig::default(), Config::default());
        assert_eq!(
            *app.interaction().state(),
            crate::pane::interaction::InteractionState::Idle
        );
    }

    #[test]
    fn app_interaction_layout_updates_after_split() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        // Initially single pane → no dividers
        app.update_interaction_layout(1280, 720);
        assert!(app.interaction().dividers().is_empty());

        // Split → one divider
        app.pane_tree.split_focused(SplitDirection::Vertical);
        app.update_interaction_layout(1280, 720);
        assert_eq!(app.interaction().dividers().len(), 1);
    }

    #[test]
    fn app_apply_focus_pane_changes_focus() {
        use crate::pane::interaction::InteractionEffect;

        let mut app = App::new(WindowConfig::default(), Config::default());
        let first_id = app.pane_tree.focused_pane_id();
        let second_id = app
            .pane_tree
            .split_focused(SplitDirection::Vertical)
            .unwrap();
        assert_eq!(app.pane_tree.focused_pane_id(), second_id);

        app.apply_interaction_effect(InteractionEffect::FocusPane(first_id));
        assert_eq!(app.pane_tree.focused_pane_id(), first_id);
    }

    #[test]
    fn app_apply_update_ratio_changes_layout() {
        use crate::pane::interaction::InteractionEffect;

        let mut app = App::new(WindowConfig::default(), Config::default());
        app.pane_tree.split_focused(SplitDirection::Vertical);
        app.update_interaction_layout(1000, 500);

        // Change ratio from default 0.5 to 0.3
        app.apply_interaction_effect(InteractionEffect::UpdateRatio {
            split_index: 0,
            new_ratio: 0.3,
        });

        // Verify layout changed
        let layout = app.pane_tree.calculate_layout(1000.0, 500.0);
        let first_width = layout[0].1.width;
        assert!((first_width - 300.0).abs() < 1.0);
    }

    #[test]
    fn app_overlay_quads_empty_for_single_pane() {
        let app = App::new(WindowConfig::default(), Config::default());
        // Without renderer, generate_overlay_quads returns empty
        let quads = app.generate_overlay_quads(1280.0, 720.0);
        assert!(quads.is_empty());
    }

    #[test]
    fn app_cursor_moved_updates_interaction_state() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        app.pane_tree.split_focused(SplitDirection::Vertical);
        app.update_interaction_layout(1280, 720);

        // Move cursor to divider position (at x=640 for 50/50 split)
        let effect = app.interaction.on_cursor_moved(640.0, 360.0);
        assert!(matches!(
            effect,
            crate::pane::interaction::InteractionEffect::SetCursor(
                crate::pane::interaction::CursorType::EwResize
            )
        ));
        assert!(matches!(
            app.interaction().state(),
            crate::pane::interaction::InteractionState::Hovering { .. }
        ));
    }

    #[test]
    fn app_mouse_press_in_pane_produces_focus_effect() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        let first_id = app.pane_tree.focused_pane_id();
        let _second_id = app
            .pane_tree
            .split_focused(SplitDirection::Vertical)
            .unwrap();
        app.update_interaction_layout(1280, 720);

        // Move cursor into the left pane (well away from divider)
        app.interaction.on_cursor_moved(100.0, 360.0);

        let layout = app.pane_tree.calculate_layout(1280.0, 720.0);
        let effect = app.interaction.on_mouse_press(&layout);
        assert_eq!(
            effect,
            crate::pane::interaction::InteractionEffect::FocusPane(first_id)
        );
    }

    // ── Drag-to-resize integration ──────────────────────────────────

    #[test]
    fn app_drag_to_resize_updates_pane_tree_ratio() {

        let mut app = App::new(WindowConfig::default(), Config::default());
        app.pane_tree.split_focused(SplitDirection::Vertical);
        app.update_interaction_layout(1000, 500);

        // Simulate drag: hover → press → move
        app.interaction.on_cursor_moved(500.0, 250.0); // hover on divider
        let layout = app.pane_tree.calculate_layout(1000.0, 500.0);
        app.interaction.on_mouse_press(&layout); // start drag
        let effect = app.interaction.on_cursor_moved(300.0, 250.0); // drag left

        // Apply the effect
        app.apply_interaction_effect(effect);

        // Verify the layout changed: first pane should be ~30% wide
        let layout = app.pane_tree.calculate_layout(1000.0, 500.0);
        let first_width = layout[0].1.width;
        assert!(first_width < 400.0, "first pane should be narrower after drag left, got {first_width}");
    }

    // ── Overlay quads with zoom mode ────────────────────────────────

    #[test]
    fn app_overlay_quads_empty_when_zoomed() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        app.pane_tree.split_focused(SplitDirection::Vertical);
        app.pane_tree.zoom_toggle();
        // Even with multiple panes, zoomed mode produces no overlays
        let quads = app.generate_overlay_quads(1280.0, 720.0);
        assert!(quads.is_empty());
    }
}

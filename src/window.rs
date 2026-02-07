// Window creation and event loop management for VeloTerm.

use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::ModifiersState;
use winit::window::{Window, WindowAttributes, WindowId};

use crate::config::theme::Theme;
use crate::config::types::Config;

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

/// Main application state implementing the winit event loop handler.
pub struct App {
    config: WindowConfig,
    app_config: Config,
    window: Option<Arc<Window>>,
    renderer: Option<crate::renderer::Renderer>,
    pty: Option<crate::pty::PtySession>,
    terminal: Option<crate::terminal::Terminal>,
    modifiers: ModifiersState,
}

impl App {
    pub fn new(config: WindowConfig, app_config: Config) -> Self {
        Self {
            config,
            app_config,
            window: None,
            renderer: None,
            pty: None,
            terminal: None,
            modifiers: ModifiersState::empty(),
        }
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

                        // Spawn PTY and terminal with grid dimensions
                        let cols = renderer.grid().columns as u16;
                        let rows = renderer.grid().rows as u16;
                        let scrollback = self.app_config.scrollback.lines as usize;
                        let shell = crate::pty::default_shell();
                        match crate::pty::PtySession::new(&shell, cols, rows) {
                            Ok(pty) => {
                                log::info!("PTY spawned: {shell} ({cols}x{rows})");
                                let terminal = crate::terminal::Terminal::new(
                                    cols as usize,
                                    rows as usize,
                                    scrollback,
                                );
                                self.pty = Some(pty);
                                self.terminal = Some(terminal);
                            }
                            Err(e) => {
                                log::error!("Failed to spawn PTY: {e}");
                            }
                        }

                        self.renderer = Some(renderer);
                    }
                    Err(e) => {
                        log::error!("Failed to initialize renderer: {e}");
                        event_loop.exit();
                        return;
                    }
                }

                self.window = Some(window);
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
                    let bytes = crate::input::translate_key(
                        &event.logical_key,
                        event.text.as_ref().map(|s| s.as_ref()),
                        event.state,
                        self.modifiers,
                    );
                    if let (Some(bytes), Some(pty)) = (bytes, self.pty.as_mut()) {
                        if let Err(e) = pty.write(&bytes) {
                            log::warn!("PTY write error: {e}");
                        }
                    }
                }
            }
            WindowEvent::Resized(size) => {
                log::debug!("Window resized to {}x{}", size.width, size.height);
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);

                    // Update PTY and terminal dimensions
                    let cols = renderer.grid().columns as u16;
                    let rows = renderer.grid().rows as u16;
                    if let Some(pty) = &self.pty {
                        let _ = pty.resize(cols, rows);
                    }
                    if let Some(terminal) = &mut self.terminal {
                        terminal.resize(cols as usize, rows as usize);
                    }
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                log::debug!("Scale factor changed to {scale_factor:.2}");
            }
            WindowEvent::RedrawRequested => {
                // Drain PTY output into terminal
                if let (Some(pty), Some(terminal)) = (&self.pty, &mut self.terminal) {
                    while let Ok(bytes) = pty.reader_rx.try_recv() {
                        terminal.feed(&bytes);
                    }
                }

                // Update renderer with live terminal cells
                if let (Some(renderer), Some(terminal)) = (&mut self.renderer, &self.terminal) {
                    let cells = crate::terminal::grid_bridge::extract_grid_cells(terminal);
                    renderer.update_cells(&cells);
                }

                if let Some(renderer) = &self.renderer {
                    match renderer.render_frame() {
                        Ok(()) => {}
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            // Reconfigure surface
                            if let Some(r) = &mut self.renderer {
                                let size = self
                                    .window
                                    .as_ref()
                                    .map(|w| w.inner_size())
                                    .unwrap_or_default();
                                r.resize(size.width, size.height);
                            }
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
}

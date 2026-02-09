// Window creation and event loop management for VeloTerm.

use std::collections::HashMap;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{CursorIcon, Window, WindowAttributes, WindowId};

use crate::config::theme::Theme;
use crate::config::types::{Config, ConfigDelta};
use crate::config::watcher::UserEvent;
use crate::header_bar::HEADER_BAR_HEIGHT;
use crate::input::{
    match_app_command, match_pane_command, match_tab_command, match_search_command,
    should_open_search, AppCommand, InputMode, PaneCommand, SearchCommand, TabCommand,
};
use crate::link::opener::open_link;
use crate::link::LinkDetector;
use crate::search::SearchState;
use crate::pane::header::PANE_HEADER_HEIGHT;
use crate::pane::interaction::{CursorType, InteractionEffect, PaneInteraction};
use crate::pane::{PaneId, Rect, SplitDirection};
use crate::renderer::iced_layer::{DividerDisplay, PaneInfo, TabInfo, UiMessage, UiState};
use crate::renderer::PaneRenderDescriptor;
use crate::status_bar::STATUS_BAR_HEIGHT;
use crate::tab::bar::TAB_BAR_HEIGHT;
use crate::tab::TabManager;

/// Default window width in logical pixels.
pub const DEFAULT_WIDTH: f64 = 1280.0;
/// Default window height in logical pixels.
pub const DEFAULT_HEIGHT: f64 = 720.0;
/// Default window title.
pub const DEFAULT_TITLE: &str = "Claude Terminal \u{2014} Anthropic";

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
    /// Per-pane vi-mode state. None = vi-mode not active.
    pub vi_state: Option<crate::vi_mode::ViState>,
    /// Per-pane cursor state for rendering and blink.
    pub cursor: crate::renderer::cursor::CursorState,
    /// Per-pane mouse selection state (click counting, drag, active selection).
    pub mouse_selection: crate::input::mouse::MouseSelectionState,
    /// Per-pane scroll state for smooth animation and auto-hide.
    pub scroll_state: crate::scroll::ScrollState,
}

/// Main application state implementing the winit event loop handler.
pub struct App {
    config: WindowConfig,
    pub(crate) app_config: Config,
    window: Option<Arc<Window>>,
    renderer: Option<crate::renderer::Renderer>,
    tab_manager: TabManager,
    pane_states: HashMap<PaneId, PaneState>,
    modifiers: ModifiersState,
    interaction: PaneInteraction,
    link_detector: LinkDetector,
    link_hover_active: bool,
    input_mode: InputMode,
    search_state: SearchState,
    current_font_size: f32,
    default_font_size: f32,
    event_proxy: Option<EventLoopProxy<UserEvent>>,
    screenshot_requested: bool,
    hovered_tab: Option<usize>,
    /// Tab drag state for drag-to-reorder.
    tab_drag_index: Option<usize>,
    tab_drag_start_x: f32,
    tab_drag_active: bool,
    /// Throttle foreground process name detection (FFI syscall).
    last_process_check: std::time::Instant,
    last_process_name: Option<String>,
    /// Visual bell flash end time.
    bell_flash_until: Option<std::time::Instant>,
    /// Whether the window is hidden via quick terminal toggle.
    quick_terminal_hidden: bool,
}

impl App {
    pub fn new(config: WindowConfig, app_config: Config) -> Self {
        let font_size = app_config.font.size as f32;
        Self {
            config,
            app_config,
            window: None,
            renderer: None,
            tab_manager: TabManager::new(),
            pane_states: HashMap::new(),
            modifiers: ModifiersState::empty(),
            interaction: PaneInteraction::new(),
            link_detector: LinkDetector::new(),
            link_hover_active: false,
            input_mode: InputMode::default(),
            search_state: SearchState::default(),
            current_font_size: font_size,
            default_font_size: font_size,
            event_proxy: None,
            screenshot_requested: false,
            hovered_tab: None,
            tab_drag_index: None,
            tab_drag_start_x: 0.0,
            tab_drag_active: false,
            last_process_check: std::time::Instant::now(),
            last_process_name: None,
            bell_flash_until: None,
            quick_terminal_hidden: false,
        }
    }

    /// Get the tab manager (for testing).
    pub fn tab_manager(&self) -> &TabManager {
        &self.tab_manager
    }

    /// Get a mutable reference to the tab manager (for testing).
    pub fn tab_manager_mut(&mut self) -> &mut TabManager {
        &mut self.tab_manager
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
                let mut cursor = crate::renderer::cursor::CursorState::with_blink_rate(
                    self.app_config.cursor.blink_rate,
                );
                if let Some(style) = crate::renderer::cursor::CursorStyle::from_config_str(
                    &self.app_config.cursor.style,
                ) {
                    cursor.set_style(style);
                }
                if !self.app_config.cursor.blink {
                    cursor.set_blink_rate(0);
                }
                self.pane_states.insert(pane_id, PaneState { terminal, pty, vi_state: None, cursor, mouse_selection: crate::input::mouse::MouseSelectionState::new(), scroll_state: crate::scroll::ScrollState::new() });
            }
            Err(e) => {
                log::error!("Failed to spawn PTY for pane {:?}: {e}", pane_id);
            }
        }
    }

    /// Total chrome height above the content area (header bar + tab bar).
    fn chrome_top_height() -> f32 {
        HEADER_BAR_HEIGHT + TAB_BAR_HEIGHT
    }

    /// Padding around the pane grid (between chrome and panes).
    const PANE_GRID_PADDING: f32 = 12.0;

    /// Compute the content bounds (below header bar + tab bar, above status bar).
    fn content_bounds(&self, width: f32, height: f32) -> Rect {
        let top = Self::chrome_top_height();
        let content_h = (height - top - STATUS_BAR_HEIGHT).max(0.0);
        Rect::new(0.0, top, width, content_h)
    }

    /// Content bounds inset by PANE_GRID_PADDING on all sides for pane layout.
    fn pane_grid_bounds(&self, width: f32, height: f32) -> Rect {
        let top = Self::chrome_top_height() + Self::PANE_GRID_PADDING;
        let pad2 = Self::PANE_GRID_PADDING * 2.0;
        let content_w = (width - pad2).max(0.0);
        let content_h = (height - top - STATUS_BAR_HEIGHT - Self::PANE_GRID_PADDING).max(0.0);
        Rect::new(Self::PANE_GRID_PADDING, top, content_w, content_h)
    }

    /// Handle a pane command (split, close, focus, zoom).
    /// Process shell state updates after PTY drain: notifications and CWD tab titles.
    fn process_shell_updates(&mut self) {
        let focused = self
            .tab_manager
            .active_tab()
            .pane_tree
            .focused_pane_id();
        let threshold = self.app_config.shell.notification_threshold_secs;
        let shell_enabled = self.app_config.shell.integration_enabled;

        // Collect pane IDs to avoid borrow conflict
        let pane_ids: Vec<_> = self.pane_states.keys().copied().collect();

        for pane_id in pane_ids {
            let state = self.pane_states.get_mut(&pane_id).unwrap();
            let shell = state.terminal.shell_state_mut();

            // Check for completed commands — notification for non-focused panes
            if shell_enabled {
                if let Some(duration) = shell.pending_completion.take() {
                    if pane_id != focused && duration.as_secs() >= threshold {
                        if let Some(tab_idx) = self.tab_manager.tab_index_for_pane(pane_id) {
                            self.tab_manager.set_notification(tab_idx, true);
                        }
                    }
                }
            }

            // Update tab title for focused pane of active tab:
            // Priority: explicit title (OSC 0/2) > process name > CWD basename > "Shell"
            if pane_id == focused {
                let active_idx = self.tab_manager.active_index();

                if shell.title_is_explicit {
                    // Explicit title from OSC 0/2 — highest priority
                    if let Some(title) = shell.title.clone() {
                        self.tab_manager.set_title(active_idx, &title);
                    }
                } else {
                    // Try foreground process name (throttled to 1Hz)
                    let now = std::time::Instant::now();
                    if now.duration_since(self.last_process_check).as_millis() >= 1000 {
                        self.last_process_check = now;
                        self.last_process_name = state.pty.child_pid().and_then(|pid| {
                            crate::pty::foreground_process_name(pid)
                        });
                    }
                    let process_title = self.last_process_name.clone();

                    if let Some(ref proc_name) = process_title {
                        if !crate::pty::is_shell_process(proc_name) {
                            // Non-shell process (e.g., "vim", "claude") — use it
                            self.tab_manager.set_title(active_idx, proc_name);
                        } else if let Some(cwd) = shell.cwd.clone() {
                            // Shell process — fall back to CWD basename
                            let dir_name =
                                crate::shell_integration::dir_name_from_path(&cwd);
                            self.tab_manager.set_title(active_idx, dir_name);
                        }
                    } else if shell.cwd_changed {
                        // No process detected — use CWD if changed
                        if let Some(cwd) = shell.cwd.clone() {
                            let dir_name =
                                crate::shell_integration::dir_name_from_path(&cwd);
                            self.tab_manager.set_title(active_idx, dir_name);
                        }
                    }
                    if shell.cwd_changed {
                        shell.cwd_changed = false;
                    }
                }
            }
        }
    }

    /// Handle a shell integration command (prompt navigation).
    /// Compute a new font size from the current size using ~10% steps, clamped to [8, 72].
    fn compute_font_size(current: f32, command: AppCommand, default: f32) -> f32 {
        const MIN_FONT: f32 = 8.0;
        const MAX_FONT: f32 = 72.0;
        let raw = match command {
            AppCommand::IncreaseFontSize => (current * 1.1).round(),
            AppCommand::DecreaseFontSize => (current / 1.1).round(),
            AppCommand::ResetFontSize => default,
            AppCommand::NewWindow | AppCommand::ClearScrollback => return current, // handled before compute
        };
        raw.clamp(MIN_FONT, MAX_FONT)
    }

    fn handle_app_command(&mut self, command: AppCommand) {
        if command == AppCommand::NewWindow {
            self.spawn_new_window();
            return;
        }

        if command == AppCommand::ClearScrollback {
            let focused_id = self.tab_manager.active_tab().pane_tree.focused_pane_id();
            if let Some(state) = self.pane_states.get_mut(&focused_id) {
                state.terminal.clear_scrollback();
                log::info!("Cleared scrollback for pane {focused_id:?}");
            }
            if let Some(window) = &self.window {
                window.request_redraw();
            }
            return;
        }

        let new_size = Self::compute_font_size(
            self.current_font_size,
            command,
            self.default_font_size,
        );
        if (new_size - self.current_font_size).abs() < 0.5 {
            return; // No effective change
        }
        self.current_font_size = new_size;
        log::info!("Font size changed to {new_size}px");

        // Rebuild atlas and recalculate all pane dimensions
        let font_family = self.app_config.font.family.clone();
        let line_height = self.app_config.font.line_height as f32;
        if let Some(renderer) = &mut self.renderer {
            renderer.rebuild_atlas(new_size, &font_family, line_height);
        }
        let (w, h) = self.window_size();
        self.resize_all_panes(w, h);
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// Spawn a new VeloTerm window by launching a new process.
    fn spawn_new_window(&self) {
        match std::env::current_exe() {
            Ok(exe_path) => {
                match std::process::Command::new(&exe_path).spawn() {
                    Ok(_) => log::info!("Spawned new VeloTerm window"),
                    Err(e) => log::error!("Failed to spawn new window: {e}"),
                }
            }
            Err(e) => log::error!("Failed to get current exe path: {e}"),
        }
    }

    fn handle_context_menu_action(
        &mut self,
        action: crate::context_menu::ContextMenuAction,
        event_loop: &ActiveEventLoop,
    ) {
        use crate::context_menu::ContextMenuAction;
        match action {
            ContextMenuAction::Copy => {
                let focused_id = self.tab_manager.active_tab().pane_tree.focused_pane_id();
                if let Some(state) = self.pane_states.get_mut(&focused_id) {
                    if let Some(ref sel) = state.mouse_selection.active_selection {
                        let cells = crate::terminal::grid_bridge::extract_grid_cells(&state.terminal);
                        let cols = state.terminal.columns();
                        let text = match sel.selection_type {
                            crate::input::selection::SelectionType::VisualBlock => {
                                crate::input::selection::selected_text_block(&cells, sel, cols)
                            }
                            crate::input::selection::SelectionType::Line => {
                                crate::input::selection::selected_text_lines(&cells, sel, cols)
                            }
                            _ => crate::input::selection::selected_text(&cells, sel, cols),
                        };
                        if !text.is_empty() {
                            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                let _ = clipboard.set_text(&text);
                            }
                        }
                        state.mouse_selection.clear_selection();
                    }
                }
            }
            ContextMenuAction::Paste => {
                let focused_id = self.tab_manager.active_tab().pane_tree.focused_pane_id();
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    if let Ok(text) = clipboard.get_text() {
                        if let Some(state) = self.pane_states.get_mut(&focused_id) {
                            let bytes = crate::input::clipboard::paste_bytes(&text, true);
                            if let Err(e) = state.pty.write(&bytes) {
                                log::warn!("PTY paste write error: {e}");
                            }
                        }
                    }
                }
            }
            ContextMenuAction::SelectAll => {
                let focused_id = self.tab_manager.active_tab().pane_tree.focused_pane_id();
                if let Some(state) = self.pane_states.get_mut(&focused_id) {
                    let rows = state.terminal.rows();
                    let cols = state.terminal.columns();
                    state.mouse_selection.active_selection = Some(
                        crate::input::selection::Selection {
                            start: (0, 0),
                            end: (rows.saturating_sub(1), cols.saturating_sub(1)),
                            selection_type: crate::input::selection::SelectionType::Range,
                        },
                    );
                }
            }
            ContextMenuAction::SplitVertical => {
                self.handle_pane_command(PaneCommand::SplitVertical, event_loop);
            }
            ContextMenuAction::SplitHorizontal => {
                self.handle_pane_command(PaneCommand::SplitHorizontal, event_loop);
            }
            ContextMenuAction::ClosePane => {
                self.handle_pane_command(PaneCommand::ClosePane, event_loop);
            }
            ContextMenuAction::ClearScrollback => {
                self.handle_app_command(AppCommand::ClearScrollback);
            }
            ContextMenuAction::NewTab => {
                self.handle_tab_command(TabCommand::NewTab, event_loop);
            }
            ContextMenuAction::NewWindow => {
                self.spawn_new_window();
            }
            ContextMenuAction::CloseTab => {
                self.handle_tab_command(TabCommand::CloseTab, event_loop);
            }
            ContextMenuAction::CloseOtherTabs => {
                self.close_other_tabs(event_loop);
            }
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn handle_config_reload(&mut self, new_config: Config, delta: ConfigDelta) {
        log::info!("Config reloaded (font_changed={}, padding_changed={})",
            delta.font_changed, delta.padding_changed);

        if delta.font_changed {
            let new_size = new_config.font.size as f32;
            let new_family = new_config.font.family.clone();
            let new_lh = new_config.font.line_height as f32;

            self.current_font_size = new_size;
            self.default_font_size = new_size;

            if let Some(renderer) = &mut self.renderer {
                renderer.rebuild_atlas(new_size, &new_family, new_lh);
            }
        }

        if delta.padding_changed {
            if let (Some(renderer), Some(window)) = (&mut self.renderer, &self.window) {
                let scale = window.scale_factor() as f32;
                let pad = &new_config.padding;
                renderer.set_padding(
                    pad.top as f32 * scale,
                    pad.bottom as f32 * scale,
                    pad.left as f32 * scale,
                    pad.right as f32 * scale,
                );
            }
        }

        self.app_config = new_config;

        if delta.font_changed || delta.padding_changed {
            let (w, h) = self.window_size();
            self.resize_all_panes(w, h);
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }

        if delta.colors_changed {
            if let Some(renderer) = &mut self.renderer {
                let _theme = Theme::from_name(&self.app_config.colors.theme)
                    .unwrap_or_else(Theme::claude_dark);
                renderer.pane_damage_mut().force_full_damage_all();
            }
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }

        if delta.cursor_changed {
            for state in self.pane_states.values_mut() {
                if let Some(style) = crate::renderer::cursor::CursorStyle::from_config_str(
                    &self.app_config.cursor.style,
                ) {
                    state.cursor.set_style(style);
                }
                if self.app_config.cursor.blink {
                    state.cursor.set_blink_rate(self.app_config.cursor.blink_rate);
                } else {
                    state.cursor.set_blink_rate(0);
                }
            }
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }

    fn handle_shell_command(&mut self, command: crate::input::ShellCommand) {
        let focused = self
            .tab_manager
            .active_tab()
            .pane_tree
            .focused_pane_id();
        if let Some(state) = self.pane_states.get_mut(&focused) {
            let moved = match command {
                crate::input::ShellCommand::PreviousPrompt => {
                    state.terminal.jump_to_previous_prompt()
                }
                crate::input::ShellCommand::NextPrompt => {
                    state.terminal.jump_to_next_prompt()
                }
            };
            if moved {
                if let Some(renderer) = &mut self.renderer {
                    renderer.pane_damage_mut().force_full_damage_all();
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
        }
    }

    fn handle_pane_command(
        &mut self,
        command: PaneCommand,
        event_loop: &ActiveEventLoop,
    ) {
        let (width, height) = self.window_size();
        let pgrid = self.pane_grid_bounds(width as f32, height as f32);

        match command {
            PaneCommand::SplitVertical | PaneCommand::SplitHorizontal => {
                let direction = match command {
                    PaneCommand::SplitVertical => SplitDirection::Vertical,
                    _ => SplitDirection::Horizontal,
                };
                let pane_tree = &mut self.tab_manager.active_tab_mut().pane_tree;
                if let Some(new_id) = pane_tree.split_focused(direction) {
                    let layout = pane_tree.calculate_layout(pgrid.width, pgrid.height);
                    if let Some((_, rect)) = layout.iter().find(|(id, _)| *id == new_id) {
                        let (cols, rows) = self.grid_dims_for_rect(rect);
                        self.spawn_pane(new_id, cols, rows);
                    }
                    self.resize_all_panes(width, height);
                    self.update_interaction_layout(width, height);
                    if let Some(renderer) = &mut self.renderer {
                        renderer.pane_damage_mut().force_full_damage_all();
                    }
                }
            }
            PaneCommand::ClosePane => {
                let pane_tree = &self.tab_manager.active_tab().pane_tree;
                if pane_tree.pane_count() == 1 {
                    // Single pane in tab — close the tab instead
                    self.handle_close_active_tab(event_loop);
                    return;
                }
                let closing_id = self.tab_manager.active_tab().pane_tree.focused_pane_id();
                let pane_tree = &mut self.tab_manager.active_tab_mut().pane_tree;
                match pane_tree.close_focused() {
                    Some(_) => {
                        self.pane_states.remove(&closing_id);
                        if let Some(renderer) = &mut self.renderer {
                            renderer.remove_pane_damage(closing_id);
                            renderer.pane_damage_mut().force_full_damage_all();
                        }
                        self.resize_all_panes(width, height);
                        self.update_interaction_layout(width, height);
                    }
                    None => {
                        // Should not reach here due to pane_count check above
                        log::warn!("close_focused returned None unexpectedly");
                    }
                }
            }
            PaneCommand::FocusDirection(direction) => {
                self.tab_manager.active_tab_mut().pane_tree.focus_direction(
                    direction,
                    pgrid.width,
                    pgrid.height,
                );
            }
            PaneCommand::ZoomToggle => {
                let pane_tree = &mut self.tab_manager.active_tab_mut().pane_tree;
                pane_tree.zoom_toggle();
                self.resize_all_panes(width, height);
                self.update_interaction_layout(width, height);
                if let Some(renderer) = &mut self.renderer {
                    renderer.pane_damage_mut().force_full_damage_all();
                }
            }
        }
    }

    /// Handle a tab command (new, close, switch, move).
    fn handle_tab_command(
        &mut self,
        command: TabCommand,
        event_loop: &ActiveEventLoop,
    ) {
        let (width, height) = self.window_size();

        match command {
            TabCommand::CloseTab => {
                self.handle_close_active_tab(event_loop);
                return; // handle_close_active_tab handles redraw/layout
            }
            TabCommand::NewTab => {
                self.tab_manager.new_tab();
                // Spawn PTY for the new tab's initial pane
                let pane_id = self.tab_manager.active_tab().pane_tree.focused_pane_id();
                let pgrid = self.pane_grid_bounds(width as f32, height as f32);
                let rect = Rect::new(0.0, 0.0, pgrid.width, pgrid.height);
                let (cols, rows) = self.grid_dims_for_rect(&rect);
                self.spawn_pane(pane_id, cols, rows);
                self.update_interaction_layout(width, height);
                if let Some(renderer) = &mut self.renderer {
                    renderer.pane_damage_mut().force_full_damage_all();
                }
            }
            TabCommand::NextTab => {
                self.tab_manager.next_tab();
                self.update_interaction_layout(width, height);
                if let Some(renderer) = &mut self.renderer {
                    renderer.pane_damage_mut().force_full_damage_all();
                }
            }
            TabCommand::PrevTab => {
                self.tab_manager.prev_tab();
                self.update_interaction_layout(width, height);
                if let Some(renderer) = &mut self.renderer {
                    renderer.pane_damage_mut().force_full_damage_all();
                }
            }
            TabCommand::SelectTab(index) => {
                self.tab_manager.select_tab(index);
                self.update_interaction_layout(width, height);
                if let Some(renderer) = &mut self.renderer {
                    renderer.pane_damage_mut().force_full_damage_all();
                }
            }
            TabCommand::MoveTabLeft => {
                let idx = self.tab_manager.active_index();
                if idx > 0 {
                    self.tab_manager.move_tab(idx, idx - 1);
                }
            }
            TabCommand::MoveTabRight => {
                let idx = self.tab_manager.active_index();
                if idx + 1 < self.tab_manager.tab_count() {
                    self.tab_manager.move_tab(idx, idx + 1);
                }
            }
        }

        // Clear notification badge on the newly active tab
        self.tab_manager.clear_active_notification();

        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// Close the active tab, removing all its pane states.
    fn handle_close_active_tab(&mut self, event_loop: &ActiveEventLoop) {
        let index = self.tab_manager.active_index();
        match self.tab_manager.close_tab(index) {
            Some(pane_ids) => {
                for pane_id in &pane_ids {
                    self.pane_states.remove(pane_id);
                    if let Some(renderer) = &mut self.renderer {
                        renderer.remove_pane_damage(*pane_id);
                    }
                }
                let (w, h) = self.window_size();
                self.update_interaction_layout(w, h);
                if let Some(renderer) = &mut self.renderer {
                    renderer.pane_damage_mut().force_full_damage_all();
                }
            }
            None => {
                // Last tab — exit application
                log::info!("Last tab closed, exiting");
                event_loop.exit();
            }
        }
    }

    /// Close all tabs except the currently active one.
    fn close_other_tabs(&mut self, _event_loop: &ActiveEventLoop) {
        let active = self.tab_manager.active_index();
        let count = self.tab_manager.tab_count();
        if count <= 1 {
            return;
        }
        // Close tabs from the end to avoid shifting indices
        for i in (0..count).rev() {
            if i == active {
                continue;
            }
            if let Some(pane_ids) = self.tab_manager.close_tab(i) {
                for pane_id in &pane_ids {
                    self.pane_states.remove(pane_id);
                    if let Some(renderer) = &mut self.renderer {
                        renderer.remove_pane_damage(*pane_id);
                    }
                }
            }
        }
        let (w, h) = self.window_size();
        self.update_interaction_layout(w, h);
        if let Some(renderer) = &mut self.renderer {
            renderer.pane_damage_mut().force_full_damage_all();
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// Toggle window visibility for quick terminal mode.
    fn toggle_quick_terminal(&mut self) {
        if let Some(window) = &self.window {
            if self.quick_terminal_hidden {
                window.set_visible(true);
                window.focus_window();
                self.quick_terminal_hidden = false;
                log::info!("Quick terminal: window shown");
            } else {
                window.set_visible(false);
                self.quick_terminal_hidden = true;
                log::info!("Quick terminal: window hidden");
            }
        }
    }

    /// Compute grid columns and rows for a pane rect, accounting for padding.
    fn grid_dims_for_rect(&self, rect: &Rect) -> (u16, u16) {
        if let Some(renderer) = &self.renderer {
            let cw = renderer.cell_width();
            let ch = renderer.cell_height();
            let pad = &self.app_config.padding;
            let usable_w = (rect.width - pad.left as f32 - pad.right as f32).max(0.0);
            let usable_h = (rect.height - PANE_HEADER_HEIGHT - pad.top as f32 - pad.bottom as f32).max(0.0);
            let cols = (usable_w / cw).floor().max(1.0) as u16;
            let rows = (usable_h / ch).floor().max(1.0) as u16;
            (cols, rows)
        } else {
            (80, 24)
        }
    }

    /// Resize all pane terminals and PTYs to match their current layout rects.
    fn resize_all_panes(&mut self, width: u32, height: u32) {
        let pgrid = self.pane_grid_bounds(width as f32, height as f32);
        let pane_tree = &self.tab_manager.active_tab().pane_tree;
        let layout = pane_tree.calculate_layout(pgrid.width, pgrid.height);
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
                self.tab_manager
                    .active_tab_mut()
                    .pane_tree
                    .set_split_ratio_by_index(split_index, new_ratio);
                let (w, h) = self.window_size();
                self.resize_all_panes(w, h);
                self.update_interaction_layout(w, h);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            InteractionEffect::FocusPane(pane_id) => {
                self.tab_manager
                    .active_tab_mut()
                    .pane_tree
                    .set_focus(pane_id);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
        }
    }

    /// Update the interaction state machine's cached layout.
    fn update_interaction_layout(&mut self, width: u32, height: u32) {
        let content = self.content_bounds(width as f32, height as f32);
        let pane_tree = &self.tab_manager.active_tab().pane_tree;
        self.interaction
            .update_layout(pane_tree.root(), content, 20.0);
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

    /// Returns true if the "super" modifier is held (Cmd on macOS, Ctrl on Linux).
    fn is_link_modifier_held(&self) -> bool {
        if !self.app_config.links.enabled {
            return false;
        }
        if cfg!(target_os = "macos") {
            self.modifiers.super_key()
        } else {
            self.modifiers.control_key()
        }
    }

    /// Rescan terminal content for links in the focused pane.
    fn rescan_links(&mut self) {
        let focused = self.tab_manager.active_tab().pane_tree.focused_pane_id();
        if let Some(state) = self.pane_states.get(&focused) {
            let lines = crate::terminal::grid_bridge::extract_text_lines(&state.terminal);
            self.link_detector.scan(&lines);
        }
    }

    /// Check if cursor is over a link and update highlight state.
    /// Returns true if cursor is over a link.
    fn update_link_hover(&mut self, pixel_x: f32, pixel_y: f32) -> bool {
        if !self.is_link_modifier_held() {
            if self.link_hover_active {
                self.link_hover_active = false;
            }
            return false;
        }

        let renderer = match &self.renderer {
            Some(r) => r,
            None => return false,
        };

        let cell_width = renderer.cell_width();
        let cell_height = renderer.cell_height();
        let [pad_top, _pad_bottom, pad_left, _pad_right] = renderer.padding();

        // Convert pixel position (in content space) to grid coords, accounting for padding
        let adj_x = pixel_x - pad_left;
        let adj_y = pixel_y - pad_top;
        if adj_x < 0.0 || adj_y < 0.0 {
            return false;
        }
        let col = (adj_x / cell_width).floor() as usize;
        let row = (adj_y / cell_height).floor() as usize;

        if let Some(_link) = self.link_detector.link_at(row, col) {
            self.link_hover_active = true;
            true
        } else {
            if self.link_hover_active {
                self.link_hover_active = false;
            }
            false
        }
    }

    /// Handle modifier+click on a link.
    fn handle_link_click(&self, pixel_x: f32, pixel_y: f32) -> bool {
        if !self.is_link_modifier_held() {
            return false;
        }

        let renderer = match &self.renderer {
            Some(r) => r,
            None => return false,
        };

        let cell_width = renderer.cell_width();
        let cell_height = renderer.cell_height();
        let [pad_top, _pad_bottom, pad_left, _pad_right] = renderer.padding();

        let adj_x = pixel_x - pad_left;
        let adj_y = pixel_y - pad_top;
        if adj_x < 0.0 || adj_y < 0.0 {
            return false;
        }
        let col = (adj_x / cell_width).floor() as usize;
        let row = (adj_y / cell_height).floor() as usize;

        if let Some(link) = self.link_detector.link_at(row, col) {
            open_link(link);
            true
        } else {
            false
        }
    }

    /// Get the link detector (for testing).
    pub fn link_detector(&self) -> &LinkDetector {
        &self.link_detector
    }

    /// Handle a search command: update query, navigate matches, or close search.
    fn handle_search_command(&mut self, cmd: SearchCommand) {
        match cmd {
            SearchCommand::InsertChar(ch) => {
                self.search_state.query.push(ch);
                self.run_incremental_search();
            }
            SearchCommand::DeleteChar => {
                self.search_state.query.pop();
                self.run_incremental_search();
            }
            SearchCommand::NextMatch => {
                self.search_state.next_match();
                self.scroll_to_current_match();
            }
            SearchCommand::PrevMatch => {
                self.search_state.prev_match();
                self.scroll_to_current_match();
            }
            SearchCommand::Close => {
                self.input_mode = InputMode::Normal;
                self.search_state.is_active = false;
                self.search_state.query.clear();
                self.search_state.matches.clear();
            }
            SearchCommand::Open => {
                // Already handled by should_open_search
            }
        }
        if let Some(renderer) = &mut self.renderer {
            renderer.pane_damage_mut().force_full_damage_all();
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// Convert a winit key event to a character for vi-mode processing.
    fn key_to_vi_char(logical_key: &Key, text: Option<&str>) -> Option<char> {
        match logical_key {
            Key::Character(s) => s.chars().next(),
            Key::Named(named) => match named {
                NamedKey::Escape => Some('\x1b'),
                NamedKey::Enter => Some('\r'),
                NamedKey::Backspace => Some('\x7f'),
                NamedKey::Space => Some(' '),
                _ => None,
            },
            _ => {
                // Fall back to text content
                text.and_then(|t| t.chars().next())
            }
        }
    }

    /// Handle a ViAction produced by the vi-mode state machine.
    fn handle_vi_action(
        &mut self,
        action: crate::vi_mode::ViAction,
        pane_id: PaneId,
    ) {
        use crate::vi_mode::ViAction;

        match action {
            ViAction::Motion(motion) => {
                if let Some(state) = self.pane_states.get_mut(&pane_id) {
                    if let Some(ref mut vi) = state.vi_state {
                        let cols = state.terminal.cols();
                        let total_rows = state.terminal.total_rows();
                        let display_offset = state.terminal.display_offset();
                        let viewport_rows = state.terminal.rows();
                        let viewport_top = total_rows.saturating_sub(viewport_rows + display_offset);
                        let grid: Vec<Vec<char>> = (0..total_rows)
                            .map(|r| {
                                (0..cols)
                                    .map(|c| {
                                        state.terminal.char_at(r, c).unwrap_or(' ')
                                    })
                                    .collect()
                            })
                            .collect();
                        let ctx = crate::vi_mode::BufferContext {
                            total_rows,
                            cols,
                            viewport_top,
                            viewport_rows,
                            char_at_fn: &|row, col| {
                                grid.get(row).and_then(|r| r.get(col).copied())
                            },
                        };
                        vi.apply_motion(&motion, &ctx);
                    }
                }
            }
            ViAction::ExitViMode => {
                if let Some(state) = self.pane_states.get_mut(&pane_id) {
                    state.vi_state = None;
                    log::info!("Vi-mode exited for pane {:?}", pane_id);
                }
            }
            ViAction::Yank => {
                if let Some(state) = self.pane_states.get_mut(&pane_id) {
                    if let Some(ref vi) = state.vi_state {
                        let cols = state.terminal.cols();
                        let cells = crate::terminal::grid_bridge::extract_grid_cells(
                            &state.terminal,
                        );
                        if let Some(text) = vi.yank_text(&cells, cols) {
                            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                if let Err(e) = clipboard.set_text(&text) {
                                    log::warn!("Clipboard write error: {e}");
                                } else {
                                    log::info!("Yanked {} bytes to clipboard", text.len());
                                }
                            }
                        }
                    }
                }
            }
            ViAction::SearchExecute => {
                // Execute vi-mode search using SearchEngine
                if let Some(state) = self.pane_states.get_mut(&pane_id) {
                    if let Some(ref mut vi) = state.vi_state {
                        let query = vi.search_query.clone();
                        if !query.is_empty() {
                            let lines = crate::terminal::grid_bridge::extract_text_lines(
                                &state.terminal,
                            );
                            let engine = crate::search::SearchEngine::new();
                            let result = engine.search(&query, &lines);
                            if let Some(first) = result.matches.first() {
                                vi.move_to_match(first.row as usize, first.start_col);
                            }
                        }
                    }
                }
            }
            ViAction::NextMatch | ViAction::PrevMatch => {
                if let Some(state) = self.pane_states.get_mut(&pane_id) {
                    if let Some(ref mut vi) = state.vi_state {
                        let query = vi.search_query.clone();
                        if !query.is_empty() {
                            let lines = crate::terminal::grid_bridge::extract_text_lines(
                                &state.terminal,
                            );
                            let engine = crate::search::SearchEngine::new();
                            let result = engine.search(&query, &lines);
                            if !result.matches.is_empty() {
                                let cursor_row = vi.cursor.row as i32;
                                let cursor_col = vi.cursor.col;
                                let forward = matches!(action, ViAction::NextMatch)
                                    == (vi.search_direction
                                        == crate::vi_mode::SearchDirection::Forward);
                                let next = if forward {
                                    result
                                        .matches
                                        .iter()
                                        .find(|m| {
                                            m.row > cursor_row
                                                || (m.row == cursor_row
                                                    && m.start_col > cursor_col)
                                        })
                                        .or(result.matches.first())
                                } else {
                                    result
                                        .matches
                                        .iter()
                                        .rev()
                                        .find(|m| {
                                            m.row < cursor_row
                                                || (m.row == cursor_row
                                                    && m.start_col < cursor_col)
                                        })
                                        .or(result.matches.last())
                                };
                                if let Some(m) = next {
                                    vi.move_to_match(m.row as usize, m.start_col);
                                }
                            }
                        }
                    }
                }
            }
            ViAction::EnterVisual(_) | ViAction::ExitVisual => {
                // Selection state changes are handled by ViState internally.
                // Redraw will pick up the updated selection via to_selection().
            }
            ViAction::SearchForward | ViAction::SearchBackward | ViAction::SearchCancel => {
                // Search input mode changes handled by ViState internally.
            }
            ViAction::None => {}
        }

        if let Some(renderer) = &mut self.renderer {
            renderer.pane_damage_mut().force_full_damage_all();
        }
    }

    /// Re-run search after query changes (incremental search).
    fn run_incremental_search(&mut self) {
        let focused = self.tab_manager.active_tab().pane_tree.focused_pane_id();
        if let Some(state) = self.pane_states.get(&focused) {
            let lines = crate::terminal::grid_bridge::extract_text_lines(&state.terminal);
            self.search_state.set_query(&self.search_state.query.clone(), &lines);
        }
    }

    /// Scroll the terminal viewport to show the current search match.
    fn scroll_to_current_match(&mut self) {
        let focused = self.tab_manager.active_tab().pane_tree.focused_pane_id();
        if let (Some(target_row), Some(state)) = (
            self.search_state.scroll_target(),
            self.pane_states.get_mut(&focused),
        ) {
            let viewport_rows = state.terminal.rows();
            let current_offset = state.terminal.display_offset();
            let max_offset = state.terminal.history_size();
            if let Some(new_offset) = crate::search::compute_scroll_offset(
                target_row,
                viewport_rows,
                current_offset,
                max_offset,
            ) {
                state.terminal.set_display_offset(new_offset);
            }
        }
    }

    /// Get the current input mode (for testing).
    pub fn input_mode(&self) -> InputMode {
        self.input_mode
    }

    /// Get the search state (for testing).
    pub fn search_state(&self) -> &SearchState {
        &self.search_state
    }

    /// Run the application event loop. This blocks until the window is closed.
    pub fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
        let proxy = event_loop.create_proxy();
        self.event_proxy = Some(proxy.clone());

        // Start config file watcher (best-effort — non-fatal if it fails)
        let _config_watcher = Self::start_config_watcher(&self.app_config, proxy.clone());

        // Start quick terminal global hotkey (if enabled)
        let _hotkey_manager = if self.app_config.quick_terminal.enabled {
            match crate::hotkey::HotkeyManager::new(
                &self.app_config.quick_terminal.hotkey,
                proxy,
            ) {
                Ok(mgr) => {
                    log::info!(
                        "Quick terminal hotkey registered: {}",
                        self.app_config.quick_terminal.hotkey
                    );
                    Some(mgr)
                }
                Err(e) => {
                    log::warn!("Failed to register quick terminal hotkey: {e}");
                    None
                }
            }
        } else {
            None
        };

        event_loop.run_app(&mut self)?;
        Ok(())
    }

    /// Start watching the config file and send reload events via the proxy.
    fn start_config_watcher(
        config: &Config,
        proxy: EventLoopProxy<UserEvent>,
    ) -> Option<crate::config::watcher::ConfigWatcher> {
        let config_path = Self::config_file_path();
        if !config_path.exists() {
            log::info!("No config file at {}, skipping watcher", config_path.display());
            return None;
        }
        match crate::config::watcher::ConfigWatcher::new(
            &config_path,
            config.clone(),
            move |new_config, delta| {
                let _ = proxy.send_event(UserEvent::ConfigReloaded(new_config, delta));
            },
        ) {
            Ok(w) => {
                log::info!("Config watcher started for {}", config_path.display());
                Some(w)
            }
            Err(e) => {
                log::warn!("Failed to start config watcher: {e}");
                None
            }
        }
    }

    /// Get the config file path (~/.config/veloterm/config.toml).
    fn config_file_path() -> std::path::PathBuf {
        let home = std::env::var("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("."));
        home.join(".config").join("veloterm").join("config.toml")
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::ConfigReloaded(new_config, delta) => {
                self.handle_config_reload(new_config, delta);
            }
            UserEvent::QuickTerminalToggle => {
                self.toggle_quick_terminal();
            }
        }
    }

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

                // Set macOS title bar color to match tab bar surface
                #[cfg(target_os = "macos")]
                crate::platform::macos::set_titlebar_color(
                    &window,
                    theme.surface.r as f64,
                    theme.surface.g as f64,
                    theme.surface.b as f64,
                );

                let font_size = self.app_config.font.size as f32;
                let font_family = self.app_config.font.family.as_str();
                let line_height = self.app_config.font.line_height as f32;

                // Initialize renderer
                match pollster::block_on(crate::renderer::Renderer::new(
                    window.clone(),
                    theme,
                    font_size,
                    font_family,
                    line_height,
                )) {
                    Ok(mut renderer) => {
                        log::info!("Renderer initialized");

                        // Apply terminal padding from config (scaled to physical pixels)
                        let scale = window.scale_factor() as f32;
                        let pad = &self.app_config.padding;
                        renderer.set_padding(
                            pad.top as f32 * scale,
                            pad.bottom as f32 * scale,
                            pad.left as f32 * scale,
                            pad.right as f32 * scale,
                        );

                        self.renderer = Some(renderer);

                        // Spawn PTY and terminal for the initial tab's pane
                        // Use grid_dims_for_rect to account for padding
                        let initial_pane_id =
                            self.tab_manager.active_tab().pane_tree.focused_pane_id();
                        let pgrid = self.pane_grid_bounds(size.width as f32, size.height as f32);
                        let rect = Rect::new(0.0, 0.0, pgrid.width, pgrid.height);
                        let (cols, rows) = self.grid_dims_for_rect(&rect);
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
        // Forward events to iced layer for UI widget interaction
        if let Some(renderer) = &mut self.renderer {
            let scale = self.window.as_ref().map(|w| w.scale_factor() as f32).unwrap_or(1.0);
            renderer.iced_layer_mut().push_event(&event, scale, self.modifiers);
        }

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
                    // Check for screenshot (Cmd+Shift+S on macOS, Ctrl+Shift+S elsewhere)
                    let is_screenshot_key = matches!(event.logical_key, Key::Character(ref s) if s.as_str() == "s" || s.as_str() == "S")
                        && self.modifiers.shift_key()
                        && (self.modifiers.super_key() || self.modifiers.control_key());

                    if is_screenshot_key {
                        self.screenshot_requested = true;
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                        log::info!("📸 Screenshot requested (Cmd+Shift+S) - will capture on next frame");
                        return;
                    }

                    // Check for search toggle (Ctrl+Shift+F) — works in any mode
                    if should_open_search(&event.logical_key, self.modifiers) {
                        if self.input_mode == InputMode::Search {
                            // Close search
                            self.input_mode = InputMode::Normal;
                            self.search_state.is_active = false;
                            self.search_state.query.clear();
                            self.search_state.matches.clear();
                            if let Some(renderer) = &mut self.renderer {
                                renderer.pane_damage_mut().force_full_damage_all();
                            }
                        } else {
                            // Open search
                            self.input_mode = InputMode::Search;
                            self.search_state.is_active = true;
                        }
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                        return;
                    }

                    // In search mode, intercept keys for search commands
                    if self.input_mode == InputMode::Search {
                        if let Some(cmd) = match_search_command(
                            &event.logical_key,
                            event.text.as_ref().map(|s| s.as_ref()),
                            self.modifiers,
                        ) {
                            self.handle_search_command(cmd);
                            return;
                        }
                        return; // Consume all keys in search mode
                    }

                    // Check for app-level commands (font size)
                    if let Some(cmd) =
                        match_app_command(&event.logical_key, self.modifiers)
                    {
                        self.handle_app_command(cmd);
                        return;
                    }

                    // Check for tab commands first
                    if let Some(cmd) =
                        match_tab_command(&event.logical_key, self.modifiers)
                    {
                        self.handle_tab_command(cmd, event_loop);
                        return;
                    }

                    // Then check for pane commands
                    if let Some(cmd) =
                        match_pane_command(&event.logical_key, self.modifiers)
                    {
                        self.handle_pane_command(cmd, event_loop);
                        return;
                    }

                    // Then check for shell integration commands
                    if let Some(cmd) =
                        crate::input::match_shell_command(&event.logical_key, self.modifiers)
                    {
                        self.handle_shell_command(cmd);
                        return;
                    }

                    // Check for vi-mode toggle (Ctrl+Shift+Space)
                    let focused_id = self
                        .tab_manager
                        .active_tab()
                        .pane_tree
                        .focused_pane_id();
                    if self.app_config.vi_mode.enabled
                        && crate::input::should_toggle_vi_mode(
                            &event.logical_key,
                            self.modifiers,
                        )
                    {
                        if let Some(state) = self.pane_states.get_mut(&focused_id) {
                            if state.vi_state.is_some() {
                                // Exit vi-mode
                                state.vi_state = None;
                                log::info!("Vi-mode deactivated for pane {:?}", focused_id);
                            } else {
                                // Enter vi-mode at terminal cursor position
                                let (row, col) = state.terminal.cursor_position();
                                state.vi_state =
                                    Some(crate::vi_mode::ViState::new(row, col));
                                log::info!("Vi-mode activated for pane {:?}", focused_id);
                            }
                        }
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                        return;
                    }

                    // If focused pane has vi-mode active, route keys there
                    if let Some(state) = self.pane_states.get_mut(&focused_id) {
                        if let Some(ref mut vi) = state.vi_state {
                            if let Some(ch) = Self::key_to_vi_char(
                                &event.logical_key,
                                event.text.as_ref().map(|s| s.as_ref()),
                            ) {
                                let ctrl = self.modifiers.control_key();
                                let action = vi.process_key(ch, ctrl);
                                self.handle_vi_action(action, focused_id);
                            }
                            if let Some(window) = &self.window {
                                window.request_redraw();
                            }
                            return; // PTY receives no input while vi-mode is active
                        }
                    }

                    // Check for clipboard commands (Cmd+C, Cmd+V, Cmd+A)
                    if crate::input::clipboard::is_copy_keybinding(&event.logical_key, self.modifiers) {
                        if let Some(state) = self.pane_states.get_mut(&focused_id) {
                            if let Some(ref sel) = state.mouse_selection.active_selection {
                                let cells = crate::terminal::grid_bridge::extract_grid_cells(&state.terminal);
                                let cols = state.terminal.columns();
                                let text = match sel.selection_type {
                                    crate::input::selection::SelectionType::VisualBlock => {
                                        crate::input::selection::selected_text_block(&cells, sel, cols)
                                    }
                                    crate::input::selection::SelectionType::Line => {
                                        crate::input::selection::selected_text_lines(&cells, sel, cols)
                                    }
                                    _ => crate::input::selection::selected_text(&cells, sel, cols),
                                };
                                if !text.is_empty() {
                                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                        let _ = clipboard.set_text(&text);
                                    }
                                }
                                state.mouse_selection.clear_selection();
                            }
                        }
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                        return;
                    }
                    if crate::input::clipboard::is_paste_keybinding(&event.logical_key, self.modifiers) {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            if let Ok(text) = clipboard.get_text() {
                                if let Some(state) = self.pane_states.get_mut(&focused_id) {
                                    let bytes = crate::input::clipboard::paste_bytes(&text, true);
                                    if let Err(e) = state.pty.write(&bytes) {
                                        log::warn!("PTY paste write error: {e}");
                                    }
                                }
                            }
                        }
                        return;
                    }
                    if crate::input::clipboard::is_select_all_keybinding(&event.logical_key, self.modifiers) {
                        if let Some(state) = self.pane_states.get_mut(&focused_id) {
                            let rows = state.terminal.rows();
                            let cols = state.terminal.columns();
                            state.mouse_selection.active_selection = Some(
                                crate::input::selection::Selection {
                                    start: (0, 0),
                                    end: (rows.saturating_sub(1), cols.saturating_sub(1)),
                                    selection_type: crate::input::selection::SelectionType::Range,
                                },
                            );
                        }
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                        return;
                    }

                    // Route normal keys to focused pane's PTY
                    let bytes = crate::input::translate_key(
                        &event.logical_key,
                        event.text.as_ref().map(|s| s.as_ref()),
                        event.state,
                        self.modifiers,
                    );
                    if let (Some(bytes), Some(state)) =
                        (bytes, self.pane_states.get_mut(&focused_id))
                    {
                        if let Err(e) = state.pty.write(&bytes) {
                            log::warn!("PTY write error: {e}");
                        }
                        state.cursor.on_keystroke();
                        state.mouse_selection.clear_selection();
                        // Snap scroll to bottom on keyboard input (return to live view)
                        state.scroll_state.snap_to_bottom();
                        state.terminal.set_display_offset(0);
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let y = position.y as f32;
                let chrome_top = Self::chrome_top_height();
                let tab_bar_top = HEADER_BAR_HEIGHT;
                let tab_bar_bottom = chrome_top;
                if y >= tab_bar_top && y < tab_bar_bottom {
                    // In tab bar area — track hovered tab
                    let (width, _) = self.window_size();
                    let tw = crate::tab::bar::tab_width(width as f32, self.tab_manager.tab_count());
                    let new_hovered = if tw > 0.0 && position.x >= 0.0 {
                        let idx = (position.x as f32 / tw) as usize;
                        if idx < self.tab_manager.tab_count() {
                            Some(idx)
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    if new_hovered != self.hovered_tab {
                        self.hovered_tab = new_hovered;
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                    // Tab drag-to-reorder: activate after 5px movement, swap on hover
                    if let Some(drag_idx) = self.tab_drag_index {
                        let dx = (position.x as f32 - self.tab_drag_start_x).abs();
                        if !self.tab_drag_active && dx > 5.0 {
                            self.tab_drag_active = true;
                        }
                        if self.tab_drag_active && tw > 0.0 {
                            let target = (position.x as f32 / tw) as usize;
                            let target = target.min(self.tab_manager.tab_count().saturating_sub(1));
                            if target != drag_idx {
                                self.tab_manager.move_tab(drag_idx, target);
                                self.tab_drag_index = Some(target);
                                if let Some(window) = &self.window {
                                    window.request_redraw();
                                }
                            }
                        }
                    }
                    // Reset pane interaction cursor and link hover
                    if self.link_hover_active {
                        self.link_hover_active = false;
                    }
                    let effect = self.interaction.on_cursor_moved(position.x as f32, -1.0);
                    self.apply_interaction_effect(effect);
                } else if y >= chrome_top {
                    // Clear tab hover when cursor leaves tab bar
                    if self.hovered_tab.is_some() {
                        self.hovered_tab = None;
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                    let content_y = y - chrome_top;
                    // Check for link hover when modifier is held
                    // Use focused pane's local coordinates
                    let on_link = self.update_link_hover(position.x as f32, content_y);

                    if on_link {
                        if let Some(window) = &self.window {
                            window.set_cursor(CursorIcon::Pointer);
                        }
                    } else {
                        // Below tab bar — offset y for pane interaction
                        let effect = self
                            .interaction
                            .on_cursor_moved(position.x as f32, content_y);
                        self.apply_interaction_effect(effect);
                    }

                    // Update scrollbar drag or text selection drag on focused pane
                    {
                        let focused_pane = self.tab_manager.active_tab().pane_tree.focused_pane_id();
                        let padding = self.renderer.as_ref().map(|r| r.padding()).unwrap_or([0.0; 4]);
                        let (win_w, win_h) = self.window_size();
                        let pgrid_bounds = self.pane_grid_bounds(win_w as f32, win_h as f32);
                        let layout = self.tab_manager.active_tab().pane_tree.calculate_layout(pgrid_bounds.width, pgrid_bounds.height);
                        let pane_rect = layout.iter().find(|(id, _)| *id == focused_pane).map(|(_, r)| *r);

                        let mut scrollbar_dragging = false;
                        // Check scrollbar drag first
                        if let Some(state) = self.pane_states.get_mut(&focused_pane) {
                            if state.scroll_state.is_dragging_scrollbar {
                                scrollbar_dragging = true;
                                if let Some(rect) = pane_rect {
                                    let track_height = rect.height - padding[0] - padding[1];
                                    let history_size = state.terminal.history_size();
                                    state.scroll_state.update_drag(content_y, track_height, history_size);
                                    let offset = state.scroll_state.current_line_offset();
                                    state.terminal.set_display_offset(offset);
                                }
                                if let Some(window) = &self.window {
                                    window.request_redraw();
                                }
                            }
                        }
                        // Fall back to text selection drag
                        if !scrollbar_dragging {
                            if let Some(renderer) = &self.renderer {
                                let cell_width = renderer.cell_width();
                                let cell_height = renderer.cell_height();
                                let padding = renderer.padding();
                                if let Some(state) = self.pane_states.get_mut(&focused_pane) {
                                    if state.mouse_selection.is_dragging {
                                        let local_x = position.x as f32 - padding[2];
                                        let local_y = content_y - padding[0];
                                        let cols = state.terminal.columns();
                                        let rows = state.terminal.rows();
                                        let cells = crate::terminal::grid_bridge::extract_grid_cells(&state.terminal);
                                        state.mouse_selection.on_mouse_drag(
                                            local_x, local_y, cell_width, cell_height, cols, rows, &cells,
                                        );
                                        if let Some(window) = &self.window {
                                            window.request_redraw();
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // Header bar region — clear hover state
                    if self.hovered_tab.is_some() {
                        self.hovered_tab = None;
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                }
            }
            WindowEvent::MouseInput {
                state: btn_state,
                button: MouseButton::Left,
                ..
            } => {
                let cursor_pos = self.interaction.cursor_pos();
                let chrome_top = Self::chrome_top_height();
                let raw_y = cursor_pos.1 + chrome_top; // reconstruct raw y

                // End tab drag on any mouse release
                if btn_state == ElementState::Released && self.tab_drag_index.is_some() {
                    self.tab_drag_index = None;
                    self.tab_drag_active = false;
                }

                if raw_y >= HEADER_BAR_HEIGHT && raw_y < chrome_top {
                    // Tab bar clicks handled by iced UI layer (Phase 2 migration)
                    // Drag-reorder tracking starts from iced TabSelected message
                } else {
                    // Check for modifier+click link activation first
                    if btn_state == ElementState::Pressed {
                        let content_y = cursor_pos.1; // already in content space
                        if self.handle_link_click(cursor_pos.0, content_y) {
                            return; // Link click consumed the event
                        }
                    }

                    // Check for scrollbar interaction before text selection
                    let mut scrollbar_handled = false;
                    {
                        let padding = self.renderer.as_ref().map(|r| r.padding()).unwrap_or([0.0; 4]);
                        let focused_pane = self.tab_manager.active_tab().pane_tree.focused_pane_id();
                        let (width, height) = self.window_size();
                        let pgrid = self.pane_grid_bounds(width as f32, height as f32);
                        let layout = self.tab_manager.active_tab().pane_tree.calculate_layout(pgrid.width, pgrid.height);
                        let screen_rect = layout.iter().find(|(id, _)| *id == focused_pane).map(|(_, r)| {
                            Rect::new(r.x + pgrid.x, r.y + pgrid.y, r.width, r.height)
                        });
                        if let (Some(screen_rect), Some(state)) = (screen_rect, self.pane_states.get_mut(&focused_pane)) {
                            let history_size = state.terminal.history_size();
                            let visible_rows = state.terminal.rows();
                            let display_offset = state.scroll_state.current_line_offset();
                            match btn_state {
                                ElementState::Pressed => {
                                    let hit = crate::scroll::scrollbar_hit_test(
                                        cursor_pos.0, cursor_pos.1,
                                        screen_rect.x, screen_rect.y,
                                        screen_rect.width, screen_rect.height,
                                        padding, visible_rows, history_size, display_offset,
                                    );
                                    match hit {
                                        crate::scroll::ScrollbarHit::Track(y) => {
                                            let offset = crate::scroll::track_click_to_offset(
                                                y, screen_rect.y, screen_rect.height,
                                                padding, history_size,
                                            );
                                            state.scroll_state.set_target(offset, history_size);
                                            state.terminal.set_display_offset(offset);
                                            scrollbar_handled = true;
                                        }
                                        crate::scroll::ScrollbarHit::Thumb(y) => {
                                            state.scroll_state.begin_drag(y);
                                            scrollbar_handled = true;
                                        }
                                        crate::scroll::ScrollbarHit::None => {}
                                    }
                                }
                                ElementState::Released => {
                                    if state.scroll_state.is_dragging_scrollbar {
                                        state.scroll_state.end_drag();
                                        scrollbar_handled = true;
                                    }
                                }
                            }
                        }
                    }

                    // Handle text selection on focused pane (if not handled by scrollbar)
                    if !scrollbar_handled {
                        if let Some(renderer) = &self.renderer {
                            let cell_width = renderer.cell_width();
                            let cell_height = renderer.cell_height();
                            let padding = renderer.padding();
                            let focused_pane = self.tab_manager.active_tab().pane_tree.focused_pane_id();
                            if let Some(state) = self.pane_states.get_mut(&focused_pane) {
                                let cols = state.terminal.columns();
                                let rows = state.terminal.rows();
                                // Convert from content-space to pane-local coords (subtract padding)
                                let local_x = cursor_pos.0 - padding[2]; // subtract left padding
                                let local_y = cursor_pos.1 - padding[0]; // subtract top padding
                                match btn_state {
                                    ElementState::Pressed => {
                                        let cells = crate::terminal::grid_bridge::extract_grid_cells(&state.terminal);
                                        if self.modifiers.shift_key() {
                                            let (crow, ccol) = state.terminal.cursor_position();
                                            state.mouse_selection.on_shift_click(
                                                local_x, local_y, cell_width, cell_height, cols, rows, crow, ccol,
                                            );
                                        } else {
                                            state.mouse_selection.on_mouse_press(
                                                local_x, local_y, cell_width, cell_height, cols, rows, &cells,
                                            );
                                        }
                                    }
                                    ElementState::Released => {
                                        state.mouse_selection.on_mouse_release();
                                    }
                                }
                            }
                        }
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }

                    // Click below tab bar — route to pane interaction
                    let (width, height) = self.window_size();
                    let content = self.content_bounds(width as f32, height as f32);
                    let pane_tree = &self.tab_manager.active_tab().pane_tree;
                    let layout = pane_tree.calculate_layout(content.width, content.height);
                    let effect = match btn_state {
                        ElementState::Pressed => self.interaction.on_mouse_press(&layout),
                        ElementState::Released => self.interaction.on_mouse_release(),
                    };
                    self.apply_interaction_effect(effect);
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Right,
                ..
            } => {
                let cursor_pos = self.interaction.cursor_pos();
                let chrome_top = Self::chrome_top_height();
                let raw_y = cursor_pos.1 + chrome_top;

                if raw_y < chrome_top && raw_y >= HEADER_BAR_HEIGHT {
                    // Right-click on tab bar — show tab context menu
                    if let Some(window) = &self.window {
                        if let Some(action) =
                            crate::context_menu::show_tab_context_menu(window)
                        {
                            self.handle_context_menu_action(action, event_loop);
                        }
                    }
                } else if raw_y >= chrome_top {
                    // Right-click in content area — show terminal context menu
                    let focused_pane = self.tab_manager.active_tab().pane_tree.focused_pane_id();
                    let has_selection = self
                        .pane_states
                        .get(&focused_pane)
                        .map_or(false, |s| s.mouse_selection.has_selection());
                    if let Some(window) = &self.window {
                        if let Some(action) =
                            crate::context_menu::show_context_menu(has_selection, window)
                        {
                            self.handle_context_menu_action(action, event_loop);
                        }
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let focused_pane = self.tab_manager.active_tab().pane_tree.focused_pane_id();
                if let Some(state) = self.pane_states.get_mut(&focused_pane) {
                    let history_size = state.terminal.history_size();
                    match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, y) => {
                            state.scroll_state.apply_line_delta(y, history_size);
                        }
                        winit::event::MouseScrollDelta::PixelDelta(pos) => {
                            let cell_height = self
                                .renderer
                                .as_ref()
                                .map(|r| r.cell_height())
                                .unwrap_or(20.0);
                            state.scroll_state.apply_pixel_delta(
                                pos.y as f32,
                                cell_height,
                                history_size,
                            );
                        }
                    }
                    let offset = state.scroll_state.current_line_offset();
                    state.terminal.set_display_offset(offset);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::Resized(size) => {
                log::debug!("Window resized to {}x{}", size.width, size.height);
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                    renderer.pane_damage_mut().force_full_damage_all();
                }
                self.resize_all_panes(size.width, size.height);
                self.update_interaction_layout(size.width, size.height);
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                log::debug!("Scale factor changed to {scale_factor:.2}");
            }
            WindowEvent::Focused(focused) => {
                for state in self.pane_states.values_mut() {
                    state.cursor.set_focused(focused);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
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

                // Drain PTY output into terminals for all panes, update cursor positions
                for state in self.pane_states.values_mut() {
                    while let Ok(bytes) = state.pty.reader_rx.try_recv() {
                        state.terminal.feed(&bytes);
                    }
                    // Check for bell events
                    if state.terminal.take_bell() && self.app_config.shell.bell_enabled {
                        self.bell_flash_until = Some(
                            std::time::Instant::now() + std::time::Duration::from_millis(150),
                        );
                    }
                    // Sync cursor position from terminal state
                    let (row, col) = state.terminal.cursor_position();
                    state.cursor.update_position(row, col);
                    state.cursor.tick_blink();
                    // Tick scroll animation (~60fps assumed)
                    state.scroll_state.tick(1.0 / 60.0);
                    let offset = state.scroll_state.current_line_offset();
                    state.terminal.set_display_offset(offset);
                    // Clamp scroll to current history size (may have shrunk)
                    state.scroll_state.clamp_to_history(state.terminal.history_size());
                }

                // Process shell integration: notifications and CWD tab titles
                self.process_shell_updates();

                // Rescan links for the focused pane after PTY drain
                self.rescan_links();

                // Build render descriptors for active tab's visible panes
                let pgrid = self.pane_grid_bounds(width as f32, height as f32);
                let pane_tree = &self.tab_manager.active_tab().pane_tree;
                let layout = pane_tree.calculate_layout(pgrid.width, pgrid.height);
                let visible = pane_tree.visible_panes();

                let focused_pane = pane_tree.focused_pane_id();
                let mut pane_descs: Vec<PaneRenderDescriptor> = Vec::new();
                for (pane_id, rect) in &layout {
                    if !visible.contains(pane_id) {
                        continue;
                    }
                    if let Some(state) = self.pane_states.get(pane_id) {
                        let mut cells =
                            crate::terminal::grid_bridge::extract_grid_cells(&state.terminal);

                        // Apply search highlights to the focused pane
                        if self.search_state.is_active && *pane_id == focused_pane {
                            let theme = self.renderer.as_ref().unwrap().theme();
                            let cols = state.terminal.columns();
                            let viewport_rows = state.terminal.rows() as i32;
                            let offset = state.terminal.display_offset() as i32;
                            let visible_matches = self.search_state.visible_matches(
                                -offset,
                                -offset + viewport_rows - 1,
                                0,
                            );
                            // Convert visible matches to viewport-relative rows
                            let viewport_matches: Vec<crate::search::SearchMatch> = visible_matches
                                .iter()
                                .map(|m| crate::search::SearchMatch {
                                    row: m.row + offset,
                                    start_col: m.start_col,
                                    end_col: m.end_col,
                                })
                                .collect();
                            // Find the current match index in the viewport_matches
                            let current_viewport_idx = if let Some(current) = self.search_state.current_match() {
                                viewport_matches.iter().position(|m| {
                                    m.row == current.row + offset
                                        && m.start_col == current.start_col
                                        && m.end_col == current.end_col
                                }).unwrap_or(usize::MAX)
                            } else {
                                usize::MAX
                            };
                            crate::search::highlight::apply_search_highlights(
                                &mut cells,
                                &viewport_matches,
                                current_viewport_idx,
                                cols,
                                theme.search_match,
                                theme.search_match_active,
                            );
                        }

                        // Apply mouse selection highlight flags
                        if let Some(ref sel) = state.mouse_selection.active_selection {
                            let cols = state.terminal.columns();
                            crate::input::selection::apply_selection_flags(&mut cells, sel, cols);
                        }

                        // Offset rect by pane grid origin + pane header for screen-space rendering
                        let screen_rect = Rect::new(
                            rect.x + pgrid.x,
                            rect.y + pgrid.y + PANE_HEADER_HEIGHT,
                            rect.width,
                            (rect.height - PANE_HEADER_HEIGHT).max(0.0),
                        );
                        // Generate cursor instance for this pane
                        let cursor_instance = if let Some(state) = self.pane_states.get(pane_id) {
                            state.cursor.to_cell_instance()
                        } else {
                            None
                        };

                        pane_descs.push(PaneRenderDescriptor {
                            pane_id: *pane_id,
                            rect: screen_rect,
                            cells,
                            cursor_instance,
                        });
                    }
                }

                // Build iced UI state from current application state (before mutable borrow)
                let ui_tabs: Vec<TabInfo> = self.tab_manager.tabs().iter().enumerate().map(|(i, tab)| TabInfo {
                    title: tab.title.clone(),
                    is_active: i == self.tab_manager.active_index(),
                    has_notification: tab.has_notification,
                }).collect();
                let ui_active_tab = self.tab_manager.active_index();
                let ui_hovered_tab = self.hovered_tab;
                let ui_scale = self.window.as_ref().map(|w| w.scale_factor() as f32).unwrap_or(1.0);
                let pane_tree = &self.tab_manager.active_tab().pane_tree;
                let ui_pane_count = pane_tree.pane_count();
                let ui_is_zoomed = pane_tree.is_zoomed();
                let now = std::time::Instant::now();
                let (ui_active_pane, ui_panes, ui_dividers) = {
                    let pane_ids: Vec<_> = pane_tree.visible_panes();
                    let focused = pane_tree.focused_pane_id();
                    let active_idx = pane_ids.iter().position(|id| *id == focused).unwrap_or(0);
                    let pgrid = self.pane_grid_bounds(width as f32, height as f32);
                    let chrome_top = Self::chrome_top_height();
                    let layout = pane_tree.calculate_layout(pgrid.width, pgrid.height);
                    let padding = self.renderer.as_ref().map(|r| r.padding()).unwrap_or([0.0; 4]);
                    let panes: Vec<PaneInfo> = layout.iter().enumerate().map(|(idx, (pane_id, rect))| {
                        let title = self.pane_states.get(pane_id)
                            .and_then(|s| s.terminal.shell_state().cwd.clone())
                            .map(|cwd| crate::shell_integration::dir_name_from_path(&cwd).to_string())
                            .unwrap_or_else(|| "Shell".to_string());

                        // Compute scrollbar thumb for this pane
                        let (scrollbar_thumb, scrollbar_alpha) = if let Some(state) = self.pane_states.get(pane_id) {
                            let alpha = state.scroll_state.scrollbar_alpha(now);
                            if alpha > 0.0 {
                                let history_size = state.terminal.history_size();
                                let visible_rows = state.terminal.rows();
                                let display_offset = state.scroll_state.current_line_offset();
                                let screen_rect = Rect::new(
                                    rect.x + pgrid.x,
                                    rect.y + pgrid.y,
                                    rect.width,
                                    rect.height,
                                );
                                let thumb = crate::scroll::scrollbar_thumb_rect(
                                    screen_rect.x, screen_rect.y,
                                    screen_rect.width, screen_rect.height,
                                    padding, visible_rows, history_size, display_offset,
                                ).map(|t| (t.x, t.y - chrome_top, t.width, t.height));
                                (thumb, alpha)
                            } else {
                                (None, 0.0)
                            }
                        } else {
                            (None, 0.0)
                        };

                        PaneInfo {
                            // Positions relative to content area (subtract chrome_top from screen y)
                            x: rect.x + pgrid.x,
                            y: rect.y + pgrid.y - chrome_top,
                            width: rect.width,
                            height: rect.height,
                            is_focused: *pane_id == focused,
                            index: idx,
                            title,
                            shell_name: "bash".to_string(),
                            scrollbar_thumb,
                            scrollbar_alpha,
                        }
                    }).collect();

                    // Build divider display info
                    let dividers: Vec<DividerDisplay> = if pane_tree.pane_count() > 1 && !pane_tree.is_zoomed() {
                        let hovered_index = match self.interaction.state() {
                            crate::pane::interaction::InteractionState::Hovering { divider_index } => {
                                Some(*divider_index)
                            }
                            _ => None,
                        };
                        self.interaction.dividers().iter().enumerate().map(|(i, d)| {
                            DividerDisplay {
                                x: d.rect.x,
                                y: d.rect.y - chrome_top,
                                width: d.rect.width,
                                height: d.rect.height,
                                is_hovered: Some(i) == hovered_index,
                            }
                        }).collect()
                    } else {
                        Vec::new()
                    };

                    (active_idx, panes, dividers)
                };

                if let Some(renderer) = &mut self.renderer {
                    let theme_clone = renderer.theme().clone();
                    let ui_state = UiState {
                        tabs: ui_tabs,
                        active_tab_index: ui_active_tab,
                        hovered_tab: ui_hovered_tab,
                        active_pane_index: ui_active_pane,
                        panes: ui_panes,
                        pane_count: ui_pane_count,
                        is_zoomed: ui_is_zoomed,
                        theme: &theme_clone,
                        window_width: width as f32,
                        window_height: height as f32,
                        scale_factor: ui_scale,
                        search_active: self.search_state.is_active,
                        search_query: self.search_state.query.clone(),
                        search_current: self.search_state.current_index + 1,
                        search_total: self.search_state.total_count(),
                        search_error: self.search_state.error.is_some(),
                        dividers: ui_dividers,
                        bell_flash: self.bell_flash_until.is_some_and(|t| std::time::Instant::now() < t),
                    };

                    let mut iced_msgs = Vec::new();
                    match renderer.render_panes(&mut pane_descs, &ui_state) {
                        Ok((surface_texture, messages)) => {
                            iced_msgs = messages;
                            // Take screenshot if requested
                            if self.screenshot_requested {
                                self.screenshot_requested = false;
                                let path = match std::env::var("VELOTERM_PROJECT_DIR") {
                                    Ok(dir) => std::path::PathBuf::from(dir).join("veloterm-latest.png"),
                                    Err(_) => std::path::PathBuf::from("veloterm-latest.png"),
                                };
                                match renderer.capture_screenshot(&surface_texture.texture, &path) {
                                    Ok(_) => {},
                                    Err(e) => log::error!("✗ Screenshot failed: {}", e),
                                }
                            }
                            surface_texture.present();
                        }
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

                    // Process iced UI messages (tab interactions)
                    for msg in iced_msgs {
                        match msg {
                            UiMessage::TabSelected(idx) => {
                                self.handle_tab_command(TabCommand::SelectTab(idx), event_loop);
                                // Begin drag tracking
                                self.tab_drag_index = Some(idx);
                                self.tab_drag_start_x = self.interaction.cursor_pos().0;
                                self.tab_drag_active = false;
                            }
                            UiMessage::TabClosed(idx) => {
                                let pane_ids = self.tab_manager.close_tab(idx);
                                if let Some(ids) = pane_ids {
                                    for id in &ids {
                                        self.pane_states.remove(id);
                                        if let Some(r) = &mut self.renderer {
                                            r.remove_pane_damage(*id);
                                        }
                                    }
                                    let (w, h) = self.window_size();
                                    self.update_interaction_layout(w, h);
                                    if let Some(r) = &mut self.renderer {
                                        r.pane_damage_mut().force_full_damage_all();
                                    }
                                } else {
                                    log::info!("Last tab closed via iced close button, exiting");
                                    event_loop.exit();
                                }
                            }
                            UiMessage::NewTab => {
                                self.handle_tab_command(TabCommand::NewTab, event_loop);
                            }
                            UiMessage::TabHovered(idx) => {
                                if idx != self.hovered_tab {
                                    self.hovered_tab = idx;
                                    if let Some(window) = &self.window {
                                        window.request_redraw();
                                    }
                                }
                            }
                            UiMessage::Noop => {}
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
        assert_eq!(cfg.title, "Claude Terminal \u{2014} Anthropic");
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
        assert_eq!(attrs.inner_size, Some(expected.into()));
    }

    #[test]
    fn window_attributes_has_correct_title() {
        let cfg = WindowConfig::default();
        let attrs = cfg.to_window_attributes();
        assert_eq!(attrs.title, "Claude Terminal \u{2014} Anthropic");
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
        assert_eq!(app.app_config.font.size, 18.0);
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
        let result = Theme::from_name("nonexistent");
        assert!(result.is_none());
        let fallback = result.unwrap_or_else(Theme::claude_dark);
        assert_eq!(fallback.name, "Claude Dark");
    }

    #[test]
    fn app_config_scrollback_passed_to_terminal() {
        let scrollback = 5000_usize;
        let terminal = crate::terminal::Terminal::new(80, 24, scrollback);
        assert_eq!(terminal.history_size(), 0);
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

    // ── TabManager integration ──────────────────────────────────────

    #[test]
    fn app_creates_single_tab_on_startup() {
        let app = App::new(WindowConfig::default(), Config::default());
        assert_eq!(app.tab_manager.tab_count(), 1);
        assert_eq!(app.tab_manager.active_tab().pane_tree.pane_count(), 1);
    }

    #[test]
    fn app_pane_states_empty_before_resumed() {
        let app = App::new(WindowConfig::default(), Config::default());
        assert!(app.pane_states.is_empty());
    }

    #[test]
    fn app_active_pane_is_initial() {
        let app = App::new(WindowConfig::default(), Config::default());
        let pane_tree = &app.tab_manager.active_tab().pane_tree;
        let ids = pane_tree.pane_ids();
        assert_eq!(pane_tree.focused_pane_id(), ids[0]);
    }

    #[test]
    fn app_spawn_pane_adds_state() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        let pane_id = app.tab_manager.active_tab().pane_tree.focused_pane_id();
        app.spawn_pane(pane_id, 80, 24);
        assert!(app.pane_states.contains_key(&pane_id));
    }

    #[test]
    fn app_spawn_pane_terminal_has_correct_dims() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        let pane_id = app.tab_manager.active_tab().pane_tree.focused_pane_id();
        app.spawn_pane(pane_id, 120, 40);
        let state = app.pane_states.get(&pane_id).unwrap();
        assert_eq!(state.terminal.columns(), 120);
        assert_eq!(state.terminal.rows(), 40);
    }

    #[test]
    fn app_pane_count_matches_tree_leaf_count() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        let pane_id = app.tab_manager.active_tab().pane_tree.focused_pane_id();
        app.spawn_pane(pane_id, 80, 24);
        assert_eq!(
            app.tab_manager.active_tab().pane_tree.pane_count(),
            app.pane_states.len()
        );
    }

    #[test]
    fn app_multiple_panes_have_independent_terminals() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        let first_id = app.tab_manager.active_tab().pane_tree.focused_pane_id();
        app.spawn_pane(first_id, 80, 24);

        let second_id = app
            .tab_manager
            .active_tab_mut()
            .pane_tree
            .split_focused(SplitDirection::Vertical)
            .unwrap();
        app.spawn_pane(second_id, 60, 20);

        let s1 = app.pane_states.get(&first_id).unwrap();
        let s2 = app.pane_states.get(&second_id).unwrap();
        assert_eq!(s1.terminal.columns(), 80);
        assert_eq!(s2.terminal.columns(), 60);
    }

    #[test]
    fn app_close_pane_removes_state() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        let first_id = app.tab_manager.active_tab().pane_tree.focused_pane_id();
        app.spawn_pane(first_id, 80, 24);

        let second_id = app
            .tab_manager
            .active_tab_mut()
            .pane_tree
            .split_focused(SplitDirection::Vertical)
            .unwrap();
        app.spawn_pane(second_id, 80, 24);

        let closing_id = app.tab_manager.active_tab().pane_tree.focused_pane_id();
        app.tab_manager.active_tab_mut().pane_tree.close_focused();
        app.pane_states.remove(&closing_id);

        assert_eq!(app.tab_manager.active_tab().pane_tree.pane_count(), 1);
        assert_eq!(app.pane_states.len(), 1);
        assert!(!app.pane_states.contains_key(&closing_id));
    }

    #[test]
    fn app_grid_dims_for_rect_default_without_renderer() {
        let app = App::new(WindowConfig::default(), Config::default());
        let rect = Rect::new(0.0, 0.0, 640.0, 480.0);
        let (cols, rows) = app.grid_dims_for_rect(&rect);
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
        app.update_interaction_layout(1280, 720);
        assert!(app.interaction().dividers().is_empty());

        app.tab_manager
            .active_tab_mut()
            .pane_tree
            .split_focused(SplitDirection::Vertical);
        app.update_interaction_layout(1280, 720);
        assert_eq!(app.interaction().dividers().len(), 1);
    }

    #[test]
    fn app_apply_focus_pane_changes_focus() {
        use crate::pane::interaction::InteractionEffect;

        let mut app = App::new(WindowConfig::default(), Config::default());
        let first_id = app.tab_manager.active_tab().pane_tree.focused_pane_id();
        let second_id = app
            .tab_manager
            .active_tab_mut()
            .pane_tree
            .split_focused(SplitDirection::Vertical)
            .unwrap();
        assert_eq!(
            app.tab_manager.active_tab().pane_tree.focused_pane_id(),
            second_id
        );

        app.apply_interaction_effect(InteractionEffect::FocusPane(first_id));
        assert_eq!(
            app.tab_manager.active_tab().pane_tree.focused_pane_id(),
            first_id
        );
    }

    #[test]
    fn app_apply_update_ratio_changes_layout() {
        use crate::pane::interaction::InteractionEffect;

        let mut app = App::new(WindowConfig::default(), Config::default());
        app.tab_manager
            .active_tab_mut()
            .pane_tree
            .split_focused(SplitDirection::Vertical);
        app.update_interaction_layout(1000, 500);

        app.apply_interaction_effect(InteractionEffect::UpdateRatio {
            split_index: 0,
            new_ratio: 0.3,
        });

        let content = app.content_bounds(1000.0, 500.0);
        let layout = app
            .tab_manager
            .active_tab()
            .pane_tree
            .calculate_layout(content.width, content.height);
        let first_width = layout[0].1.width;
        // With 8px gap: usable = 992, 30% = 297.6
        let expected = (content.width - crate::pane::PANE_GAP) * 0.3;
        assert!((first_width - expected).abs() < 1.0);
    }

    #[test]
    fn app_cursor_moved_updates_interaction_state() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        app.tab_manager
            .active_tab_mut()
            .pane_tree
            .split_focused(SplitDirection::Vertical);
        // Content area is offset by chrome top, so divider is at x=640
        // in content coordinates (0.0 to 1280.0 width, 0.0 to 692.0 height)
        let content = app.content_bounds(1280.0, 720.0);
        app.interaction.update_layout(
            app.tab_manager.active_tab().pane_tree.root(),
            content,
            20.0,
        );

        // Move cursor to divider (x=640, content y=346)
        let effect = app.interaction.on_cursor_moved(640.0, 346.0);
        assert!(matches!(
            effect,
            crate::pane::interaction::InteractionEffect::SetCursor(
                crate::pane::interaction::CursorType::EwResize
            )
        ));
    }

    #[test]
    fn app_mouse_press_in_pane_produces_focus_effect() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        let first_id = app.tab_manager.active_tab().pane_tree.focused_pane_id();
        let _second_id = app
            .tab_manager
            .active_tab_mut()
            .pane_tree
            .split_focused(SplitDirection::Vertical)
            .unwrap();
        let content = app.content_bounds(1280.0, 720.0);
        app.interaction.update_layout(
            app.tab_manager.active_tab().pane_tree.root(),
            content,
            20.0,
        );

        // Move cursor to left pane in content coordinates
        app.interaction.on_cursor_moved(100.0, 300.0);

        let pane_tree = &app.tab_manager.active_tab().pane_tree;
        let layout = pane_tree.calculate_layout(content.width, content.height);
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
        app.tab_manager
            .active_tab_mut()
            .pane_tree
            .split_focused(SplitDirection::Vertical);
        app.update_interaction_layout(1000, 500);

        app.interaction.on_cursor_moved(500.0, 236.0); // hover on divider
        let content = app.content_bounds(1000.0, 500.0);
        let pane_tree = &app.tab_manager.active_tab().pane_tree;
        let layout = pane_tree.calculate_layout(content.width, content.height);
        app.interaction.on_mouse_press(&layout);
        let effect = app.interaction.on_cursor_moved(300.0, 236.0);

        app.apply_interaction_effect(effect);

        let layout = app
            .tab_manager
            .active_tab()
            .pane_tree
            .calculate_layout(content.width, content.height);
        let first_width = layout[0].1.width;
        assert!(
            first_width < 400.0,
            "first pane should be narrower after drag left, got {first_width}"
        );
    }

    // ── Tab integration tests ─────────────────────────────────────

    #[test]
    fn app_new_tab_creates_second_tab() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        let pane_id = app.tab_manager.active_tab().pane_tree.focused_pane_id();
        app.spawn_pane(pane_id, 80, 24);

        // Simulate new tab
        app.tab_manager.new_tab();
        let new_pane_id = app.tab_manager.active_tab().pane_tree.focused_pane_id();
        app.spawn_pane(new_pane_id, 80, 24);

        assert_eq!(app.tab_manager.tab_count(), 2);
        assert_eq!(app.tab_manager.active_index(), 1);
        assert_eq!(app.pane_states.len(), 2);
    }

    #[test]
    fn app_tab_switch_preserves_pane_states() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        let p1 = app.tab_manager.active_tab().pane_tree.focused_pane_id();
        app.spawn_pane(p1, 80, 24);

        app.tab_manager.new_tab();
        let p2 = app.tab_manager.active_tab().pane_tree.focused_pane_id();
        app.spawn_pane(p2, 80, 24);

        // Switch back to tab 0
        app.tab_manager.select_tab(0);
        assert_eq!(
            app.tab_manager.active_tab().pane_tree.focused_pane_id(),
            p1
        );
        // Both pane states still exist
        assert!(app.pane_states.contains_key(&p1));
        assert!(app.pane_states.contains_key(&p2));
    }

    #[test]
    fn app_close_tab_removes_pane_states() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        let p1 = app.tab_manager.active_tab().pane_tree.focused_pane_id();
        app.spawn_pane(p1, 80, 24);

        app.tab_manager.new_tab();
        let p2 = app.tab_manager.active_tab().pane_tree.focused_pane_id();
        app.spawn_pane(p2, 80, 24);

        // Close tab 1 (active)
        if let Some(pane_ids) = app.tab_manager.close_tab(1) {
            for id in &pane_ids {
                app.pane_states.remove(id);
            }
        }

        assert_eq!(app.tab_manager.tab_count(), 1);
        assert_eq!(app.pane_states.len(), 1);
        assert!(app.pane_states.contains_key(&p1));
        assert!(!app.pane_states.contains_key(&p2));
    }

    #[test]
    fn app_content_bounds_accounts_for_chrome() {
        let app = App::new(WindowConfig::default(), Config::default());
        let content = app.content_bounds(1280.0, 720.0);
        let chrome_top = App::chrome_top_height();
        assert_eq!(content.y, chrome_top);
        assert_eq!(content.width, 1280.0);
        assert_eq!(content.height, 720.0 - chrome_top - STATUS_BAR_HEIGHT);
    }

    #[test]
    fn app_tab_with_splits_close_cleans_all_panes() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        let p1 = app.tab_manager.active_tab().pane_tree.focused_pane_id();
        app.spawn_pane(p1, 80, 24);

        // Split to create second pane in first tab
        let p2 = app
            .tab_manager
            .active_tab_mut()
            .pane_tree
            .split_focused(SplitDirection::Vertical)
            .unwrap();
        app.spawn_pane(p2, 80, 24);

        // Create second tab
        app.tab_manager.new_tab();
        let p3 = app.tab_manager.active_tab().pane_tree.focused_pane_id();
        app.spawn_pane(p3, 80, 24);

        // Close first tab (index 0) which has 2 panes
        if let Some(pane_ids) = app.tab_manager.close_tab(0) {
            for id in &pane_ids {
                app.pane_states.remove(id);
            }
        }

        assert_eq!(app.tab_manager.tab_count(), 1);
        assert_eq!(app.pane_states.len(), 1);
        assert!(!app.pane_states.contains_key(&p1));
        assert!(!app.pane_states.contains_key(&p2));
        assert!(app.pane_states.contains_key(&p3));
    }

    // ── Link detection integration ──────────────────────────────────

    #[test]
    fn app_link_detector_starts_empty() {
        let app = App::new(WindowConfig::default(), Config::default());
        assert!(app.link_detector().links().is_empty());
        assert_eq!(app.link_detector().generation(), 0);
    }

    #[test]
    fn app_link_hover_starts_inactive() {
        let app = App::new(WindowConfig::default(), Config::default());
        assert!(!app.link_hover_active);
    }

    #[test]
    fn app_rescan_links_detects_urls_in_terminal() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        let pane_id = app.tab_manager.active_tab().pane_tree.focused_pane_id();
        // Create terminal directly (no PTY needed for this test)
        let terminal = crate::terminal::Terminal::new(80, 24, 10_000);
        app.pane_states.insert(
            pane_id,
            PaneState {
                terminal,
                pty: crate::pty::PtySession::new(&crate::pty::default_shell(), 80, 24).unwrap(),
                vi_state: None,
                cursor: crate::renderer::cursor::CursorState::new(),
                mouse_selection: crate::input::mouse::MouseSelectionState::new(),
                scroll_state: crate::scroll::ScrollState::new(),
            },
        );

        // Feed a URL into the terminal
        app.pane_states
            .get_mut(&pane_id)
            .unwrap()
            .terminal
            .feed(b"Visit https://example.com here");

        app.rescan_links();
        assert_eq!(app.link_detector().generation(), 1);
        assert_eq!(app.link_detector().links().len(), 1);
        assert_eq!(app.link_detector().links()[0].text, "https://example.com");
    }

    #[test]
    fn app_rescan_links_detects_file_paths() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        let pane_id = app.tab_manager.active_tab().pane_tree.focused_pane_id();
        let terminal = crate::terminal::Terminal::new(80, 24, 10_000);
        app.pane_states.insert(
            pane_id,
            PaneState {
                terminal,
                pty: crate::pty::PtySession::new(&crate::pty::default_shell(), 80, 24).unwrap(),
                vi_state: None,
                cursor: crate::renderer::cursor::CursorState::new(),
                mouse_selection: crate::input::mouse::MouseSelectionState::new(),
                scroll_state: crate::scroll::ScrollState::new(),
            },
        );

        app.pane_states
            .get_mut(&pane_id)
            .unwrap()
            .terminal
            .feed(b"Edit /usr/local/bin/test here");

        app.rescan_links();
        assert_eq!(app.link_detector().links().len(), 1);
        assert_eq!(
            app.link_detector().links()[0].kind,
            crate::link::LinkKind::FilePath
        );
    }

    #[test]
    fn app_is_link_modifier_disabled_when_links_disabled() {
        let mut config = Config::default();
        config.links.enabled = false;
        let app = App::new(WindowConfig::default(), config);
        assert!(!app.is_link_modifier_held());
    }

    // ── Vi-mode integration ────────────────────────────────────────

    #[test]
    fn pane_state_vi_mode_initially_none() {
        let app = App::new(WindowConfig::default(), Config::default());
        let pane_id = app.tab_manager.active_tab().pane_tree.focused_pane_id();
        if let Some(state) = app.pane_states.get(&pane_id) {
            assert!(state.vi_state.is_none());
        }
    }

    #[test]
    fn vi_mode_toggle_activates_when_enabled() {
        let mut config = Config::default();
        config.vi_mode.enabled = true;
        let mut app = App::new(WindowConfig::default(), config);
        let pane_id = app.tab_manager.active_tab().pane_tree.focused_pane_id();
        app.spawn_pane(pane_id, 80, 24);

        // vi_state should start as None
        assert!(app.pane_states.get(&pane_id).unwrap().vi_state.is_none());

        // Activate vi-mode
        app.pane_states.get_mut(&pane_id).unwrap().vi_state =
            Some(crate::vi_mode::ViState::new(0, 0));
        assert!(app.pane_states.get(&pane_id).unwrap().vi_state.is_some());

        // Deactivate vi-mode
        app.pane_states.get_mut(&pane_id).unwrap().vi_state = None;
        assert!(app.pane_states.get(&pane_id).unwrap().vi_state.is_none());
    }

    #[test]
    fn vi_mode_per_pane_independence() {
        let mut config = Config::default();
        config.vi_mode.enabled = true;
        let mut app = App::new(WindowConfig::default(), config);
        let p1 = app.tab_manager.active_tab().pane_tree.focused_pane_id();
        app.spawn_pane(p1, 80, 24);

        let p2 = app
            .tab_manager
            .active_tab_mut()
            .pane_tree
            .split_focused(SplitDirection::Vertical)
            .unwrap();
        app.spawn_pane(p2, 80, 24);

        // Activate vi-mode on pane 1 only
        app.pane_states.get_mut(&p1).unwrap().vi_state =
            Some(crate::vi_mode::ViState::new(0, 0));

        // p1 has vi-mode, p2 does not
        assert!(app.pane_states.get(&p1).unwrap().vi_state.is_some());
        assert!(app.pane_states.get(&p2).unwrap().vi_state.is_none());
    }

    #[test]
    fn key_to_vi_char_maps_characters() {
        let ch = App::key_to_vi_char(&Key::Character("v".into()), None);
        assert_eq!(ch, Some('v'));
    }

    #[test]
    fn key_to_vi_char_maps_escape() {
        let ch = App::key_to_vi_char(&Key::Named(NamedKey::Escape), None);
        assert_eq!(ch, Some('\x1b'));
    }

    #[test]
    fn key_to_vi_char_maps_enter() {
        let ch = App::key_to_vi_char(&Key::Named(NamedKey::Enter), None);
        assert_eq!(ch, Some('\r'));
    }

    #[test]
    fn key_to_vi_char_maps_backspace() {
        let ch = App::key_to_vi_char(&Key::Named(NamedKey::Backspace), None);
        assert_eq!(ch, Some('\x7f'));
    }

    #[test]
    fn handle_vi_action_exit_clears_state() {
        let mut config = Config::default();
        config.vi_mode.enabled = true;
        let mut app = App::new(WindowConfig::default(), config);
        let pane_id = app.tab_manager.active_tab().pane_tree.focused_pane_id();
        app.spawn_pane(pane_id, 80, 24);

        app.pane_states.get_mut(&pane_id).unwrap().vi_state =
            Some(crate::vi_mode::ViState::new(0, 0));
        assert!(app.pane_states.get(&pane_id).unwrap().vi_state.is_some());

        app.handle_vi_action(crate::vi_mode::ViAction::ExitViMode, pane_id);
        assert!(app.pane_states.get(&pane_id).unwrap().vi_state.is_none());
    }

    #[test]
    fn handle_vi_action_motion_updates_cursor() {
        let mut config = Config::default();
        config.vi_mode.enabled = true;
        let mut app = App::new(WindowConfig::default(), config);
        let pane_id = app.tab_manager.active_tab().pane_tree.focused_pane_id();
        app.spawn_pane(pane_id, 80, 24);

        app.pane_states.get_mut(&pane_id).unwrap().vi_state =
            Some(crate::vi_mode::ViState::new(0, 5));

        app.handle_vi_action(
            crate::vi_mode::ViAction::Motion(crate::vi_mode::Motion::CharLeft(3)),
            pane_id,
        );

        let vi = app
            .pane_states
            .get(&pane_id)
            .unwrap()
            .vi_state
            .as_ref()
            .unwrap();
        assert_eq!(vi.cursor.col, 2);
    }

    // ── Font size computation ────────────────────────────────────

    #[test]
    fn font_size_increase_10_percent() {
        let result = App::compute_font_size(13.0, AppCommand::IncreaseFontSize, 13.0);
        assert_eq!(result, 14.0); // 13 * 1.1 = 14.3 → round → 14
    }

    #[test]
    fn font_size_decrease_10_percent() {
        let result = App::compute_font_size(13.0, AppCommand::DecreaseFontSize, 13.0);
        assert_eq!(result, 12.0); // 13 / 1.1 = 11.8 → round → 12
    }

    #[test]
    fn font_size_reset_to_default() {
        let result = App::compute_font_size(20.0, AppCommand::ResetFontSize, 13.0);
        assert_eq!(result, 13.0);
    }

    #[test]
    fn font_size_clamps_to_min() {
        let result = App::compute_font_size(8.0, AppCommand::DecreaseFontSize, 13.0);
        assert_eq!(result, 8.0); // 8 / 1.1 = 7.27 → round → 7 → clamp → 8
    }

    #[test]
    fn font_size_clamps_to_max() {
        let result = App::compute_font_size(72.0, AppCommand::IncreaseFontSize, 13.0);
        assert_eq!(result, 72.0); // 72 * 1.1 = 79.2 → round → 79 → clamp → 72
    }

    #[test]
    fn font_size_tracks_in_app() {
        let app = App::new(WindowConfig::default(), Config::default());
        assert_eq!(app.current_font_size, 18.0);
        assert_eq!(app.default_font_size, 18.0);
    }

    // ── Config hot-reload ────────────────────────────────────────

    #[test]
    fn config_reload_font_updates_app_state() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        assert_eq!(app.current_font_size, 18.0);

        let mut new_config = Config::default();
        new_config.font.size = 20.0;
        new_config.font.family = "Menlo".to_string();
        let delta = app.app_config.diff(&new_config);

        app.handle_config_reload(new_config.clone(), delta);
        assert_eq!(app.current_font_size, 20.0);
        assert_eq!(app.default_font_size, 20.0);
        assert_eq!(app.app_config.font.family, "Menlo");
    }

    #[test]
    fn config_reload_padding_updates_config() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        assert_eq!(app.app_config.padding.top, 16.0);

        let mut new_config = Config::default();
        new_config.padding.top = 24.0;
        new_config.padding.left = 20.0;
        let delta = app.app_config.diff(&new_config);
        assert!(delta.padding_changed);

        app.handle_config_reload(new_config, delta);
        assert_eq!(app.app_config.padding.top, 24.0);
        assert_eq!(app.app_config.padding.left, 20.0);
    }

    #[test]
    fn config_reload_cursor_blink_rate_updates_panes() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        let pane_id = app.tab_manager.active_tab().pane_tree.focused_pane_id();
        let terminal = crate::terminal::Terminal::new(80, 24, 10_000);
        app.pane_states.insert(
            pane_id,
            PaneState {
                terminal,
                pty: crate::pty::PtySession::new(&crate::pty::default_shell(), 80, 24).unwrap(),
                vi_state: None,
                cursor: crate::renderer::cursor::CursorState::new(),
                mouse_selection: crate::input::mouse::MouseSelectionState::new(),
                scroll_state: crate::scroll::ScrollState::new(),
            },
        );

        // Default blink rate is 500
        assert_eq!(app.pane_states[&pane_id].cursor.blink_rate_ms, 500);

        // Hot-reload with new blink rate
        let mut new_config = Config::default();
        new_config.cursor.blink_rate = 750;
        let delta = app.app_config.diff(&new_config);
        assert!(delta.cursor_changed);

        app.handle_config_reload(new_config, delta);
        assert_eq!(app.pane_states[&pane_id].cursor.blink_rate_ms, 750);
    }

    #[test]
    fn config_reload_cursor_blink_disabled_sets_rate_zero() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        let pane_id = app.tab_manager.active_tab().pane_tree.focused_pane_id();
        let terminal = crate::terminal::Terminal::new(80, 24, 10_000);
        app.pane_states.insert(
            pane_id,
            PaneState {
                terminal,
                pty: crate::pty::PtySession::new(&crate::pty::default_shell(), 80, 24).unwrap(),
                vi_state: None,
                cursor: crate::renderer::cursor::CursorState::new(),
                mouse_selection: crate::input::mouse::MouseSelectionState::new(),
                scroll_state: crate::scroll::ScrollState::new(),
            },
        );

        // Disable blink via config
        let mut new_config = Config::default();
        new_config.cursor.blink = false;
        let delta = app.app_config.diff(&new_config);

        app.handle_config_reload(new_config, delta);
        assert_eq!(app.pane_states[&pane_id].cursor.blink_rate_ms, 0);
    }

    #[test]
    fn config_reload_cursor_style_updates_panes() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        let pane_id = app.tab_manager.active_tab().pane_tree.focused_pane_id();
        let terminal = crate::terminal::Terminal::new(80, 24, 10_000);
        app.pane_states.insert(
            pane_id,
            PaneState {
                terminal,
                pty: crate::pty::PtySession::new(&crate::pty::default_shell(), 80, 24).unwrap(),
                vi_state: None,
                cursor: crate::renderer::cursor::CursorState::new(),
                mouse_selection: crate::input::mouse::MouseSelectionState::new(),
                scroll_state: crate::scroll::ScrollState::new(),
            },
        );

        let mut new_config = Config::default();
        new_config.cursor.style = "beam".to_string();
        let delta = app.app_config.diff(&new_config);

        app.handle_config_reload(new_config, delta);
        assert_eq!(
            app.pane_states[&pane_id].cursor.style,
            crate::renderer::cursor::CursorStyle::Beam
        );
    }

    #[test]
    fn config_reload_no_changes_preserves_state() {
        let mut app = App::new(WindowConfig::default(), Config::default());
        let original_size = app.current_font_size;

        let new_config = Config::default();
        let delta = app.app_config.diff(&new_config);
        assert!(delta.is_empty());

        app.handle_config_reload(new_config, delta);
        assert_eq!(app.current_font_size, original_size);
    }
}

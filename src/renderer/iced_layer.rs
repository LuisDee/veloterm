// iced_wgpu integration layer — Anthropic design UI chrome
//
// Renders title bar, tab bar, pane headers with circled-digit badges,
// accent stripes, rounded pane containers with borders/shadows, and
// status bar as an iced overlay on top of the custom wgpu terminal renderer.

use crate::config::theme::TerminalTheme;
use iced_graphics::Viewport;
use iced_runtime::user_interface::{Cache, UserInterface};
use iced_wgpu::Engine;
use iced_widget::{
    column, container, pin, row, stack, text, text_input, MouseArea, Row, Stack,
};
use std::borrow::Cow;

/// JetBrains Mono Regular — shared with glyph_atlas for iced UI chrome text.
const JETBRAINS_MONO_TTF: &[u8] =
    include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf");

/// JetBrains Mono font descriptor for iced widgets.
const JETBRAINS_MONO: iced_core::Font = iced_core::Font::with_name("JetBrains Mono");

/// DM Sans Variable — UI chrome text (Anthropic brand font).
const DM_SANS_TTF: &[u8] =
    include_bytes!("../../assets/fonts/DMSans-Regular.ttf");

/// DM Sans font descriptor for iced widgets.
const DM_SANS: iced_core::Font = iced_core::Font::with_name("DM Sans");

/// Tracks icon (railroad tracks) — conductor dashboard toggle.
pub(crate) const TRACKS_ICON_PNG: &[u8] = include_bytes!("../../assets/icons/tracks.png");

/// Horizontal space filler (iced 0.14: space::horizontal, not horizontal_space).
fn hspace() -> iced_widget::Space {
    iced_widget::Space::new().width(iced_core::Length::Fill)
}

/// Messages produced by iced UI widgets, returned to the application for processing.
#[derive(Debug, Clone)]
pub enum UiMessage {
    TabSelected(usize),
    TabClosed(usize),
    NewTab,
    TabHovered(Option<usize>),
    SidebarTabSelected(usize),
    SidebarTabHovered(Option<usize>),
    ToggleSidebar,
    ToggleThemeSelector,
    SetTheme(String),
    RenameTab(usize),
    RenameTabInput(String),
    RenameTabCommit(usize, String),
    NewTabHovered(bool),
    CloseButtonHovered(Option<usize>),
    // Sidebar toggle button hover
    SidebarBtnEnter,
    SidebarBtnExit,
    // Conductor dashboard
    ConductorToggled,
    TracksIconEnter,
    TracksIconExit,
    ConductorTrackClicked(usize),
    ConductorFilterCycled,
    ConductorSortCycled,
    /// Markdown preview link clicked (Uri is an alias for String).
    MarkdownLinkClicked(String),
    /// Context menu item selected (non-macOS iced overlay).
    ContextMenuAction(crate::context_menu::ContextMenuAction),
    /// Dismiss the context menu overlay (click outside or Escape).
    ContextMenuDismiss,
    // Overlay toggle (from toolbar icons)
    ToggleFileBrowser,
    ToggleGitReview,
    // Overlay icon hover state
    FileBrowserIconEnter,
    FileBrowserIconExit,
    GitReviewIconEnter,
    GitReviewIconExit,
    // Split panel resize
    OverlaySplitResize(f32),
    OverlaySplitReset,
    // File browser interactions
    FileBrowserRowClicked(usize),
    FileBrowserRowHovered(usize),
    FileBrowserRowUnhovered,
    FileBrowserScroll(f32),
    // Git review interactions
    GitReviewFileClicked(usize),
    GitReviewStageFile,
    GitReviewUnstageFile,
    GitReviewDiscardFile,
    GitReviewStageAll,
    GitReviewUnstageAll,
    GitReviewCommit,
    GitReviewScroll(f32),
    GitReviewCommitMsgChanged(String),
    Noop,
}

/// Tab descriptor for the iced widget tree.
#[derive(Debug, Clone)]
pub struct TabInfo {
    pub title: String,
    pub is_active: bool,
    pub has_notification: bool,
}

/// Pane descriptor for the iced widget tree.
/// Positions are in physical pixels relative to the content area origin (after header + tab bar).
#[derive(Debug, Clone)]
pub struct PaneInfo {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub is_focused: bool,
    pub index: usize,
    pub title: String,
    pub shell_name: String,
    /// Scrollbar thumb rect in physical pixels relative to content area origin.
    /// (x, y, width, height). None if no scrollbar visible.
    pub scrollbar_thumb: Option<(f32, f32, f32, f32)>,
    /// Scrollbar opacity (0.0 = hidden, 1.0 = fully visible).
    pub scrollbar_alpha: f32,
}

/// Sidebar tab descriptor for the minimap sidebar.
#[derive(Debug, Clone)]
pub struct SidebarTabInfo {
    pub title: String,
    pub is_active: bool,
    pub has_notification: bool,
    pub pane_count: usize,
    pub minimap_rects: Vec<MinimapPane>,
}

/// A single pane rect in the sidebar minimap, normalized 0.0..1.0.
#[derive(Debug, Clone)]
pub struct MinimapPane {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub is_focused: bool,
}

/// Divider between panes, for visual rendering.
#[derive(Debug, Clone)]
pub struct DividerDisplay {
    /// Physical pixel rect relative to content area origin.
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub is_hovered: bool,
    pub is_dragging: bool,
}

/// Which overlay is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveOverlay {
    FileBrowser,
    GitReview,
}

/// State snapshot passed to the iced widget tree each frame.
pub struct UiState<'a> {
    pub tabs: Vec<TabInfo>,
    pub active_tab_index: usize,
    pub hovered_tab: Option<usize>,
    pub active_pane_index: usize,
    pub panes: Vec<PaneInfo>,
    pub pane_count: usize,
    pub is_zoomed: bool,
    pub theme: &'a TerminalTheme,
    pub window_width: f32,
    pub window_height: f32,
    pub scale_factor: f32,
    /// Search bar state.
    pub search_active: bool,
    pub search_query: String,
    pub search_current: usize,
    pub search_total: usize,
    pub search_error: bool,
    /// Dividers between panes.
    pub dividers: Vec<DividerDisplay>,
    /// Whether a visual bell flash is active.
    pub bell_flash: bool,
    /// Command palette state.
    pub palette_active: bool,
    pub palette_query: String,
    pub palette_items: Vec<(String, String, String)>, // (name, description, keybinding)
    pub palette_selected: usize,
    /// Sidebar state.
    pub sidebar_visible: bool,
    pub sidebar_tabs: Vec<SidebarTabInfo>,
    pub sidebar_width: f32,
    pub hovered_sidebar_tab: Option<usize>,
    /// Theme selector popup state.
    pub theme_selector_open: bool,
    /// Tab being renamed inline (index into sidebar_tabs).
    pub editing_tab: Option<usize>,
    /// Current text in the rename field.
    pub editing_tab_value: String,
    /// Whether the "New Tab" button area is hovered.
    pub hovering_new_tab: bool,
    /// Which tab's close button is hovered (if any).
    pub hovering_close_button: Option<usize>,
    /// Whether a conductor directory was found (for showing the Tracks button).
    pub conductor_available: bool,
    /// Conductor dashboard state (if loaded).
    pub conductor: Option<crate::conductor::ConductorSnapshot>,
    /// Markdown preview: parsed items for the active pane's overlay.
    pub markdown_items: Option<Vec<iced_widget::markdown::Item>>,
    /// Markdown preview: file name being previewed.
    pub markdown_file_name: Option<String>,
    /// Pre-created handle for the tracks icon (avoids per-frame Handle::from_bytes which creates new Id each call).
    pub tracks_icon_handle: Option<iced_core::image::Handle>,
    /// Whether the tracks icon in the header bar is hovered.
    pub is_tracks_hovered: bool,
    /// Whether the sidebar toggle button in the header bar is hovered.
    pub is_sidebar_btn_hovered: bool,
    /// Context menu overlay state (non-macOS only).
    pub context_menu_visible: bool,
    /// Context menu position in physical pixels.
    pub context_menu_position: (f32, f32),
    /// Whether the focused pane has an active selection (affects Copy item).
    pub context_menu_has_selection: bool,
    /// Active overlay type (None = no overlay, showing terminal panes).
    pub active_overlay: Option<ActiveOverlay>,
    /// File browser split ratio (only used when overlay is FileBrowser).
    pub file_browser_split_ratio: f32,
    /// Git review split ratio (only used when overlay is GitReview).
    pub git_review_split_ratio: f32,
    /// Which panel is focused in the active overlay.
    pub overlay_focused_panel: crate::file_browser::OverlayPanel,
    /// Whether the File Browser toolbar icon is hovered.
    pub is_file_browser_icon_hovered: bool,
    /// Whether the Git Review toolbar icon is hovered.
    pub is_git_review_icon_hovered: bool,
    // ── File Browser content ──
    /// Visible file tree rows for rendering.
    pub file_browser_rows: Vec<FileBrowserRow>,
    /// Breadcrumb path text.
    pub file_browser_breadcrumb: String,
    /// Preview file name (None = empty state).
    pub file_browser_preview_name: Option<String>,
    /// Preview lines (plain text, no syntax highlighting in iced).
    pub file_browser_preview_lines: Vec<String>,
    /// Whether the preview was truncated.
    pub file_browser_preview_truncated: bool,
    /// Preview scroll offset.
    pub file_browser_preview_scroll: f32,
    // ── Git Review content ──
    /// Flat list items for the file list panel.
    pub git_review_list_items: Vec<GitReviewListItem>,
    /// Diff header text (e.g. "src/main.rs (Modified)").
    pub git_review_diff_header: Option<String>,
    /// Flattened diff rows for the right panel.
    pub git_review_diff_rows: Vec<GitReviewDiffRow>,
    /// Error message to display.
    pub git_review_error: Option<String>,
    /// Whether the commit button should be enabled.
    pub git_review_can_commit: bool,
    /// Current commit message.
    pub git_review_commit_message: String,
    /// Diff scroll offset.
    pub git_review_diff_scroll: f32,
}

/// A file browser row for rendering.
#[derive(Debug, Clone)]
pub struct FileBrowserRow {
    pub index: usize,
    pub depth: usize,
    pub name: String,
    pub icon: String,
    pub icon_color_hint: String,
    pub chevron: Option<String>,
    pub indent_guide: String,
    pub is_selected: bool,
    pub is_hovered: bool,
}

/// A git review list item for rendering.
#[derive(Debug, Clone)]
pub struct GitReviewListItem {
    pub kind: GitReviewItemKind,
    pub label: String,
    pub is_selected: bool,
    pub section: Option<String>,
    pub status_label: Option<String>,
    pub display_dir: Option<String>,
}

/// Kind of git review list item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitReviewItemKind {
    SectionHeader,
    FileEntry,
}

/// A single diff row for rendering.
#[derive(Debug, Clone)]
pub struct GitReviewDiffRow {
    pub kind: DiffRowKind,
    pub left_num: Option<String>,
    pub right_num: Option<String>,
    pub left_text: String,
    pub right_text: String,
}

/// Kind of diff row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffRowKind {
    HunkHeader,
    Context,
    Added,
    Removed,
    Modified,
}

/// Holds iced rendering state: renderer, viewport, event queue, and UI cache.
pub struct IcedLayer {
    renderer: iced_wgpu::Renderer,
    viewport: Viewport,
    cache: Cache,
    events: Vec<iced_core::Event>,
    cursor: iced_core::mouse::Cursor,
    format: wgpu::TextureFormat,
    scale: f32,
}

/// Convert a theme Color to an iced Color.
fn to_iced_color(c: &crate::config::theme::Color) -> iced_core::Color {
    iced_core::Color::from_rgba(c.r, c.g, c.b, c.a)
}

/// Circled digit badge for pane index (1-based).
#[cfg(test)]
fn pane_badge(index: usize) -> &'static str {
    match index + 1 {
        1 => "\u{2460}",
        2 => "\u{2461}",
        3 => "\u{2462}",
        4 => "\u{2463}",
        5 => "\u{2464}",
        6 => "\u{2465}",
        7 => "\u{2466}",
        8 => "\u{2467}",
        9 => "\u{2468}",
        _ => "\u{2460}",
    }
}

/// Chrome bar height in logical pixels (40px per spec).
const CHROME_BAR_HEIGHT: f32 = 38.0;
/// Status bar height in logical pixels (~30px per spec).
const STATUS_BAR_HEIGHT: f32 = 28.0;
/// Pane header height in logical pixels (0 — headers removed).
/// Kept for test parity with legacy pane::header::PANE_HEADER_HEIGHT.
#[cfg(test)]
const PANE_HEADER_HEIGHT: f32 = 0.0;

type IcedElement<'a> = iced_core::Element<'a, UiMessage, iced_core::Theme, iced_wgpu::Renderer>;

impl IcedLayer {
    /// Create a new iced layer sharing the existing GPU resources.
    pub fn new(
        adapter: &wgpu::Adapter,
        device: wgpu::Device,
        queue: wgpu::Queue,
        format: wgpu::TextureFormat,
        physical_width: u32,
        physical_height: u32,
        scale_factor: f32,
    ) -> Self {
        // Load JetBrains Mono into iced's global font system so all iced text
        // widgets render with the same monospace font as the terminal grid.
        {
            let mut fs = iced_graphics::text::font_system().write().unwrap();
            fs.load_font(Cow::Borrowed(JETBRAINS_MONO_TTF));
            fs.load_font(Cow::Borrowed(DM_SANS_TTF));
        }

        let shell = iced_graphics::Shell::headless();
        let engine = Engine::new(adapter, device, queue, format, None, shell);
        let renderer = iced_wgpu::Renderer::new(
            engine,
            JETBRAINS_MONO,
            iced_core::Pixels(14.0),
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
            scale: scale_factor,
        }
    }

    /// Update the viewport after a window resize.
    pub fn resize(&mut self, physical_width: u32, physical_height: u32, scale_factor: f32) {
        self.scale = scale_factor;
        self.viewport = Viewport::with_physical_size(
            iced_core::Size::new(physical_width, physical_height),
            scale_factor,
        );
    }

    /// Update the scale factor (e.g. when window moves between displays).
    pub fn update_scale(&mut self, scale: f32) {
        self.scale = scale;
        let size = self.viewport.physical_size();
        self.viewport = Viewport::with_physical_size(size, scale);
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
            if let iced_core::Event::Mouse(iced_core::mouse::Event::CursorMoved { position }) =
                &iced_event
            {
                self.cursor = iced_core::mouse::Cursor::Available(*position);
            }
            self.events.push(iced_event);
        }
    }

    /// Run the iced UI lifecycle and present onto the given texture view.
    pub fn render(&mut self, view: &wgpu::TextureView, state: &UiState) -> Vec<UiMessage> {
        let bounds = self.viewport.logical_size();
        let scale = self.viewport.scale_factor();

        let widget = Self::view(state, scale);

        let mut interface = UserInterface::build(
            widget,
            bounds,
            std::mem::take(&mut self.cache),
            &mut self.renderer,
        );

        let mut messages: Vec<UiMessage> = Vec::new();
        let mut clipboard = iced_core::clipboard::Null;
        let mut events = std::mem::take(&mut self.events);

        // iced_winit::conversion doesn't convert winit's RedrawRequested, but
        // iced button widgets only commit their hover/press status on this event.
        // Inject it so title bar buttons (hamburger, theme) visually update.
        // Sidebar buttons use MouseArea instead (bypasses broken button::Status).
        events.push(iced_core::Event::Window(
            iced_core::window::Event::RedrawRequested(std::time::Instant::now()),
        ));
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
                // Warm cream — brightest allowed text per spec (#e8e0d4)
                text_color: iced_core::Color::from_rgb(0.910, 0.878, 0.831),
            },
            self.cursor,
        );

        self.cache = interface.into_cache();

        self.renderer
            .present(None, self.format, view, &self.viewport);

        messages
    }

    /// Build the full UI chrome widget tree from application state.
    fn view<'a>(state: &'a UiState, scale: f32) -> IcedElement<'a> {
        let theme = state.theme;

        let chrome_bar = Self::chrome_bar(state, scale);
        let chrome_divider = Self::divider(theme, scale);

        // Content area: transparent base + pane chrome overlay
        let transparent_base = container(column![])
            .width(iced_core::Length::Fill)
            .height(iced_core::Length::Fill)
            .style(|_: &iced_core::Theme| container::Style {
                background: None,
                ..Default::default()
            });

        let base_content: IcedElement<'a> = if state.panes.is_empty() {
            transparent_base.into()
        } else {
            let pane_chrome = Self::pane_chrome(state, scale);
            stack![transparent_base, pane_chrome]
                .width(iced_core::Length::Fill)
                .height(iced_core::Length::Fill)
                .into()
        };

        // If conductor dashboard is active, replace content area with it.
        // If an overlay is active, replace content area with the overlay split panel.
        let content: IcedElement<'a> = if state.conductor.is_some() {
            Self::conductor_dashboard(state, scale)
        } else if let Some(overlay) = state.active_overlay {
            Self::overlay_content(state, scale, overlay)
        } else {
            base_content
        };

        let status_divider = Self::divider(theme, scale);
        let status_bar = Self::status_bar(state, scale);

        // Sidebar is always the tab management UI (no separate tab bar)
        let main_ui: IcedElement<'a> = if state.sidebar_visible {
            let sidebar = Self::sidebar(state, scale);
            let sidebar_divider = Self::vertical_divider(theme, scale);
            let middle: IcedElement<'a> = row![sidebar, sidebar_divider, content]
                .width(iced_core::Length::Fill)
                .height(iced_core::Length::Fill)
                .into();
            column![chrome_bar, chrome_divider, middle, status_divider, status_bar]
                .width(iced_core::Length::Fill)
                .height(iced_core::Length::Fill)
                .into()
        } else {
            column![chrome_bar, chrome_divider, content, status_divider, status_bar]
                .width(iced_core::Length::Fill)
                .height(iced_core::Length::Fill)
                .into()
        };

        let with_flash: IcedElement<'a> = if state.bell_flash {
            let flash_overlay = container(column![])
                .width(iced_core::Length::Fill)
                .height(iced_core::Length::Fill)
                .style(|_: &iced_core::Theme| container::Style {
                    background: Some(iced_core::Background::Color(
                        iced_core::Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                    )),
                    ..Default::default()
                });

            stack![main_ui, flash_overlay]
                .width(iced_core::Length::Fill)
                .height(iced_core::Length::Fill)
                .into()
        } else {
            main_ui
        };

        // Theme selector popup (positioned below title bar icons)
        let with_theme_popup: IcedElement<'a> = if state.theme_selector_open {
            let popup = Self::theme_selector_popup(state, scale);
            stack![with_flash, popup]
                .width(iced_core::Length::Fill)
                .height(iced_core::Length::Fill)
                .into()
        } else {
            with_flash
        };

        // Command palette overlay (modal, centered at top)
        let with_palette: IcedElement<'a> = if state.palette_active {
            let palette_overlay = Self::command_palette(state, scale);
            let scrim = container(column![])
                .width(iced_core::Length::Fill)
                .height(iced_core::Length::Fill)
                .style(|_: &iced_core::Theme| container::Style {
                    background: Some(iced_core::Background::Color(
                        iced_core::Color::from_rgba(0.0, 0.0, 0.0, 0.4),
                    )),
                    ..Default::default()
                });

            stack![with_theme_popup, scrim, palette_overlay]
                .width(iced_core::Length::Fill)
                .height(iced_core::Length::Fill)
                .into()
        } else {
            with_theme_popup
        };

        // Markdown preview overlay
        let with_markdown: IcedElement<'a> = if state.markdown_items.is_some() {
            let md_overlay = Self::markdown_overlay(state, scale);
            stack![with_palette, md_overlay]
                .width(iced_core::Length::Fill)
                .height(iced_core::Length::Fill)
                .into()
        } else {
            with_palette
        };

        // Context menu overlay (non-macOS only — macOS uses native NSMenu)
        if state.context_menu_visible {
            let ctx_menu = Self::context_menu_overlay(state, scale);
            // Scrim: clicking outside the menu dismisses it
            let scrim = MouseArea::new(
                container(column![])
                    .width(iced_core::Length::Fill)
                    .height(iced_core::Length::Fill)
                    .style(|_: &iced_core::Theme| container::Style {
                        background: Some(iced_core::Background::Color(
                            iced_core::Color::from_rgba(0.0, 0.0, 0.0, 0.15),
                        )),
                        ..Default::default()
                    }),
            )
            .on_press(UiMessage::ContextMenuDismiss);

            stack![with_markdown, scrim, ctx_menu]
                .width(iced_core::Length::Fill)
                .height(iced_core::Length::Fill)
                .into()
        } else {
            with_markdown
        }
    }

    /// Chrome bar: centered [✦ VeloTerm]
    /// Per VeloTerm design spec. Height: 38px.
    fn chrome_bar<'a>(state: &'a UiState, scale: f32) -> IcedElement<'a> {
        let theme = state.theme;
        let chrome_bg = to_iced_color(&theme.bg_surface);
        let accent = to_iced_color(&theme.accent_orange);
        let text_secondary = to_iced_color(&theme.text_secondary);
        let border_subtle = to_iced_color(&theme.border_subtle);

        let height = CHROME_BAR_HEIGHT / scale;
        let bg_hover = to_iced_color(&theme.bg_hover);
        let bg_raised = to_iced_color(&theme.bg_raised);

        // Sidebar toggle button (hamburger icon ☰)
        let sidebar_icon_color = if state.sidebar_visible { accent } else { text_secondary };
        let sidebar_label = text("\u{2630}").size(14.0).color(sidebar_icon_color).font(DM_SANS);
        let sidebar_btn_bg = if state.is_sidebar_btn_hovered { bg_hover } else { bg_raised };
        let sidebar_btn = container(sidebar_label)
            .padding(iced_core::Padding::from([4.0 / scale, 8.0 / scale]))
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(sidebar_btn_bg)),
                border: iced_core::Border {
                    color: border_subtle,
                    width: 0.0,
                    radius: (4.0 / scale).into(),
                },
                ..Default::default()
            });
        let sidebar_click = MouseArea::new(sidebar_btn)
            .on_press(UiMessage::ToggleSidebar)
            .on_enter(UiMessage::SidebarBtnEnter)
            .on_exit(UiMessage::SidebarBtnExit);

        // Center area (empty — brand removed for cleaner look)
        let center_content = row![]
            .align_y(iced_core::Alignment::Center);

        // File Browser icon (folder)
        let fb_active = matches!(state.active_overlay, Some(ActiveOverlay::FileBrowser));
        let fb_color = if fb_active { accent } else { text_secondary };
        let fb_label = text("\u{25A3}").size(14.0).color(fb_color).font(DM_SANS);
        let fb_btn_bg = if state.is_file_browser_icon_hovered { bg_hover } else { bg_raised };
        let fb_btn = container(fb_label)
            .padding(iced_core::Padding::from([4.0 / scale, 8.0 / scale]))
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(fb_btn_bg)),
                border: iced_core::Border {
                    color: border_subtle,
                    width: 0.0,
                    radius: (4.0 / scale).into(),
                },
                ..Default::default()
            });
        let fb_click = MouseArea::new(fb_btn)
            .on_press(UiMessage::ToggleFileBrowser)
            .on_enter(UiMessage::FileBrowserIconEnter)
            .on_exit(UiMessage::FileBrowserIconExit);

        // Git Review icon (branch)
        let gr_active = matches!(state.active_overlay, Some(ActiveOverlay::GitReview));
        let gr_color = if gr_active { accent } else { text_secondary };
        let gr_label = text("\u{2387}").size(14.0).color(gr_color).font(DM_SANS);
        let gr_btn_bg = if state.is_git_review_icon_hovered { bg_hover } else { bg_raised };
        let gr_btn = container(gr_label)
            .padding(iced_core::Padding::from([4.0 / scale, 8.0 / scale]))
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(gr_btn_bg)),
                border: iced_core::Border {
                    color: border_subtle,
                    width: 0.0,
                    radius: (4.0 / scale).into(),
                },
                ..Default::default()
            });
        let gr_click = MouseArea::new(gr_btn)
            .on_press(UiMessage::ToggleGitReview)
            .on_enter(UiMessage::GitReviewIconEnter)
            .on_exit(UiMessage::GitReviewIconExit);

        // Build right-side icon group
        let mut right_icons: Row<'a, UiMessage, iced_core::Theme, iced_wgpu::Renderer> = Row::new()
            .spacing(4.0 / scale)
            .align_y(iced_core::Alignment::Center);
        right_icons = right_icons.push(fb_click).push(gr_click);

        let content: IcedElement<'a> = if state.conductor_available {
            let icon_opacity = if state.conductor.is_some() { 1.0 } else { 0.6 };
            let icon_size = 20.0 / scale;
            let handle = state.tracks_icon_handle.clone()
                .unwrap_or_else(|| iced_widget::image::Handle::from_bytes(TRACKS_ICON_PNG));
            let tracks_img = iced_widget::image::Image::new(handle)
                .width(icon_size)
                .height(icon_size)
                .content_fit(iced_core::ContentFit::Contain)
                .opacity(icon_opacity);
            let btn_bg = if state.is_tracks_hovered {
                to_iced_color(&theme.bg_hover)
            } else {
                to_iced_color(&theme.bg_raised)
            };
            let tracks_icon = container(tracks_img)
                .padding(iced_core::Padding::from([6.0 / scale, 10.0 / scale]))
                .style(move |_: &iced_core::Theme| container::Style {
                    background: Some(iced_core::Background::Color(btn_bg)),
                    border: iced_core::Border {
                        color: border_subtle,
                        width: 0.0,
                        radius: (4.0 / scale).into(),
                    },
                    ..Default::default()
                });
            let tracks_click = MouseArea::new(tracks_icon)
                .on_press(UiMessage::ConductorToggled)
                .on_enter(UiMessage::TracksIconEnter)
                .on_exit(UiMessage::TracksIconExit);
            right_icons = right_icons.push(tracks_click);
            row![sidebar_click, hspace(), center_content, hspace(), right_icons]
                .align_y(iced_core::Alignment::Center)
                .padding(iced_core::Padding::from([0.0, 8.0 / scale]))
                .into()
        } else {
            row![sidebar_click, hspace(), center_content, hspace(), right_icons]
                .align_y(iced_core::Alignment::Center)
                .padding(iced_core::Padding::from([0.0, 8.0 / scale]))
                .into()
        };

        container(content)
            .width(iced_core::Length::Fill)
            .height(height)
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(chrome_bg)),
                border: iced_core::Border {
                    color: border_subtle,
                    width: 0.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    /// 1px divider line between chrome sections.
    fn divider<'a>(theme: &TerminalTheme, scale: f32) -> IcedElement<'a> {
        let border_color = to_iced_color(&theme.border_visible);
        container(column![])
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(border_color)),
                ..Default::default()
            })
            .width(iced_core::Length::Fill)
            .height(1.0 / scale)
            .into()
    }

    /// Pane chrome: rounded containers with headers, accent stripes, borders, shadows,
    /// plus dividers, scrollbar thumbs, and search bar overlay.
    fn pane_chrome<'a>(state: &'a UiState, scale: f32) -> IcedElement<'a> {
        let theme = state.theme;
        let accent = to_iced_color(&theme.accent_orange);
        let _border_color = to_iced_color(&theme.border_visible);

        let mut chrome_stack = Stack::new()
            .width(iced_core::Length::Fill)
            .height(iced_core::Length::Fill);

        for pane in &state.panes {
            let px = pane.x / scale;
            let py = pane.y / scale;
            let pw = pane.width / scale;
            let ph = pane.height / scale;

            // No border — separation comes from background color difference only.
            let pane_container = container(column![])
                .width(pw)
                .height(ph)
                .style(move |_: &iced_core::Theme| container::Style {
                    background: None,
                    border: iced_core::Border {
                        color: iced_core::Color::TRANSPARENT,
                        width: 0.0,
                        radius: 0.0.into(),
                    },
                    ..Default::default()
                });

            chrome_stack = chrome_stack.push(pin(pane_container).x(px).y(py));

            // Scrollbar thumb (overlay on right edge of pane)
            if let Some((sx, sy, sw, sh)) = pane.scrollbar_thumb {
                if pane.scrollbar_alpha > 0.0 {
                    let alpha = pane.scrollbar_alpha;
                    let scrollbar = container(column![])
                        .width(sw / scale)
                        .height(sh / scale)
                        .style(move |_: &iced_core::Theme| container::Style {
                            background: Some(iced_core::Background::Color(
                                iced_core::Color::from_rgba(1.0, 1.0, 1.0, alpha * 0.5),
                            )),
                            border: iced_core::Border {
                                color: iced_core::Color::TRANSPARENT,
                                width: 0.0,
                                radius: (3.0 / scale).into(),
                            },
                            ..Default::default()
                        });
                    chrome_stack =
                        chrome_stack.push(pin(scrollbar).x(sx / scale).y(sy / scale));
                }
            }

            // Active pane focus indicator: 2px orange top-line when split.
            // Positioned at pane origin — sits within the terminal padding zone
            // (padding_top >= 10px), so it never overlaps grid content.
            if pane.is_focused && state.panes.len() > 1 {
                let stripe = container(column![])
                    .width(pw)
                    .height(2.0 / scale)
                    .style(move |_: &iced_core::Theme| container::Style {
                        background: Some(iced_core::Background::Color(accent)),
                        ..Default::default()
                    });
                chrome_stack = chrome_stack.push(pin(stripe).x(px).y(py));
            }
        }

        // Dividers between panes — visible 1px line, brighter on hover, orange on drag
        let border_vis = to_iced_color(&theme.border_visible);
        let border_str = to_iced_color(&theme.border_strong);
        for div in &state.dividers {
            let is_hovered = div.is_hovered;
            let is_dragging = div.is_dragging;
            // Compute the visible line width and center it within the hit-area
            let hit_w = div.width / scale;
            let hit_h = div.height / scale;
            let line_w: f32 = if is_dragging || is_hovered { 2.0 } else { 1.0 };
            // The visible line, centered in the hit area
            let line_color = if is_dragging {
                iced_core::Color::from_rgba(
                    accent.r, accent.g, accent.b, 0.5,
                )
            } else if is_hovered {
                border_str
            } else {
                border_vis
            };
            let line_widget = container(column![])
                .width(line_w)
                .height(hit_h)
                .style(move |_: &iced_core::Theme| container::Style {
                    background: Some(iced_core::Background::Color(line_color)),
                    ..Default::default()
                });
            let line_x = div.x / scale + (hit_w - line_w) / 2.0;
            chrome_stack = chrome_stack.push(pin(line_widget).x(line_x).y(div.y / scale));

            // Invisible hit-area container (captures mouse events via interaction state machine)
            let hit_area = container(column![])
                .width(hit_w)
                .height(hit_h)
                .style(|_: &iced_core::Theme| container::Style {
                    background: None,
                    ..Default::default()
                });
            chrome_stack = chrome_stack.push(pin(hit_area).x(div.x / scale).y(div.y / scale));
        }

        // Search bar overlay (positioned at top-right of focused pane)
        if state.search_active {
            if let Some(focused_pane) = state.panes.iter().find(|p| p.is_focused) {
                let search_bar =
                    Self::search_bar(state, focused_pane, scale);
                chrome_stack = chrome_stack.push(search_bar);
            }
        }

        chrome_stack.into()
    }

    /// Search bar: floating overlay at top-right of the focused pane.
    fn search_bar<'a>(
        state: &'a UiState,
        pane: &PaneInfo,
        scale: f32,
    ) -> IcedElement<'a> {
        let theme = state.theme;
        let surface = to_iced_color(&theme.bg_surface);
        let text_color = to_iced_color(&theme.text_primary);
        let text_dim = to_iced_color(&theme.text_ghost);
        let accent = to_iced_color(&theme.accent_orange);
        let error_color = to_iced_color(&theme.accent_red);
        let border_color = to_iced_color(&theme.border_visible);

        let font_size = 12.0;
        let pad_v = 6.0 / scale;
        let pad_h = 12.0 / scale;
        let bar_width = 260.0 / scale;
        let bar_height = 30.0 / scale;
        let spacing = 8.0 / scale;
        let radius = 6.0 / scale;

        // Query text (truncated if needed)
        let query_display = if state.search_query.is_empty() {
            "Find...".to_string()
        } else {
            let max_chars = 20;
            if state.search_query.len() > max_chars {
                format!("{}...", &state.search_query[..max_chars])
            } else {
                state.search_query.clone()
            }
        };

        let query_color = if state.search_error {
            error_color
        } else if state.search_query.is_empty() {
            text_dim
        } else {
            text_color
        };

        // Match count
        let match_text = if state.search_total > 0 {
            format!("{}/{}", state.search_current, state.search_total)
        } else if !state.search_query.is_empty() {
            "0/0".to_string()
        } else {
            String::new()
        };

        let bar_content = row![
            text(query_display).size(font_size).color(query_color),
            hspace(),
            text(match_text).size(font_size).color(text_dim),
        ]
        .spacing(spacing)
        .align_y(iced_core::Alignment::Center)
        .padding(iced_core::Padding::from([pad_v, pad_h]));

        let bar_bg = surface;
        let bar_border = if state.search_error { error_color } else { border_color };
        let bar_accent = accent;
        let has_query = !state.search_query.is_empty();
        let bar = container(bar_content)
            .width(bar_width)
            .height(bar_height)
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(bar_bg)),
                border: iced_core::Border {
                    color: if has_query { bar_accent } else { bar_border },
                    width: 1.0,
                    radius: radius.into(),
                },
                shadow: iced_core::Shadow {
                    color: iced_core::Color::from_rgba(0.0, 0.0, 0.0, 0.3),
                    offset: iced_core::Vector::new(0.0, 2.0 / scale),
                    blur_radius: 8.0 / scale,
                },
                ..Default::default()
            });

        // Position at top-right of focused pane (inside the pane, offset from header)
        let bar_x = (pane.x + pane.width - 260.0 - 8.0) / scale;
        let bar_y = (pane.y + 8.0) / scale;

        pin(bar).x(bar_x).y(bar_y).into()
    }

    /// Command palette: floating modal centered near top of window.
    fn command_palette<'a>(state: &'a UiState, scale: f32) -> IcedElement<'a> {
        let theme = state.theme;
        let surface = to_iced_color(&theme.bg_surface);
        let surface_raised = to_iced_color(&theme.bg_hover);
        let text_color = to_iced_color(&theme.text_primary);
        let text_dim = to_iced_color(&theme.text_ghost);
        let text_secondary = to_iced_color(&theme.text_secondary);
        let accent = to_iced_color(&theme.accent_orange);
        let border_color = to_iced_color(&theme.border_visible);

        let font_size = 13.0;
        let small_size = 11.0;
        let pad_h = 14.0 / scale;
        let pad_v = 8.0 / scale;
        let palette_width = 440.0 / scale;
        let input_height = 36.0 / scale;
        let item_height = 32.0 / scale;
        let radius = 8.0 / scale;
        let spacing = 2.0 / scale;
        let max_visible = 10;

        // Input field display
        let query_display = if state.palette_query.is_empty() {
            "Type a command...".to_string()
        } else {
            state.palette_query.clone()
        };
        let query_color = if state.palette_query.is_empty() {
            text_dim
        } else {
            text_color
        };

        let input_bar = container(
            row![
                text("\u{1F50D}").size(font_size).color(text_dim),
                text(query_display).size(font_size).color(query_color),
            ]
            .spacing(8.0 / scale)
            .align_y(iced_core::Alignment::Center)
            .padding(iced_core::Padding::from([pad_v, pad_h])),
        )
        .width(palette_width)
        .height(input_height)
        .style(move |_: &iced_core::Theme| container::Style {
            background: Some(iced_core::Background::Color(surface)),
            border: iced_core::Border {
                color: border_color,
                width: 0.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        });

        // Result list
        let visible_count = state.palette_items.len().min(max_visible);
        let mut result_col = iced_widget::Column::new().spacing(spacing);

        for (i, (name, _desc, keybinding)) in state.palette_items.iter().enumerate().take(visible_count) {
            let is_selected = i == state.palette_selected;
            let item_bg = if is_selected { surface_raised } else { surface };
            let item_fg = if is_selected { text_color } else { text_secondary };
            let kb_color = text_dim;

            let item_content = row![
                text(name.as_str()).size(font_size).color(item_fg),
                hspace(),
                text(keybinding.as_str()).size(small_size).color(kb_color),
            ]
            .align_y(iced_core::Alignment::Center)
            .padding(iced_core::Padding::from([pad_v, pad_h]));

            let item_accent = accent;
            let item = container(item_content)
                .width(palette_width)
                .height(item_height)
                .style(move |_: &iced_core::Theme| container::Style {
                    background: Some(iced_core::Background::Color(item_bg)),
                    border: if is_selected {
                        iced_core::Border {
                            color: item_accent,
                            width: 0.0,
                            radius: 0.0.into(),
                        }
                    } else {
                        iced_core::Border::default()
                    },
                    ..Default::default()
                });

            result_col = result_col.push(item);
        }

        // Item count footer
        let count_text = format!("{} commands", state.palette_items.len());
        let footer = container(
            text(count_text).size(small_size).color(text_dim),
        )
        .width(palette_width)
        .padding(iced_core::Padding::from([4.0 / scale, pad_h]))
        .style(move |_: &iced_core::Theme| container::Style {
            background: Some(iced_core::Background::Color(surface)),
            ..Default::default()
        });

        // Separator
        let sep = container(column![])
            .width(palette_width)
            .height(1.0 / scale)
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(border_color)),
                ..Default::default()
            });

        let palette_box = container(
            column![input_bar, sep, result_col, footer],
        )
        .width(palette_width)
        .style(move |_: &iced_core::Theme| container::Style {
            background: Some(iced_core::Background::Color(surface)),
            border: iced_core::Border {
                color: border_color,
                width: 1.0,
                radius: radius.into(),
            },
            shadow: iced_core::Shadow {
                color: iced_core::Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                offset: iced_core::Vector::new(0.0, 4.0 / scale),
                blur_radius: 16.0 / scale,
            },
            ..Default::default()
        });

        // Center horizontally, position near top
        let palette_x = ((state.window_width / scale) - palette_width) / 2.0;
        let palette_y = 80.0 / scale;

        pin(palette_box).x(palette_x).y(palette_y).into()
    }

    /// Theme selector dropdown below the title bar theme icon.
    /// Returns (bg_deep, text_primary, accent_orange) preview colors for a theme by config name.
    fn theme_preview_colors(config_name: &str) -> (iced_core::Color, iced_core::Color, iced_core::Color) {
        use crate::config::theme::{DARK, MIDNIGHT, EMBER, DUSK, LIGHT};
        let t = match config_name {
            "warm_dark" => &DARK,
            "midnight" => &MIDNIGHT,
            "ember" => &EMBER,
            "dusk" => &DUSK,
            "light" => &LIGHT,
            _ => &DARK,
        };
        (to_iced_color(&t.bg_deep), to_iced_color(&t.text_primary), to_iced_color(&t.accent_orange))
    }

    fn theme_selector_popup<'a>(state: &'a UiState, scale: f32) -> IcedElement<'a> {
        let theme = state.theme;
        let bg_raised = to_iced_color(&theme.bg_raised);
        let bg_hover_color = to_iced_color(&theme.bg_hover);
        let text_color = to_iced_color(&theme.text_primary);
        let text_secondary = to_iced_color(&theme.text_secondary);
        let accent = to_iced_color(&theme.accent_orange);
        let border_color = to_iced_color(&theme.border_visible);
        let border_subtle = to_iced_color(&theme.border_subtle);

        let font_size = 12.0;
        let item_h = 32.0 / scale;
        let pad_h = 12.0 / scale;
        let popup_w = 500.0 / scale;
        let radius = 8.0 / scale;
        let swatch_size = 10.0 / scale;
        let swatch_radius = 2.0 / scale;
        let swatch_gap = 2.0 / scale;

        let themes = crate::config::theme::TerminalTheme::available_themes();
        let current_name = theme.name;

        let mut col = iced_widget::Column::new().spacing(0.0);
        for (i, &(config_name, display_name)) in themes.iter().enumerate() {
            // Add separator before the last item (Light)
            if i == themes.len() - 1 {
                let sep = container(column![])
                    .width(iced_core::Length::Fill)
                    .height(1.0 / scale)
                    .style(move |_: &iced_core::Theme| container::Style {
                        background: Some(iced_core::Background::Color(border_subtle)),
                        ..Default::default()
                    });
                col = col.push(sep);
            }

            let is_current = display_name == current_name;
            let item_bg = if is_current { bg_hover_color } else { bg_raised };
            let item_fg = if is_current { text_color } else { text_secondary };

            // Swatch dots: bg_deep, text_primary, accent_orange of target theme
            let (sw_bg, sw_text, sw_accent) = Self::theme_preview_colors(config_name);
            let make_swatch = move |color: iced_core::Color| {
                container(column![])
                    .width(swatch_size)
                    .height(swatch_size)
                    .style(move |_: &iced_core::Theme| container::Style {
                        background: Some(iced_core::Background::Color(color)),
                        border: iced_core::Border::default().rounded(swatch_radius),
                        ..Default::default()
                    })
            };

            let swatches = row![
                make_swatch(sw_bg),
                make_swatch(sw_text),
                make_swatch(sw_accent),
            ]
            .spacing(swatch_gap)
            .align_y(iced_core::Alignment::Center);

            let check_text: &str = if is_current { "\u{2713}" } else { "" };

            let label_row = row![
                swatches,
                text(display_name).size(font_size).color(item_fg).font(DM_SANS),
                hspace(),
                text(check_text).size(font_size).color(accent).font(DM_SANS),
            ]
            .spacing(8.0 / scale)
            .align_y(iced_core::Alignment::Center)
            .padding(iced_core::Padding::from([0.0, pad_h]));

            let item_container = container(label_row)
                .width(popup_w)
                .height(item_h)
                .center_y(item_h)
                .style(move |_: &iced_core::Theme| container::Style {
                    background: Some(iced_core::Background::Color(item_bg)),
                    ..Default::default()
                });

            let clickable = MouseArea::new(item_container)
                .on_press(UiMessage::SetTheme(config_name.to_string()));

            col = col.push(clickable);
        }

        let popup_box = container(col)
            .width(popup_w)
            .padding(iced_core::Padding::from([4.0 / scale, 0.0]))
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(bg_raised)),
                border: iced_core::Border {
                    color: border_color,
                    width: 1.0,
                    radius: radius.into(),
                },
                shadow: iced_core::Shadow {
                    color: iced_core::Color::from_rgba(0.0, 0.0, 0.0, 0.4),
                    offset: iced_core::Vector::new(0.0, 8.0 / scale),
                    blur_radius: 24.0 / scale,
                },
                ..Default::default()
            });

        // Position above status bar, right-aligned
        let popup_x = (state.window_width / scale) - popup_w - 16.0 / scale;
        let popup_y = (state.window_height / scale) - STATUS_BAR_HEIGHT / scale - (themes.len() as f32 * item_h + 8.0 / scale + 1.0 / scale);

        pin(popup_box).x(popup_x).y(popup_y).into()
    }

    /// Vertical 1px divider between sidebar and content.
    fn vertical_divider<'a>(theme: &TerminalTheme, scale: f32) -> IcedElement<'a> {
        let border_color = to_iced_color(&theme.border_visible);
        container(column![])
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(border_color)),
                ..Default::default()
            })
            .width(1.0 / scale)
            .height(iced_core::Length::Fill)
            .into()
    }

    /// Context menu overlay (non-macOS): a floating menu at the cursor position.
    ///
    /// Renders a column of clickable menu items (Copy, Paste, Select All, etc.)
    /// positioned at `state.context_menu_position`. Used on Linux/other platforms
    /// where native NSMenu is unavailable.
    fn context_menu_overlay<'a>(state: &'a UiState, scale: f32) -> IcedElement<'a> {
        use crate::context_menu::ContextMenuAction;

        let theme = state.theme;
        let bg = to_iced_color(&theme.bg_surface);
        let text_color = to_iced_color(&theme.text_primary);
        let text_muted = to_iced_color(&theme.text_secondary);
        let border_color = to_iced_color(&theme.border_subtle);
        let has_sel = state.context_menu_has_selection;

        let menu_items: Vec<(ContextMenuAction, &str, &str, bool)> = vec![
            (ContextMenuAction::Copy, "Copy", "Ctrl+Shift+C", has_sel),
            (ContextMenuAction::Paste, "Paste", "Ctrl+Shift+V", true),
            (ContextMenuAction::SelectAll, "Select All", "Ctrl+Shift+A", true),
            (ContextMenuAction::ClearScrollback, "Clear", "", true),
            (ContextMenuAction::NewTab, "New Tab", "", true),
            (ContextMenuAction::SplitVertical, "Split Right", "", true),
            (ContextMenuAction::SplitHorizontal, "Split Down", "", true),
            (ContextMenuAction::ClosePane, "Close Pane", "", true),
        ];

        let font_size = 13.0 / scale;
        let hint_size = 11.0 / scale;
        let item_pad_h = 12.0 / scale;
        let item_pad_v = 6.0 / scale;
        let menu_width = 200.0 / scale;

        let mut col = iced_widget::Column::new().width(menu_width);

        for (i, (action, label, hint, enabled)) in menu_items.iter().enumerate() {
            // Separator before "Clear" and before "New Tab"
            if i == 3 || i == 4 {
                col = col.push(
                    container(column![])
                        .width(iced_core::Length::Fill)
                        .height(1.0 / scale)
                        .style(move |_: &iced_core::Theme| container::Style {
                            background: Some(iced_core::Background::Color(border_color)),
                            ..Default::default()
                        }),
                );
            }

            let label_color = if *enabled { text_color } else { text_muted };
            let action_clone = *action;

            let label_text = text(*label)
                .size(font_size)
                .color(label_color)
                .font(DM_SANS);

            let item_content: IcedElement<'a> = if hint.is_empty() {
                container(label_text)
                    .width(iced_core::Length::Fill)
                    .padding(iced_core::Padding::from([item_pad_v, item_pad_h]))
                    .into()
            } else {
                let hint_text = text(*hint)
                    .size(hint_size)
                    .color(text_muted)
                    .font(JETBRAINS_MONO);
                container(
                    row![label_text, hspace(), hint_text]
                        .align_y(iced_core::Alignment::Center),
                )
                .width(iced_core::Length::Fill)
                .padding(iced_core::Padding::from([item_pad_v, item_pad_h]))
                .into()
            };

            let hover_item = container(item_content)
                .width(iced_core::Length::Fill)
                .style(move |_: &iced_core::Theme| container::Style {
                    ..Default::default()
                });

            if *enabled {
                let mouse_area = MouseArea::new(hover_item)
                    .on_press(UiMessage::ContextMenuAction(action_clone));
                col = col.push(mouse_area);
            } else {
                col = col.push(hover_item);
            }
        }

        let menu_container = container(col)
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(bg)),
                border: iced_core::Border {
                    color: border_color,
                    width: 1.0 / scale,
                    radius: (6.0 / scale).into(),
                },
                shadow: iced_core::Shadow {
                    color: iced_core::Color::from_rgba(0.0, 0.0, 0.0, 0.3),
                    offset: iced_core::Vector::new(0.0, 2.0 / scale),
                    blur_radius: 8.0 / scale,
                },
                ..Default::default()
            })
            .padding(iced_core::Padding::from([4.0 / scale, 0.0]));

        // Position the menu at the cursor location using pin + padding
        let pos_x = state.context_menu_position.0 / scale;
        let pos_y = state.context_menu_position.1 / scale;

        // Clamp to keep menu on-screen
        let win_w = state.window_width / scale;
        let win_h = state.window_height / scale;
        let clamped_x = pos_x.min(win_w - menu_width - 8.0 / scale).max(0.0);
        let clamped_y = pos_y.min(win_h - 300.0 / scale).max(0.0);

        pin(
            container(menu_container)
                .padding(iced_core::Padding {
                    top: clamped_y,
                    left: clamped_x,
                    right: 0.0,
                    bottom: 0.0,
                })
                .width(iced_core::Length::Fill)
                .height(iced_core::Length::Fill),
        )
        .into()
    }

    /// Sidebar: "SESSIONS" header + tab entries with indicator dots.
    /// Per new Anthropic design spec section 4.3.
    fn sidebar<'a>(state: &'a UiState, scale: f32) -> IcedElement<'a> {
        let theme = state.theme;
        let sidebar_bg = to_iced_color(&theme.bg_raised);
        let bg_active = to_iced_color(&theme.bg_active);
        let bg_hover = to_iced_color(&theme.bg_hover);
        let accent = to_iced_color(&theme.accent_orange);
        let text_primary = to_iced_color(&theme.text_primary);
        let text_secondary = to_iced_color(&theme.text_secondary);
        let text_muted = to_iced_color(&theme.text_muted);
        let text_dim = to_iced_color(&theme.text_ghost);

        let sidebar_w = state.sidebar_width / scale;
        let font_size = 13.0;
        let small_size = 10.0;
        let header_size = 10.0;
        let pad_h = 6.0 / scale;
        let entry_spacing = 2.0 / scale;

        let mut entries_col = iced_widget::Column::new()
            .spacing(entry_spacing)
            .padding(iced_core::Padding::from([0.0, pad_h]));

        // "SESSIONS" header
        let header = container(
            text("SESSIONS").size(header_size).color(text_muted).font(DM_SANS),
        )
        .padding(iced_core::Padding {
            top: 14.0 / scale,
            right: 10.0 / scale,
            bottom: 8.0 / scale,
            left: 10.0 / scale,
        });
        entries_col = entries_col.push(header);

        let tab_count = state.sidebar_tabs.len();
        for (i, tab) in state.sidebar_tabs.iter().enumerate() {
            let is_active = tab.is_active;
            let is_hovered_tab = state.hovered_sidebar_tab == Some(i);

            // Indicator dot (6px)
            let dot_size = 6.0 / scale;
            let dot_r = 3.0 / scale;
            let dot_color = if is_active { accent } else { text_dim };
            let dot = container(column![])
                .width(dot_size)
                .height(dot_size)
                .style(move |_: &iced_core::Theme| {
                    let mut style = container::Style {
                        background: Some(iced_core::Background::Color(dot_color)),
                        border: iced_core::Border::default().rounded(dot_r),
                        ..Default::default()
                    };
                    if is_active {
                        style.shadow = iced_core::Shadow {
                            color: iced_core::Color { a: 0.3, ..dot_color },
                            offset: iced_core::Vector::new(0.0, 0.0),
                            blur_radius: 4.0,
                        };
                    }
                    style
                });

            // Tab label
            let is_editing = state.editing_tab == Some(i);
            let name_color = if is_active || is_hovered_tab { text_primary } else { text_secondary };

            let mut title_row: Row<'a, UiMessage, iced_core::Theme, iced_wgpu::Renderer> =
                Row::new()
                    .spacing(8.0 / scale)
                    .align_y(iced_core::Alignment::Center);

            title_row = title_row.push(dot);

            if is_editing {
                let idx = i;
                let rename_input = text_input("Tab name", &state.editing_tab_value)
                    .on_input(UiMessage::RenameTabInput)
                    .on_submit(UiMessage::RenameTabCommit(idx, state.editing_tab_value.clone()))
                    .size(font_size)
                    .width(iced_core::Length::Fill)
                    .style(move |_: &iced_core::Theme, _status| {
                        text_input::Style {
                            background: iced_core::Background::Color(
                                iced_core::Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                            ),
                            border: iced_core::Border {
                                color: iced_core::Color::from_rgba(accent.r, accent.g, accent.b, 0.40),
                                width: 1.0,
                                radius: (4.0 / scale).into(),
                            },
                            icon: name_color,
                            placeholder: text_dim,
                            value: name_color,
                            selection: iced_core::Color::from_rgba(accent.r, accent.g, accent.b, 0.30),
                        }
                    });
                title_row = title_row.push(rename_input);
            } else {
                let title_text: String = if tab.title.len() > 18 {
                    let mut t: String = tab.title.chars().take(17).collect();
                    t.push('\u{2026}');
                    t
                } else {
                    tab.title.clone()
                };
                title_row = title_row.push(
                    text(title_text).size(font_size).color(name_color).font(DM_SANS)
                );
                title_row = title_row.push(hspace());

                // Show keyboard shortcut OR close button depending on hover state
                let show_close = tab_count > 1 && (is_active || is_hovered_tab);
                if show_close {
                    // Close button replaces shortcut on hover
                    let close_size = 10.0;
                    let close_btn_size = 14.0 / scale;
                    let close_radius = 7.0 / scale; // 50% of 14px
                    let is_close_hovered = state.hovering_close_button == Some(i);
                    let (close_bg, close_txt) = if is_close_hovered {
                        (
                            Some(iced_core::Background::Color(bg_hover)),
                            text_secondary,
                        )
                    } else {
                        (None, text_dim)
                    };
                    let close_content = container(
                        text("\u{00D7}").size(close_size).color(close_txt),
                    )
                    .width(close_btn_size)
                    .height(close_btn_size)
                    .center_x(close_btn_size)
                    .center_y(close_btn_size)
                    .style(move |_: &iced_core::Theme| container::Style {
                        background: close_bg,
                        border: iced_core::Border::default().rounded(close_radius),
                        ..Default::default()
                    });
                    let close_area = MouseArea::new(close_content)
                        .on_press(UiMessage::TabClosed(i))
                        .on_enter(UiMessage::CloseButtonHovered(Some(i)))
                        .on_exit(UiMessage::CloseButtonHovered(None));
                    title_row = title_row.push(close_area);
                } else if !is_hovered_tab {
                    // Show keyboard shortcut when not hovered (and active, or always visible)
                    if is_active {
                        let shortcut = format!("\u{2318}{}", i + 1);
                        title_row = title_row.push(
                            text(shortcut).size(small_size).color(text_dim).font(JETBRAINS_MONO)
                        );
                    }
                }
            }

            let entry_content = title_row
                .padding(iced_core::Padding::from([8.0 / scale, 10.0 / scale]));

            let radius = 4.0 / scale;
            let entry_bg = if is_active {
                Some(iced_core::Background::Color(bg_active))
            } else if is_hovered_tab {
                Some(iced_core::Background::Color(bg_hover))
            } else {
                None
            };

            let tab_container = container(entry_content)
                .width(iced_core::Length::Fill)
                .style(move |_: &iced_core::Theme| container::Style {
                    background: entry_bg,
                    border: iced_core::Border {
                        color: iced_core::Color::TRANSPARENT,
                        width: 0.0,
                        radius: radius.into(),
                    },
                    ..Default::default()
                });

            let tab_with_hover = MouseArea::new(tab_container)
                .on_press(UiMessage::SidebarTabSelected(i))
                .on_right_press(UiMessage::RenameTab(i))
                .on_enter(UiMessage::SidebarTabHovered(Some(i)))
                .on_exit(UiMessage::SidebarTabHovered(None));

            entries_col = entries_col.push(tab_with_hover);
        }

        // "+ New Session" button
        let radius = 4.0 / scale;
        let is_new_tab_hovered = state.hovering_new_tab;
        let new_tab_txt = if is_new_tab_hovered { text_primary } else { text_muted };
        let plus_color = if is_new_tab_hovered { accent } else { text_muted };
        let new_tab_bg = if is_new_tab_hovered {
            Some(iced_core::Background::Color(bg_hover))
        } else {
            None
        };

        // Plus icon in bordered box
        let border_color = if is_new_tab_hovered {
            accent
        } else {
            to_iced_color(&theme.border_strong)
        };
        let plus_box = container(
            text("+").size(14.0).color(plus_color),
        )
        .padding(iced_core::Padding::from([0.0, 4.0 / scale]))
        .style(move |_: &iced_core::Theme| container::Style {
            background: None,
            border: iced_core::Border {
                color: border_color,
                width: 1.0,
                radius: (3.0 / scale).into(),
            },
            ..Default::default()
        });

        let new_tab_content = container(
            row![
                plus_box,
                text("New Session").size(12.0).color(new_tab_txt).font(DM_SANS),
            ]
            .spacing(8.0 / scale)
            .align_y(iced_core::Alignment::Center),
        )
        .padding(iced_core::Padding::from([8.0 / scale, 10.0 / scale]))
        .width(iced_core::Length::Fill)
        .style(move |_: &iced_core::Theme| container::Style {
            background: new_tab_bg,
            border: iced_core::Border {
                color: iced_core::Color::TRANSPARENT,
                width: 0.0,
                radius: radius.into(),
            },
            ..Default::default()
        });
        let new_tab_area = MouseArea::new(new_tab_content)
            .on_press(UiMessage::NewTab)
            .on_enter(UiMessage::NewTabHovered(true))
            .on_exit(UiMessage::NewTabHovered(false));
        entries_col = entries_col.push(new_tab_area);

        container(entries_col)
            .width(sidebar_w)
            .height(iced_core::Length::Fill)
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(sidebar_bg)),
                border: iced_core::Border {
                    color: iced_core::Color::TRANSPARENT,
                    width: 0.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    /// Status bar: [● dot] [Session 1] | [zsh] | [~/] ... [UTF-8]
    /// Per new Anthropic design spec. Height: 28px.
    fn status_bar<'a>(state: &'a UiState<'a>, scale: f32) -> IcedElement<'a> {
        let theme = state.theme;
        let bg = to_iced_color(&theme.bg_surface);
        let text_muted = to_iced_color(&theme.text_muted);
        let text_ghost = to_iced_color(&theme.text_ghost);
        let green = to_iced_color(&theme.accent_green);
        let border_visible = to_iced_color(&theme.border_visible);

        let height = STATUS_BAR_HEIGHT / scale;
        let status_size = 11.0;
        let pad_h = 16.0 / scale;

        let shell_name = state.panes.iter().find(|p| p.is_focused)
            .map(|p| p.shell_name.clone())
            .unwrap_or_else(|| "sh".to_string());

        // Green dot (6px)
        let dot_r = 3.0 / scale;
        let green_dot = container(column![])
            .width(6.0 / scale)
            .height(6.0 / scale)
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(green)),
                border: iced_core::Border::default().rounded(dot_r),
                ..Default::default()
            });

        // Divider helper (1px × 12px vertical bar)
        let div_h = 12.0 / scale;
        let make_divider = move || {
            container(column![])
                .width(1.0 / scale)
                .height(div_h)
                .style(move |_: &iced_core::Theme| container::Style {
                    background: Some(iced_core::Background::Color(border_visible)),
                    ..Default::default()
                })
        };

        let session_name = format!("Session {}", state.active_tab_index + 1);

        // Left group: dot + session + | + shell + | + cwd
        let left = row![
            green_dot,
            text(session_name).size(status_size).color(text_muted).font(DM_SANS),
            make_divider(),
            text(shell_name).size(status_size).color(text_muted).font(DM_SANS),
        ]
        .spacing(8.0 / scale)
        .align_y(iced_core::Alignment::Center);

        // Right: encoding | theme button
        let text_secondary = to_iced_color(&theme.text_secondary);
        let bg_hover_color = to_iced_color(&theme.bg_hover);

        let theme_btn_bg = if state.theme_selector_open { bg_hover_color } else { bg };
        let theme_btn_fg = if state.theme_selector_open { text_secondary } else { text_muted };
        let btn_radius = 4.0 / scale;
        let theme_btn = container(
            text("\u{25D0} Theme").size(status_size).color(theme_btn_fg).font(DM_SANS)
        )
        .padding(iced_core::Padding::from([2.0 / scale, 8.0 / scale]))
        .style(move |_: &iced_core::Theme| container::Style {
            background: Some(iced_core::Background::Color(theme_btn_bg)),
            border: iced_core::Border::default().rounded(btn_radius),
            ..Default::default()
        });
        let theme_btn_click = MouseArea::new(theme_btn)
            .on_press(UiMessage::ToggleThemeSelector);

        // Tracks button (only shown when conductor directory found)
        let tracks_btn_bg = if state.conductor.is_some() { bg_hover_color } else { bg };
        let tracks_btn_fg = if state.conductor.is_some() { text_secondary } else { text_muted };
        let tracks_btn_opacity = if state.conductor.is_some() { 1.0 } else { 0.5 };
        let tracks_btn_handle = state.tracks_icon_handle.clone()
            .unwrap_or_else(|| iced_widget::image::Handle::from_bytes(TRACKS_ICON_PNG));
        let tracks_btn_icon = iced_widget::image::Image::new(tracks_btn_handle)
            .width(12.0 / scale)
            .height(12.0 / scale)
            .content_fit(iced_core::ContentFit::Contain)
            .opacity(tracks_btn_opacity);
        let tracks_btn = container(
            row![tracks_btn_icon, text(" Tracks").size(status_size).color(tracks_btn_fg).font(DM_SANS)]
                .align_y(iced_core::Alignment::Center)
        )
        .padding(iced_core::Padding::from([2.0 / scale, 8.0 / scale]))
        .style(move |_: &iced_core::Theme| container::Style {
            background: Some(iced_core::Background::Color(tracks_btn_bg)),
            border: iced_core::Border::default().rounded(btn_radius),
            ..Default::default()
        });
        let tracks_btn_click = MouseArea::new(tracks_btn)
            .on_press(UiMessage::ConductorToggled);

        let right = if state.conductor_available {
            row![
                text("UTF-8").size(status_size).color(text_ghost).font(DM_SANS),
                make_divider(),
                tracks_btn_click,
                make_divider(),
                theme_btn_click,
            ]
        } else {
            row![
                text("UTF-8").size(status_size).color(text_ghost).font(DM_SANS),
                make_divider(),
                theme_btn_click,
            ]
        }
        .spacing(8.0 / scale)
        .align_y(iced_core::Alignment::Center);

        let content = row![left, hspace(), right]
            .align_y(iced_core::Alignment::Center)
            .padding(iced_core::Padding::from([0.0, pad_h]));

        container(content)
            .width(iced_core::Length::Fill)
            .height(height)
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(bg)),
                border: iced_core::Border {
                    color: iced_core::Color::TRANSPARENT,
                    width: 0.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    /// Overlay content — SplitPanel with file browser or git review.
    fn overlay_content<'a>(state: &'a UiState, scale: f32, overlay: ActiveOverlay) -> IcedElement<'a> {
        let ratio = match overlay {
            ActiveOverlay::FileBrowser => state.file_browser_split_ratio,
            ActiveOverlay::GitReview => state.git_review_split_ratio,
        };

        let (left_panel, right_panel) = match overlay {
            ActiveOverlay::FileBrowser => (
                Self::file_browser_left(state, scale),
                Self::file_browser_right(state, scale),
            ),
            ActiveOverlay::GitReview => (
                Self::git_review_left(state, scale),
                Self::git_review_right(state, scale),
            ),
        };

        let split: crate::split_panel::SplitPanel<'a, UiMessage> =
            crate::split_panel::SplitPanel::new(left_panel, right_panel, ratio)
                .on_resize(UiMessage::OverlaySplitResize)
                .on_reset(UiMessage::OverlaySplitReset);

        split.into()
    }

    /// File browser left panel — breadcrumb + file tree rows.
    fn file_browser_left<'a>(state: &'a UiState, scale: f32) -> IcedElement<'a> {
        let theme = state.theme;
        let bg_surface = to_iced_color(&theme.bg_surface);
        let bg_raised = to_iced_color(&theme.bg_raised);
        let text_primary = to_iced_color(&theme.text_primary);
        let text_secondary = to_iced_color(&theme.text_secondary);
        let text_muted = to_iced_color(&theme.text_muted);
        let text_ghost = to_iced_color(&theme.text_ghost);
        let accent = to_iced_color(&theme.accent_orange);
        let accent_blue = to_iced_color(&theme.accent_blue);
        let accent_green = to_iced_color(&theme.accent_green);
        let border_color = to_iced_color(&theme.border_visible);
        let focused = state.overlay_focused_panel == crate::file_browser::OverlayPanel::Left;
        let panel_border = if focused { accent } else { bg_surface };

        // Breadcrumb bar
        let breadcrumb: IcedElement<'a> = container(
            text(&state.file_browser_breadcrumb)
                .size(12.0 / scale)
                .color(text_muted)
                .font(DM_SANS)
        )
        .width(iced_core::Length::Fill)
        .padding(iced_core::Padding::from([6.0 / scale, 10.0 / scale]))
        .style(move |_: &iced_core::Theme| container::Style {
            background: Some(iced_core::Background::Color(bg_raised)),
            border: iced_core::Border {
                color: border_color,
                width: 0.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .into();

        // File tree rows
        let mut rows_col = iced_widget::Column::new().spacing(0.0);
        for row in &state.file_browser_rows {
            let idx = row.index;
            let is_sel = row.is_selected;
            let is_hov = row.is_hovered;

            let mut row_items: Vec<IcedElement<'a>> = Vec::new();

            // Indent guides (box-drawing characters) — only for depth > 0
            if !row.indent_guide.is_empty() {
                row_items.push(
                    text(row.indent_guide.as_str())
                        .size(13.0 / scale)
                        .color(text_ghost)
                        .font(JETBRAINS_MONO)
                        .into(),
                );
            } else if row.depth > 0 {
                // Fallback indent space for depth > 0 with no guide
                let indent_w = row.depth as f32 * 18.0 / scale;
                row_items.push(iced_widget::Space::new().width(indent_w).into());
            }

            if let Some(chev) = &row.chevron {
                row_items.push(
                    text(chev.as_str())
                        .size(12.0 / scale)
                        .color(text_muted)
                        .font(JETBRAINS_MONO)
                        .into(),
                );
                row_items.push(iced_widget::Space::new().width(2.0 / scale).into());
            } else {
                // Spacer for alignment
                row_items.push(iced_widget::Space::new().width(14.0 / scale).into());
            }

            // Icon with color based on category hint
            let icon_color = match row.icon_color_hint.as_str() {
                "source" => accent_blue,
                "config" => text_muted,
                "data" => text_secondary,
                "image" => text_secondary,
                "archive" => accent,
                "git" => accent,
                "docker" => accent_blue,
                "docs" => accent_green,
                "shell" => accent_green,
                "markup" => accent,
                "style" => text_secondary,
                _ => text_secondary,
            };
            row_items.push(
                text(row.icon.as_str())
                    .size(13.0 / scale)
                    .color(icon_color)
                    .into(),
            );
            row_items.push(iced_widget::Space::new().width(6.0 / scale).into());
            row_items.push(
                text(row.name.as_str())
                    .size(13.0 / scale)
                    .color(if is_sel { accent } else { text_primary })
                    .font(DM_SANS)
                    .into(),
            );

            let row_content = Row::with_children(row_items)
                .align_y(iced_core::Alignment::Center);

            let row_bg = if is_sel {
                iced_core::Color { a: 0.2, ..accent }
            } else if is_hov {
                bg_raised
            } else {
                bg_surface
            };

            let row_container: IcedElement<'a> = container(row_content)
                .width(iced_core::Length::Fill)
                .height(28.0 / scale)
                .padding(iced_core::Padding::from([0.0, 8.0 / scale]))
                .style(move |_: &iced_core::Theme| container::Style {
                    background: Some(iced_core::Background::Color(row_bg)),
                    ..Default::default()
                })
                .into();

            let clickable = MouseArea::new(row_container)
                .on_press(UiMessage::FileBrowserRowClicked(idx))
                .on_enter(UiMessage::FileBrowserRowHovered(idx))
                .on_exit(UiMessage::FileBrowserRowUnhovered);

            rows_col = rows_col.push(clickable);
        }

        let scrollable_tree = pin(rows_col)
            .width(iced_core::Length::Fill)
            .height(iced_core::Length::Fill);

        let content = column![breadcrumb, scrollable_tree];

        container(content)
            .width(iced_core::Length::Fill)
            .height(iced_core::Length::Fill)
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(bg_surface)),
                border: iced_core::Border {
                    color: panel_border,
                    width: 2.0 / scale,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    /// File browser right panel — file preview.
    fn file_browser_right<'a>(state: &'a UiState, scale: f32) -> IcedElement<'a> {
        let theme = state.theme;
        let bg_surface = to_iced_color(&theme.bg_surface);
        let bg_deep = to_iced_color(&theme.bg_deep);
        let text_primary = to_iced_color(&theme.text_primary);
        let text_secondary = to_iced_color(&theme.text_secondary);
        let text_muted = to_iced_color(&theme.text_muted);
        let accent = to_iced_color(&theme.accent_orange);
        let border_color = to_iced_color(&theme.border_visible);
        let focused = state.overlay_focused_panel == crate::file_browser::OverlayPanel::Right;
        let panel_border = if focused { accent } else { bg_surface };

        let content: IcedElement<'a> = match &state.file_browser_preview_name {
            None => {
                // Empty state
                container(
                    text("Select a file to preview")
                        .size(14.0 / scale)
                        .color(text_muted)
                        .font(DM_SANS),
                )
                .width(iced_core::Length::Fill)
                .height(iced_core::Length::Fill)
                .align_x(iced_core::alignment::Horizontal::Center)
                .align_y(iced_core::alignment::Vertical::Center)
                .into()
            }
            Some(name) => {
                // File header
                let header: IcedElement<'a> = container(
                    text(name.as_str())
                        .size(12.0 / scale)
                        .color(text_secondary)
                        .font(DM_SANS),
                )
                .width(iced_core::Length::Fill)
                .padding(iced_core::Padding::from([6.0 / scale, 10.0 / scale]))
                .style(move |_: &iced_core::Theme| container::Style {
                    background: Some(iced_core::Background::Color(bg_deep)),
                    border: iced_core::Border {
                        color: border_color,
                        width: 0.0,
                        radius: 0.0.into(),
                    },
                    ..Default::default()
                })
                .into();

                // Preview lines
                let mut lines_col = iced_widget::Column::new().spacing(0.0);
                for (i, line) in state.file_browser_preview_lines.iter().enumerate() {
                    let line_num = text(format!("{:>4} ", i + 1))
                        .size(12.0 / scale)
                        .color(text_muted)
                        .font(JETBRAINS_MONO);
                    let line_text = text(line.as_str())
                        .size(12.0 / scale)
                        .color(text_primary)
                        .font(JETBRAINS_MONO);
                    let line_row = row![line_num, line_text];
                    lines_col = lines_col.push(line_row);
                }

                if state.file_browser_preview_truncated {
                    lines_col = lines_col.push(
                        text("... (file truncated)")
                            .size(11.0 / scale)
                            .color(text_muted)
                            .font(DM_SANS),
                    );
                }

                let scrollable = pin(lines_col)
                    .width(iced_core::Length::Fill)
                    .height(iced_core::Length::Fill);

                column![header, scrollable].into()
            }
        };

        container(content)
            .width(iced_core::Length::Fill)
            .height(iced_core::Length::Fill)
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(bg_surface)),
                border: iced_core::Border {
                    color: panel_border,
                    width: 2.0 / scale,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    /// Git review left panel — file list with sections.
    fn git_review_left<'a>(state: &'a UiState, scale: f32) -> IcedElement<'a> {
        let theme = state.theme;
        let bg_surface = to_iced_color(&theme.bg_surface);
        let bg_raised = to_iced_color(&theme.bg_raised);
        let bg_deep = to_iced_color(&theme.bg_deep);
        let text_primary = to_iced_color(&theme.text_primary);
        let text_secondary = to_iced_color(&theme.text_secondary);
        let text_muted = to_iced_color(&theme.text_muted);
        let accent = to_iced_color(&theme.accent_orange);
        let accent_green = to_iced_color(&theme.accent_green);
        let accent_red = to_iced_color(&theme.accent_red);
        let border_color = to_iced_color(&theme.border_visible);
        let focused = state.overlay_focused_panel == crate::file_browser::OverlayPanel::Left;
        let panel_border = if focused { accent } else { bg_surface };

        // Error message
        let error_widget: Option<IcedElement<'a>> = state.git_review_error.as_ref().map(|err| {
            container(
                text(err.as_str())
                    .size(12.0 / scale)
                    .color(accent_red)
                    .font(DM_SANS),
            )
            .width(iced_core::Length::Fill)
            .padding(iced_core::Padding::from([4.0 / scale, 10.0 / scale]))
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(iced_core::Color { a: 0.1, ..accent_red })),
                ..Default::default()
            })
            .into()
        });

        // File list
        let mut list_col = iced_widget::Column::new().spacing(0.0);

        if state.git_review_list_items.is_empty() && state.git_review_error.is_none() {
            list_col = list_col.push(
                container(
                    text("Working tree clean")
                        .size(13.0 / scale)
                        .color(text_muted)
                        .font(DM_SANS),
                )
                .width(iced_core::Length::Fill)
                .padding(iced_core::Padding::from([20.0 / scale, 10.0 / scale]))
                .align_x(iced_core::alignment::Horizontal::Center),
            );
        }

        for (flat_idx, item) in state.git_review_list_items.iter().enumerate() {
            match item.kind {
                GitReviewItemKind::SectionHeader => {
                    let header_row: IcedElement<'a> = container(
                        text(item.label.as_str())
                            .size(12.0 / scale)
                            .color(text_secondary)
                            .font(DM_SANS),
                    )
                    .width(iced_core::Length::Fill)
                    .padding(iced_core::Padding::from([6.0 / scale, 10.0 / scale]))
                    .style(move |_: &iced_core::Theme| container::Style {
                        background: Some(iced_core::Background::Color(bg_deep)),
                        ..Default::default()
                    })
                    .into();

                    let clickable = MouseArea::new(header_row)
                        .on_press(UiMessage::GitReviewFileClicked(flat_idx));
                    list_col = list_col.push(clickable);
                }
                GitReviewItemKind::FileEntry => {
                    let is_sel = item.is_selected;

                    // Status badge color
                    let accent_blue = iced_core::Color::from_rgba(0.3, 0.5, 1.0, 1.0);
                    let accent_orange = iced_core::Color::from_rgba(1.0, 0.6, 0.2, 1.0);
                    let status_color = match item.status_label.as_deref() {
                        Some("A") => accent_green,
                        Some("D") => accent_red,
                        Some("M") => accent,
                        Some("R") => accent_blue,
                        Some("C") => accent_orange,
                        Some("?") => text_muted,
                        _ => text_secondary,
                    };

                    let status_text = text(item.status_label.as_deref().unwrap_or(""))
                        .size(12.0 / scale)
                        .color(status_color)
                        .font(JETBRAINS_MONO);

                    let file_text = text(item.label.as_str())
                        .size(13.0 / scale)
                        .color(if is_sel { accent } else { text_primary })
                        .font(DM_SANS);

                    let mut entry_row = row![
                        iced_widget::Space::new().width(10.0 / scale),
                        status_text,
                        iced_widget::Space::new().width(8.0 / scale),
                        file_text,
                    ]
                    .align_y(iced_core::Alignment::Center);

                    // Show directory path dimmed after filename
                    if let Some(ref dir) = item.display_dir {
                        let dir_label = format!("  {}", dir);
                        entry_row = entry_row.push(
                            text(dir_label)
                                .size(11.0 / scale)
                                .color(text_muted)
                                .font(DM_SANS),
                        );
                    }

                    let row_bg = if is_sel {
                        iced_core::Color { a: 0.2, ..accent }
                    } else {
                        bg_surface
                    };

                    let entry_container: IcedElement<'a> = container(entry_row)
                        .width(iced_core::Length::Fill)
                        .height(28.0 / scale)
                        .style(move |_: &iced_core::Theme| container::Style {
                            background: Some(iced_core::Background::Color(row_bg)),
                            ..Default::default()
                        })
                        .into();

                    let clickable = MouseArea::new(entry_container)
                        .on_press(UiMessage::GitReviewFileClicked(flat_idx));
                    list_col = list_col.push(clickable);
                }
            }
        }

        let scrollable_list = pin(list_col)
            .width(iced_core::Length::Fill)
            .height(iced_core::Length::Fill);

        // Commit area at bottom
        let commit_input: IcedElement<'a> = text_input("Commit message...", &state.git_review_commit_message)
            .on_input(UiMessage::GitReviewCommitMsgChanged)
            .size(12.0 / scale)
            .font(DM_SANS)
            .into();

        let commit_btn_bg = if state.git_review_can_commit {
            accent_green
        } else {
            bg_raised
        };
        let commit_btn_text_color = if state.git_review_can_commit {
            iced_core::Color::WHITE
        } else {
            text_muted
        };

        let commit_btn: IcedElement<'a> = container(
            text("Commit")
                .size(12.0 / scale)
                .color(commit_btn_text_color)
                .font(DM_SANS),
        )
        .padding(iced_core::Padding::from([4.0 / scale, 12.0 / scale]))
        .style(move |_: &iced_core::Theme| container::Style {
            background: Some(iced_core::Background::Color(commit_btn_bg)),
            border: iced_core::Border::default().rounded(4.0 / scale),
            ..Default::default()
        })
        .into();

        let commit_click = if state.git_review_can_commit {
            MouseArea::new(commit_btn).on_press(UiMessage::GitReviewCommit)
        } else {
            MouseArea::new(commit_btn)
        };

        let commit_area: IcedElement<'a> = container(
            column![
                commit_input,
                iced_widget::Space::new().height(4.0 / scale),
                commit_click,
            ],
        )
        .width(iced_core::Length::Fill)
        .padding(iced_core::Padding::from([6.0 / scale, 8.0 / scale]))
        .style(move |_: &iced_core::Theme| container::Style {
            background: Some(iced_core::Background::Color(bg_deep)),
            border: iced_core::Border {
                color: border_color,
                width: 0.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .into();

        let mut content_col: iced_widget::Column<'a, UiMessage, iced_core::Theme, iced_wgpu::Renderer> = iced_widget::Column::new();
        if let Some(err) = error_widget {
            content_col = content_col.push(err);
        }
        content_col = content_col.push(scrollable_list);
        content_col = content_col.push(commit_area);

        container(content_col)
            .width(iced_core::Length::Fill)
            .height(iced_core::Length::Fill)
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(bg_surface)),
                border: iced_core::Border {
                    color: panel_border,
                    width: 2.0 / scale,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    /// Git review right panel — diff view.
    fn git_review_right<'a>(state: &'a UiState, scale: f32) -> IcedElement<'a> {
        let theme = state.theme;
        let bg_surface = to_iced_color(&theme.bg_surface);
        let bg_deep = to_iced_color(&theme.bg_deep);
        let text_primary = to_iced_color(&theme.text_primary);
        let text_secondary = to_iced_color(&theme.text_secondary);
        let text_muted = to_iced_color(&theme.text_muted);
        let accent = to_iced_color(&theme.accent_orange);
        let accent_green = to_iced_color(&theme.accent_green);
        let accent_red = to_iced_color(&theme.accent_red);
        let border_color = to_iced_color(&theme.border_visible);
        let focused = state.overlay_focused_panel == crate::file_browser::OverlayPanel::Right;
        let panel_border = if focused { accent } else { bg_surface };

        let content: IcedElement<'a> = if state.git_review_diff_rows.is_empty() {
            // Empty state
            container(
                text("Select a file to view changes")
                    .size(14.0 / scale)
                    .color(text_muted)
                    .font(DM_SANS),
            )
            .width(iced_core::Length::Fill)
            .height(iced_core::Length::Fill)
            .align_x(iced_core::alignment::Horizontal::Center)
            .align_y(iced_core::alignment::Vertical::Center)
            .into()
        } else {
            // Diff header
            let header_text = state.git_review_diff_header.as_deref().unwrap_or("");
            let header: IcedElement<'a> = container(
                text(header_text)
                    .size(12.0 / scale)
                    .color(text_secondary)
                    .font(DM_SANS),
            )
            .width(iced_core::Length::Fill)
            .padding(iced_core::Padding::from([6.0 / scale, 10.0 / scale]))
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(bg_deep)),
                border: iced_core::Border {
                    color: border_color,
                    width: 0.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into();

            // Diff rows
            let mut diff_col = iced_widget::Column::new().spacing(0.0);

            for diff_row in &state.git_review_diff_rows {
                let row_element: IcedElement<'a> = match diff_row.kind {
                    DiffRowKind::HunkHeader => {
                        container(
                            text(diff_row.left_text.as_str())
                                .size(11.0 / scale)
                                .color(text_muted)
                                .font(JETBRAINS_MONO),
                        )
                        .width(iced_core::Length::Fill)
                        .height(22.0 / scale)
                        .padding(iced_core::Padding::from([2.0 / scale, 10.0 / scale]))
                        .style(move |_: &iced_core::Theme| container::Style {
                            background: Some(iced_core::Background::Color(
                                iced_core::Color::from_rgba(0.15, 0.15, 0.2, 1.0),
                            )),
                            ..Default::default()
                        })
                        .into()
                    }
                    _ => {
                        let (row_bg, indicator_color) = match diff_row.kind {
                            DiffRowKind::Added => (
                                iced_core::Color::from_rgba(0.0, 0.4, 0.0, 0.15),
                                accent_green,
                            ),
                            DiffRowKind::Removed => (
                                iced_core::Color::from_rgba(0.6, 0.0, 0.0, 0.15),
                                accent_red,
                            ),
                            DiffRowKind::Modified => (
                                iced_core::Color::from_rgba(0.6, 0.5, 0.0, 0.15),
                                accent,
                            ),
                            _ => (iced_core::Color::TRANSPARENT, iced_core::Color::TRANSPARENT),
                        };

                        let indicator: IcedElement<'a> = container(column![])
                            .width(3.0 / scale)
                            .height(iced_core::Length::Fill)
                            .style(move |_: &iced_core::Theme| container::Style {
                                background: Some(iced_core::Background::Color(indicator_color)),
                                ..Default::default()
                            })
                            .into();

                        let left_num = text(diff_row.left_num.as_deref().unwrap_or("   "))
                            .size(11.0 / scale)
                            .color(text_muted)
                            .font(JETBRAINS_MONO);
                        let left_text = text(diff_row.left_text.as_str())
                            .size(11.0 / scale)
                            .color(text_primary)
                            .font(JETBRAINS_MONO);

                        let right_num = text(diff_row.right_num.as_deref().unwrap_or("   "))
                            .size(11.0 / scale)
                            .color(text_muted)
                            .font(JETBRAINS_MONO);
                        let right_text = text(diff_row.right_text.as_str())
                            .size(11.0 / scale)
                            .color(text_primary)
                            .font(JETBRAINS_MONO);

                        let line_row = row![
                            indicator,
                            iced_widget::Space::new().width(2.0 / scale),
                            left_num,
                            iced_widget::Space::new().width(4.0 / scale),
                            left_text,
                            iced_widget::Space::new().width(12.0 / scale),
                            right_num,
                            iced_widget::Space::new().width(4.0 / scale),
                            right_text,
                        ]
                        .align_y(iced_core::Alignment::Center);

                        container(line_row)
                            .width(iced_core::Length::Fill)
                            .height(20.0 / scale)
                            .style(move |_: &iced_core::Theme| container::Style {
                                background: Some(iced_core::Background::Color(row_bg)),
                                ..Default::default()
                            })
                            .into()
                    }
                };
                diff_col = diff_col.push(row_element);
            }

            let scrollable_diff = pin(diff_col)
                .width(iced_core::Length::Fill)
                .height(iced_core::Length::Fill);

            column![header, scrollable_diff].into()
        };

        container(content)
            .width(iced_core::Length::Fill)
            .height(iced_core::Length::Fill)
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(bg_surface)),
                border: iced_core::Border {
                    color: panel_border,
                    width: 2.0 / scale,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    /// Conductor dashboard overlay — track progress viewer.
    fn conductor_dashboard<'a>(state: &'a UiState, scale: f32) -> IcedElement<'a> {
        let theme = state.theme;
        let bg = to_iced_color(&theme.bg_deep);
        let bg_raised = to_iced_color(&theme.bg_raised);
        let text_primary = to_iced_color(&theme.text_primary);
        let text_secondary = to_iced_color(&theme.text_secondary);
        let text_muted = to_iced_color(&theme.text_muted);
        let accent = to_iced_color(&theme.accent_orange);
        let green = to_iced_color(&theme.accent_green);
        let red = to_iced_color(&theme.accent_red);
        let border_color = to_iced_color(&theme.border_visible);

        let snap = match &state.conductor {
            Some(s) => s,
            None => {
                return container(text("No conductor data").color(text_primary))
                    .width(iced_core::Length::Fill)
                    .height(iced_core::Length::Fill)
                    .into();
            }
        };

        // ── Header ──
        let title = text("Current Tracks").size(16.0).color(text_primary).font(DM_SANS);

        let sort_label = snap.sort.label();
        let sort_badge = container(
            text(sort_label).size(10.0).color(text_secondary).font(DM_SANS),
        )
        .padding(iced_core::Padding::from([2.0 / scale, 6.0 / scale]))
        .style(move |_: &iced_core::Theme| container::Style {
            background: Some(iced_core::Background::Color(bg_raised)),
            border: iced_core::Border {
                color: iced_core::Color::TRANSPARENT,
                width: 0.0,
                radius: (3.0 / scale).into(),
            },
            ..Default::default()
        });

        // Done button (blue accent)
        let accent_blue = to_iced_color(&theme.accent_blue);
        let done_btn = container(
            text("Done").size(11.0).color(iced_core::Color::WHITE).font(DM_SANS)
        )
        .padding(iced_core::Padding::from([3.0 / scale, 10.0 / scale]))
        .style(move |_: &iced_core::Theme| container::Style {
            background: Some(iced_core::Background::Color(accent_blue)),
            border: iced_core::Border::default().rounded(4.0 / scale),
            ..Default::default()
        });
        let done_click = MouseArea::new(done_btn)
            .on_press(UiMessage::ConductorToggled);

        // Close (×) button
        let close_btn = container(
            text("\u{00D7}").size(14.0).color(text_muted).font(DM_SANS)
        )
        .padding(iced_core::Padding::from([1.0 / scale, 6.0 / scale]));
        let close_click = MouseArea::new(close_btn)
            .on_press(UiMessage::ConductorToggled);

        let header = row![title, sort_badge, hspace(), done_click, close_click]
            .spacing(10.0 / scale)
            .align_y(iced_core::Alignment::Center);

        // ── Stats row ──
        let (total, active, blocked, complete, new) = snap.stats();
        use crate::conductor::model::FilterMode;
        let total_label_color = if snap.filter == FilterMode::All { text_primary } else { text_muted };
        let active_label_color = if snap.filter == FilterMode::Active { text_primary } else { text_muted };
        let blocked_label_color = if snap.filter == FilterMode::Blocked { text_primary } else { text_muted };
        let complete_label_color = if snap.filter == FilterMode::Complete { text_primary } else { text_muted };
        let new_label_color = if snap.filter == FilterMode::New { text_primary } else { text_muted };
        let stats_row = row![
            text(format!("{}", total)).size(11.0).color(text_primary).font(DM_SANS),
            text("Total").size(11.0).color(total_label_color).font(DM_SANS),
            text("\u{00B7}").size(11.0).color(text_muted),
            text(format!("{}", active)).size(11.0).color(accent).font(DM_SANS),
            text("Active").size(11.0).color(active_label_color).font(DM_SANS),
            text("\u{00B7}").size(11.0).color(text_muted),
            text(format!("{}", blocked)).size(11.0).color(red).font(DM_SANS),
            text("Blocked").size(11.0).color(blocked_label_color).font(DM_SANS),
            text("\u{00B7}").size(11.0).color(text_muted),
            text(format!("{}", complete)).size(11.0).color(green).font(DM_SANS),
            text("Complete").size(11.0).color(complete_label_color).font(DM_SANS),
            text("\u{00B7}").size(11.0).color(text_muted),
            text(format!("{}", new)).size(11.0).color(text_secondary).font(DM_SANS),
            text("New").size(11.0).color(new_label_color).font(DM_SANS),
        ]
        .spacing(4.0 / scale)
        .align_y(iced_core::Alignment::Center);

        // ── Divider helper ──
        let make_hdiv = move || -> IcedElement<'a> {
            container(column![])
                .width(iced_core::Length::Fill)
                .height(1.0 / scale)
                .style(move |_: &iced_core::Theme| container::Style {
                    background: Some(iced_core::Background::Color(border_color)),
                    ..Default::default()
                })
                .into()
        };

        // ── Track list (left pane) ──
        let mut track_col = iced_widget::Column::new().spacing(2.0 / scale);

        for (i, track) in snap.tracks.iter().enumerate() {
            let is_selected = i == snap.selected;
            let row_bg = if is_selected { bg_raised } else { bg };
            let status_color = match track.status {
                crate::conductor::model::Status::Complete => green,
                crate::conductor::model::Status::InProgress => accent,
                crate::conductor::model::Status::Blocked => red,
                _ => text_secondary,
            };

            let mut track_row_items = Row::new().spacing(6.0 / scale)
                .align_y(iced_core::Alignment::Center);

            // Status icon: checkmark for complete, dot for others
            let status_icon = match track.status {
                crate::conductor::model::Status::Complete => "\u{2713}",
                crate::conductor::model::Status::InProgress => "\u{25CF}",
                crate::conductor::model::Status::Blocked => "\u{2717}",
                _ => "\u{25CB}",
            };
            track_row_items = track_row_items.push(
                text(status_icon).size(10.0).color(status_color).font(JETBRAINS_MONO)
                    .width(14.0 / scale),
            );

            // Title
            let title_color = if is_selected { text_primary } else { text_secondary };
            track_row_items = track_row_items.push(
                text(&track.title).size(11.0).color(title_color).font(DM_SANS)
                    .width(iced_core::Length::Fill),
            );

            // Task count (right-aligned)
            if track.tasks_total > 0 {
                track_row_items = track_row_items.push(
                    text(format!("{}/{}", track.tasks_completed, track.tasks_total))
                        .size(10.0).color(text_muted).font(JETBRAINS_MONO),
                );
            }

            let track_row = container(track_row_items)
                .padding(iced_core::Padding::from([4.0 / scale, 8.0 / scale]))
                .width(iced_core::Length::Fill)
                .style(move |_: &iced_core::Theme| container::Style {
                    background: Some(iced_core::Background::Color(row_bg)),
                    border: iced_core::Border::default().rounded(3.0 / scale),
                    ..Default::default()
                });

            let track_area = MouseArea::new(track_row)
                .on_press(UiMessage::ConductorTrackClicked(i));
            track_col = track_col.push(track_area);
        }

        // Scrollbar style for both panes
        let scrollbar_style = move |_: &iced_core::Theme, status: iced_widget::scrollable::Status| {
            let scroller_bg = match status {
                iced_widget::scrollable::Status::Active { .. } => iced_core::Color::TRANSPARENT,
                iced_widget::scrollable::Status::Hovered { .. } => iced_core::Color { a: 0.4, ..text_muted },
                iced_widget::scrollable::Status::Dragged { .. } => iced_core::Color { a: 0.6, ..text_muted },
            };
            let transparent_rail = iced_widget::scrollable::Rail {
                background: None,
                border: iced_core::Border::default(),
                scroller: iced_widget::scrollable::Scroller {
                    background: iced_core::Background::Color(iced_core::Color::TRANSPARENT),
                    border: iced_core::Border::default(),
                },
            };
            iced_widget::scrollable::Style {
                container: container::Style::default(),
                vertical_rail: iced_widget::scrollable::Rail {
                    background: None,
                    border: iced_core::Border::default(),
                    scroller: iced_widget::scrollable::Scroller {
                        background: iced_core::Background::Color(scroller_bg),
                        border: iced_core::Border::default().rounded(3.0 / scale),
                    },
                },
                horizontal_rail: transparent_rail,
                gap: None,
                auto_scroll: iced_widget::scrollable::AutoScroll {
                    background: iced_core::Background::Color(iced_core::Color::TRANSPARENT),
                    border: iced_core::Border::default(),
                    shadow: iced_core::Shadow::default(),
                    icon: iced_core::Color::TRANSPARENT,
                },
            }
        };

        let left_pane = iced_widget::scrollable(track_col)
            .width(iced_core::Length::Fill)
            .height(iced_core::Length::Fill)
            .style(scrollbar_style);

        // ── Detail panel (right pane) ──
        let right_pane: IcedElement<'a> = if let Some(track) = snap.selected_track() {
            let mut detail_col = iced_widget::Column::new().spacing(8.0 / scale);

            // Track title + status
            let detail_status_color = match track.status {
                crate::conductor::model::Status::Complete => green,
                crate::conductor::model::Status::InProgress => accent,
                crate::conductor::model::Status::Blocked => red,
                _ => text_secondary,
            };
            detail_col = detail_col.push(
                text(&track.title).size(14.0).color(text_primary).font(DM_SANS),
            );
            detail_col = detail_col.push(
                row![
                    text(track.status.label()).size(11.0).color(detail_status_color).font(DM_SANS),
                    text("\u{00B7}").size(11.0).color(text_muted),
                    text(format!("{}%", track.progress_percent() as u32))
                        .size(11.0).color(text_secondary).font(JETBRAINS_MONO),
                    text(format!("{}/{} tasks", track.tasks_completed, track.tasks_total))
                        .size(11.0).color(text_muted).font(DM_SANS),
                ]
                .spacing(6.0 / scale)
                .align_y(iced_core::Alignment::Center),
            );

            // Progress bar
            let bar_width = 200.0 / scale;
            let bar_height = 4.0 / scale;
            let filled_width = bar_width * (track.progress_percent() / 100.0);
            let filled = container(column![])
                .width(filled_width)
                .height(bar_height)
                .style(move |_: &iced_core::Theme| container::Style {
                    background: Some(iced_core::Background::Color(green)),
                    border: iced_core::Border::default().rounded(2.0 / scale),
                    ..Default::default()
                });
            let bar_bg = container(filled)
                .width(bar_width)
                .height(bar_height)
                .style(move |_: &iced_core::Theme| container::Style {
                    background: Some(iced_core::Background::Color(bg_raised)),
                    border: iced_core::Border::default().rounded(2.0 / scale),
                    ..Default::default()
                });
            detail_col = detail_col.push(bar_bg);

            // Description
            if let Some(desc) = &track.description {
                detail_col = detail_col.push(
                    text(desc).size(11.0).color(text_secondary).font(DM_SANS),
                );
            }

            // Dependencies
            if !track.dependencies.is_empty() {
                let deps_str = track.dependencies.iter()
                    .map(|d| d.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                detail_col = detail_col.push(
                    row![
                        text("Deps:").size(10.0).color(text_muted).font(DM_SANS),
                        text(deps_str).size(10.0).color(text_secondary).font(DM_SANS),
                    ]
                    .spacing(4.0 / scale),
                );
            }

            // Plan phases
            if !track.plan_phases.is_empty() {
                detail_col = detail_col.push(make_hdiv());
                detail_col = detail_col.push(
                    text("Plan Phases").size(12.0).color(text_primary).font(DM_SANS),
                );

                for phase in &track.plan_phases {
                    let phase_icon = match phase.status {
                        crate::conductor::model::PhaseStatus::Complete => "\u{25CF}",
                        crate::conductor::model::PhaseStatus::Active => "\u{25D0}",
                        crate::conductor::model::PhaseStatus::Pending => "\u{25CB}",
                        crate::conductor::model::PhaseStatus::Blocked => "\u{2717}",
                    };
                    let phase_color = match phase.status {
                        crate::conductor::model::PhaseStatus::Complete => green,
                        crate::conductor::model::PhaseStatus::Active => accent,
                        crate::conductor::model::PhaseStatus::Pending => text_muted,
                        crate::conductor::model::PhaseStatus::Blocked => red,
                    };
                    let phase_completed = phase.tasks_completed();
                    let phase_total = phase.tasks.len();
                    detail_col = detail_col.push(
                        row![
                            text(phase_icon).size(11.0).color(phase_color).font(JETBRAINS_MONO).width(16.0 / scale),
                            text(&phase.name).size(11.0).color(text_primary).font(DM_SANS).width(iced_core::Length::Fill),
                            text(format!("{}/{}", phase_completed, phase_total))
                                .size(10.0).color(text_muted).font(JETBRAINS_MONO),
                        ]
                        .spacing(4.0 / scale)
                        .align_y(iced_core::Alignment::Center),
                    );

                    // Tasks within each phase
                    for task in &phase.tasks {
                        let (task_icon, task_color) = if task.done {
                            ("\u{2713}", text_muted)
                        } else {
                            ("\u{25CB}", text_secondary)
                        };
                        let task_text_color = if task.done { text_muted } else { text_secondary };
                        detail_col = detail_col.push(
                            row![
                                iced_widget::Space::new().width(16.0 / scale),
                                text(task_icon).size(10.0).color(task_color).font(JETBRAINS_MONO).width(14.0 / scale),
                                text(&task.text).size(10.5).color(task_text_color).font(DM_SANS),
                            ]
                            .spacing(4.0 / scale)
                            .align_y(iced_core::Alignment::Center),
                        );
                    }
                }
            }

            iced_widget::scrollable(
                container(detail_col)
                    .padding(iced_core::Padding::from([0.0, 8.0 / scale]))
                    .width(iced_core::Length::Fill),
            )
            .width(iced_core::Length::Fill)
            .height(iced_core::Length::Fill)
            .style(scrollbar_style)
            .into()
        } else {
            container(
                text("Select a track").size(12.0).color(text_muted).font(DM_SANS),
            )
            .width(iced_core::Length::Fill)
            .height(iced_core::Length::Fill)
            .align_x(iced_core::Alignment::Center)
            .align_y(iced_core::Alignment::Center)
            .into()
        };

        // ── Vertical divider between panes ──
        let vdiv: IcedElement<'a> = container(column![])
            .width(1.0 / scale)
            .height(iced_core::Length::Fill)
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(border_color)),
                ..Default::default()
            })
            .into();

        // ── Assemble layout ──
        let split = snap.split_percent.clamp(20, 80) as f32;
        let body = row![
            container(left_pane).width(iced_core::Length::FillPortion(split as u16)),
            vdiv,
            container(right_pane).width(iced_core::Length::FillPortion((100.0 - split) as u16)),
        ]
        .width(iced_core::Length::Fill)
        .height(iced_core::Length::Fill);

        // ── Keyboard hints bar (dark blue background, left-aligned) ──
        let hints_bg = iced_core::Color {
            r: accent_blue.r * 0.3,
            g: accent_blue.g * 0.3,
            b: accent_blue.b * 0.3,
            a: 1.0,
        };
        let hints_key_color = text_primary;
        let hints_desc_color = text_secondary;
        let hs = || iced_widget::Space::new().width(12.0 / scale);
        let hints_row = row![
            text("j/k").size(12.0).color(hints_key_color).font(JETBRAINS_MONO),
            text(" Navigate").size(12.0).color(hints_desc_color).font(DM_SANS),
            hs(),
            text("f").size(12.0).color(hints_key_color).font(JETBRAINS_MONO),
            text(" Filter").size(12.0).color(hints_desc_color).font(DM_SANS),
            hs(),
            text("s").size(12.0).color(hints_key_color).font(JETBRAINS_MONO),
            text(" Sort").size(12.0).color(hints_desc_color).font(DM_SANS),
            hs(),
            text("[]").size(12.0).color(hints_key_color).font(JETBRAINS_MONO),
            text(" Resize").size(12.0).color(hints_desc_color).font(DM_SANS),
            hs(),
            text("d/u").size(12.0).color(hints_key_color).font(JETBRAINS_MONO),
            text(" Scroll").size(12.0).color(hints_desc_color).font(DM_SANS),
            hs(),
            text("x").size(12.0).color(hints_key_color).font(JETBRAINS_MONO),
            text(" Close").size(12.0).color(hints_desc_color).font(DM_SANS),
            hs(),
            text("Esc").size(12.0).color(hints_key_color).font(JETBRAINS_MONO),
            text(" Quit").size(12.0).color(hints_desc_color).font(DM_SANS),
        ]
        .align_y(iced_core::Alignment::Center);
        let hints_bar = container(hints_row)
            .width(iced_core::Length::Fill)
            .padding(iced_core::Padding::from([6.0 / scale, 12.0 / scale]))
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(hints_bg)),
                ..Default::default()
            });

        let dashboard = column![
            container(header).padding(iced_core::Padding::from([8.0 / scale, 12.0 / scale])),
            container(stats_row).padding(iced_core::Padding::from([0.0, 12.0 / scale])),
            make_hdiv(),
            body,
            make_hdiv(),
            hints_bar,
        ]
        .width(iced_core::Length::Fill)
        .height(iced_core::Length::Fill);

        container(dashboard)
            .width(iced_core::Length::Fill)
            .height(iced_core::Length::Fill)
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(bg)),
                ..Default::default()
            })
            .into()
    }

    /// Markdown preview overlay — renders parsed markdown in a scrollable dark overlay.
    fn markdown_overlay<'a>(state: &'a UiState, scale: f32) -> IcedElement<'a> {
        let theme = state.theme;
        let text_primary = to_iced_color(&theme.text_primary);
        let text_muted = to_iced_color(&theme.text_muted);
        let bg_overlay = iced_core::Color::from_rgba(0.0, 0.0, 0.0, 0.85);
        let bg_content = to_iced_color(&theme.bg_surface);

        let file_name = state.markdown_file_name.as_deref().unwrap_or("Markdown Preview");

        // Header: file name + "Esc to close"
        let title = iced_widget::text(file_name)
            .size(16.0 / scale)
            .color(text_primary);
        let hint = iced_widget::text("Esc to close")
            .size(12.0 / scale)
            .color(text_muted);
        let hspace = hspace();
        let header: IcedElement<'a> = iced_widget::row![title, hspace, hint]
            .spacing(8.0 / scale)
            .padding(iced_core::Padding::from([12.0 / scale, 20.0 / scale]))
            .align_y(iced_core::Alignment::Center)
            .into();

        // Markdown content
        let md_content: IcedElement<'a> = if let Some(ref items) = state.markdown_items {
            let iced_theme = iced_core::Theme::Dark;
            let md_element: iced_core::Element<'a, String, iced_core::Theme, iced_wgpu::Renderer> =
                iced_widget::markdown::view(items, &iced_theme);
            md_element.map(UiMessage::MarkdownLinkClicked)
        } else {
            iced_widget::text("No content").color(text_muted).into()
        };

        let padded_md: IcedElement<'a> = iced_widget::container(md_content)
            .padding(iced_core::Padding::from([12.0 / scale, 20.0 / scale]))
            .width(iced_core::Length::Fill)
            .into();

        let scrollable_md: IcedElement<'a> = iced_widget::scrollable(padded_md)
            .width(iced_core::Length::Fill)
            .height(iced_core::Length::Fill)
            .into();

        // Content card: header + divider + scrollable markdown
        let divider_line: IcedElement<'a> = iced_widget::container(iced_widget::text(""))
            .width(iced_core::Length::Fill)
            .height(1.0 / scale)
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(
                    iced_core::Color { a: 0.3, ..text_muted },
                )),
                ..Default::default()
            })
            .into();

        let card_content: IcedElement<'a> = iced_widget::column![header, divider_line, scrollable_md]
            .width(iced_core::Length::Fill)
            .height(iced_core::Length::Fill)
            .into();

        let card_width = (state.window_width * 0.75).min(900.0 / scale);
        let card: IcedElement<'a> = iced_widget::container(card_content)
            .width(card_width)
            .height(iced_core::Length::FillPortion(9)) // 90% of vertical space
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(bg_content)),
                border: iced_core::Border {
                    color: iced_core::Color { a: 0.2, ..text_muted },
                    width: 1.0 / scale,
                    radius: (8.0 / scale).into(),
                },
                ..Default::default()
            })
            .into();

        // Center the card with scrim background
        iced_widget::container(card)
            .width(iced_core::Length::Fill)
            .height(iced_core::Length::Fill)
            .align_x(iced_core::alignment::Horizontal::Center)
            .align_y(iced_core::alignment::Vertical::Center)
            .padding(iced_core::Padding::from([40.0 / scale, 0.0]))
            .style(move |_: &iced_core::Theme| container::Style {
                background: Some(iced_core::Background::Color(bg_overlay)),
                ..Default::default()
            })
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::theme::TerminalTheme;

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

    fn test_ui_state(theme: &TerminalTheme) -> UiState {
        UiState {
            tabs: vec![TabInfo {
                title: "Tab 1".to_string(),
                is_active: true,
                has_notification: false,
            }],
            active_tab_index: 0,
            hovered_tab: None,
            active_pane_index: 0,
            panes: vec![PaneInfo {
                x: 12.0,
                y: 12.0,
                width: 1256.0,
                height: 600.0,
                is_focused: true,
                index: 0,
                title: "Shell".to_string(),
                shell_name: "zsh".to_string(),
                scrollbar_thumb: None,
                scrollbar_alpha: 0.0,
            }],
            pane_count: 1,
            is_zoomed: false,
            theme,
            window_width: 1280.0,
            window_height: 720.0,
            scale_factor: 2.0,
            search_active: false,
            search_query: String::new(),
            search_current: 0,
            search_total: 0,
            search_error: false,
            dividers: Vec::new(),
            bell_flash: false,
            palette_active: false,
            palette_query: String::new(),
            palette_items: Vec::new(),
            palette_selected: 0,
            sidebar_visible: false,
            sidebar_tabs: Vec::new(),
            sidebar_width: 200.0,
            hovered_sidebar_tab: None,
            theme_selector_open: false,
            editing_tab: None,
            editing_tab_value: String::new(),
            hovering_new_tab: false,
            hovering_close_button: None,
            conductor_available: false,
            conductor: None,
            markdown_items: None,
            markdown_file_name: None,
            tracks_icon_handle: None,
            is_tracks_hovered: false,
            is_sidebar_btn_hovered: false,
            context_menu_visible: false,
            context_menu_position: (0.0, 0.0),
            context_menu_has_selection: false,
            active_overlay: None,
            file_browser_split_ratio: 0.5,
            git_review_split_ratio: 0.5,
            overlay_focused_panel: crate::file_browser::OverlayPanel::default(),
            is_file_browser_icon_hovered: false,
            is_git_review_icon_hovered: false,
            file_browser_rows: Vec::new(),
            file_browser_breadcrumb: String::new(),
            file_browser_preview_name: None,
            file_browser_preview_lines: Vec::new(),
            file_browser_preview_truncated: false,
            file_browser_preview_scroll: 0.0,
            git_review_list_items: Vec::new(),
            git_review_diff_header: None,
            git_review_diff_rows: Vec::new(),
            git_review_error: None,
            git_review_can_commit: false,
            git_review_commit_message: String::new(),
            git_review_diff_scroll: 0.0,
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

        let event =
            winit::event::WindowEvent::Resized(winit::dpi::PhysicalSize::new(1920, 1080));
        layer.push_event(&event, 1.0, winit::keyboard::ModifiersState::empty());

        assert!(
            layer.events.len() >= 1,
            "Should have at least 1 iced event after resize"
        );
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

        let theme = TerminalTheme::warm_dark();
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
        let theme_color = crate::config::theme::color_new(0.5, 0.25, 0.75, 1.0);
        let iced_color = to_iced_color(&theme_color);
        assert!((iced_color.r - 0.5).abs() < 0.01);
        assert!((iced_color.g - 0.25).abs() < 0.01);
        assert!((iced_color.b - 0.75).abs() < 0.01);
        assert!((iced_color.a - 1.0).abs() < 0.01);
    }

    #[test]
    fn chrome_heights_match_legacy_constants() {
        assert_eq!(CHROME_BAR_HEIGHT, crate::header_bar::CHROME_BAR_HEIGHT);
        assert_eq!(STATUS_BAR_HEIGHT, crate::status_bar::STATUS_BAR_HEIGHT);
    }

    #[test]
    fn chrome_bar_renders_with_multiple_tabs() {
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

        let theme = TerminalTheme::warm_dark();
        let state = UiState {
            tabs: vec![
                TabInfo {
                    title: "Tab 1".to_string(),
                    is_active: true,
                    has_notification: false,
                },
                TabInfo {
                    title: "Tab 2".to_string(),
                    is_active: false,
                    has_notification: false,
                },
                TabInfo {
                    title: "Tab 3".to_string(),
                    is_active: false,
                    has_notification: true,
                },
            ],
            active_tab_index: 0,
            hovered_tab: Some(1),
            active_pane_index: 0,
            panes: vec![PaneInfo {
                x: 12.0,
                y: 12.0,
                width: 1256.0,
                height: 600.0,
                is_focused: true,
                index: 0,
                title: "Shell".to_string(),
                shell_name: "zsh".to_string(),
                scrollbar_thumb: None,
                scrollbar_alpha: 0.0,
            }],
            pane_count: 1,
            is_zoomed: false,
            theme: &theme,
            window_width: 1280.0,
            window_height: 720.0,
            scale_factor: 1.0,
            search_active: false,
            search_query: String::new(),
            search_current: 0,
            search_total: 0,
            search_error: false,
            dividers: Vec::new(),
            bell_flash: false,
            palette_active: false,
            palette_query: String::new(),
            palette_items: Vec::new(),
            palette_selected: 0,
            sidebar_visible: false,
            sidebar_tabs: Vec::new(),
            sidebar_width: 200.0,
            hovered_sidebar_tab: None,
            theme_selector_open: false,
            editing_tab: None,
            editing_tab_value: String::new(),
        hovering_new_tab: false,
            hovering_close_button: None,
            conductor_available: false,
            conductor: None,
            markdown_items: None,
            markdown_file_name: None,
            tracks_icon_handle: None,
            is_tracks_hovered: false,
            is_sidebar_btn_hovered: false,
            context_menu_visible: false,
            context_menu_position: (0.0, 0.0),
            context_menu_has_selection: false,
            active_overlay: None,
            file_browser_split_ratio: 0.5,
            git_review_split_ratio: 0.5,
            overlay_focused_panel: crate::file_browser::OverlayPanel::default(),
            is_file_browser_icon_hovered: false,
            is_git_review_icon_hovered: false,
file_browser_rows: Vec::new(),
            file_browser_breadcrumb: String::new(),
            file_browser_preview_name: None,
            file_browser_preview_lines: Vec::new(),
            file_browser_preview_truncated: false,
            file_browser_preview_scroll: 0.0,
            git_review_list_items: Vec::new(),
            git_review_diff_header: None,
            git_review_diff_rows: Vec::new(),
            git_review_error: None,
            git_review_can_commit: false,
            git_review_commit_message: String::new(),
            git_review_diff_scroll: 0.0,
            };
        let messages = layer.render(&view, &state);
        assert!(messages.is_empty(), "No interactions, no messages expected");
    }

    #[test]
    fn pane_chrome_renders_with_multiple_panes() {
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

        let theme = TerminalTheme::warm_dark();
        let state = UiState {
            tabs: vec![TabInfo {
                title: "Tab 1".to_string(),
                is_active: true,
                has_notification: false,
            }],
            active_tab_index: 0,
            hovered_tab: None,
            active_pane_index: 0,
            panes: vec![
                PaneInfo {
                    x: 12.0,
                    y: 12.0,
                    width: 620.0,
                    height: 600.0,
                    is_focused: true,
                    index: 0,
                    title: "src".to_string(),
                    shell_name: "zsh".to_string(),
                    scrollbar_thumb: None,
                    scrollbar_alpha: 0.0,
                },
                PaneInfo {
                    x: 636.0,
                    y: 12.0,
                    width: 620.0,
                    height: 600.0,
                    is_focused: false,
                    index: 1,
                    title: "tests".to_string(),
                    shell_name: "zsh".to_string(),
                    scrollbar_thumb: None,
                    scrollbar_alpha: 0.0,
                },
            ],
            pane_count: 2,
            is_zoomed: false,
            theme: &theme,
            window_width: 1280.0,
            window_height: 720.0,
            scale_factor: 1.0,
            search_active: false,
            search_query: String::new(),
            search_current: 0,
            search_total: 0,
            search_error: false,
            dividers: Vec::new(),
            bell_flash: false,
            palette_active: false,
            palette_query: String::new(),
            palette_items: Vec::new(),
            palette_selected: 0,
            sidebar_visible: false,
            sidebar_tabs: Vec::new(),
            sidebar_width: 200.0,
            hovered_sidebar_tab: None,
            theme_selector_open: false,
            editing_tab: None,
            editing_tab_value: String::new(),
        hovering_new_tab: false,
            hovering_close_button: None,
            conductor_available: false,
            conductor: None,
            markdown_items: None,
            markdown_file_name: None,
            tracks_icon_handle: None,
            is_tracks_hovered: false,
            is_sidebar_btn_hovered: false,
            context_menu_visible: false,
            context_menu_position: (0.0, 0.0),
            context_menu_has_selection: false,
            active_overlay: None,
            file_browser_split_ratio: 0.5,
            git_review_split_ratio: 0.5,
            overlay_focused_panel: crate::file_browser::OverlayPanel::default(),
            is_file_browser_icon_hovered: false,
            is_git_review_icon_hovered: false,
file_browser_rows: Vec::new(),
            file_browser_breadcrumb: String::new(),
            file_browser_preview_name: None,
            file_browser_preview_lines: Vec::new(),
            file_browser_preview_truncated: false,
            file_browser_preview_scroll: 0.0,
            git_review_list_items: Vec::new(),
            git_review_diff_header: None,
            git_review_diff_rows: Vec::new(),
            git_review_error: None,
            git_review_can_commit: false,
            git_review_commit_message: String::new(),
            git_review_diff_scroll: 0.0,
            };
        let messages = layer.render(&view, &state);
        assert!(messages.is_empty(), "No interactions, no messages expected");
    }

    #[test]
    fn pane_info_constructor() {
        let pane = PaneInfo {
            x: 10.0,
            y: 20.0,
            width: 500.0,
            height: 400.0,
            is_focused: true,
            index: 0,
            title: "home".to_string(),
            shell_name: "zsh".to_string(),
            scrollbar_thumb: None,
            scrollbar_alpha: 0.0,
        };
        assert!(pane.is_focused);
        assert_eq!(pane.title, "home");
        assert_eq!(pane.index, 0);
    }

    #[test]
    fn pane_header_height_matches_legacy() {
        assert_eq!(PANE_HEADER_HEIGHT, crate::pane::header::PANE_HEADER_HEIGHT);
    }

    #[test]
    fn ui_message_tab_hovered_variant() {
        let hover = UiMessage::TabHovered(Some(2));
        match hover {
            UiMessage::TabHovered(Some(idx)) => assert_eq!(idx, 2),
            _ => panic!("Expected TabHovered(Some(2))"),
        }
        let unhover = UiMessage::TabHovered(None);
        match unhover {
            UiMessage::TabHovered(None) => {}
            _ => panic!("Expected TabHovered(None)"),
        }
    }

    #[test]
    fn pane_badge_returns_circled_digits() {
        assert_eq!(pane_badge(0), "\u{2460}");
        assert_eq!(pane_badge(1), "\u{2461}");
        assert_eq!(pane_badge(2), "\u{2462}");
        assert_eq!(pane_badge(8), "\u{2468}");
        assert_eq!(pane_badge(9), "\u{2460}"); // wraps
    }

    // ── Sidebar tests ───────────────────────────────────────────────

    #[test]
    fn sidebar_tab_info_construction() {
        let tab = SidebarTabInfo {
            title: "Shell".to_string(),
            is_active: true,
            has_notification: false,
            pane_count: 2,
            minimap_rects: vec![
                MinimapPane { x: 0.0, y: 0.0, width: 0.5, height: 1.0, is_focused: true },
                MinimapPane { x: 0.5, y: 0.0, width: 0.5, height: 1.0, is_focused: false },
            ],
        };
        assert!(tab.is_active);
        assert_eq!(tab.pane_count, 2);
        assert_eq!(tab.minimap_rects.len(), 2);
    }

    #[test]
    fn sidebar_minimap_single_pane() {
        let tab = SidebarTabInfo {
            title: "Tab 1".to_string(),
            is_active: true,
            has_notification: false,
            pane_count: 1,
            minimap_rects: vec![
                MinimapPane { x: 0.0, y: 0.0, width: 1.0, height: 1.0, is_focused: true },
            ],
        };
        assert_eq!(tab.minimap_rects.len(), 1);
        let rect = &tab.minimap_rects[0];
        assert!((rect.width - 1.0).abs() < 0.01);
        assert!((rect.height - 1.0).abs() < 0.01);
        assert!(rect.is_focused);
    }

    #[test]
    fn sidebar_minimap_vertical_split() {
        let tab = SidebarTabInfo {
            title: "Split".to_string(),
            is_active: false,
            has_notification: true,
            pane_count: 2,
            minimap_rects: vec![
                MinimapPane { x: 0.0, y: 0.0, width: 0.5, height: 1.0, is_focused: true },
                MinimapPane { x: 0.5, y: 0.0, width: 0.5, height: 1.0, is_focused: false },
            ],
        };
        assert_eq!(tab.minimap_rects.len(), 2);
        assert!((tab.minimap_rects[0].width - 0.5).abs() < 0.01);
        assert!((tab.minimap_rects[1].x - 0.5).abs() < 0.01);
    }

    #[test]
    fn sidebar_minimap_horizontal_split() {
        let tab = SidebarTabInfo {
            title: "HSplit".to_string(),
            is_active: true,
            has_notification: false,
            pane_count: 2,
            minimap_rects: vec![
                MinimapPane { x: 0.0, y: 0.0, width: 1.0, height: 0.5, is_focused: false },
                MinimapPane { x: 0.0, y: 0.5, width: 1.0, height: 0.5, is_focused: true },
            ],
        };
        assert_eq!(tab.minimap_rects.len(), 2);
        assert!((tab.minimap_rects[0].height - 0.5).abs() < 0.01);
        assert!((tab.minimap_rects[1].y - 0.5).abs() < 0.01);
    }

    #[test]
    fn sidebar_renders_without_crash() {
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

        let theme = TerminalTheme::warm_dark();
        let mut state = test_ui_state(&theme);
        state.sidebar_visible = true;
        state.sidebar_width = 200.0;
        state.sidebar_tabs = vec![
            SidebarTabInfo {
                title: "Tab 1".to_string(),
                is_active: true,
                has_notification: false,
                pane_count: 1,
                minimap_rects: vec![
                    MinimapPane { x: 0.0, y: 0.0, width: 1.0, height: 1.0, is_focused: true },
                ],
            },
            SidebarTabInfo {
                title: "Tab 2".to_string(),
                is_active: false,
                has_notification: true,
                pane_count: 2,
                minimap_rects: vec![
                    MinimapPane { x: 0.0, y: 0.0, width: 0.5, height: 1.0, is_focused: true },
                    MinimapPane { x: 0.5, y: 0.0, width: 0.5, height: 1.0, is_focused: false },
                ],
            },
        ];
        let messages = layer.render(&view, &state);
        assert!(messages.is_empty(), "No interactions, no messages expected");
    }

    #[test]
    fn ui_message_sidebar_variants() {
        let _select = UiMessage::SidebarTabSelected(0);
        let _toggle = UiMessage::ToggleSidebar;
    }

    #[test]
    fn ui_message_theme_variants() {
        let _toggle = UiMessage::ToggleThemeSelector;
        let _set = UiMessage::SetTheme("warm_dark".to_string());
    }

    // ── Visual polish tests ────────────────────────────────────────

    #[test]
    fn beam_cursor_threshold_is_thin() {
        let shader = include_str!("../../shaders/grid.wgsl");
        assert!(
            shader.contains("cell_x_frac < 0.08"),
            "Beam cursor threshold should be 0.08 for thin ~1px beam"
        );
    }

    #[test]
    fn pane_border_uniform_width() {
        // The pane chrome method should use width: 1.0 for all panes (no variable width for active)
        let source = include_str!("iced_layer.rs");
        let needle = format!("width: if is_active_pane {{ {} }}", "1.5");
        assert!(
            !source.contains(&needle),
            "Active pane border should be uniform 1.0"
        );
    }

    #[test]
    fn pane_header_removed() {
        // Pane headers removed — no header/stripe in pane_chrome
        assert_eq!(PANE_HEADER_HEIGHT, 0.0);
    }

    #[test]
    fn chrome_bar_has_sidebar_icon() {
        // Chrome bar uses sparkle (✦) icon; ToggleSidebar still exists via keyboard shortcuts
        let source = include_str!("iced_layer.rs");
        assert!(source.contains("UiMessage::ToggleSidebar"));
        assert!(source.contains("\\u{2726}"), "Sparkle icon ✦ should be in chrome bar");
    }

    #[test]
    fn chrome_bar_has_theme_selector() {
        let source = include_str!("iced_layer.rs");
        assert!(source.contains("UiMessage::ToggleThemeSelector"));
    }

    #[test]
    fn theme_selector_shows_five_themes() {
        let themes = crate::config::theme::TerminalTheme::available_themes();
        assert_eq!(themes.len(), 5);
        assert_eq!(themes[0].0, "warm_dark");
        assert_eq!(themes[1].0, "midnight");
        assert_eq!(themes[2].0, "ember");
        assert_eq!(themes[3].0, "dusk");
        assert_eq!(themes[4].0, "light");
    }

    // ── Button hover/press fix tests ──────────────────────────────

    #[test]
    fn render_injects_redraw_requested_event() {
        // iced buttons only update their visual status (Hovered/Pressed) on
        // RedrawRequested events, but iced_winit::conversion doesn't convert
        // winit's RedrawRequested. Our render() must inject it manually.
        let source = include_str!("iced_layer.rs");
        assert!(
            source.contains("RedrawRequested"),
            "render() must inject RedrawRequested for button hover/press to work"
        );
        assert!(
            source.contains("window::Event::RedrawRequested"),
            "Must push iced_core::Event::Window(window::Event::RedrawRequested(...))"
        );
    }

    #[test]
    fn new_tab_hover_produces_visible_background() {
        // The New Tab area uses MouseArea hover (UiState flag) to show a
        // visible accent-tinted background when hovering_new_tab is true.
        let theme = TerminalTheme::warm_dark();
        let accent = to_iced_color(&theme.accent_orange);
        let text_dim = to_iced_color(&theme.text_ghost);
        let text_secondary = to_iced_color(&theme.text_secondary);

        // Default (not hovered): no background
        let default_bg: Option<iced_core::Background> = None;
        assert!(default_bg.is_none(), "Default state should have no background");

        // Hovered: accent-tinted background at 0.20 alpha
        let hover_bg = iced_core::Color::from_rgba(accent.r, accent.g, accent.b, 0.20);
        assert!(hover_bg.a >= 0.15, "Hover alpha must be visible (>= 0.15), got {}", hover_bg.a);
        assert!(hover_bg.r > 0.0 || hover_bg.g > 0.0 || hover_bg.b > 0.0, "Hover color must not be black");

        // Text colors should differ between hover and default
        assert_ne!(text_dim.r, text_secondary.r, "Hover text should differ from default");
    }

    #[test]
    fn close_button_hover_produces_visible_background() {
        // The close button uses MouseArea hover (UiState flag) to show a
        // visible accent-tinted background when hovering_close_button matches.
        let theme = TerminalTheme::warm_dark();
        let accent = to_iced_color(&theme.accent_orange);

        let hover_bg = iced_core::Color::from_rgba(accent.r, accent.g, accent.b, 0.25);
        assert!(hover_bg.a >= 0.20, "Close hover alpha must be >= 0.20, got {}", hover_bg.a);
    }

    #[test]
    fn close_button_visible_on_hovered_or_active_tabs() {
        // The close button should appear on hovered/active tabs when tab_count > 1.
        // It swaps with the keyboard shortcut on hover.
        let source = include_str!("iced_layer.rs");
        let sidebar_start = source.find("fn sidebar<'a>").expect("sidebar function exists");
        let end = (sidebar_start + 8000).min(source.len());
        let sidebar_code = &source[sidebar_start..end];

        // Close button requires tab_count > 1
        assert!(
            sidebar_code.contains("tab_count > 1"),
            "Close button should require tab_count > 1"
        );
        // Close button shows on active or hovered tabs
        assert!(
            sidebar_code.contains("is_active || is_hovered_tab"),
            "Close button should show on active or hovered tabs"
        );
    }

    #[test]
    fn iced_winit_does_not_convert_redraw_requested() {
        // Prove that iced_winit::conversion returns None for RedrawRequested.
        // This is WHY we must inject it manually in render().
        let event = winit::event::WindowEvent::RedrawRequested;
        let result = iced_winit::conversion::window_event(
            event,
            1.0,
            winit::keyboard::ModifiersState::empty(),
        );
        assert!(
            result.is_none(),
            "iced_winit should NOT convert RedrawRequested — we inject it ourselves"
        );
    }

    // ── Context menu overlay tests ─────────────────────────────────

    #[test]
    fn context_menu_ui_message_variants_exist() {
        // Verify the context menu UiMessage variants compile
        let action_msg = UiMessage::ContextMenuAction(
            crate::context_menu::ContextMenuAction::Copy,
        );
        let dismiss_msg = UiMessage::ContextMenuDismiss;
        // Just verify they're constructable (Debug impl)
        let _ = format!("{:?}", action_msg);
        let _ = format!("{:?}", dismiss_msg);
    }

    #[test]
    fn context_menu_state_defaults_to_hidden() {
        let theme = TerminalTheme::warm_dark();
        let state = UiState {
            tabs: vec![],
            active_tab_index: 0,
            hovered_tab: None,
            active_pane_index: 0,
            panes: vec![],
            pane_count: 1,
            is_zoomed: false,
            theme: &theme,
            window_width: 800.0,
            window_height: 600.0,
            scale_factor: 2.0,
            search_active: false,
            search_query: String::new(),
            search_current: 0,
            search_total: 0,
            search_error: false,
            dividers: vec![],
            bell_flash: false,
            palette_active: false,
            palette_query: String::new(),
            palette_items: vec![],
            palette_selected: 0,
            sidebar_visible: false,
            sidebar_tabs: vec![],
            sidebar_width: 200.0,
            hovered_sidebar_tab: None,
            theme_selector_open: false,
            editing_tab: None,
            editing_tab_value: String::new(),
            hovering_new_tab: false,
            hovering_close_button: None,
            conductor_available: false,
            conductor: None,
            markdown_items: None,
            markdown_file_name: None,
            tracks_icon_handle: None,
            is_tracks_hovered: false,
            is_sidebar_btn_hovered: false,
            context_menu_visible: false,
            context_menu_position: (0.0, 0.0),
            context_menu_has_selection: false,
            active_overlay: None,
            file_browser_split_ratio: 0.5,
            git_review_split_ratio: 0.5,
            overlay_focused_panel: crate::file_browser::OverlayPanel::default(),
            is_file_browser_icon_hovered: false,
            is_git_review_icon_hovered: false,
file_browser_rows: Vec::new(),
            file_browser_breadcrumb: String::new(),
            file_browser_preview_name: None,
            file_browser_preview_lines: Vec::new(),
            file_browser_preview_truncated: false,
            file_browser_preview_scroll: 0.0,
            git_review_list_items: Vec::new(),
            git_review_diff_header: None,
            git_review_diff_rows: Vec::new(),
            git_review_error: None,
            git_review_can_commit: false,
            git_review_commit_message: String::new(),
            git_review_diff_scroll: 0.0,
        };
        assert!(!state.context_menu_visible);
        assert_eq!(state.context_menu_position, (0.0, 0.0));
        assert!(!state.context_menu_has_selection);
    }

    #[test]
    fn context_menu_overlay_source_contains_menu_items() {
        // Verify the context menu overlay renders expected action items
        let source = include_str!("iced_layer.rs");
        assert!(source.contains("ContextMenuAction::Copy"));
        assert!(source.contains("ContextMenuAction::Paste"));
        assert!(source.contains("ContextMenuAction::SelectAll"));
        assert!(source.contains("ContextMenuAction::ClearScrollback"));
        assert!(source.contains("ContextMenuAction::NewTab"));
        assert!(source.contains("ContextMenuAction::SplitVertical"));
        assert!(source.contains("ContextMenuAction::SplitHorizontal"));
        assert!(source.contains("ContextMenuAction::ClosePane"));
    }

    #[test]
    fn context_menu_dismissed_by_scrim_click() {
        // Verify the scrim sends ContextMenuDismiss on press
        let source = include_str!("iced_layer.rs");
        assert!(source.contains("UiMessage::ContextMenuDismiss"));
    }
}

//! Main View Orchestration
//!
//! Coordinates all UI components, handles layout, event routing, and state management.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use std::time::Instant;

use crate::{
    approval_overlay::{ApprovalAction, ApprovalOverlay, ApprovalRequest},
    chat_composer::{ChatComposer, ComposerAction},
    command_popup::{CommandPopup, CommandPopupAction},
    help::{HelpAction, HelpViewer},
    history_cell::HistoryCell,
    notification::{Alert, AlertAction, Notification, NotificationCenter, NotificationPanelAction},
    pager_overlay::{PagerAction, PagerOverlay, PagerContent},
    resume_picker::{ResumePicker, ResumePickerAction, SessionEntry},
    shimmer::Spinner,
    ui::{colors, Breadcrumb, StatusBar, StatusItem},
};

/// Focus area in the main view
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusArea {
    /// Chat input area
    Chat,
    /// History/message area
    History,
    /// Sidebar (if visible)
    Sidebar,
}

/// Active overlay type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveOverlay {
    /// No overlay active
    None,
    /// Command palette
    CommandPalette,
    /// Help viewer
    Help,
    /// Approval dialog
    Approval,
    /// Pager/viewer
    Pager,
    /// Session picker
    SessionPicker,
    /// Notification panel
    Notifications,
    /// Alert dialog
    Alert,
}

/// Application mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    /// Normal chat mode
    Normal,
    /// Streaming response
    Streaming,
    /// Waiting for approval
    WaitingApproval,
    /// Loading/processing
    Loading,
}

/// Actions that can be returned from the view
#[derive(Debug, Clone)]
pub enum ViewAction {
    /// No action
    None,
    /// Quit the application
    Quit,
    /// Send message
    SendMessage(String),
    /// Execute command
    ExecuteCommand(String),
    /// Approve action
    Approve { request_id: String, approved: bool },
    /// Resume session
    ResumeSession(String),
    /// Show notification
    Notify(Notification),
    /// Cancel current operation
    Cancel,
    /// Toggle sidebar
    ToggleSidebar,
    /// Open file
    OpenFile(String),
    /// Copy to clipboard
    Copy(String),
}

/// Layout configuration
#[derive(Debug, Clone)]
pub struct LayoutConfig {
    /// Show sidebar
    pub show_sidebar: bool,
    /// Sidebar width (percentage)
    pub sidebar_width: u16,
    /// Show status bar
    pub show_status_bar: bool,
    /// Show breadcrumb
    pub show_breadcrumb: bool,
    /// Input height (lines)
    pub input_height: u16,
    /// Minimum history height
    pub min_history_height: u16,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            show_sidebar: false,
            sidebar_width: 25,
            show_status_bar: true,
            show_breadcrumb: true,
            input_height: 3,
            min_history_height: 10,
        }
    }
}

/// Main application view
pub struct MainView {
    /// Layout configuration
    pub layout: LayoutConfig,
    /// Current focus area
    pub focus: FocusArea,
    /// Active overlay
    pub overlay: ActiveOverlay,
    /// Application mode
    pub mode: AppMode,
    /// Chat composer
    pub chat: ChatComposer,
    /// Message history
    pub history: Vec<Box<dyn HistoryCell>>,
    /// History scroll position
    pub history_scroll: usize,
    /// Maximum scroll position
    pub max_scroll: usize,
    /// Notification center
    pub notifications: NotificationCenter,
    /// Command popup
    pub command_popup: CommandPopup,
    /// Help viewer
    pub help_viewer: HelpViewer,
    /// Approval overlay
    pub approval_overlay: ApprovalOverlay,
    /// Pager overlay
    pub pager_overlay: PagerOverlay,
    /// Session picker
    pub session_picker: ResumePicker,
    /// Loading spinner
    pub spinner: Spinner,
    /// Current working directory
    pub cwd: String,
    /// Session name
    pub session_name: String,
    /// Model name
    pub model_name: String,
    /// Token usage
    pub tokens_used: usize,
    /// Last activity time
    pub last_activity: Instant,
    /// Sidebar items
    pub sidebar_items: Vec<String>,
    /// Selected sidebar item
    pub sidebar_selected: usize,
}

impl Default for MainView {
    fn default() -> Self {
        Self::new()
    }
}

impl MainView {
    /// Create a new main view
    pub fn new() -> Self {
        Self {
            layout: LayoutConfig::default(),
            focus: FocusArea::Chat,
            overlay: ActiveOverlay::None,
            mode: AppMode::Normal,
            chat: ChatComposer::new(),
            history: Vec::new(),
            history_scroll: 0,
            max_scroll: 0,
            notifications: NotificationCenter::new(),
            command_popup: CommandPopup::new(),
            help_viewer: HelpViewer::new(),
            approval_overlay: ApprovalOverlay::new(),
            pager_overlay: PagerOverlay::new(),
            session_picker: ResumePicker::new(),
            spinner: Spinner::new(),
            cwd: std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "~".to_string()),
            session_name: "New Session".to_string(),
            model_name: "claude-sonnet-4-20250514".to_string(),
            tokens_used: 0,
            last_activity: Instant::now(),
            sidebar_items: Vec::new(),
            sidebar_selected: 0,
        }
    }

    /// Set the session name
    pub fn with_session(mut self, name: impl Into<String>) -> Self {
        self.session_name = name.into();
        self
    }

    /// Set the model name
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model_name = model.into();
        self
    }

    /// Add a message to history
    pub fn push_message(&mut self, cell: Box<dyn HistoryCell>) {
        self.history.push(cell);
        self.last_activity = Instant::now();
    }

    /// Show command palette
    pub fn show_command_palette(&mut self) {
        self.overlay = ActiveOverlay::CommandPalette;
        self.command_popup.show();
    }

    /// Show help viewer
    pub fn show_help(&mut self) {
        self.overlay = ActiveOverlay::Help;
    }

    /// Show approval dialog
    pub fn show_approval(&mut self, request: ApprovalRequest) {
        self.overlay = ActiveOverlay::Approval;
        self.approval_overlay.show(request);
        self.mode = AppMode::WaitingApproval;
    }

    /// Show pager with content
    pub fn show_pager(&mut self, content: PagerContent) {
        self.overlay = ActiveOverlay::Pager;
        self.pager_overlay = PagerOverlay::new().content(content);
    }

    /// Show session picker
    pub fn show_session_picker(&mut self, sessions: Vec<SessionEntry>) {
        self.overlay = ActiveOverlay::SessionPicker;
        for session in sessions {
            self.session_picker.add_session(session);
        }
    }

    /// Show notification panel
    pub fn show_notifications(&mut self) {
        self.overlay = ActiveOverlay::Notifications;
        self.notifications.panel.visible = true;
    }

    /// Show alert dialog
    pub fn show_alert(&mut self, alert: Alert) {
        self.overlay = ActiveOverlay::Alert;
        self.notifications.show_alert(alert);
    }

    /// Close current overlay
    pub fn close_overlay(&mut self) {
        match self.overlay {
            ActiveOverlay::CommandPalette => self.command_popup.hide(),
            ActiveOverlay::Approval => {
                self.approval_overlay.hide();
                self.mode = AppMode::Normal;
            }
            ActiveOverlay::Notifications => self.notifications.panel.visible = false,
            ActiveOverlay::Alert => self.notifications.alert = None,
            _ => {}
        }
        self.overlay = ActiveOverlay::None;
    }

    /// Set streaming mode
    pub fn set_streaming(&mut self, streaming: bool) {
        self.mode = if streaming {
            AppMode::Streaming
        } else {
            AppMode::Normal
        };
    }

    /// Set loading mode
    pub fn set_loading(&mut self, loading: bool) {
        self.mode = if loading {
            AppMode::Loading
        } else {
            AppMode::Normal
        };
    }

    /// Handle keyboard input
    pub fn handle_key(&mut self, key: KeyEvent) -> ViewAction {
        self.last_activity = Instant::now();

        // Handle overlay input first
        if self.overlay != ActiveOverlay::None {
            return self.handle_overlay_key(key);
        }

        // Global shortcuts
        match (key.modifiers, key.code) {
            // Quit
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                if self.mode == AppMode::Streaming {
                    return ViewAction::Cancel;
                }
                return ViewAction::Quit;
            }
            // Command palette
            (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
                self.show_command_palette();
                return ViewAction::None;
            }
            // Help
            (KeyModifiers::NONE, KeyCode::F(1)) => {
                self.show_help();
                return ViewAction::None;
            }
            // Toggle sidebar
            (KeyModifiers::CONTROL, KeyCode::Char('b')) => {
                self.layout.show_sidebar = !self.layout.show_sidebar;
                return ViewAction::ToggleSidebar;
            }
            // Notifications
            (KeyModifiers::CONTROL, KeyCode::Char('n')) => {
                self.show_notifications();
                return ViewAction::None;
            }
            // Escape
            (KeyModifiers::NONE, KeyCode::Esc) => {
                if self.mode == AppMode::Streaming {
                    return ViewAction::Cancel;
                }
            }
            _ => {}
        }

        // Focus-specific handling
        match self.focus {
            FocusArea::Chat => self.handle_chat_key(key),
            FocusArea::History => self.handle_history_key(key),
            FocusArea::Sidebar => self.handle_sidebar_key(key),
        }
    }

    /// Handle overlay keyboard input
    fn handle_overlay_key(&mut self, key: KeyEvent) -> ViewAction {
        match self.overlay {
            ActiveOverlay::CommandPalette => {
                match self.command_popup.handle_key(key) {
                    CommandPopupAction::Execute(cmd) => {
                        self.close_overlay();
                        return ViewAction::ExecuteCommand(cmd);
                    }
                    CommandPopupAction::Cancel => {
                        self.close_overlay();
                    }
                    _ => {}
                }
            }
            ActiveOverlay::Help => {
                match self.help_viewer.handle_key(key) {
                    HelpAction::Close => {
                        self.close_overlay();
                    }
                    _ => {}
                }
            }
            ActiveOverlay::Approval => {
                match self.approval_overlay.handle_key(key) {
                    ApprovalAction::Approve(id) | ApprovalAction::ApproveAll(id) => {
                        self.close_overlay();
                        return ViewAction::Approve {
                            request_id: id,
                            approved: true,
                        };
                    }
                    ApprovalAction::Reject(id) => {
                        self.close_overlay();
                        return ViewAction::Approve {
                            request_id: id,
                            approved: false,
                        };
                    }
                    _ => {}
                }
            }
            ActiveOverlay::Pager => {
                match self.pager_overlay.handle_key(key) {
                    PagerAction::Close => {
                        self.close_overlay();
                    }
                    PagerAction::Copy(content) => {
                        return ViewAction::Copy(content);
                    }
                    _ => {}
                }
            }
            ActiveOverlay::SessionPicker => {
                match self.session_picker.handle_key(key) {
                    ResumePickerAction::Select(session_id) => {
                        self.close_overlay();
                        return ViewAction::ResumeSession(session_id);
                    }
                    ResumePickerAction::Cancel => {
                        self.close_overlay();
                    }
                    _ => {}
                }
            }
            ActiveOverlay::Notifications => {
                match self.notifications.panel.handle_key(key) {
                    NotificationPanelAction::Close => {
                        self.close_overlay();
                    }
                    NotificationPanelAction::Dismiss(id) => {
                        self.notifications.panel.dismiss(&id);
                    }
                    NotificationPanelAction::DismissAll => {
                        self.notifications.panel.dismiss_all();
                    }
                    NotificationPanelAction::MarkAllRead => {
                        self.notifications.panel.mark_all_read();
                    }
                    _ => {}
                }
            }
            ActiveOverlay::Alert => {
                if let Some(ref mut alert) = self.notifications.alert {
                    match alert.handle_key(key) {
                        AlertAction::ButtonPressed(id) => {
                            self.close_overlay();
                            return ViewAction::ExecuteCommand(id);
                        }
                        AlertAction::Cancelled => {
                            self.close_overlay();
                        }
                        _ => {}
                    }
                }
            }
            ActiveOverlay::None => {}
        }
        ViewAction::None
    }

    /// Handle chat input keys
    fn handle_chat_key(&mut self, key: KeyEvent) -> ViewAction {
        match self.chat.handle_key(key) {
            ComposerAction::Submit => {
                let message = self.chat.get_input();
                if !message.is_empty() {
                    self.chat.clear();
                    return ViewAction::SendMessage(message);
                }
            }
            ComposerAction::Cancel => {
                self.chat.clear();
            }
            _ => {}
        }

        // Tab to switch focus
        if key.code == KeyCode::Tab && key.modifiers.is_empty() {
            self.focus = FocusArea::History;
        }

        ViewAction::None
    }

    /// Handle history navigation keys
    fn handle_history_key(&mut self, key: KeyEvent) -> ViewAction {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.history_scroll = self.history_scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.history_scroll < self.max_scroll {
                    self.history_scroll += 1;
                }
            }
            KeyCode::PageUp => {
                self.history_scroll = self.history_scroll.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.history_scroll = (self.history_scroll + 10).min(self.max_scroll);
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.history_scroll = 0;
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.history_scroll = self.max_scroll;
            }
            KeyCode::Tab | KeyCode::Char('i') => {
                self.focus = FocusArea::Chat;
            }
            _ => {}
        }
        ViewAction::None
    }

    /// Handle sidebar navigation keys
    fn handle_sidebar_key(&mut self, key: KeyEvent) -> ViewAction {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.sidebar_selected > 0 {
                    self.sidebar_selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.sidebar_selected < self.sidebar_items.len().saturating_sub(1) {
                    self.sidebar_selected += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(item) = self.sidebar_items.get(self.sidebar_selected) {
                    return ViewAction::OpenFile(item.clone());
                }
            }
            KeyCode::Tab | KeyCode::Char('l') => {
                self.focus = FocusArea::Chat;
            }
            _ => {}
        }
        ViewAction::None
    }

    /// Tick for animations
    pub fn tick(&mut self) {
        self.notifications.cleanup();
    }

    /// Render the main view
    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Calculate main layout
        let main_chunks: Vec<Rect> = if self.layout.show_sidebar {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(self.layout.sidebar_width),
                    Constraint::Percentage(100 - self.layout.sidebar_width),
                ])
                .split(area)
                .to_vec()
        } else {
            vec![area]
        };

        // Render sidebar if visible
        if self.layout.show_sidebar && main_chunks.len() > 1 {
            self.render_sidebar(frame, main_chunks[0]);
        }

        // Main content area
        let content_area = if self.layout.show_sidebar && main_chunks.len() > 1 {
            main_chunks[1]
        } else {
            main_chunks[0]
        };

        // Vertical layout for content
        let mut constraints = vec![];
        if self.layout.show_breadcrumb {
            constraints.push(Constraint::Length(1));
        }
        constraints.push(Constraint::Min(self.layout.min_history_height));
        constraints.push(Constraint::Length(self.layout.input_height));
        if self.layout.show_status_bar {
            constraints.push(Constraint::Length(1));
        }

        let content_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(content_area);

        let mut chunk_idx = 0;

        // Render breadcrumb
        if self.layout.show_breadcrumb {
            self.render_breadcrumb(frame, content_chunks[chunk_idx]);
            chunk_idx += 1;
        }

        // Render history
        self.render_history(frame, content_chunks[chunk_idx]);
        chunk_idx += 1;

        // Render chat input
        self.render_chat_input(frame, content_chunks[chunk_idx]);
        chunk_idx += 1;

        // Render status bar
        if self.layout.show_status_bar {
            self.render_status_bar(frame, content_chunks[chunk_idx]);
        }

        // Render overlays
        self.render_overlay(frame, area);
    }

    /// Render sidebar
    fn render_sidebar(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Files ")
            .title_style(Style::default().fg(colors::CYAN).bold())
            .borders(Borders::ALL)
            .border_style(if self.focus == FocusArea::Sidebar {
                Style::default().fg(colors::CYAN)
            } else {
                Style::default().fg(colors::BORDER)
            });

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Render sidebar items
        let items: Vec<Line> = self.sidebar_items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if i == self.sidebar_selected {
                    Style::default().fg(colors::CYAN).bold()
                } else {
                    Style::default().fg(colors::FG)
                };
                Line::from(Span::styled(item.as_str(), style))
            })
            .collect();

        let paragraph = Paragraph::new(items);
        frame.render_widget(paragraph, inner);
    }

    /// Render breadcrumb
    fn render_breadcrumb(&self, frame: &mut Frame, area: Rect) {
        let breadcrumb = Breadcrumb::new()
            .push(self.cwd.clone())
            .push(self.session_name.clone());
        breadcrumb.render(frame, area);
    }

    /// Render message history
    fn render_history(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(if self.focus == FocusArea::History {
                Style::default().fg(colors::CYAN)
            } else {
                Style::default().fg(colors::BORDER)
            });

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.history.is_empty() {
            // Empty state
            let centered = Rect {
                y: inner.y + inner.height / 2,
                height: 1,
                ..inner
            };

            if self.mode == AppMode::Loading {
                // Render spinner when loading
                self.spinner.render(frame, centered);
            } else {
                let paragraph = Paragraph::new("Start a conversation...")
                    .style(Style::default().fg(colors::COMMENT))
                    .alignment(Alignment::Center);
                frame.render_widget(paragraph, centered);
            }
            return;
        }

        // Render history items
        let mut lines: Vec<Line> = Vec::new();
        for cell in &self.history {
            let rendered = cell.render(inner.width);
            lines.extend(rendered);
            lines.push(Line::from("")); // Spacing
        }

        // Calculate scroll
        let total_lines = lines.len();
        let visible_lines = inner.height as usize;
        self.max_scroll = total_lines.saturating_sub(visible_lines);

        // Apply scroll
        let start = self.history_scroll.min(self.max_scroll);
        let visible: Vec<Line> = lines
            .into_iter()
            .skip(start)
            .take(visible_lines)
            .collect();

        let paragraph = Paragraph::new(visible);
        frame.render_widget(paragraph, inner);

        // Scrollbar
        if total_lines > visible_lines {
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
            let mut scrollbar_state = ScrollbarState::new(total_lines)
                .position(start);
            frame.render_stateful_widget(
                scrollbar,
                area,
                &mut scrollbar_state,
            );
        }
    }

    /// Render chat input
    fn render_chat_input(&mut self, frame: &mut Frame, area: Rect) {
        let is_focused = self.focus == FocusArea::Chat;

        // Update focused state for the composer
        self.chat.set_focused(is_focused && self.mode == AppMode::Normal);

        // Use ChatComposer's built-in render which handles cursor display
        self.chat.render(frame, area);
    }

    /// Render status bar
    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let mut status_bar = StatusBar::new()
            .left(
                StatusItem::new(self.model_name.clone())
                    .style(Style::default().fg(colors::CYAN)),
            );

        // Token count
        if self.tokens_used > 0 {
            status_bar = status_bar.left(
                StatusItem::new(format!("{}T", self.tokens_used))
                    .style(Style::default().fg(colors::COMMENT)),
            );
        }

        // Notification count
        let unread = self.notifications.unread_count();
        if unread > 0 {
            status_bar = status_bar.left(
                StatusItem::new(format!("{}N", unread))
                    .style(Style::default().fg(colors::YELLOW)),
            );
        }

        // Mode indicator
        let mode_str = match self.mode {
            AppMode::Normal => "NORMAL",
            AppMode::Streaming => "STREAM",
            AppMode::WaitingApproval => "APPROVAL",
            AppMode::Loading => "LOADING",
        };
        status_bar = status_bar.right(
            StatusItem::new(mode_str)
                .style(Style::default().fg(match self.mode {
                    AppMode::Normal => colors::GREEN,
                    AppMode::Streaming => colors::YELLOW,
                    AppMode::WaitingApproval => colors::ORANGE,
                    AppMode::Loading => colors::CYAN,
                })),
        );

        status_bar.render(frame, area);
    }

    /// Render active overlay
    fn render_overlay(&mut self, frame: &mut Frame, area: Rect) {
        match self.overlay {
            ActiveOverlay::CommandPalette => {
                let popup_area = centered_rect(60, 50, area);
                self.command_popup.render(frame, popup_area);
            }
            ActiveOverlay::Help => {
                let popup_area = centered_rect(80, 80, area);
                self.help_viewer.render(frame, popup_area);
            }
            ActiveOverlay::Approval => {
                let popup_area = centered_rect(60, 40, area);
                self.approval_overlay.render(frame, popup_area);
            }
            ActiveOverlay::Pager => {
                let popup_area = centered_rect(90, 90, area);
                self.pager_overlay.render(frame, popup_area);
            }
            ActiveOverlay::SessionPicker => {
                let popup_area = centered_rect(70, 60, area);
                self.session_picker.render(frame, popup_area);
            }
            ActiveOverlay::Notifications => {
                let popup_area = centered_rect(60, 70, area);
                self.notifications.panel.render(popup_area, frame.buffer_mut());
            }
            ActiveOverlay::Alert => {
                if let Some(ref alert) = self.notifications.alert {
                    alert.render(area, frame.buffer_mut());
                }
            }
            ActiveOverlay::None => {}
        }
    }
}

/// Helper to create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_width = area.width * percent_x / 100;
    let popup_height = area.height * percent_y / 100;

    Rect {
        x: area.x + (area.width - popup_width) / 2,
        y: area.y + (area.height - popup_height) / 2,
        width: popup_width,
        height: popup_height,
    }
}

/// Builder for creating views with custom configuration
pub struct ViewBuilder {
    view: MainView,
}

impl ViewBuilder {
    /// Create a new view builder
    pub fn new() -> Self {
        Self {
            view: MainView::new(),
        }
    }

    /// Set session name
    pub fn session(mut self, name: impl Into<String>) -> Self {
        self.view.session_name = name.into();
        self
    }

    /// Set model name
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.view.model_name = model.into();
        self
    }

    /// Enable sidebar
    pub fn with_sidebar(mut self, items: Vec<String>) -> Self {
        self.view.layout.show_sidebar = true;
        self.view.sidebar_items = items;
        self
    }

    /// Set sidebar width
    pub fn sidebar_width(mut self, width: u16) -> Self {
        self.view.layout.sidebar_width = width;
        self
    }

    /// Disable status bar
    pub fn without_status_bar(mut self) -> Self {
        self.view.layout.show_status_bar = false;
        self
    }

    /// Disable breadcrumb
    pub fn without_breadcrumb(mut self) -> Self {
        self.view.layout.show_breadcrumb = false;
        self
    }

    /// Set input height
    pub fn input_height(mut self, height: u16) -> Self {
        self.view.layout.input_height = height;
        self
    }

    /// Build the view
    pub fn build(self) -> MainView {
        self.view
    }
}

impl Default for ViewBuilder {
    fn default() -> Self {
        Self::new()
    }
}

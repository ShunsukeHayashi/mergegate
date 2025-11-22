//! Notification system for the TUI
//!
//! Provides alerts, banners, notification center, and history management.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::ui::colors;

/// Notification priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NotificationPriority {
    /// Low priority - informational
    Low,
    /// Normal priority - standard notifications
    Normal,
    /// High priority - important notifications
    High,
    /// Critical priority - requires immediate attention
    Critical,
}

impl NotificationPriority {
    /// Get color for this priority
    pub fn color(&self) -> Color {
        match self {
            Self::Low => colors::COMMENT,
            Self::Normal => colors::FG,
            Self::High => colors::YELLOW,
            Self::Critical => colors::RED,
        }
    }

    /// Get icon for this priority
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Low => "ℹ",
            Self::Normal => "●",
            Self::High => "⚠",
            Self::Critical => "🔴",
        }
    }
}

/// A single notification
#[derive(Debug, Clone)]
pub struct Notification {
    /// Unique identifier
    pub id: String,
    /// Title of the notification
    pub title: String,
    /// Message content
    pub message: String,
    /// Priority level
    pub priority: NotificationPriority,
    /// Source of the notification
    pub source: Option<String>,
    /// When the notification was created
    pub created_at: Instant,
    /// Duration to show (None = until dismissed)
    pub duration: Option<Duration>,
    /// Whether the notification has been read
    pub read: bool,
    /// Actions available
    pub actions: Vec<NotificationAction>,
}

impl Notification {
    /// Create a new notification
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title: title.into(),
            message: message.into(),
            priority: NotificationPriority::Normal,
            source: None,
            created_at: Instant::now(),
            duration: Some(Duration::from_secs(5)),
            read: false,
            actions: Vec::new(),
        }
    }

    /// Set priority
    pub fn with_priority(mut self, priority: NotificationPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set source
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set duration (None for persistent)
    pub fn with_duration(mut self, duration: Option<Duration>) -> Self {
        self.duration = duration;
        self
    }

    /// Add an action
    pub fn with_action(mut self, action: NotificationAction) -> Self {
        self.actions.push(action);
        self
    }

    /// Check if notification has expired
    pub fn is_expired(&self) -> bool {
        if let Some(duration) = self.duration {
            self.created_at.elapsed() >= duration
        } else {
            false
        }
    }

    /// Get age of notification
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Format age for display
    pub fn age_string(&self) -> String {
        let secs = self.age().as_secs();
        if secs < 60 {
            format!("{}s ago", secs)
        } else if secs < 3600 {
            format!("{}m ago", secs / 60)
        } else if secs < 86400 {
            format!("{}h ago", secs / 3600)
        } else {
            format!("{}d ago", secs / 86400)
        }
    }
}

/// Action that can be taken on a notification
#[derive(Debug, Clone)]
pub struct NotificationAction {
    /// Unique identifier
    pub id: String,
    /// Display label
    pub label: String,
    /// Keyboard shortcut
    pub shortcut: Option<char>,
    /// Whether this is the primary action
    pub primary: bool,
}

impl NotificationAction {
    /// Create a new action
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            shortcut: None,
            primary: false,
        }
    }

    /// Set as primary action
    pub fn primary(mut self) -> Self {
        self.primary = true;
        self
    }

    /// Set keyboard shortcut
    pub fn with_shortcut(mut self, shortcut: char) -> Self {
        self.shortcut = Some(shortcut);
        self
    }
}

/// Result of notification panel actions
#[derive(Debug, Clone)]
pub enum NotificationPanelAction {
    /// No action taken
    None,
    /// Close the panel
    Close,
    /// Dismiss notification by ID
    Dismiss(String),
    /// Dismiss all notifications
    DismissAll,
    /// Execute action on notification
    ExecuteAction { notification_id: String, action_id: String },
    /// Mark all as read
    MarkAllRead,
}

/// Notification panel UI
#[derive(Debug)]
pub struct NotificationPanel {
    /// All notifications
    notifications: VecDeque<Notification>,
    /// Selected index
    selected: usize,
    /// List state for scrolling
    list_state: ListState,
    /// Maximum notifications to keep
    max_notifications: usize,
    /// Whether panel is visible
    pub visible: bool,
    /// Filter by priority
    filter_priority: Option<NotificationPriority>,
    /// Show only unread
    filter_unread: bool,
}

impl Default for NotificationPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationPanel {
    /// Create a new notification panel
    pub fn new() -> Self {
        Self {
            notifications: VecDeque::new(),
            selected: 0,
            list_state: ListState::default(),
            max_notifications: 100,
            visible: false,
            filter_priority: None,
            filter_unread: false,
        }
    }

    /// Add a notification
    pub fn push(&mut self, notification: Notification) {
        self.notifications.push_front(notification);

        // Trim if over limit
        while self.notifications.len() > self.max_notifications {
            self.notifications.pop_back();
        }
    }

    /// Get filtered notifications
    fn filtered(&self) -> Vec<&Notification> {
        self.notifications
            .iter()
            .filter(|n| {
                if self.filter_unread && n.read {
                    return false;
                }
                if let Some(priority) = self.filter_priority {
                    if n.priority != priority {
                        return false;
                    }
                }
                true
            })
            .collect()
    }

    /// Get unread count
    pub fn unread_count(&self) -> usize {
        self.notifications.iter().filter(|n| !n.read).count()
    }

    /// Get total count
    pub fn count(&self) -> usize {
        self.notifications.len()
    }

    /// Remove expired notifications
    pub fn cleanup_expired(&mut self) {
        self.notifications.retain(|n| !n.is_expired());
    }

    /// Handle key event
    pub fn handle_key(&mut self, key: KeyEvent) -> NotificationPanelAction {
        let filtered = self.filtered();
        let count = filtered.len();

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.visible = false;
                NotificationPanelAction::Close
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.list_state.select(Some(self.selected));
                }
                NotificationPanelAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if count > 0 && self.selected < count - 1 {
                    self.selected += 1;
                    self.list_state.select(Some(self.selected));
                }
                NotificationPanelAction::None
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.selected = 0;
                self.list_state.select(Some(0));
                NotificationPanelAction::None
            }
            KeyCode::End | KeyCode::Char('G') => {
                if count > 0 {
                    self.selected = count - 1;
                    self.list_state.select(Some(self.selected));
                }
                NotificationPanelAction::None
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                if let Some(notification) = filtered.get(self.selected) {
                    let id = notification.id.clone();
                    NotificationPanelAction::Dismiss(id)
                } else {
                    NotificationPanelAction::None
                }
            }
            KeyCode::Char('D') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                NotificationPanelAction::DismissAll
            }
            KeyCode::Char('r') => {
                // Mark selected as read
                if let Some(notification) = filtered.get(self.selected) {
                    let id = notification.id.clone();
                    if let Some(n) = self.notifications.iter_mut().find(|n| n.id == id) {
                        n.read = true;
                    }
                }
                NotificationPanelAction::None
            }
            KeyCode::Char('R') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                NotificationPanelAction::MarkAllRead
            }
            KeyCode::Char('u') => {
                // Toggle unread filter
                self.filter_unread = !self.filter_unread;
                self.selected = 0;
                self.list_state.select(Some(0));
                NotificationPanelAction::None
            }
            KeyCode::Enter => {
                // Execute primary action
                if let Some(notification) = filtered.get(self.selected) {
                    if let Some(action) = notification.actions.iter().find(|a| a.primary) {
                        return NotificationPanelAction::ExecuteAction {
                            notification_id: notification.id.clone(),
                            action_id: action.id.clone(),
                        };
                    }
                }
                NotificationPanelAction::None
            }
            _ => NotificationPanelAction::None,
        }
    }

    /// Dismiss notification by ID
    pub fn dismiss(&mut self, id: &str) {
        self.notifications.retain(|n| n.id != id);
        let count = self.filtered().len();
        if self.selected >= count && count > 0 {
            self.selected = count - 1;
        }
    }

    /// Dismiss all notifications
    pub fn dismiss_all(&mut self) {
        self.notifications.clear();
        self.selected = 0;
    }

    /// Mark all as read
    pub fn mark_all_read(&mut self) {
        for notification in &mut self.notifications {
            notification.read = true;
        }
    }

    /// Render the notification panel
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // Clear background
        Clear.render(area, buf);

        // Draw border
        let block = Block::default()
            .title(format!(" Notifications ({}) ", self.unread_count()))
            .title_style(Style::default().fg(colors::CYAN).bold())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(colors::SELECTION));

        let inner = block.inner(area);
        block.render(area, buf);

        if self.notifications.is_empty() {
            let empty = Paragraph::new("No notifications")
                .style(Style::default().fg(colors::COMMENT))
                .alignment(Alignment::Center);
            let centered = Rect {
                y: inner.y + inner.height / 2,
                height: 1,
                ..inner
            };
            empty.render(centered, buf);
            return;
        }

        // Build list items
        let filtered = self.filtered();
        let items: Vec<ListItem> = filtered
            .iter()
            .enumerate()
            .map(|(i, notification)| {
                let is_selected = i == self.selected;

                // Build content
                let icon = notification.priority.icon();
                let read_indicator = if notification.read { " " } else { "•" };
                let age = notification.age_string();

                let header = format!(
                    "{} {} {} - {}",
                    read_indicator,
                    icon,
                    notification.title,
                    age
                );

                let mut lines = vec![
                    Line::from(Span::styled(
                        header,
                        Style::default()
                            .fg(notification.priority.color())
                            .add_modifier(if is_selected {
                                Modifier::BOLD
                            } else if notification.read {
                                Modifier::DIM
                            } else {
                                Modifier::empty()
                            }),
                    )),
                ];

                // Add message (truncated)
                let msg = if notification.message.len() > 60 {
                    format!("  {}...", &notification.message[..57])
                } else {
                    format!("  {}", notification.message)
                };
                lines.push(Line::from(Span::styled(
                    msg,
                    Style::default().fg(colors::FG).add_modifier(
                        if notification.read {
                            Modifier::DIM
                        } else {
                            Modifier::empty()
                        },
                    ),
                )));

                // Add actions if selected
                if is_selected && !notification.actions.is_empty() {
                    let actions_str: Vec<String> = notification
                        .actions
                        .iter()
                        .map(|a| {
                            if let Some(shortcut) = a.shortcut {
                                format!("[{}] {}", shortcut, a.label)
                            } else {
                                a.label.clone()
                            }
                        })
                        .collect();
                    lines.push(Line::from(Span::styled(
                        format!("  {}", actions_str.join(" | ")),
                        Style::default().fg(colors::CYAN),
                    )));
                }

                ListItem::new(lines)
            })
            .collect();

        let list = List::new(items)
            .highlight_style(Style::default().bg(colors::SELECTION));

        // Render with state
        ratatui::widgets::StatefulWidget::render(list, inner, buf, &mut self.list_state);

        // Draw help at bottom
        let help = " ↑↓:Navigate | d:Dismiss | D:Dismiss All | r:Read | R:Read All | u:Toggle Unread | q:Close ";
        let help_area = Rect {
            x: area.x,
            y: area.y + area.height - 1,
            width: area.width,
            height: 1,
        };
        Paragraph::new(help)
            .style(Style::default().fg(colors::COMMENT))
            .render(help_area, buf);
    }
}

/// A banner notification (non-blocking, appears at top/bottom)
#[derive(Debug, Clone)]
pub struct Banner {
    /// Message to display
    pub message: String,
    /// Priority level
    pub priority: NotificationPriority,
    /// When banner was created
    pub created_at: Instant,
    /// Duration to show
    pub duration: Duration,
    /// Whether banner is dismissible
    pub dismissible: bool,
    /// Progress value (0.0 - 1.0) for progress banners
    pub progress: Option<f64>,
}

impl Banner {
    /// Create a new banner
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            priority: NotificationPriority::Normal,
            created_at: Instant::now(),
            duration: Duration::from_secs(3),
            dismissible: true,
            progress: None,
        }
    }

    /// Set priority
    pub fn with_priority(mut self, priority: NotificationPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set duration
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Set as progress banner
    pub fn with_progress(mut self, progress: f64) -> Self {
        self.progress = Some(progress.clamp(0.0, 1.0));
        self
    }

    /// Check if banner has expired
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.duration
    }

    /// Render the banner
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let bg_color = match self.priority {
            NotificationPriority::Low => colors::COMMENT,
            NotificationPriority::Normal => colors::SELECTION,
            NotificationPriority::High => colors::YELLOW,
            NotificationPriority::Critical => colors::RED,
        };

        let fg_color = match self.priority {
            NotificationPriority::Low | NotificationPriority::Normal => colors::FG,
            NotificationPriority::High | NotificationPriority::Critical => colors::BG,
        };

        // Fill background
        for x in area.x..area.x + area.width {
            for y in area.y..area.y + area.height {
                if let Some(cell) = buf.cell_mut(Position { x, y }) {
                    cell.set_bg(bg_color);
                }
            }
        }

        // Draw message
        let icon = self.priority.icon();
        let text = format!(" {} {} ", icon, self.message);

        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(fg_color).bg(bg_color))
            .alignment(Alignment::Center);
        paragraph.render(area, buf);

        // Draw progress bar if present
        if let Some(progress) = self.progress {
            if area.height > 1 {
                let progress_area = Rect {
                    y: area.y + 1,
                    height: 1,
                    ..area
                };
                let filled = (progress * progress_area.width as f64) as u16;

                for x in progress_area.x..progress_area.x + filled {
                    if let Some(cell) = buf.cell_mut(Position { x, y: progress_area.y }) {
                        cell.set_bg(colors::GREEN);
                    }
                }
            }
        }
    }
}

/// Alert dialog (blocking, modal)
#[derive(Debug, Clone)]
pub struct Alert {
    /// Title
    pub title: String,
    /// Message
    pub message: String,
    /// Alert type
    pub alert_type: AlertType,
    /// Buttons
    pub buttons: Vec<AlertButton>,
    /// Currently selected button
    selected_button: usize,
}

/// Type of alert
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertType {
    /// Informational alert
    Info,
    /// Success alert
    Success,
    /// Warning alert
    Warning,
    /// Error alert
    Error,
    /// Confirmation alert
    Confirm,
}

impl AlertType {
    /// Get color for this alert type
    pub fn color(&self) -> Color {
        match self {
            Self::Info => colors::CYAN,
            Self::Success => colors::GREEN,
            Self::Warning => colors::YELLOW,
            Self::Error => colors::RED,
            Self::Confirm => colors::MAGENTA,
        }
    }

    /// Get icon for this alert type
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Info => "ℹ",
            Self::Success => "✓",
            Self::Warning => "⚠",
            Self::Error => "✗",
            Self::Confirm => "?",
        }
    }
}

/// Alert button
#[derive(Debug, Clone)]
pub struct AlertButton {
    /// Button ID
    pub id: String,
    /// Button label
    pub label: String,
    /// Whether this is the primary button
    pub primary: bool,
}

impl AlertButton {
    /// Create a new button
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            primary: false,
        }
    }

    /// Set as primary button
    pub fn primary(mut self) -> Self {
        self.primary = true;
        self
    }
}

/// Result of alert actions
#[derive(Debug, Clone)]
pub enum AlertAction {
    /// No action
    None,
    /// Button pressed
    ButtonPressed(String),
    /// Alert cancelled
    Cancelled,
}

impl Alert {
    /// Create a new alert
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            alert_type: AlertType::Info,
            buttons: vec![AlertButton::new("ok", "OK").primary()],
            selected_button: 0,
        }
    }

    /// Create an info alert
    pub fn info(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(title, message).with_type(AlertType::Info)
    }

    /// Create a success alert
    pub fn success(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(title, message).with_type(AlertType::Success)
    }

    /// Create a warning alert
    pub fn warning(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(title, message).with_type(AlertType::Warning)
    }

    /// Create an error alert
    pub fn error(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(title, message).with_type(AlertType::Error)
    }

    /// Create a confirmation alert
    pub fn confirm(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(title, message)
            .with_type(AlertType::Confirm)
            .with_buttons(vec![
                AlertButton::new("cancel", "Cancel"),
                AlertButton::new("confirm", "Confirm").primary(),
            ])
    }

    /// Set alert type
    pub fn with_type(mut self, alert_type: AlertType) -> Self {
        self.alert_type = alert_type;
        self
    }

    /// Set buttons
    pub fn with_buttons(mut self, buttons: Vec<AlertButton>) -> Self {
        self.buttons = buttons;
        self.selected_button = 0;
        self
    }

    /// Handle key event
    pub fn handle_key(&mut self, key: KeyEvent) -> AlertAction {
        match key.code {
            KeyCode::Esc => AlertAction::Cancelled,
            KeyCode::Left | KeyCode::Char('h') => {
                if self.selected_button > 0 {
                    self.selected_button -= 1;
                }
                AlertAction::None
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.selected_button < self.buttons.len().saturating_sub(1) {
                    self.selected_button += 1;
                }
                AlertAction::None
            }
            KeyCode::Tab => {
                self.selected_button = (self.selected_button + 1) % self.buttons.len();
                AlertAction::None
            }
            KeyCode::Enter => {
                if let Some(button) = self.buttons.get(self.selected_button) {
                    AlertAction::ButtonPressed(button.id.clone())
                } else {
                    AlertAction::None
                }
            }
            _ => AlertAction::None,
        }
    }

    /// Render the alert
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        // Calculate alert size
        let width = 50.min(area.width.saturating_sub(4));
        let height = 8.min(area.height.saturating_sub(4));

        let alert_area = Rect {
            x: area.x + (area.width - width) / 2,
            y: area.y + (area.height - height) / 2,
            width,
            height,
        };

        // Clear background
        Clear.render(alert_area, buf);

        // Draw border
        let icon = self.alert_type.icon();
        let color = self.alert_type.color();

        let block = Block::default()
            .title(format!(" {} {} ", icon, self.title))
            .title_style(Style::default().fg(color).bold())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(color));

        let inner = block.inner(alert_area);
        block.render(alert_area, buf);

        // Draw message
        let message_area = Rect {
            height: inner.height.saturating_sub(2),
            ..inner
        };
        let message = Paragraph::new(self.message.as_str())
            .style(Style::default().fg(colors::FG))
            .wrap(Wrap { trim: true });
        message.render(message_area, buf);

        // Draw buttons
        let button_area = Rect {
            y: inner.y + inner.height - 1,
            height: 1,
            ..inner
        };

        let mut x = button_area.x;
        for (i, button) in self.buttons.iter().enumerate() {
            let is_selected = i == self.selected_button;
            let style = if is_selected {
                Style::default().fg(colors::BG).bg(color).bold()
            } else if button.primary {
                Style::default().fg(color)
            } else {
                Style::default().fg(colors::FG)
            };

            let label = format!(" {} ", button.label);
            let span = Span::styled(label, style);
            let label_width = button.label.len() as u16 + 2;

            if x + label_width <= button_area.x + button_area.width {
                buf.set_span(x, button_area.y, &span, label_width);
                x += label_width + 1;
            }
        }
    }
}

/// Notification center that manages all notifications
#[derive(Debug)]
pub struct NotificationCenter {
    /// Active banners
    pub banners: VecDeque<Banner>,
    /// Notification panel
    pub panel: NotificationPanel,
    /// Current alert
    pub alert: Option<Alert>,
}

impl Default for NotificationCenter {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationCenter {
    /// Create a new notification center
    pub fn new() -> Self {
        Self {
            banners: VecDeque::new(),
            panel: NotificationPanel::new(),
            alert: None,
        }
    }

    /// Push a notification to the panel
    pub fn notify(&mut self, notification: Notification) {
        self.panel.push(notification);
    }

    /// Show a banner
    pub fn show_banner(&mut self, banner: Banner) {
        self.banners.push_back(banner);
    }

    /// Show an alert
    pub fn show_alert(&mut self, alert: Alert) {
        self.alert = Some(alert);
    }

    /// Cleanup expired items
    pub fn cleanup(&mut self) {
        self.banners.retain(|b| !b.is_expired());
        self.panel.cleanup_expired();
    }

    /// Check if there are any active overlays
    pub fn has_overlay(&self) -> bool {
        self.alert.is_some() || self.panel.visible
    }

    /// Get unread notification count
    pub fn unread_count(&self) -> usize {
        self.panel.unread_count()
    }

    /// Convenience method for info notification
    pub fn info(&mut self, title: impl Into<String>, message: impl Into<String>) {
        self.notify(
            Notification::new(title, message)
                .with_priority(NotificationPriority::Low)
        );
    }

    /// Convenience method for success notification
    pub fn success(&mut self, title: impl Into<String>, message: impl Into<String>) {
        self.notify(
            Notification::new(title, message)
                .with_priority(NotificationPriority::Normal)
        );
    }

    /// Convenience method for warning notification
    pub fn warning(&mut self, title: impl Into<String>, message: impl Into<String>) {
        self.notify(
            Notification::new(title, message)
                .with_priority(NotificationPriority::High)
        );
    }

    /// Convenience method for error notification
    pub fn error(&mut self, title: impl Into<String>, message: impl Into<String>) {
        self.notify(
            Notification::new(title, message)
                .with_priority(NotificationPriority::Critical)
                .with_duration(None) // Errors persist until dismissed
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // NotificationPriority tests
    #[test]
    fn test_priority_order() {
        assert!(NotificationPriority::Low < NotificationPriority::Normal);
        assert!(NotificationPriority::Normal < NotificationPriority::High);
        assert!(NotificationPriority::High < NotificationPriority::Critical);
    }

    #[test]
    fn test_priority_color() {
        assert_eq!(NotificationPriority::Low.color(), colors::COMMENT);
        assert_eq!(NotificationPriority::Normal.color(), colors::FG);
        assert_eq!(NotificationPriority::High.color(), colors::YELLOW);
        assert_eq!(NotificationPriority::Critical.color(), colors::RED);
    }

    #[test]
    fn test_priority_icon() {
        assert_eq!(NotificationPriority::Low.icon(), "ℹ");
        assert_eq!(NotificationPriority::Normal.icon(), "●");
        assert_eq!(NotificationPriority::High.icon(), "⚠");
        assert_eq!(NotificationPriority::Critical.icon(), "🔴");
    }

    // Notification tests
    #[test]
    fn test_notification_creation() {
        let notification = Notification::new("Test Title", "Test Message");
        assert_eq!(notification.title, "Test Title");
        assert_eq!(notification.message, "Test Message");
        assert_eq!(notification.priority, NotificationPriority::Normal);
        assert!(!notification.read);
        assert!(notification.actions.is_empty());
    }

    #[test]
    fn test_notification_with_priority() {
        let notification = Notification::new("Title", "Message")
            .with_priority(NotificationPriority::Critical);
        assert_eq!(notification.priority, NotificationPriority::Critical);
    }

    #[test]
    fn test_notification_with_source() {
        let notification = Notification::new("Title", "Message")
            .with_source("system");
        assert_eq!(notification.source, Some("system".to_string()));
    }

    #[test]
    fn test_notification_with_duration() {
        let notification = Notification::new("Title", "Message")
            .with_duration(Some(Duration::from_secs(10)));
        assert_eq!(notification.duration, Some(Duration::from_secs(10)));

        let persistent = Notification::new("Title", "Message")
            .with_duration(None);
        assert_eq!(persistent.duration, None);
    }

    #[test]
    fn test_notification_with_action() {
        let notification = Notification::new("Title", "Message")
            .with_action(NotificationAction::new("view", "View"));
        assert_eq!(notification.actions.len(), 1);
        assert_eq!(notification.actions[0].id, "view");
    }

    #[test]
    fn test_notification_is_expired() {
        // Short duration notification
        let notification = Notification::new("Title", "Message")
            .with_duration(Some(Duration::from_millis(1)));
        std::thread::sleep(Duration::from_millis(5));
        assert!(notification.is_expired());

        // Persistent notification never expires
        let persistent = Notification::new("Title", "Message")
            .with_duration(None);
        assert!(!persistent.is_expired());
    }

    #[test]
    fn test_notification_age() {
        let notification = Notification::new("Title", "Message");
        std::thread::sleep(Duration::from_millis(10));
        assert!(notification.age().as_millis() >= 10);
    }

    #[test]
    fn test_notification_age_string() {
        let notification = Notification::new("Title", "Message");
        let age_str = notification.age_string();
        assert!(age_str.contains("s ago"));
    }

    // NotificationAction tests
    #[test]
    fn test_notification_action_creation() {
        let action = NotificationAction::new("dismiss", "Dismiss");
        assert_eq!(action.id, "dismiss");
        assert_eq!(action.label, "Dismiss");
        assert!(!action.primary);
        assert!(action.shortcut.is_none());
    }

    #[test]
    fn test_notification_action_primary() {
        let action = NotificationAction::new("ok", "OK").primary();
        assert!(action.primary);
    }

    #[test]
    fn test_notification_action_with_shortcut() {
        let action = NotificationAction::new("view", "View").with_shortcut('v');
        assert_eq!(action.shortcut, Some('v'));
    }

    // NotificationPanel tests
    #[test]
    fn test_panel_creation() {
        let panel = NotificationPanel::new();
        assert_eq!(panel.count(), 0);
        assert_eq!(panel.unread_count(), 0);
        assert!(!panel.visible);
    }

    #[test]
    fn test_panel_push() {
        let mut panel = NotificationPanel::new();
        panel.push(Notification::new("Test", "Message"));
        assert_eq!(panel.count(), 1);
        assert_eq!(panel.unread_count(), 1);
    }

    #[test]
    fn test_panel_max_notifications() {
        let mut panel = NotificationPanel::new();
        panel.max_notifications = 3;

        for i in 0..5 {
            panel.push(Notification::new(format!("Title {}", i), "Message"));
        }

        assert_eq!(panel.count(), 3);
    }

    #[test]
    fn test_panel_dismiss() {
        let mut panel = NotificationPanel::new();
        let notification = Notification::new("Test", "Message");
        let id = notification.id.clone();
        panel.push(notification);

        assert_eq!(panel.count(), 1);
        panel.dismiss(&id);
        assert_eq!(panel.count(), 0);
    }

    #[test]
    fn test_panel_dismiss_all() {
        let mut panel = NotificationPanel::new();
        panel.push(Notification::new("Test 1", "Message"));
        panel.push(Notification::new("Test 2", "Message"));

        panel.dismiss_all();
        assert_eq!(panel.count(), 0);
    }

    #[test]
    fn test_panel_mark_all_read() {
        let mut panel = NotificationPanel::new();
        panel.push(Notification::new("Test 1", "Message"));
        panel.push(Notification::new("Test 2", "Message"));

        assert_eq!(panel.unread_count(), 2);
        panel.mark_all_read();
        assert_eq!(panel.unread_count(), 0);
    }

    #[test]
    fn test_panel_cleanup_expired() {
        let mut panel = NotificationPanel::new();
        panel.push(
            Notification::new("Test", "Message")
                .with_duration(Some(Duration::from_millis(1)))
        );

        std::thread::sleep(Duration::from_millis(5));
        panel.cleanup_expired();
        assert_eq!(panel.count(), 0);
    }

    #[test]
    fn test_panel_handle_key_close() {
        let mut panel = NotificationPanel::new();
        panel.visible = true;

        let action = panel.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert!(matches!(action, NotificationPanelAction::Close));
        assert!(!panel.visible);

        panel.visible = true;
        let action = panel.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty()));
        assert!(matches!(action, NotificationPanelAction::Close));
    }

    #[test]
    fn test_panel_handle_key_navigation() {
        let mut panel = NotificationPanel::new();
        panel.push(Notification::new("Test 1", "Message"));
        panel.push(Notification::new("Test 2", "Message"));

        // Navigate down
        panel.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
        assert_eq!(panel.selected, 1);

        // Navigate up
        panel.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
        assert_eq!(panel.selected, 0);

        // Navigate with j/k
        panel.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty()));
        assert_eq!(panel.selected, 1);

        panel.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::empty()));
        assert_eq!(panel.selected, 0);
    }

    #[test]
    fn test_panel_handle_key_home_end() {
        let mut panel = NotificationPanel::new();
        for i in 0..5 {
            panel.push(Notification::new(format!("Test {}", i), "Message"));
        }

        // Go to end
        panel.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::empty()));
        assert_eq!(panel.selected, 4);

        // Go to home
        panel.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::empty()));
        assert_eq!(panel.selected, 0);
    }

    #[test]
    fn test_panel_handle_key_dismiss() {
        let mut panel = NotificationPanel::new();
        let notification = Notification::new("Test", "Message");
        let id = notification.id.clone();
        panel.push(notification);

        let action = panel.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::empty()));
        assert!(matches!(action, NotificationPanelAction::Dismiss(ref i) if i == &id));
    }

    #[test]
    fn test_panel_handle_key_dismiss_all() {
        let mut panel = NotificationPanel::new();
        panel.push(Notification::new("Test", "Message"));

        let action = panel.handle_key(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::SHIFT));
        assert!(matches!(action, NotificationPanelAction::DismissAll));
    }

    #[test]
    fn test_panel_handle_key_mark_all_read() {
        let mut panel = NotificationPanel::new();
        panel.push(Notification::new("Test", "Message"));

        let action = panel.handle_key(KeyEvent::new(KeyCode::Char('R'), KeyModifiers::SHIFT));
        assert!(matches!(action, NotificationPanelAction::MarkAllRead));
    }

    #[test]
    fn test_panel_handle_key_toggle_unread() {
        let mut panel = NotificationPanel::new();
        assert!(!panel.filter_unread);

        panel.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::empty()));
        assert!(panel.filter_unread);

        panel.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::empty()));
        assert!(!panel.filter_unread);
    }

    // Banner tests
    #[test]
    fn test_banner_creation() {
        let banner = Banner::new("Test message");
        assert_eq!(banner.message, "Test message");
        assert_eq!(banner.priority, NotificationPriority::Normal);
        assert!(banner.dismissible);
        assert!(banner.progress.is_none());
    }

    #[test]
    fn test_banner_with_priority() {
        let banner = Banner::new("Test").with_priority(NotificationPriority::High);
        assert_eq!(banner.priority, NotificationPriority::High);
    }

    #[test]
    fn test_banner_with_duration() {
        let banner = Banner::new("Test").with_duration(Duration::from_secs(10));
        assert_eq!(banner.duration, Duration::from_secs(10));
    }

    #[test]
    fn test_banner_with_progress() {
        let banner = Banner::new("Test").with_progress(0.5);
        assert_eq!(banner.progress, Some(0.5));

        // Test clamping
        let banner = Banner::new("Test").with_progress(1.5);
        assert_eq!(banner.progress, Some(1.0));

        let banner = Banner::new("Test").with_progress(-0.5);
        assert_eq!(banner.progress, Some(0.0));
    }

    #[test]
    fn test_banner_is_expired() {
        let banner = Banner::new("Test").with_duration(Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(5));
        assert!(banner.is_expired());
    }

    // AlertType tests
    #[test]
    fn test_alert_type_color() {
        assert_eq!(AlertType::Info.color(), colors::CYAN);
        assert_eq!(AlertType::Success.color(), colors::GREEN);
        assert_eq!(AlertType::Warning.color(), colors::YELLOW);
        assert_eq!(AlertType::Error.color(), colors::RED);
        assert_eq!(AlertType::Confirm.color(), colors::MAGENTA);
    }

    #[test]
    fn test_alert_type_icon() {
        assert_eq!(AlertType::Info.icon(), "ℹ");
        assert_eq!(AlertType::Success.icon(), "✓");
        assert_eq!(AlertType::Warning.icon(), "⚠");
        assert_eq!(AlertType::Error.icon(), "✗");
        assert_eq!(AlertType::Confirm.icon(), "?");
    }

    // AlertButton tests
    #[test]
    fn test_alert_button_creation() {
        let button = AlertButton::new("ok", "OK");
        assert_eq!(button.id, "ok");
        assert_eq!(button.label, "OK");
        assert!(!button.primary);
    }

    #[test]
    fn test_alert_button_primary() {
        let button = AlertButton::new("ok", "OK").primary();
        assert!(button.primary);
    }

    // Alert tests
    #[test]
    fn test_alert_creation() {
        let alert = Alert::new("Title", "Message");
        assert_eq!(alert.title, "Title");
        assert_eq!(alert.message, "Message");
        assert_eq!(alert.alert_type, AlertType::Info);
        assert_eq!(alert.buttons.len(), 1);
        assert_eq!(alert.buttons[0].label, "OK");
    }

    #[test]
    fn test_alert_info() {
        let alert = Alert::info("Info", "Message");
        assert_eq!(alert.alert_type, AlertType::Info);
    }

    #[test]
    fn test_alert_success() {
        let alert = Alert::success("Success", "Message");
        assert_eq!(alert.alert_type, AlertType::Success);
    }

    #[test]
    fn test_alert_warning() {
        let alert = Alert::warning("Warning", "Message");
        assert_eq!(alert.alert_type, AlertType::Warning);
    }

    #[test]
    fn test_alert_error() {
        let alert = Alert::error("Error", "Message");
        assert_eq!(alert.alert_type, AlertType::Error);
    }

    #[test]
    fn test_alert_confirm() {
        let alert = Alert::confirm("Confirm", "Are you sure?");
        assert_eq!(alert.alert_type, AlertType::Confirm);
        assert_eq!(alert.buttons.len(), 2);
        assert_eq!(alert.buttons[0].label, "Cancel");
        assert_eq!(alert.buttons[1].label, "Confirm");
    }

    #[test]
    fn test_alert_with_type() {
        let alert = Alert::new("Title", "Message").with_type(AlertType::Error);
        assert_eq!(alert.alert_type, AlertType::Error);
    }

    #[test]
    fn test_alert_with_buttons() {
        let alert = Alert::new("Title", "Message").with_buttons(vec![
            AlertButton::new("yes", "Yes"),
            AlertButton::new("no", "No"),
        ]);
        assert_eq!(alert.buttons.len(), 2);
    }

    #[test]
    fn test_alert_handle_key_escape() {
        let mut alert = Alert::new("Title", "Message");
        let action = alert.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert!(matches!(action, AlertAction::Cancelled));
    }

    #[test]
    fn test_alert_handle_key_navigation() {
        let mut alert = Alert::new("Title", "Message").with_buttons(vec![
            AlertButton::new("a", "A"),
            AlertButton::new("b", "B"),
            AlertButton::new("c", "C"),
        ]);

        // Navigate right
        alert.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::empty()));
        assert_eq!(alert.selected_button, 1);

        // Navigate left
        alert.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::empty()));
        assert_eq!(alert.selected_button, 0);

        // Tab cycles
        alert.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()));
        assert_eq!(alert.selected_button, 1);
        alert.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()));
        assert_eq!(alert.selected_button, 2);
        alert.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()));
        assert_eq!(alert.selected_button, 0);
    }

    #[test]
    fn test_alert_handle_key_enter() {
        let mut alert = Alert::new("Title", "Message").with_buttons(vec![
            AlertButton::new("ok", "OK"),
        ]);

        let action = alert.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
        assert!(matches!(action, AlertAction::ButtonPressed(ref id) if id == "ok"));
    }

    // NotificationCenter tests
    #[test]
    fn test_center_creation() {
        let center = NotificationCenter::new();
        assert!(center.banners.is_empty());
        assert!(center.alert.is_none());
        assert_eq!(center.unread_count(), 0);
    }

    #[test]
    fn test_center_notify() {
        let mut center = NotificationCenter::new();
        center.notify(Notification::new("Test", "Message"));
        assert_eq!(center.panel.count(), 1);
        assert_eq!(center.unread_count(), 1);
    }

    #[test]
    fn test_center_show_banner() {
        let mut center = NotificationCenter::new();
        center.show_banner(Banner::new("Test"));
        assert_eq!(center.banners.len(), 1);
    }

    #[test]
    fn test_center_show_alert() {
        let mut center = NotificationCenter::new();
        center.show_alert(Alert::new("Title", "Message"));
        assert!(center.alert.is_some());
    }

    #[test]
    fn test_center_has_overlay() {
        let mut center = NotificationCenter::new();
        assert!(!center.has_overlay());

        center.show_alert(Alert::new("Title", "Message"));
        assert!(center.has_overlay());

        center.alert = None;
        center.panel.visible = true;
        assert!(center.has_overlay());
    }

    #[test]
    fn test_center_cleanup() {
        let mut center = NotificationCenter::new();
        center.show_banner(Banner::new("Test").with_duration(Duration::from_millis(1)));
        center.notify(
            Notification::new("Test", "Message")
                .with_duration(Some(Duration::from_millis(1)))
        );

        std::thread::sleep(Duration::from_millis(5));
        center.cleanup();

        assert!(center.banners.is_empty());
        assert_eq!(center.panel.count(), 0);
    }

    #[test]
    fn test_center_convenience_info() {
        let mut center = NotificationCenter::new();
        center.info("Info", "Message");
        assert_eq!(center.panel.count(), 1);
    }

    #[test]
    fn test_center_convenience_success() {
        let mut center = NotificationCenter::new();
        center.success("Success", "Message");
        assert_eq!(center.panel.count(), 1);
    }

    #[test]
    fn test_center_convenience_warning() {
        let mut center = NotificationCenter::new();
        center.warning("Warning", "Message");
        assert_eq!(center.panel.count(), 1);
    }

    #[test]
    fn test_center_convenience_error() {
        let mut center = NotificationCenter::new();
        center.error("Error", "Message");
        assert_eq!(center.panel.count(), 1);
    }

    #[test]
    fn test_notification_panel_action_enum() {
        // Just test the enum variants exist
        let _none = NotificationPanelAction::None;
        let _close = NotificationPanelAction::Close;
        let _dismiss = NotificationPanelAction::Dismiss("id".to_string());
        let _dismiss_all = NotificationPanelAction::DismissAll;
        let _execute = NotificationPanelAction::ExecuteAction {
            notification_id: "n".to_string(),
            action_id: "a".to_string(),
        };
        let _mark_read = NotificationPanelAction::MarkAllRead;
    }

    #[test]
    fn test_alert_action_enum() {
        let _none = AlertAction::None;
        let _pressed = AlertAction::ButtonPressed("ok".to_string());
        let _cancelled = AlertAction::Cancelled;
    }
}

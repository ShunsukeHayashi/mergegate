//! Resume Picker - Session/conversation picker UI
//!
//! Following Codex patterns for session management.
//! Features:
//! - List previous sessions
//! - Search/filter sessions
//! - Session metadata display
//! - Delete sessions

use chrono::{DateTime, Local, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

/// Session entry
#[derive(Debug, Clone)]
pub struct SessionEntry {
    /// Session ID
    pub id: String,
    /// Session title/name
    pub title: String,
    /// Last message preview
    pub preview: String,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Message count
    pub message_count: usize,
    /// Model used
    pub model: String,
    /// Token usage
    pub tokens_used: usize,
    /// Session tags
    pub tags: Vec<String>,
    /// Whether session is pinned
    pub pinned: bool,
}

impl SessionEntry {
    /// Create a new session entry
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            title: title.into(),
            preview: String::new(),
            created_at: now,
            updated_at: now,
            message_count: 0,
            model: "claude-3-sonnet".to_string(),
            tokens_used: 0,
            tags: Vec::new(),
            pinned: false,
        }
    }

    /// Set preview
    pub fn preview(mut self, preview: impl Into<String>) -> Self {
        self.preview = preview.into();
        self
    }

    /// Set timestamps
    pub fn timestamps(mut self, created: DateTime<Utc>, updated: DateTime<Utc>) -> Self {
        self.created_at = created;
        self.updated_at = updated;
        self
    }

    /// Set message count
    pub fn message_count(mut self, count: usize) -> Self {
        self.message_count = count;
        self
    }

    /// Set model
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set token usage
    pub fn tokens_used(mut self, tokens: usize) -> Self {
        self.tokens_used = tokens;
        self
    }

    /// Set tags
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Set pinned state
    pub fn pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self
    }

    /// Format time ago
    fn time_ago(&self) -> String {
        let duration = Utc::now().signed_duration_since(self.updated_at);

        if duration.num_minutes() < 1 {
            "just now".to_string()
        } else if duration.num_hours() < 1 {
            format!("{}m ago", duration.num_minutes())
        } else if duration.num_days() < 1 {
            format!("{}h ago", duration.num_hours())
        } else if duration.num_days() < 7 {
            format!("{}d ago", duration.num_days())
        } else if duration.num_weeks() < 4 {
            format!("{}w ago", duration.num_weeks())
        } else {
            self.updated_at.format("%Y-%m-%d").to_string()
        }
    }
}

/// Sort order for sessions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SessionSortOrder {
    /// Most recently updated first
    RecentFirst,
    /// Oldest first
    OldestFirst,
    /// Alphabetical by title
    Alphabetical,
    /// By message count
    MessageCount,
    /// By token usage
    TokenUsage,
}

/// Resume picker state
pub struct ResumePicker {
    /// All sessions
    sessions: Vec<SessionEntry>,
    /// Filtered sessions (indices)
    filtered: Vec<usize>,
    /// Search query
    query: String,
    /// Selected index
    selected: usize,
    /// List state
    list_state: ListState,
    /// Whether picker is visible
    visible: bool,
    /// Sort order
    sort_order: SessionSortOrder,
    /// Title
    title: String,
    /// Show details panel
    show_details: bool,
}

impl ResumePicker {
    /// Create a new resume picker
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            filtered: Vec::new(),
            query: String::new(),
            selected: 0,
            list_state: ListState::default(),
            visible: false,
            sort_order: SessionSortOrder::RecentFirst,
            title: "Resume Session".to_string(),
            show_details: true,
        }
    }

    /// Set sessions
    pub fn sessions(mut self, sessions: Vec<SessionEntry>) -> Self {
        self.sessions = sessions;
        self.sort_sessions();
        self.update_filtered();
        self
    }

    /// Set title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Add a session
    pub fn add_session(&mut self, session: SessionEntry) {
        self.sessions.push(session);
        self.sort_sessions();
        self.update_filtered();
    }

    /// Remove a session
    pub fn remove_session(&mut self, id: &str) -> bool {
        if let Some(pos) = self.sessions.iter().position(|s| s.id == id) {
            self.sessions.remove(pos);
            self.update_filtered();
            true
        } else {
            false
        }
    }

    /// Show the picker
    pub fn show(&mut self) {
        self.visible = true;
        self.query.clear();
        self.selected = 0;
        self.update_filtered();
    }

    /// Hide the picker
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Get selected session
    pub fn selected_session(&self) -> Option<&SessionEntry> {
        self.filtered.get(self.selected).map(|&idx| &self.sessions[idx])
    }

    /// Set sort order
    pub fn set_sort_order(&mut self, order: SessionSortOrder) {
        self.sort_order = order;
        self.sort_sessions();
        self.update_filtered();
    }

    /// Toggle details panel
    pub fn toggle_details(&mut self) {
        self.show_details = !self.show_details;
    }

    /// Handle key event
    pub fn handle_key(&mut self, key: KeyEvent) -> ResumePickerAction {
        if !self.visible {
            return ResumePickerAction::None;
        }

        match key.code {
            KeyCode::Esc => {
                self.hide();
                ResumePickerAction::Cancel
            }
            KeyCode::Enter => {
                if let Some(session) = self.selected_session() {
                    let id = session.id.clone();
                    self.hide();
                    ResumePickerAction::Select(id)
                } else {
                    ResumePickerAction::None
                }
            }
            KeyCode::Up | KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_up();
                ResumePickerAction::None
            }
            KeyCode::Down | KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_down();
                ResumePickerAction::None
            }
            KeyCode::Up => {
                self.move_up();
                ResumePickerAction::None
            }
            KeyCode::Down | KeyCode::Tab => {
                self.move_down();
                ResumePickerAction::None
            }
            KeyCode::PageUp => {
                for _ in 0..10 {
                    self.move_up();
                }
                ResumePickerAction::None
            }
            KeyCode::PageDown => {
                for _ in 0..10 {
                    self.move_down();
                }
                ResumePickerAction::None
            }
            KeyCode::Home => {
                self.selected = 0;
                self.update_list_state();
                ResumePickerAction::None
            }
            KeyCode::End => {
                if !self.filtered.is_empty() {
                    self.selected = self.filtered.len() - 1;
                    self.update_list_state();
                }
                ResumePickerAction::None
            }
            KeyCode::Delete | KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(session) = self.selected_session() {
                    let id = session.id.clone();
                    ResumePickerAction::Delete(id)
                } else {
                    ResumePickerAction::None
                }
            }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Toggle pin
                if let Some(&idx) = self.filtered.get(self.selected) {
                    self.sessions[idx].pinned = !self.sessions[idx].pinned;
                    self.sort_sessions();
                    self.update_filtered();
                }
                ResumePickerAction::None
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.toggle_details();
                ResumePickerAction::None
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Cycle sort order
                self.sort_order = match self.sort_order {
                    SessionSortOrder::RecentFirst => SessionSortOrder::OldestFirst,
                    SessionSortOrder::OldestFirst => SessionSortOrder::Alphabetical,
                    SessionSortOrder::Alphabetical => SessionSortOrder::MessageCount,
                    SessionSortOrder::MessageCount => SessionSortOrder::TokenUsage,
                    SessionSortOrder::TokenUsage => SessionSortOrder::RecentFirst,
                };
                self.sort_sessions();
                self.update_filtered();
                ResumePickerAction::None
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.hide();
                ResumePickerAction::NewSession
            }
            KeyCode::Char(c) => {
                self.query.push(c);
                self.update_filtered();
                ResumePickerAction::None
            }
            KeyCode::Backspace => {
                self.query.pop();
                self.update_filtered();
                ResumePickerAction::None
            }
            _ => ResumePickerAction::None,
        }
    }

    /// Move selection up
    fn move_up(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = self.selected.saturating_sub(1);
            self.update_list_state();
        }
    }

    /// Move selection down
    fn move_down(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = (self.selected + 1).min(self.filtered.len() - 1);
            self.update_list_state();
        }
    }

    /// Update list state
    fn update_list_state(&mut self) {
        self.list_state.select(Some(self.selected));
    }

    /// Sort sessions
    fn sort_sessions(&mut self) {
        // Pinned items always first
        self.sessions.sort_by(|a, b| {
            match (a.pinned, b.pinned) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => match self.sort_order {
                    SessionSortOrder::RecentFirst => b.updated_at.cmp(&a.updated_at),
                    SessionSortOrder::OldestFirst => a.updated_at.cmp(&b.updated_at),
                    SessionSortOrder::Alphabetical => a.title.cmp(&b.title),
                    SessionSortOrder::MessageCount => b.message_count.cmp(&a.message_count),
                    SessionSortOrder::TokenUsage => b.tokens_used.cmp(&a.tokens_used),
                },
            }
        });
    }

    /// Update filtered sessions
    fn update_filtered(&mut self) {
        let query = self.query.to_lowercase();

        if query.is_empty() {
            self.filtered = (0..self.sessions.len()).collect();
        } else {
            self.filtered = self
                .sessions
                .iter()
                .enumerate()
                .filter(|(_, session)| {
                    session.title.to_lowercase().contains(&query)
                        || session.preview.to_lowercase().contains(&query)
                        || session.tags.iter().any(|t| t.to_lowercase().contains(&query))
                })
                .map(|(i, _)| i)
                .collect();
        }

        if self.selected >= self.filtered.len() {
            self.selected = 0;
        }
        self.update_list_state();
    }

    /// Render the picker
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Calculate popup dimensions
        let popup_width = area.width.min(80);
        let popup_height = area.height.min(24);

        let popup_area = Rect {
            x: (area.width - popup_width) / 2 + area.x,
            y: (area.height - popup_height) / 2 + area.y,
            width: popup_width,
            height: popup_height,
        };

        // Clear background
        frame.render_widget(Clear, popup_area);

        // Main block
        let block = Block::default()
            .title(Span::styled(
                format!(" {} ", self.title),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        // Layout
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search
                Constraint::Min(1),    // Content
                Constraint::Length(2), // Status
            ])
            .split(inner);

        // Search input
        self.render_search(frame, main_chunks[0]);

        // Content (list + details)
        if self.show_details {
            let content_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_chunks[1]);

            self.render_list(frame, content_chunks[0]);
            self.render_details(frame, content_chunks[1]);
        } else {
            self.render_list(frame, main_chunks[1]);
        }

        // Status bar
        self.render_status(frame, main_chunks[2]);
    }

    /// Render search input
    fn render_search(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(86, 95, 137)));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let content = if self.query.is_empty() {
            Line::from(vec![
                Span::styled("› ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    "Search sessions...",
                    Style::default().fg(Color::Rgb(86, 95, 137)),
                ),
            ])
        } else {
            Line::from(vec![
                Span::styled("› ", Style::default().fg(Color::Cyan)),
                Span::raw(&self.query),
                Span::styled("█", Style::default().fg(Color::Cyan)),
            ])
        };

        let search = Paragraph::new(content);
        frame.render_widget(search, inner);
    }

    /// Render session list
    fn render_list(&mut self, frame: &mut Frame, area: Rect) {
        if self.filtered.is_empty() {
            let no_results = Paragraph::new(Line::from(Span::styled(
                "No sessions found",
                Style::default().fg(Color::Rgb(86, 95, 137)),
            )))
            .alignment(Alignment::Center);
            frame.render_widget(no_results, area);
            return;
        }

        let items: Vec<ListItem> = self
            .filtered
            .iter()
            .enumerate()
            .map(|(i, &idx)| {
                let session = &self.sessions[idx];
                let is_selected = i == self.selected;

                let mut spans = vec![];

                // Pin indicator
                if session.pinned {
                    spans.push(Span::styled("📌 ", Style::default().fg(Color::Yellow)));
                } else {
                    spans.push(Span::raw("   "));
                }

                // Title
                let title_style = if is_selected {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Rgb(192, 202, 245))
                };

                let title = if session.title.len() > 25 {
                    format!("{}...", &session.title[..22])
                } else {
                    session.title.clone()
                };
                spans.push(Span::styled(title, title_style));

                // Time ago
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    session.time_ago(),
                    Style::default().fg(Color::Rgb(86, 95, 137)),
                ));

                let line = Line::from(spans);

                let style = if is_selected {
                    Style::default().bg(Color::Rgb(86, 95, 137))
                } else {
                    Style::default()
                };

                ListItem::new(line).style(style)
            })
            .collect();

        let list = List::new(items).highlight_symbol("");
        frame.render_stateful_widget(list, area, &mut self.list_state);
    }

    /// Render details panel
    fn render_details(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Details ")
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(Color::Rgb(86, 95, 137)));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let session = match self.selected_session() {
            Some(s) => s,
            None => return,
        };

        let mut lines = vec![];

        // Title
        lines.push(Line::from(Span::styled(
            &session.title,
            Style::default()
                .fg(Color::Rgb(192, 202, 245))
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        // Preview
        if !session.preview.is_empty() {
            let preview = if session.preview.len() > 100 {
                format!("{}...", &session.preview[..97])
            } else {
                session.preview.clone()
            };
            lines.push(Line::from(Span::styled(
                preview,
                Style::default().fg(Color::Rgb(169, 177, 214)),
            )));
            lines.push(Line::from(""));
        }

        // Metadata
        lines.push(Line::from(vec![
            Span::styled("Model: ", Style::default().fg(Color::Rgb(86, 95, 137))),
            Span::styled(&session.model, Style::default().fg(Color::Cyan)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Messages: ", Style::default().fg(Color::Rgb(86, 95, 137))),
            Span::styled(
                session.message_count.to_string(),
                Style::default().fg(Color::Rgb(192, 202, 245)),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Tokens: ", Style::default().fg(Color::Rgb(86, 95, 137))),
            Span::styled(
                format_tokens(session.tokens_used),
                Style::default().fg(Color::Rgb(192, 202, 245)),
            ),
        ]));

        // Created/Updated
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Created: ", Style::default().fg(Color::Rgb(86, 95, 137))),
            Span::styled(
                session.created_at.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string(),
                Style::default().fg(Color::Rgb(169, 177, 214)),
            ),
        ]));

        // Tags
        if !session.tags.is_empty() {
            lines.push(Line::from(""));
            let mut tag_spans = vec![Span::styled(
                "Tags: ",
                Style::default().fg(Color::Rgb(86, 95, 137)),
            )];
            for tag in &session.tags {
                tag_spans.push(Span::styled(
                    format!("[{}] ", tag),
                    Style::default().fg(Color::Magenta),
                ));
            }
            lines.push(Line::from(tag_spans));
        }

        let details = Paragraph::new(lines);
        frame.render_widget(details, inner);
    }

    /// Render status bar
    fn render_status(&self, frame: &mut Frame, area: Rect) {
        let sort_label = match self.sort_order {
            SessionSortOrder::RecentFirst => "Recent",
            SessionSortOrder::OldestFirst => "Oldest",
            SessionSortOrder::Alphabetical => "A-Z",
            SessionSortOrder::MessageCount => "Messages",
            SessionSortOrder::TokenUsage => "Tokens",
        };

        let status = Line::from(vec![
            Span::styled(
                format!(" {} sessions ", self.filtered.len()),
                Style::default().fg(Color::Rgb(86, 95, 137)),
            ),
            Span::raw("│"),
            Span::styled(
                format!(" Sort: {} ", sort_label),
                Style::default().fg(Color::Rgb(86, 95, 137)),
            ),
            Span::raw("│"),
            Span::styled(
                " Ctrl+N: New ",
                Style::default().fg(Color::Rgb(86, 95, 137)),
            ),
            Span::styled(
                " Del: Delete ",
                Style::default().fg(Color::Rgb(86, 95, 137)),
            ),
        ]);

        let status_bar = Paragraph::new(status);
        frame.render_widget(status_bar, area);
    }

    /// Get session count
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
}

impl Default for ResumePicker {
    fn default() -> Self {
        Self::new()
    }
}

/// Action returned by resume picker
#[derive(Debug, Clone, PartialEq)]
pub enum ResumePickerAction {
    /// No action
    None,
    /// Select session
    Select(String),
    /// Delete session
    Delete(String),
    /// Create new session
    NewSession,
    /// Cancel/close
    Cancel,
}

/// Format token count
fn format_tokens(tokens: usize) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

/// Session storage manager
pub struct SessionManager {
    /// Sessions
    sessions: Vec<SessionEntry>,
    /// Storage path
    #[allow(dead_code)]
    storage_path: String,
}

impl SessionManager {
    /// Create new session manager
    pub fn new(storage_path: impl Into<String>) -> Self {
        Self {
            sessions: Vec::new(),
            storage_path: storage_path.into(),
        }
    }

    /// Get all sessions
    pub fn sessions(&self) -> &[SessionEntry] {
        &self.sessions
    }

    /// Add session
    pub fn add(&mut self, session: SessionEntry) {
        self.sessions.push(session);
    }

    /// Remove session
    pub fn remove(&mut self, id: &str) -> Option<SessionEntry> {
        if let Some(pos) = self.sessions.iter().position(|s| s.id == id) {
            Some(self.sessions.remove(pos))
        } else {
            None
        }
    }

    /// Get session by ID
    pub fn get(&self, id: &str) -> Option<&SessionEntry> {
        self.sessions.iter().find(|s| s.id == id)
    }

    /// Get mutable session by ID
    pub fn get_mut(&mut self, id: &str) -> Option<&mut SessionEntry> {
        self.sessions.iter_mut().find(|s| s.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    // SessionEntry tests
    #[test]
    fn test_session_entry_creation() {
        let entry = SessionEntry::new("id1", "Test Session");
        assert_eq!(entry.id, "id1");
        assert_eq!(entry.title, "Test Session");
        assert!(entry.preview.is_empty());
        assert_eq!(entry.message_count, 0);
        assert_eq!(entry.tokens_used, 0);
        assert!(!entry.pinned);
    }

    #[test]
    fn test_session_entry_preview() {
        let entry = SessionEntry::new("id1", "Test").preview("Hello world");
        assert_eq!(entry.preview, "Hello world");
    }

    #[test]
    fn test_session_entry_message_count() {
        let entry = SessionEntry::new("id1", "Test").message_count(42);
        assert_eq!(entry.message_count, 42);
    }

    #[test]
    fn test_session_entry_model() {
        let entry = SessionEntry::new("id1", "Test").model("claude-3-opus");
        assert_eq!(entry.model, "claude-3-opus");
    }

    #[test]
    fn test_session_entry_tokens() {
        let entry = SessionEntry::new("id1", "Test").tokens_used(5000);
        assert_eq!(entry.tokens_used, 5000);
    }

    #[test]
    fn test_session_entry_tags() {
        let entry = SessionEntry::new("id1", "Test").tags(vec!["rust".to_string(), "cli".to_string()]);
        assert_eq!(entry.tags.len(), 2);
        assert_eq!(entry.tags[0], "rust");
    }

    #[test]
    fn test_session_entry_pinned() {
        let entry = SessionEntry::new("id1", "Test").pinned(true);
        assert!(entry.pinned);
    }

    #[test]
    fn test_session_entry_timestamps() {
        let created = Utc::now() - Duration::days(1);
        let updated = Utc::now();
        let entry = SessionEntry::new("id1", "Test").timestamps(created, updated);
        assert_eq!(entry.created_at, created);
        assert_eq!(entry.updated_at, updated);
    }

    #[test]
    fn test_session_entry_time_ago_just_now() {
        let entry = SessionEntry::new("id1", "Test");
        assert_eq!(entry.time_ago(), "just now");
    }

    #[test]
    fn test_session_entry_time_ago_minutes() {
        let mut entry = SessionEntry::new("id1", "Test");
        entry.updated_at = Utc::now() - Duration::minutes(30);
        assert!(entry.time_ago().contains("m ago"));
    }

    #[test]
    fn test_session_entry_time_ago_hours() {
        let mut entry = SessionEntry::new("id1", "Test");
        entry.updated_at = Utc::now() - Duration::hours(5);
        assert!(entry.time_ago().contains("h ago"));
    }

    #[test]
    fn test_session_entry_time_ago_days() {
        let mut entry = SessionEntry::new("id1", "Test");
        entry.updated_at = Utc::now() - Duration::days(3);
        assert!(entry.time_ago().contains("d ago"));
    }

    // SessionSortOrder tests
    #[test]
    fn test_sort_order_enum() {
        let order = SessionSortOrder::RecentFirst;
        assert_eq!(order, SessionSortOrder::RecentFirst);

        let order2 = SessionSortOrder::Alphabetical;
        assert_eq!(order2, SessionSortOrder::Alphabetical);
    }

    // ResumePicker tests
    #[test]
    fn test_picker_creation() {
        let picker = ResumePicker::new();
        assert!(!picker.is_visible());
        assert_eq!(picker.session_count(), 0);
    }

    #[test]
    fn test_picker_title() {
        let picker = ResumePicker::new().title("My Sessions");
        assert_eq!(picker.title, "My Sessions");
    }

    #[test]
    fn test_picker_sessions() {
        let sessions = vec![
            SessionEntry::new("1", "Session 1"),
            SessionEntry::new("2", "Session 2"),
        ];
        let picker = ResumePicker::new().sessions(sessions);
        assert_eq!(picker.session_count(), 2);
    }

    #[test]
    fn test_picker_show_hide() {
        let mut picker = ResumePicker::new();
        assert!(!picker.is_visible());
        picker.show();
        assert!(picker.is_visible());
        picker.hide();
        assert!(!picker.is_visible());
    }

    #[test]
    fn test_picker_add_session() {
        let mut picker = ResumePicker::new();
        picker.add_session(SessionEntry::new("1", "Test"));
        assert_eq!(picker.session_count(), 1);
    }

    #[test]
    fn test_picker_remove_session() {
        let mut picker = ResumePicker::new();
        picker.add_session(SessionEntry::new("1", "Test"));
        assert_eq!(picker.session_count(), 1);
        assert!(picker.remove_session("1"));
        assert_eq!(picker.session_count(), 0);
    }

    #[test]
    fn test_picker_remove_nonexistent() {
        let mut picker = ResumePicker::new();
        assert!(!picker.remove_session("nonexistent"));
    }

    #[test]
    fn test_picker_selected_session() {
        let now = Utc::now();
        let sessions = vec![
            SessionEntry::new("1", "Session 1").timestamps(now - Duration::hours(2), now - Duration::hours(2)),
            SessionEntry::new("2", "Session 2").timestamps(now - Duration::hours(1), now - Duration::hours(1)),
        ];
        let mut picker = ResumePicker::new().sessions(sessions);
        picker.show();

        let selected = picker.selected_session();
        assert!(selected.is_some());
        // Most recent first, so "2" should be first
        assert_eq!(selected.unwrap().id, "2");
    }

    #[test]
    fn test_picker_selected_session_empty() {
        let mut picker = ResumePicker::new();
        picker.show();
        assert!(picker.selected_session().is_none());
    }

    #[test]
    fn test_picker_set_sort_order() {
        let mut picker = ResumePicker::new();
        picker.set_sort_order(SessionSortOrder::Alphabetical);
        assert_eq!(picker.sort_order, SessionSortOrder::Alphabetical);
    }

    #[test]
    fn test_picker_toggle_details() {
        let mut picker = ResumePicker::new();
        assert!(picker.show_details);
        picker.toggle_details();
        assert!(!picker.show_details);
        picker.toggle_details();
        assert!(picker.show_details);
    }

    #[test]
    fn test_picker_handle_key_not_visible() {
        let mut picker = ResumePicker::new();
        let action = picker.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
        assert_eq!(action, ResumePickerAction::None);
    }

    #[test]
    fn test_picker_handle_key_escape() {
        let mut picker = ResumePicker::new();
        picker.show();
        let action = picker.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert_eq!(action, ResumePickerAction::Cancel);
        assert!(!picker.is_visible());
    }

    #[test]
    fn test_picker_handle_key_enter() {
        let sessions = vec![SessionEntry::new("test-id", "Test Session")];
        let mut picker = ResumePicker::new().sessions(sessions);
        picker.show();
        let action = picker.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
        assert_eq!(action, ResumePickerAction::Select("test-id".to_string()));
    }

    #[test]
    fn test_picker_handle_key_enter_empty() {
        let mut picker = ResumePicker::new();
        picker.show();
        let action = picker.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
        assert_eq!(action, ResumePickerAction::None);
    }

    #[test]
    fn test_picker_handle_key_navigation() {
        let now = Utc::now();
        let sessions = vec![
            SessionEntry::new("1", "Session 1").timestamps(now - Duration::hours(3), now - Duration::hours(3)),
            SessionEntry::new("2", "Session 2").timestamps(now - Duration::hours(2), now - Duration::hours(2)),
            SessionEntry::new("3", "Session 3").timestamps(now - Duration::hours(1), now - Duration::hours(1)),
        ];
        let mut picker = ResumePicker::new().sessions(sessions);
        picker.show();

        // First item is "3" (most recent), move down to "2"
        picker.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
        assert_eq!(picker.selected_session().unwrap().id, "2");

        // Move up back to "3"
        picker.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
        assert_eq!(picker.selected_session().unwrap().id, "3");
    }

    #[test]
    fn test_picker_handle_key_tab() {
        let now = Utc::now();
        let sessions = vec![
            SessionEntry::new("1", "Session 1").timestamps(now - Duration::hours(2), now - Duration::hours(2)),
            SessionEntry::new("2", "Session 2").timestamps(now - Duration::hours(1), now - Duration::hours(1)),
        ];
        let mut picker = ResumePicker::new().sessions(sessions);
        picker.show();

        // First is "2" (most recent), tab moves to "1"
        picker.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()));
        assert_eq!(picker.selected_session().unwrap().id, "1");
    }

    #[test]
    fn test_picker_handle_key_home_end() {
        let now = Utc::now();
        let sessions = vec![
            SessionEntry::new("1", "Session 1").timestamps(now - Duration::hours(3), now - Duration::hours(3)),
            SessionEntry::new("2", "Session 2").timestamps(now - Duration::hours(2), now - Duration::hours(2)),
            SessionEntry::new("3", "Session 3").timestamps(now - Duration::hours(1), now - Duration::hours(1)),
        ];
        let mut picker = ResumePicker::new().sessions(sessions);
        picker.show();

        // End goes to last item (oldest = "1")
        picker.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::empty()));
        assert_eq!(picker.selected_session().unwrap().id, "1");

        // Home goes to first item (newest = "3")
        picker.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::empty()));
        assert_eq!(picker.selected_session().unwrap().id, "3");
    }

    #[test]
    fn test_picker_handle_key_page_down() {
        let sessions: Vec<_> = (0..20)
            .map(|i| SessionEntry::new(format!("{}", i), format!("Session {}", i)))
            .collect();
        let mut picker = ResumePicker::new().sessions(sessions);
        picker.show();

        picker.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty()));
        assert!(picker.selected >= 10);
    }

    #[test]
    fn test_picker_handle_key_delete() {
        let sessions = vec![SessionEntry::new("test-id", "Test")];
        let mut picker = ResumePicker::new().sessions(sessions);
        picker.show();

        // Delete requires CONTROL modifier
        let action = picker.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::CONTROL));
        assert_eq!(action, ResumePickerAction::Delete("test-id".to_string()));
    }

    #[test]
    fn test_picker_handle_key_ctrl_n() {
        let mut picker = ResumePicker::new();
        picker.show();
        let action = picker.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL));
        assert_eq!(action, ResumePickerAction::NewSession);
        assert!(!picker.is_visible());
    }

    #[test]
    fn test_picker_handle_key_search() {
        let sessions = vec![
            SessionEntry::new("1", "Rust project"),
            SessionEntry::new("2", "Python project"),
        ];
        let mut picker = ResumePicker::new().sessions(sessions);
        picker.show();

        picker.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::empty()));
        picker.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::empty()));
        picker.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::empty()));
        picker.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::empty()));

        assert_eq!(picker.query, "rust");
        assert_eq!(picker.filtered.len(), 1);
    }

    #[test]
    fn test_picker_handle_key_backspace() {
        let mut picker = ResumePicker::new();
        picker.show();
        picker.query = "test".to_string();

        picker.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty()));
        assert_eq!(picker.query, "tes");
    }

    #[test]
    fn test_picker_handle_key_ctrl_d_toggle_details() {
        let mut picker = ResumePicker::new();
        picker.show();
        assert!(picker.show_details);

        picker.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert!(!picker.show_details);
    }

    #[test]
    fn test_picker_handle_key_ctrl_s_cycle_sort() {
        let mut picker = ResumePicker::new();
        picker.show();
        assert_eq!(picker.sort_order, SessionSortOrder::RecentFirst);

        picker.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
        assert_eq!(picker.sort_order, SessionSortOrder::OldestFirst);

        picker.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
        assert_eq!(picker.sort_order, SessionSortOrder::Alphabetical);
    }

    #[test]
    fn test_picker_handle_key_ctrl_p_toggle_pin() {
        let sessions = vec![SessionEntry::new("1", "Test")];
        let mut picker = ResumePicker::new().sessions(sessions);
        picker.show();

        assert!(!picker.sessions[0].pinned);
        picker.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL));
        assert!(picker.sessions[0].pinned);
    }

    #[test]
    fn test_picker_sorting_pinned_first() {
        let sessions = vec![
            SessionEntry::new("1", "First"),
            SessionEntry::new("2", "Second").pinned(true),
        ];
        let picker = ResumePicker::new().sessions(sessions);

        // Pinned should come first
        assert!(picker.sessions[0].pinned);
        assert_eq!(picker.sessions[0].id, "2");
    }

    #[test]
    fn test_picker_filtering() {
        let sessions = vec![
            SessionEntry::new("1", "Rust development"),
            SessionEntry::new("2", "Python script"),
            SessionEntry::new("3", "Rust CLI tool"),
        ];
        let mut picker = ResumePicker::new().sessions(sessions);
        picker.show();

        picker.query = "rust".to_string();
        picker.update_filtered();

        assert_eq!(picker.filtered.len(), 2);
    }

    #[test]
    fn test_picker_filtering_by_tags() {
        let sessions = vec![
            SessionEntry::new("1", "Project A").tags(vec!["important".to_string()]),
            SessionEntry::new("2", "Project B"),
        ];
        let mut picker = ResumePicker::new().sessions(sessions);
        picker.show();

        picker.query = "important".to_string();
        picker.update_filtered();

        assert_eq!(picker.filtered.len(), 1);
    }

    #[test]
    fn test_picker_show_resets_state() {
        let sessions = vec![
            SessionEntry::new("1", "Session 1"),
            SessionEntry::new("2", "Session 2"),
        ];
        let mut picker = ResumePicker::new().sessions(sessions);
        picker.show();
        picker.query = "test".to_string();
        picker.selected = 1;
        picker.hide();

        picker.show();
        assert!(picker.query.is_empty());
        assert_eq!(picker.selected, 0);
    }

    #[test]
    fn test_picker_default() {
        let picker = ResumePicker::default();
        assert!(!picker.is_visible());
        assert_eq!(picker.session_count(), 0);
    }

    // ResumePickerAction tests
    #[test]
    fn test_resume_picker_action_enum() {
        let action = ResumePickerAction::None;
        assert_eq!(action, ResumePickerAction::None);

        let action = ResumePickerAction::Select("id".to_string());
        assert!(matches!(action, ResumePickerAction::Select(_)));

        let action = ResumePickerAction::Delete("id".to_string());
        assert!(matches!(action, ResumePickerAction::Delete(_)));

        let action = ResumePickerAction::NewSession;
        assert_eq!(action, ResumePickerAction::NewSession);

        let action = ResumePickerAction::Cancel;
        assert_eq!(action, ResumePickerAction::Cancel);
    }

    // format_tokens tests
    #[test]
    fn test_format_tokens_small() {
        assert_eq!(format_tokens(100), "100");
        assert_eq!(format_tokens(999), "999");
    }

    #[test]
    fn test_format_tokens_thousands() {
        assert_eq!(format_tokens(1000), "1.0K");
        assert_eq!(format_tokens(5500), "5.5K");
        assert_eq!(format_tokens(999999), "1000.0K");
    }

    #[test]
    fn test_format_tokens_millions() {
        assert_eq!(format_tokens(1_000_000), "1.0M");
        assert_eq!(format_tokens(2_500_000), "2.5M");
    }

    // SessionManager tests
    #[test]
    fn test_manager_creation() {
        let manager = SessionManager::new("/tmp/sessions");
        assert_eq!(manager.sessions().len(), 0);
    }

    #[test]
    fn test_manager_add() {
        let mut manager = SessionManager::new("/tmp");
        manager.add(SessionEntry::new("1", "Test"));
        assert_eq!(manager.sessions().len(), 1);
    }

    #[test]
    fn test_manager_remove() {
        let mut manager = SessionManager::new("/tmp");
        manager.add(SessionEntry::new("1", "Test"));

        let removed = manager.remove("1");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, "1");
        assert_eq!(manager.sessions().len(), 0);
    }

    #[test]
    fn test_manager_remove_nonexistent() {
        let mut manager = SessionManager::new("/tmp");
        assert!(manager.remove("nonexistent").is_none());
    }

    #[test]
    fn test_manager_get() {
        let mut manager = SessionManager::new("/tmp");
        manager.add(SessionEntry::new("1", "Test"));

        let session = manager.get("1");
        assert!(session.is_some());
        assert_eq!(session.unwrap().title, "Test");
    }

    #[test]
    fn test_manager_get_nonexistent() {
        let manager = SessionManager::new("/tmp");
        assert!(manager.get("nonexistent").is_none());
    }

    #[test]
    fn test_manager_get_mut() {
        let mut manager = SessionManager::new("/tmp");
        manager.add(SessionEntry::new("1", "Test"));

        if let Some(session) = manager.get_mut("1") {
            session.title = "Updated".to_string();
        }

        assert_eq!(manager.get("1").unwrap().title, "Updated");
    }
}

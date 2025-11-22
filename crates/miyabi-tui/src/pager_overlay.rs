//! Pager Overlay - Scrollable content viewer
//!
//! Following Codex patterns for viewing long content.
//! Features:
//! - Vi-like navigation
//! - Search within content
//! - Line numbers
//! - Syntax highlighting support

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

/// Pager content type
#[derive(Debug, Clone)]
pub enum PagerContent {
    /// Plain text
    Plain(String),
    /// Markdown content
    Markdown(String),
    /// Code with language
    Code { content: String, language: String },
    /// Pre-styled lines
    Styled(Vec<Line<'static>>),
}

impl PagerContent {
    /// Get raw content
    pub fn raw(&self) -> &str {
        match self {
            PagerContent::Plain(s) => s,
            PagerContent::Markdown(s) => s,
            PagerContent::Code { content, .. } => content,
            PagerContent::Styled(_) => "",
        }
    }

    /// Get line count
    pub fn line_count(&self) -> usize {
        match self {
            PagerContent::Plain(s) => s.lines().count(),
            PagerContent::Markdown(s) => s.lines().count(),
            PagerContent::Code { content, .. } => content.lines().count(),
            PagerContent::Styled(lines) => lines.len(),
        }
    }
}

/// Search state
#[derive(Debug, Clone)]
struct SearchState {
    /// Search query
    query: String,
    /// Current match index
    current_match: usize,
    /// All match positions (line, start_col, end_col)
    matches: Vec<(usize, usize, usize)>,
    /// Search direction
    forward: bool,
}

impl Default for SearchState {
    fn default() -> Self {
        Self {
            query: String::new(),
            current_match: 0,
            matches: Vec::new(),
            forward: true,
        }
    }
}

/// Pager overlay state
pub struct PagerOverlay {
    /// Content to display
    content: PagerContent,
    /// Title
    title: String,
    /// Scroll offset (line)
    scroll: usize,
    /// Horizontal scroll
    h_scroll: usize,
    /// Whether overlay is visible
    visible: bool,
    /// Show line numbers
    show_line_numbers: bool,
    /// Wrap lines
    wrap_lines: bool,
    /// Search state
    search: Option<SearchState>,
    /// Input mode (for search)
    search_mode: bool,
    /// Viewport height
    viewport_height: usize,
}

impl PagerOverlay {
    /// Create a new pager overlay
    pub fn new() -> Self {
        Self {
            content: PagerContent::Plain(String::new()),
            title: "Pager".to_string(),
            scroll: 0,
            h_scroll: 0,
            visible: false,
            show_line_numbers: true,
            wrap_lines: true,
            search: None,
            search_mode: false,
            viewport_height: 20,
        }
    }

    /// Set content
    pub fn content(mut self, content: PagerContent) -> Self {
        self.content = content;
        self
    }

    /// Set title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set line numbers visibility
    pub fn line_numbers(mut self, show: bool) -> Self {
        self.show_line_numbers = show;
        self
    }

    /// Set wrap mode
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap_lines = wrap;
        self
    }

    /// Show the pager with content
    pub fn show(&mut self, content: PagerContent) {
        self.content = content;
        self.scroll = 0;
        self.h_scroll = 0;
        self.visible = true;
        self.search = None;
        self.search_mode = false;
    }

    /// Show with plain text
    pub fn show_text(&mut self, text: impl Into<String>) {
        self.show(PagerContent::Plain(text.into()));
    }

    /// Show with markdown
    pub fn show_markdown(&mut self, text: impl Into<String>) {
        self.show(PagerContent::Markdown(text.into()));
    }

    /// Show with code
    pub fn show_code(&mut self, content: impl Into<String>, language: impl Into<String>) {
        self.show(PagerContent::Code {
            content: content.into(),
            language: language.into(),
        });
    }

    /// Hide the pager
    pub fn hide(&mut self) {
        self.visible = false;
        self.search_mode = false;
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Get current scroll position
    pub fn scroll_position(&self) -> usize {
        self.scroll
    }

    /// Handle key event
    pub fn handle_key(&mut self, key: KeyEvent) -> PagerAction {
        if !self.visible {
            return PagerAction::None;
        }

        // Handle search mode
        if self.search_mode {
            return self.handle_search_key(key);
        }

        match key.code {
            // Exit
            KeyCode::Esc | KeyCode::Char('q') => {
                self.hide();
                PagerAction::Close
            }

            // Vi-style navigation
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll_down(1);
                PagerAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll_up(1);
                PagerAction::None
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_down(self.viewport_height / 2);
                PagerAction::None
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_up(self.viewport_height / 2);
                PagerAction::None
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_down(self.viewport_height);
                PagerAction::None
            }
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_up(self.viewport_height);
                PagerAction::None
            }
            KeyCode::PageDown => {
                self.scroll_down(self.viewport_height);
                PagerAction::None
            }
            KeyCode::PageUp => {
                self.scroll_up(self.viewport_height);
                PagerAction::None
            }
            KeyCode::Char('g') | KeyCode::Home => {
                self.scroll = 0;
                PagerAction::None
            }
            KeyCode::Char('G') | KeyCode::End => {
                self.scroll_to_end();
                PagerAction::None
            }

            // Horizontal scroll
            KeyCode::Char('h') | KeyCode::Left => {
                self.h_scroll = self.h_scroll.saturating_sub(4);
                PagerAction::None
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.h_scroll += 4;
                PagerAction::None
            }

            // Search
            KeyCode::Char('/') => {
                self.search_mode = true;
                self.search = Some(SearchState::default());
                PagerAction::None
            }
            KeyCode::Char('?') => {
                self.search_mode = true;
                let mut search = SearchState::default();
                search.forward = false;
                self.search = Some(search);
                PagerAction::None
            }
            KeyCode::Char('n') => {
                self.next_match();
                PagerAction::None
            }
            KeyCode::Char('N') => {
                self.prev_match();
                PagerAction::None
            }

            // Toggle options
            KeyCode::Char('w') => {
                self.wrap_lines = !self.wrap_lines;
                PagerAction::None
            }
            KeyCode::Char('L') => {
                self.show_line_numbers = !self.show_line_numbers;
                PagerAction::None
            }

            // Copy (if supported)
            KeyCode::Char('y') => {
                let content = self.content.raw().to_string();
                PagerAction::Copy(content)
            }

            _ => PagerAction::None,
        }
    }

    /// Handle search mode key
    fn handle_search_key(&mut self, key: KeyEvent) -> PagerAction {
        let search = match &mut self.search {
            Some(s) => s,
            None => return PagerAction::None,
        };

        match key.code {
            KeyCode::Esc => {
                self.search_mode = false;
                PagerAction::None
            }
            KeyCode::Enter => {
                self.search_mode = false;
                self.perform_search();
                PagerAction::None
            }
            KeyCode::Char(c) => {
                search.query.push(c);
                PagerAction::None
            }
            KeyCode::Backspace => {
                search.query.pop();
                PagerAction::None
            }
            _ => PagerAction::None,
        }
    }

    /// Scroll down
    fn scroll_down(&mut self, lines: usize) {
        let max_scroll = self.content.line_count().saturating_sub(self.viewport_height);
        self.scroll = (self.scroll + lines).min(max_scroll);
    }

    /// Scroll up
    fn scroll_up(&mut self, lines: usize) {
        self.scroll = self.scroll.saturating_sub(lines);
    }

    /// Scroll to end
    fn scroll_to_end(&mut self) {
        let max_scroll = self.content.line_count().saturating_sub(self.viewport_height);
        self.scroll = max_scroll;
    }

    /// Perform search
    fn perform_search(&mut self) {
        let search = match &mut self.search {
            Some(s) => s,
            None => return,
        };

        if search.query.is_empty() {
            return;
        }

        let query = search.query.to_lowercase();
        let content = self.content.raw();

        search.matches.clear();

        for (line_idx, line) in content.lines().enumerate() {
            let line_lower = line.to_lowercase();
            let mut start = 0;
            while let Some(pos) = line_lower[start..].find(&query) {
                let absolute_pos = start + pos;
                search.matches.push((line_idx, absolute_pos, absolute_pos + query.len()));
                start = absolute_pos + 1;
            }
        }

        search.current_match = 0;

        // Jump to first match
        if let Some(&(line, _, _)) = search.matches.first() {
            self.scroll = line.saturating_sub(self.viewport_height / 2);
        }
    }

    /// Go to next match
    fn next_match(&mut self) {
        let search = match &mut self.search {
            Some(s) if !s.matches.is_empty() => s,
            _ => return,
        };

        search.current_match = (search.current_match + 1) % search.matches.len();
        let (line, _, _) = search.matches[search.current_match];
        self.scroll = line.saturating_sub(self.viewport_height / 2);
    }

    /// Go to previous match
    fn prev_match(&mut self) {
        let search = match &mut self.search {
            Some(s) if !s.matches.is_empty() => s,
            _ => return,
        };

        search.current_match = if search.current_match == 0 {
            search.matches.len() - 1
        } else {
            search.current_match - 1
        };
        let (line, _, _) = search.matches[search.current_match];
        self.scroll = line.saturating_sub(self.viewport_height / 2);
    }

    /// Render the pager
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Use full screen
        let popup_area = area;

        // Clear background
        frame.render_widget(Clear, popup_area);

        // Update viewport height
        self.viewport_height = popup_area.height.saturating_sub(4) as usize;

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
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);

        // Content
        self.render_content(frame, chunks[0]);

        // Status bar
        self.render_status(frame, chunks[1]);
    }

    /// Render content
    fn render_content(&self, frame: &mut Frame, area: Rect) {
        let line_num_width = if self.show_line_numbers {
            self.content.line_count().to_string().len() + 1
        } else {
            0
        };

        let content_width = area.width as usize - line_num_width - 2; // -2 for scrollbar

        let content_lines: Vec<&str> = match &self.content {
            PagerContent::Plain(s) | PagerContent::Markdown(s) => s.lines().collect(),
            PagerContent::Code { content, .. } => content.lines().collect(),
            PagerContent::Styled(_) => vec![],
        };

        let mut render_lines: Vec<Line> = Vec::new();
        let visible_end = (self.scroll + area.height as usize).min(content_lines.len());

        for i in self.scroll..visible_end {
            let mut spans = Vec::new();

            // Line number
            if self.show_line_numbers {
                spans.push(Span::styled(
                    format!("{:>width$} ", i + 1, width = line_num_width - 1),
                    Style::default().fg(Color::Rgb(86, 95, 137)),
                ));
            }

            let line = content_lines[i];

            // Check for search highlights
            if let Some(search) = &self.search {
                let mut highlighted = false;
                for &(match_line, start, end) in &search.matches {
                    if match_line == i {
                        // Highlight this match
                        let before = &line[..start.min(line.len())];
                        let matched = &line[start.min(line.len())..end.min(line.len())];
                        let after = &line[end.min(line.len())..];

                        spans.push(Span::raw(before));
                        spans.push(Span::styled(
                            matched,
                            Style::default().bg(Color::Yellow).fg(Color::Black),
                        ));
                        spans.push(Span::raw(after));
                        highlighted = true;
                        break;
                    }
                }
                if !highlighted {
                    // Apply horizontal scroll
                    let display_line = if self.h_scroll < line.len() {
                        &line[self.h_scroll..]
                    } else {
                        ""
                    };
                    spans.push(Span::raw(display_line));
                }
            } else {
                // Apply horizontal scroll
                let display_line = if self.h_scroll < line.len() {
                    &line[self.h_scroll..]
                } else {
                    ""
                };
                spans.push(Span::raw(display_line));
            }

            render_lines.push(Line::from(spans));
        }

        let paragraph = if self.wrap_lines {
            Paragraph::new(render_lines).wrap(Wrap { trim: false })
        } else {
            Paragraph::new(render_lines)
        };

        // Calculate scrollbar area
        let content_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width - 1,
            height: area.height,
        };

        frame.render_widget(paragraph, content_area);

        // Scrollbar
        let scrollbar_area = Rect {
            x: area.x + area.width - 1,
            y: area.y,
            width: 1,
            height: area.height,
        };

        let mut scrollbar_state = ScrollbarState::default()
            .content_length(self.content.line_count())
            .viewport_content_length(area.height as usize)
            .position(self.scroll);

        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .style(Style::default().fg(Color::Rgb(86, 95, 137)));

        frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }

    /// Render status bar
    fn render_status(&self, frame: &mut Frame, area: Rect) {
        let mut spans = vec![];

        // Search mode
        if self.search_mode {
            let query = self
                .search
                .as_ref()
                .map(|s| s.query.as_str())
                .unwrap_or("");
            let direction = self
                .search
                .as_ref()
                .map(|s| if s.forward { "/" } else { "?" })
                .unwrap_or("/");

            spans.push(Span::styled(
                format!("{}{}", direction, query),
                Style::default().fg(Color::Yellow),
            ));
            spans.push(Span::styled("█", Style::default().fg(Color::Cyan)));
        } else {
            // Position info
            let line_count = self.content.line_count();
            let percent = if line_count > 0 {
                ((self.scroll + self.viewport_height) * 100 / line_count).min(100)
            } else {
                100
            };

            spans.push(Span::styled(
                format!(" {}% ", percent),
                Style::default().fg(Color::Cyan),
            ));

            spans.push(Span::styled(
                format!(" {}/{} ", self.scroll + 1, line_count),
                Style::default().fg(Color::Rgb(86, 95, 137)),
            ));

            // Search results
            if let Some(search) = &self.search {
                if !search.matches.is_empty() {
                    spans.push(Span::styled(
                        format!(" Match {}/{} ", search.current_match + 1, search.matches.len()),
                        Style::default().fg(Color::Yellow),
                    ));
                }
            }

            // Help hint
            spans.push(Span::styled(
                " q:quit  /:search  hjkl:nav ",
                Style::default().fg(Color::Rgb(86, 95, 137)),
            ));
        }

        let status = Paragraph::new(Line::from(spans)).alignment(Alignment::Left);
        frame.render_widget(status, area);
    }
}

impl Default for PagerOverlay {
    fn default() -> Self {
        Self::new()
    }
}

/// Action returned by pager
#[derive(Debug, Clone, PartialEq)]
pub enum PagerAction {
    /// No action
    None,
    /// Close pager
    Close,
    /// Copy content to clipboard
    Copy(String),
}

/// Helper to create pager with common configurations
pub struct PagerBuilder {
    pager: PagerOverlay,
}

impl PagerBuilder {
    /// Create builder for help text
    pub fn help(content: impl Into<String>) -> Self {
        Self {
            pager: PagerOverlay::new()
                .title("Help")
                .content(PagerContent::Plain(content.into()))
                .line_numbers(false)
                .wrap(true),
        }
    }

    /// Create builder for log viewer
    pub fn log(content: impl Into<String>) -> Self {
        Self {
            pager: PagerOverlay::new()
                .title("Log Viewer")
                .content(PagerContent::Plain(content.into()))
                .line_numbers(true)
                .wrap(false),
        }
    }

    /// Create builder for code viewer
    pub fn code(content: impl Into<String>, language: impl Into<String>) -> Self {
        Self {
            pager: PagerOverlay::new()
                .title("Code")
                .content(PagerContent::Code {
                    content: content.into(),
                    language: language.into(),
                })
                .line_numbers(true)
                .wrap(false),
        }
    }

    /// Set title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.pager.title = title.into();
        self
    }

    /// Build the pager
    pub fn build(self) -> PagerOverlay {
        self.pager
    }
}

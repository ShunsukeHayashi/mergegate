//! UI - Common UI utilities and widgets
//!
//! Shared UI components and utilities for the TUI.

pub mod theme;
pub mod widgets;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Tokyo Night color palette
pub mod colors {
    use ratatui::style::Color;

    // Background colors
    pub const BG: Color = Color::Rgb(26, 27, 38);
    pub const BG_DARK: Color = Color::Rgb(22, 23, 32);
    pub const BG_HIGHLIGHT: Color = Color::Rgb(41, 46, 66);

    // Foreground colors
    pub const FG: Color = Color::Rgb(192, 202, 245);
    pub const FG_DARK: Color = Color::Rgb(169, 177, 214);
    pub const FG_GUTTER: Color = Color::Rgb(86, 95, 137);

    // Syntax colors
    pub const BLUE: Color = Color::Rgb(122, 162, 247);
    pub const CYAN: Color = Color::Rgb(125, 207, 255);
    pub const MAGENTA: Color = Color::Rgb(187, 154, 247);
    pub const ORANGE: Color = Color::Rgb(255, 158, 100);
    pub const YELLOW: Color = Color::Rgb(224, 175, 104);
    pub const GREEN: Color = Color::Rgb(158, 206, 106);
    pub const RED: Color = Color::Rgb(247, 118, 142);
    pub const TEAL: Color = Color::Rgb(26, 188, 156);

    // UI colors
    pub const BORDER: Color = Color::Rgb(86, 95, 137);
    pub const BORDER_FOCUS: Color = Color::Rgb(122, 162, 247);
    pub const SELECTION: Color = Color::Rgb(51, 59, 91);
    pub const COMMENT: Color = Color::Rgb(86, 95, 137);
}

/// Common UI styles
pub mod styles {
    use super::colors;
    use ratatui::style::{Modifier, Style};

    /// Default text style
    pub fn default() -> Style {
        Style::default().fg(colors::FG)
    }

    /// Bold text
    pub fn bold() -> Style {
        Style::default().fg(colors::FG).add_modifier(Modifier::BOLD)
    }

    /// Dim/muted text
    pub fn dim() -> Style {
        Style::default().fg(colors::FG_GUTTER)
    }

    /// Link/clickable style
    pub fn link() -> Style {
        Style::default()
            .fg(colors::BLUE)
            .add_modifier(Modifier::UNDERLINED)
    }

    /// Error style
    pub fn error() -> Style {
        Style::default().fg(colors::RED)
    }

    /// Warning style
    pub fn warning() -> Style {
        Style::default().fg(colors::YELLOW)
    }

    /// Success style
    pub fn success() -> Style {
        Style::default().fg(colors::GREEN)
    }

    /// Info style
    pub fn info() -> Style {
        Style::default().fg(colors::CYAN)
    }

    /// Keyword/highlight
    pub fn keyword() -> Style {
        Style::default().fg(colors::MAGENTA)
    }

    /// String literal
    pub fn string() -> Style {
        Style::default().fg(colors::GREEN)
    }

    /// Number
    pub fn number() -> Style {
        Style::default().fg(colors::ORANGE)
    }

    /// Comment
    pub fn comment() -> Style {
        Style::default().fg(colors::COMMENT)
    }

    /// Selection highlight
    pub fn selection() -> Style {
        Style::default().bg(colors::SELECTION)
    }
}

/// Layout helpers
pub mod layout {
    use ratatui::layout::{Constraint, Direction, Layout, Rect};

    /// Create a centered rect with percentage width/height
    pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(area);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(vertical[1])[1]
    }

    /// Create a centered rect with fixed width/height
    pub fn centered_fixed(width: u16, height: u16, area: Rect) -> Rect {
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        Rect {
            x,
            y,
            width: width.min(area.width),
            height: height.min(area.height),
        }
    }

    /// Split area horizontally with weights
    pub fn split_horizontal(area: Rect, weights: &[u16]) -> Vec<Rect> {
        let total: u16 = weights.iter().sum();
        let constraints: Vec<Constraint> = weights
            .iter()
            .map(|w| Constraint::Percentage(*w * 100 / total))
            .collect();

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(area)
            .to_vec()
    }

    /// Split area vertically with weights
    pub fn split_vertical(area: Rect, weights: &[u16]) -> Vec<Rect> {
        let total: u16 = weights.iter().sum();
        let constraints: Vec<Constraint> = weights
            .iter()
            .map(|w| Constraint::Percentage(*w * 100 / total))
            .collect();

        Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area)
            .to_vec()
    }

    /// Create a margin around a rect
    pub fn with_margin(area: Rect, margin: u16) -> Rect {
        Rect {
            x: area.x + margin,
            y: area.y + margin,
            width: area.width.saturating_sub(margin * 2),
            height: area.height.saturating_sub(margin * 2),
        }
    }

    /// Create a bottom dock area
    pub fn bottom_dock(area: Rect, height: u16) -> (Rect, Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(height)])
            .split(area);
        (chunks[0], chunks[1])
    }

    /// Create a top header area
    pub fn top_header(area: Rect, height: u16) -> (Rect, Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(height), Constraint::Min(1)])
            .split(area);
        (chunks[0], chunks[1])
    }

    /// Create a sidebar layout
    pub fn sidebar_layout(area: Rect, sidebar_width: u16, left: bool) -> (Rect, Rect) {
        let chunks = if left {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(sidebar_width), Constraint::Min(1)])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(1), Constraint::Length(sidebar_width)])
                .split(area)
        };
        (chunks[0], chunks[1])
    }
}

/// Modal dialog widget
pub struct Modal {
    /// Title
    title: String,
    /// Content lines
    content: Vec<Line<'static>>,
    /// Buttons
    buttons: Vec<String>,
    /// Selected button
    selected: usize,
    /// Width percentage
    width: u16,
}

impl Modal {
    /// Create a new modal
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            content: Vec::new(),
            buttons: vec!["OK".to_string()],
            selected: 0,
            width: 60,
        }
    }

    /// Set content
    pub fn content(mut self, content: Vec<Line<'static>>) -> Self {
        self.content = content;
        self
    }

    /// Set content from string
    pub fn content_str(mut self, content: impl Into<String>) -> Self {
        self.content = content
            .into()
            .lines()
            .map(|s| Line::from(s.to_string()))
            .collect();
        self
    }

    /// Set buttons
    pub fn buttons(mut self, buttons: Vec<String>) -> Self {
        self.buttons = buttons;
        self
    }

    /// Set width
    pub fn width(mut self, width: u16) -> Self {
        self.width = width;
        self
    }

    /// Get selected button index
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Select next button
    pub fn next(&mut self) {
        self.selected = (self.selected + 1) % self.buttons.len();
    }

    /// Select previous button
    pub fn prev(&mut self) {
        self.selected = if self.selected == 0 {
            self.buttons.len() - 1
        } else {
            self.selected - 1
        };
    }

    /// Render the modal
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let height = (self.content.len() + 6).min(area.height as usize) as u16;
        let popup = layout::centered_fixed(
            (area.width * self.width / 100).min(area.width),
            height,
            area,
        );

        frame.render_widget(Clear, popup);

        let block = Block::default()
            .title(Span::styled(
                format!(" {} ", self.title),
                Style::default()
                    .fg(colors::CYAN)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(colors::BORDER_FOCUS));

        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        // Content and buttons
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(inner);

        // Content
        let content = Paragraph::new(self.content.clone()).wrap(Wrap { trim: false });
        frame.render_widget(content, chunks[0]);

        // Buttons
        let mut button_spans = vec![];
        for (i, btn) in self.buttons.iter().enumerate() {
            if i > 0 {
                button_spans.push(Span::raw("  "));
            }
            let style = if i == self.selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(colors::CYAN)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors::FG)
            };
            button_spans.push(Span::styled(format!(" {} ", btn), style));
        }

        let buttons = Paragraph::new(Line::from(button_spans)).alignment(Alignment::Center);
        frame.render_widget(buttons, chunks[1]);
    }
}

/// Notification toast
pub struct Toast {
    /// Message
    message: String,
    /// Toast type
    toast_type: ToastType,
    /// Duration in ms
    duration_ms: u64,
    /// Start time
    created_at: std::time::Instant,
}

/// Toast types
#[derive(Debug, Clone, Copy)]
pub enum ToastType {
    Info,
    Success,
    Warning,
    Error,
}

impl Toast {
    /// Create a new toast
    pub fn new(message: impl Into<String>, toast_type: ToastType) -> Self {
        Self {
            message: message.into(),
            toast_type,
            duration_ms: 3000,
            created_at: std::time::Instant::now(),
        }
    }

    /// Create info toast
    pub fn info(message: impl Into<String>) -> Self {
        Self::new(message, ToastType::Info)
    }

    /// Create success toast
    pub fn success(message: impl Into<String>) -> Self {
        Self::new(message, ToastType::Success)
    }

    /// Create warning toast
    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(message, ToastType::Warning)
    }

    /// Create error toast
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(message, ToastType::Error)
    }

    /// Set duration
    pub fn duration(mut self, ms: u64) -> Self {
        self.duration_ms = ms;
        self
    }

    /// Check if toast is expired
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed().as_millis() as u64 >= self.duration_ms
    }

    /// Render the toast
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let (icon, color) = match self.toast_type {
            ToastType::Info => ("ℹ", colors::CYAN),
            ToastType::Success => ("✓", colors::GREEN),
            ToastType::Warning => ("⚠", colors::YELLOW),
            ToastType::Error => ("✗", colors::RED),
        };

        let width = (self.message.len() + 6).min(area.width as usize) as u16;
        let toast_area = Rect {
            x: area.x + area.width - width - 1,
            y: area.y + 1,
            width,
            height: 3,
        };

        frame.render_widget(Clear, toast_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(color));

        let inner = block.inner(toast_area);
        frame.render_widget(block, toast_area);

        let content = Line::from(vec![
            Span::styled(format!("{} ", icon), Style::default().fg(color)),
            Span::styled(&self.message, Style::default().fg(colors::FG)),
        ]);

        let paragraph = Paragraph::new(content);
        frame.render_widget(paragraph, inner);
    }
}

/// Toast manager
pub struct ToastManager {
    toasts: Vec<Toast>,
    max_toasts: usize,
}

impl ToastManager {
    /// Create a new toast manager
    pub fn new() -> Self {
        Self {
            toasts: Vec::new(),
            max_toasts: 5,
        }
    }

    /// Add a toast
    pub fn add(&mut self, toast: Toast) {
        self.toasts.push(toast);
        if self.toasts.len() > self.max_toasts {
            self.toasts.remove(0);
        }
    }

    /// Remove expired toasts
    pub fn cleanup(&mut self) {
        self.toasts.retain(|t| !t.is_expired());
    }

    /// Render all toasts
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.cleanup();

        for (i, toast) in self.toasts.iter().enumerate() {
            let toast_area = Rect {
                y: area.y + (i as u16 * 4),
                ..area
            };
            toast.render(frame, toast_area);
        }
    }
}

impl Default for ToastManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Breadcrumb navigation
pub struct Breadcrumb {
    /// Segments
    segments: Vec<String>,
    /// Separator
    separator: String,
}

impl Breadcrumb {
    /// Create a new breadcrumb
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
            separator: " › ".to_string(),
        }
    }

    /// Add segment
    pub fn push(mut self, segment: impl Into<String>) -> Self {
        self.segments.push(segment.into());
        self
    }

    /// Set separator
    pub fn separator(mut self, sep: impl Into<String>) -> Self {
        self.separator = sep.into();
        self
    }

    /// Render the breadcrumb
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let mut spans = vec![];

        for (i, segment) in self.segments.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(&self.separator, styles::dim()));
            }

            let style = if i == self.segments.len() - 1 {
                styles::bold()
            } else {
                styles::dim()
            };

            spans.push(Span::styled(segment, style));
        }

        let breadcrumb = Paragraph::new(Line::from(spans));
        frame.render_widget(breadcrumb, area);
    }
}

impl Default for Breadcrumb {
    fn default() -> Self {
        Self::new()
    }
}

/// Status bar
pub struct StatusBar {
    /// Left items
    left: Vec<StatusItem>,
    /// Center items
    center: Vec<StatusItem>,
    /// Right items
    right: Vec<StatusItem>,
}

/// Status bar item
#[derive(Clone)]
pub struct StatusItem {
    /// Content
    content: String,
    /// Style
    style: Style,
}

impl StatusItem {
    /// Create a new status item
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            style: styles::dim(),
        }
    }

    /// Set style
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl StatusBar {
    /// Create a new status bar
    pub fn new() -> Self {
        Self {
            left: Vec::new(),
            center: Vec::new(),
            right: Vec::new(),
        }
    }

    /// Add left item
    pub fn left(mut self, item: StatusItem) -> Self {
        self.left.push(item);
        self
    }

    /// Add center item
    pub fn center(mut self, item: StatusItem) -> Self {
        self.center.push(item);
        self
    }

    /// Add right item
    pub fn right(mut self, item: StatusItem) -> Self {
        self.right.push(item);
        self
    }

    /// Render the status bar
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Left
        let left_spans: Vec<Span> = self
            .left
            .iter()
            .flat_map(|item| vec![Span::styled(&item.content, item.style), Span::raw(" ")])
            .collect();

        // Center
        let center_spans: Vec<Span> = self
            .center
            .iter()
            .flat_map(|item| vec![Span::styled(&item.content, item.style), Span::raw(" ")])
            .collect();

        // Right
        let right_spans: Vec<Span> = self
            .right
            .iter()
            .flat_map(|item| vec![Span::raw(" "), Span::styled(&item.content, item.style)])
            .collect();

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(area);

        // Render left
        let left = Paragraph::new(Line::from(left_spans));
        frame.render_widget(left, chunks[0]);

        // Render center
        let center = Paragraph::new(Line::from(center_spans)).alignment(Alignment::Center);
        frame.render_widget(center, chunks[1]);

        // Render right
        let right = Paragraph::new(Line::from(right_spans)).alignment(Alignment::Right);
        frame.render_widget(right, chunks[2]);
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

/// Badge widget
pub struct Badge {
    /// Label
    label: String,
    /// Background color
    bg: Color,
    /// Foreground color
    fg: Color,
}

impl Badge {
    /// Create a new badge
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            bg: colors::CYAN,
            fg: Color::Black,
        }
    }

    /// Set colors
    pub fn colors(mut self, bg: Color, fg: Color) -> Self {
        self.bg = bg;
        self.fg = fg;
        self
    }

    /// Create success badge
    pub fn success(label: impl Into<String>) -> Self {
        Self::new(label).colors(colors::GREEN, Color::Black)
    }

    /// Create warning badge
    pub fn warning(label: impl Into<String>) -> Self {
        Self::new(label).colors(colors::YELLOW, Color::Black)
    }

    /// Create error badge
    pub fn error(label: impl Into<String>) -> Self {
        Self::new(label).colors(colors::RED, Color::Black)
    }

    /// Get as span
    pub fn to_span(&self) -> Span<'static> {
        Span::styled(
            format!(" {} ", self.label.clone()),
            Style::default().fg(self.fg).bg(self.bg),
        )
    }
}

/// Divider widget
pub struct Divider {
    /// Character
    char: char,
    /// Style
    style: Style,
    /// Label (optional)
    label: Option<String>,
}

impl Divider {
    /// Create a new divider
    pub fn new() -> Self {
        Self {
            char: '─',
            style: styles::dim(),
            label: None,
        }
    }

    /// Set character
    pub fn char(mut self, char: char) -> Self {
        self.char = char;
        self
    }

    /// Set style
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Set label
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Render the divider
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if let Some(label) = &self.label {
            // Divider with label
            let label_len = label.len() + 2;
            let left_len = (area.width as usize - label_len) / 2;
            let right_len = area.width as usize - label_len - left_len;

            let line = Line::from(vec![
                Span::styled(self.char.to_string().repeat(left_len), self.style),
                Span::styled(format!(" {} ", label), styles::dim()),
                Span::styled(self.char.to_string().repeat(right_len), self.style),
            ]);

            let paragraph = Paragraph::new(line);
            frame.render_widget(paragraph, area);
        } else {
            // Simple divider
            let line = Line::from(Span::styled(
                self.char.to_string().repeat(area.width as usize),
                self.style,
            ));

            let paragraph = Paragraph::new(line);
            frame.render_widget(paragraph, area);
        }
    }
}

impl Default for Divider {
    fn default() -> Self {
        Self::new()
    }
}

/// Empty state widget
pub struct EmptyState {
    /// Icon
    icon: String,
    /// Title
    title: String,
    /// Description
    description: String,
}

impl EmptyState {
    /// Create a new empty state
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            icon: "📭".to_string(),
            title: title.into(),
            description: String::new(),
        }
    }

    /// Set icon
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = icon.into();
        self
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Render the empty state
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let mut lines = vec![
            Line::from(Span::styled(
                &self.icon,
                Style::default().fg(colors::FG_GUTTER),
            )),
            Line::from(""),
            Line::from(Span::styled(&self.title, styles::bold())),
        ];

        if !self.description.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(&self.description, styles::dim())));
        }

        // Calculate dimensions before moving lines
        let line_count = lines.len() as u16;
        let y_offset = (area.height.saturating_sub(line_count)) / 2;
        let centered = Rect {
            y: area.y + y_offset,
            height: line_count,
            ..area
        };

        let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
        frame.render_widget(paragraph, centered);
    }
}

/// Key hint widget
pub struct KeyHint {
    /// Key
    key: String,
    /// Description
    description: String,
}

impl KeyHint {
    /// Create a new key hint
    pub fn new(key: impl Into<String>, desc: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            description: desc.into(),
        }
    }

    /// Get as spans
    pub fn to_spans(&self) -> Vec<Span<'static>> {
        vec![
            Span::styled(
                format!(" {} ", self.key.clone()),
                Style::default().fg(Color::Black).bg(colors::FG_GUTTER),
            ),
            Span::raw(" "),
            Span::styled(self.description.clone(), styles::dim()),
        ]
    }
}

/// Key hints bar
pub struct KeyHints {
    hints: Vec<KeyHint>,
}

impl KeyHints {
    /// Create new key hints
    pub fn new() -> Self {
        Self { hints: Vec::new() }
    }

    /// Add hint
    pub fn hint(mut self, key: impl Into<String>, desc: impl Into<String>) -> Self {
        self.hints.push(KeyHint::new(key, desc));
        self
    }

    /// Render the key hints
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let mut spans: Vec<Span> = vec![];

        for (i, hint) in self.hints.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw("  "));
            }
            spans.extend(hint.to_spans());
        }

        let line = Paragraph::new(Line::from(spans)).alignment(Alignment::Center);
        frame.render_widget(line, area);
    }
}

impl Default for KeyHints {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Colors tests
    #[test]
    fn test_colors_defined() {
        // Test that colors are properly defined RGB values
        assert!(matches!(colors::BG, Color::Rgb(26, 27, 38)));
        assert!(matches!(colors::FG, Color::Rgb(192, 202, 245)));
        assert!(matches!(colors::BLUE, Color::Rgb(122, 162, 247)));
        assert!(matches!(colors::RED, Color::Rgb(247, 118, 142)));
        assert!(matches!(colors::GREEN, Color::Rgb(158, 206, 106)));
    }

    // Styles tests
    #[test]
    fn test_styles_default() {
        let style = styles::default();
        assert_eq!(style.fg, Some(colors::FG));
    }

    #[test]
    fn test_styles_bold() {
        let style = styles::bold();
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_styles_dim() {
        let style = styles::dim();
        assert_eq!(style.fg, Some(colors::FG_GUTTER));
    }

    #[test]
    fn test_styles_link() {
        let style = styles::link();
        assert_eq!(style.fg, Some(colors::BLUE));
        assert!(style.add_modifier.contains(Modifier::UNDERLINED));
    }

    #[test]
    fn test_styles_error() {
        let style = styles::error();
        assert_eq!(style.fg, Some(colors::RED));
    }

    #[test]
    fn test_styles_warning() {
        let style = styles::warning();
        assert_eq!(style.fg, Some(colors::YELLOW));
    }

    #[test]
    fn test_styles_success() {
        let style = styles::success();
        assert_eq!(style.fg, Some(colors::GREEN));
    }

    #[test]
    fn test_styles_info() {
        let style = styles::info();
        assert_eq!(style.fg, Some(colors::CYAN));
    }

    #[test]
    fn test_styles_keyword() {
        let style = styles::keyword();
        assert_eq!(style.fg, Some(colors::MAGENTA));
    }

    #[test]
    fn test_styles_string() {
        let style = styles::string();
        assert_eq!(style.fg, Some(colors::GREEN));
    }

    #[test]
    fn test_styles_number() {
        let style = styles::number();
        assert_eq!(style.fg, Some(colors::ORANGE));
    }

    #[test]
    fn test_styles_comment() {
        let style = styles::comment();
        assert_eq!(style.fg, Some(colors::COMMENT));
    }

    #[test]
    fn test_styles_selection() {
        let style = styles::selection();
        assert_eq!(style.bg, Some(colors::SELECTION));
    }

    // Layout tests
    #[test]
    fn test_layout_centered_rect() {
        let area = Rect::new(0, 0, 100, 100);
        let centered = layout::centered_rect(50, 50, area);

        // Should be centered with 50% width and height
        assert!(centered.width <= 50);
        assert!(centered.height <= 50);
        assert!(centered.x >= 20);
        assert!(centered.y >= 20);
    }

    #[test]
    fn test_layout_centered_fixed() {
        let area = Rect::new(0, 0, 100, 100);
        let centered = layout::centered_fixed(40, 20, area);

        assert_eq!(centered.width, 40);
        assert_eq!(centered.height, 20);
        assert_eq!(centered.x, 30);
        assert_eq!(centered.y, 40);
    }

    #[test]
    fn test_layout_centered_fixed_larger_than_area() {
        let area = Rect::new(0, 0, 50, 30);
        let centered = layout::centered_fixed(100, 50, area);

        // Should be clamped to area size
        assert_eq!(centered.width, 50);
        assert_eq!(centered.height, 30);
    }

    #[test]
    fn test_layout_split_horizontal() {
        let area = Rect::new(0, 0, 100, 50);
        let splits = layout::split_horizontal(area, &[1, 1, 1]);

        assert_eq!(splits.len(), 3);
        // Each should get roughly 1/3 of the width
        for split in &splits {
            assert!(split.width >= 30 && split.width <= 34);
        }
    }

    #[test]
    fn test_layout_split_vertical() {
        let area = Rect::new(0, 0, 50, 100);
        let splits = layout::split_vertical(area, &[1, 2]);

        assert_eq!(splits.len(), 2);
        // First should be ~33%, second ~66%
        assert!(splits[0].height < splits[1].height);
    }

    #[test]
    fn test_layout_with_margin() {
        let area = Rect::new(10, 20, 100, 80);
        let margined = layout::with_margin(area, 5);

        assert_eq!(margined.x, 15);
        assert_eq!(margined.y, 25);
        assert_eq!(margined.width, 90);
        assert_eq!(margined.height, 70);
    }

    #[test]
    fn test_layout_bottom_dock() {
        let area = Rect::new(0, 0, 100, 100);
        let (main, dock) = layout::bottom_dock(area, 10);

        assert_eq!(dock.height, 10);
        assert_eq!(main.height, 90);
        assert_eq!(dock.y, 90);
    }

    #[test]
    fn test_layout_top_header() {
        let area = Rect::new(0, 0, 100, 100);
        let (header, main) = layout::top_header(area, 10);

        assert_eq!(header.height, 10);
        assert_eq!(main.height, 90);
        assert_eq!(main.y, 10);
    }

    #[test]
    fn test_layout_sidebar_left() {
        let area = Rect::new(0, 0, 100, 100);
        let (sidebar, main) = layout::sidebar_layout(area, 30, true);

        assert_eq!(sidebar.width, 30);
        assert_eq!(main.width, 70);
        assert_eq!(sidebar.x, 0);
    }

    #[test]
    fn test_layout_sidebar_right() {
        let area = Rect::new(0, 0, 100, 100);
        let (main, sidebar) = layout::sidebar_layout(area, 30, false);

        assert_eq!(main.width, 70);
        assert_eq!(sidebar.width, 30);
    }

    // Modal tests
    #[test]
    fn test_modal_creation() {
        let modal = Modal::new("Test Modal");
        assert_eq!(modal.title, "Test Modal");
        assert_eq!(modal.buttons.len(), 1);
        assert_eq!(modal.buttons[0], "OK");
    }

    #[test]
    fn test_modal_content() {
        let modal = Modal::new("Test").content(vec![Line::from("Hello")]);
        assert_eq!(modal.content.len(), 1);
    }

    #[test]
    fn test_modal_content_str() {
        let modal = Modal::new("Test").content_str("Line 1\nLine 2\nLine 3");
        assert_eq!(modal.content.len(), 3);
    }

    #[test]
    fn test_modal_buttons() {
        let modal = Modal::new("Test").buttons(vec!["Yes".to_string(), "No".to_string()]);
        assert_eq!(modal.buttons.len(), 2);
    }

    #[test]
    fn test_modal_width() {
        let modal = Modal::new("Test").width(80);
        assert_eq!(modal.width, 80);
    }

    #[test]
    fn test_modal_navigation() {
        let mut modal =
            Modal::new("Test").buttons(vec!["A".to_string(), "B".to_string(), "C".to_string()]);

        assert_eq!(modal.selected(), 0);
        modal.next();
        assert_eq!(modal.selected(), 1);
        modal.next();
        assert_eq!(modal.selected(), 2);
        modal.next();
        assert_eq!(modal.selected(), 0); // Wrap
        modal.prev();
        assert_eq!(modal.selected(), 2); // Wrap back
    }

    // Toast tests
    #[test]
    fn test_toast_creation() {
        let toast = Toast::new("Test message", ToastType::Info);
        assert_eq!(toast.message, "Test message");
        assert!(matches!(toast.toast_type, ToastType::Info));
    }

    #[test]
    fn test_toast_info() {
        let toast = Toast::info("Info message");
        assert!(matches!(toast.toast_type, ToastType::Info));
    }

    #[test]
    fn test_toast_success() {
        let toast = Toast::success("Success message");
        assert!(matches!(toast.toast_type, ToastType::Success));
    }

    #[test]
    fn test_toast_warning() {
        let toast = Toast::warning("Warning message");
        assert!(matches!(toast.toast_type, ToastType::Warning));
    }

    #[test]
    fn test_toast_error() {
        let toast = Toast::error("Error message");
        assert!(matches!(toast.toast_type, ToastType::Error));
    }

    #[test]
    fn test_toast_duration() {
        let toast = Toast::info("Test").duration(5000);
        assert_eq!(toast.duration_ms, 5000);
    }

    #[test]
    fn test_toast_is_expired() {
        let toast = Toast::info("Test").duration(0);
        // With 0 duration, should be immediately expired
        std::thread::sleep(std::time::Duration::from_millis(1));
        assert!(toast.is_expired());
    }

    #[test]
    fn test_toast_not_expired() {
        let toast = Toast::info("Test").duration(10000);
        assert!(!toast.is_expired());
    }

    // ToastManager tests
    #[test]
    fn test_toast_manager_creation() {
        let manager = ToastManager::new();
        assert_eq!(manager.toasts.len(), 0);
    }

    #[test]
    fn test_toast_manager_add() {
        let mut manager = ToastManager::new();
        manager.add(Toast::info("Test"));
        assert_eq!(manager.toasts.len(), 1);
    }

    #[test]
    fn test_toast_manager_max_toasts() {
        let mut manager = ToastManager::new();
        for i in 0..10 {
            manager.add(Toast::info(format!("Toast {}", i)));
        }
        // Max toasts is 5
        assert_eq!(manager.toasts.len(), 5);
    }

    #[test]
    fn test_toast_manager_cleanup() {
        let mut manager = ToastManager::new();
        manager.add(Toast::info("Test").duration(0));
        std::thread::sleep(std::time::Duration::from_millis(1));
        manager.cleanup();
        assert_eq!(manager.toasts.len(), 0);
    }

    #[test]
    fn test_toast_manager_default() {
        let manager = ToastManager::default();
        assert_eq!(manager.toasts.len(), 0);
    }

    // Breadcrumb tests
    #[test]
    fn test_breadcrumb_creation() {
        let bc = Breadcrumb::new();
        assert_eq!(bc.segments.len(), 0);
        assert_eq!(bc.separator, " › ");
    }

    #[test]
    fn test_breadcrumb_push() {
        let bc = Breadcrumb::new().push("Home").push("Settings");
        assert_eq!(bc.segments.len(), 2);
        assert_eq!(bc.segments[0], "Home");
        assert_eq!(bc.segments[1], "Settings");
    }

    #[test]
    fn test_breadcrumb_separator() {
        let bc = Breadcrumb::new().separator(" / ");
        assert_eq!(bc.separator, " / ");
    }

    #[test]
    fn test_breadcrumb_default() {
        let bc = Breadcrumb::default();
        assert_eq!(bc.segments.len(), 0);
    }

    // StatusItem tests
    #[test]
    fn test_status_item_creation() {
        let item = StatusItem::new("Status");
        assert_eq!(item.content, "Status");
    }

    #[test]
    fn test_status_item_style() {
        let style = Style::default().fg(colors::RED);
        let item = StatusItem::new("Error").style(style);
        assert_eq!(item.style.fg, Some(colors::RED));
    }

    // StatusBar tests
    #[test]
    fn test_status_bar_creation() {
        let bar = StatusBar::new();
        assert_eq!(bar.left.len(), 0);
        assert_eq!(bar.center.len(), 0);
        assert_eq!(bar.right.len(), 0);
    }

    #[test]
    fn test_status_bar_left() {
        let bar = StatusBar::new().left(StatusItem::new("Left"));
        assert_eq!(bar.left.len(), 1);
    }

    #[test]
    fn test_status_bar_center() {
        let bar = StatusBar::new().center(StatusItem::new("Center"));
        assert_eq!(bar.center.len(), 1);
    }

    #[test]
    fn test_status_bar_right() {
        let bar = StatusBar::new().right(StatusItem::new("Right"));
        assert_eq!(bar.right.len(), 1);
    }

    #[test]
    fn test_status_bar_default() {
        let bar = StatusBar::default();
        assert_eq!(bar.left.len(), 0);
    }

    // Badge tests
    #[test]
    fn test_badge_creation() {
        let badge = Badge::new("NEW");
        assert_eq!(badge.label, "NEW");
        assert_eq!(badge.bg, colors::CYAN);
        assert_eq!(badge.fg, Color::Black);
    }

    #[test]
    fn test_badge_colors() {
        let badge = Badge::new("Test").colors(colors::RED, Color::White);
        assert_eq!(badge.bg, colors::RED);
        assert_eq!(badge.fg, Color::White);
    }

    #[test]
    fn test_badge_success() {
        let badge = Badge::success("OK");
        assert_eq!(badge.bg, colors::GREEN);
    }

    #[test]
    fn test_badge_warning() {
        let badge = Badge::warning("WARN");
        assert_eq!(badge.bg, colors::YELLOW);
    }

    #[test]
    fn test_badge_error() {
        let badge = Badge::error("ERR");
        assert_eq!(badge.bg, colors::RED);
    }

    #[test]
    fn test_badge_to_span() {
        let badge = Badge::new("TEST");
        let span = badge.to_span();
        assert!(!span.content.is_empty());
    }

    // Divider tests
    #[test]
    fn test_divider_creation() {
        let div = Divider::new();
        assert_eq!(div.char, '─');
        assert!(div.label.is_none());
    }

    #[test]
    fn test_divider_char() {
        let div = Divider::new().char('=');
        assert_eq!(div.char, '=');
    }

    #[test]
    fn test_divider_label() {
        let div = Divider::new().label("Section");
        assert_eq!(div.label, Some("Section".to_string()));
    }

    #[test]
    fn test_divider_style() {
        let style = Style::default().fg(colors::RED);
        let div = Divider::new().style(style);
        assert_eq!(div.style.fg, Some(colors::RED));
    }

    #[test]
    fn test_divider_default() {
        let div = Divider::default();
        assert_eq!(div.char, '─');
    }

    // EmptyState tests
    #[test]
    fn test_empty_state_creation() {
        let empty = EmptyState::new("No items");
        assert_eq!(empty.title, "No items");
        assert_eq!(empty.icon, "📭");
        assert!(empty.description.is_empty());
    }

    #[test]
    fn test_empty_state_icon() {
        let empty = EmptyState::new("Empty").icon("🔍");
        assert_eq!(empty.icon, "🔍");
    }

    #[test]
    fn test_empty_state_description() {
        let empty = EmptyState::new("Empty").description("Try adding some items");
        assert_eq!(empty.description, "Try adding some items");
    }

    // KeyHint tests
    #[test]
    fn test_key_hint_creation() {
        let hint = KeyHint::new("Esc", "Close");
        assert_eq!(hint.key, "Esc");
        assert_eq!(hint.description, "Close");
    }

    #[test]
    fn test_key_hint_to_spans() {
        let hint = KeyHint::new("Enter", "Submit");
        let spans = hint.to_spans();
        assert_eq!(spans.len(), 3);
    }

    // KeyHints tests
    #[test]
    fn test_key_hints_creation() {
        let hints = KeyHints::new();
        assert_eq!(hints.hints.len(), 0);
    }

    #[test]
    fn test_key_hints_hint() {
        let hints = KeyHints::new().hint("Esc", "Close").hint("Enter", "Submit");
        assert_eq!(hints.hints.len(), 2);
    }

    #[test]
    fn test_key_hints_default() {
        let hints = KeyHints::default();
        assert_eq!(hints.hints.len(), 0);
    }
}

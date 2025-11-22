//! Help System - Keybindings and help viewer
//!
//! Following Codex patterns for help and documentation.
//! Features:
//! - Keybinding reference
//! - Searchable help
//! - Context-sensitive help
//! - Cheat sheets

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs},
    Frame,
};

/// Help category
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpCategory {
    /// Category name
    pub name: String,
    /// Category description
    pub description: String,
    /// Bindings in this category
    pub bindings: Vec<KeyBinding>,
}

impl HelpCategory {
    /// Create a new category
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            bindings: Vec::new(),
        }
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add binding
    pub fn binding(mut self, binding: KeyBinding) -> Self {
        self.bindings.push(binding);
        self
    }

    /// Add multiple bindings
    pub fn bindings(mut self, bindings: Vec<KeyBinding>) -> Self {
        self.bindings.extend(bindings);
        self
    }
}

/// Key binding definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyBinding {
    /// Key sequence (e.g., "Ctrl+C", "q")
    pub key: String,
    /// Action description
    pub action: String,
    /// Context where binding is active
    pub context: Option<String>,
    /// Whether it's a chord (multiple keys)
    pub is_chord: bool,
}

impl KeyBinding {
    /// Create a new key binding
    pub fn new(key: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            action: action.into(),
            context: None,
            is_chord: false,
        }
    }

    /// Set context
    pub fn context(mut self, ctx: impl Into<String>) -> Self {
        self.context = Some(ctx.into());
        self
    }

    /// Mark as chord
    pub fn chord(mut self) -> Self {
        self.is_chord = true;
        self
    }
}

/// Help viewer state
pub struct HelpViewer {
    /// Help categories
    categories: Vec<HelpCategory>,
    /// Selected category index
    selected_category: usize,
    /// Selected binding index
    selected_binding: usize,
    /// Search query
    query: String,
    /// Filtered bindings
    filtered: Vec<(usize, usize)>, // (category_idx, binding_idx)
    /// Whether viewer is visible
    visible: bool,
    /// List state
    list_state: ListState,
    /// Search mode
    search_mode: bool,
    /// Show all bindings (not categorized)
    show_all: bool,
    /// Title
    title: String,
}

impl HelpViewer {
    /// Create a new help viewer
    pub fn new() -> Self {
        Self {
            categories: Vec::new(),
            selected_category: 0,
            selected_binding: 0,
            query: String::new(),
            filtered: Vec::new(),
            visible: false,
            list_state: ListState::default(),
            search_mode: false,
            show_all: false,
            title: "Help".to_string(),
        }
    }

    /// Set categories
    pub fn categories(mut self, categories: Vec<HelpCategory>) -> Self {
        self.categories = categories;
        self.update_filtered();
        self
    }

    /// Add category
    pub fn add_category(&mut self, category: HelpCategory) {
        self.categories.push(category);
        self.update_filtered();
    }

    /// Set title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Show the help viewer
    pub fn show(&mut self) {
        self.visible = true;
        self.query.clear();
        self.search_mode = false;
        self.selected_binding = 0;
        self.update_filtered();
    }

    /// Hide the help viewer
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Toggle show all mode
    pub fn toggle_show_all(&mut self) {
        self.show_all = !self.show_all;
        self.update_filtered();
    }

    /// Handle key event
    pub fn handle_key(&mut self, key: KeyEvent) -> HelpAction {
        if !self.visible {
            return HelpAction::None;
        }

        // Search mode
        if self.search_mode {
            match key.code {
                KeyCode::Esc => {
                    self.search_mode = false;
                    HelpAction::None
                }
                KeyCode::Enter => {
                    self.search_mode = false;
                    HelpAction::None
                }
                KeyCode::Char(c) => {
                    self.query.push(c);
                    self.update_filtered();
                    HelpAction::None
                }
                KeyCode::Backspace => {
                    self.query.pop();
                    self.update_filtered();
                    HelpAction::None
                }
                _ => HelpAction::None,
            }
        } else {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.hide();
                    HelpAction::Close
                }
                KeyCode::Char('/') => {
                    self.search_mode = true;
                    HelpAction::None
                }
                KeyCode::Tab => {
                    self.selected_category = (self.selected_category + 1) % self.categories.len().max(1);
                    self.selected_binding = 0;
                    self.update_filtered();
                    HelpAction::None
                }
                KeyCode::BackTab => {
                    self.selected_category = if self.selected_category == 0 {
                        self.categories.len().saturating_sub(1)
                    } else {
                        self.selected_category - 1
                    };
                    self.selected_binding = 0;
                    self.update_filtered();
                    HelpAction::None
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.move_up();
                    HelpAction::None
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.move_down();
                    HelpAction::None
                }
                KeyCode::PageUp => {
                    for _ in 0..10 {
                        self.move_up();
                    }
                    HelpAction::None
                }
                KeyCode::PageDown => {
                    for _ in 0..10 {
                        self.move_down();
                    }
                    HelpAction::None
                }
                KeyCode::Home => {
                    self.selected_binding = 0;
                    self.update_list_state();
                    HelpAction::None
                }
                KeyCode::End => {
                    if !self.filtered.is_empty() {
                        self.selected_binding = self.filtered.len() - 1;
                        self.update_list_state();
                    }
                    HelpAction::None
                }
                KeyCode::Char('a') => {
                    self.toggle_show_all();
                    HelpAction::None
                }
                _ => HelpAction::None,
            }
        }
    }

    /// Move selection up
    fn move_up(&mut self) {
        if !self.filtered.is_empty() {
            self.selected_binding = self.selected_binding.saturating_sub(1);
            self.update_list_state();
        }
    }

    /// Move selection down
    fn move_down(&mut self) {
        if !self.filtered.is_empty() {
            self.selected_binding = (self.selected_binding + 1).min(self.filtered.len() - 1);
            self.update_list_state();
        }
    }

    /// Update list state
    fn update_list_state(&mut self) {
        self.list_state.select(Some(self.selected_binding));
    }

    /// Update filtered bindings
    fn update_filtered(&mut self) {
        let query = self.query.to_lowercase();
        self.filtered.clear();

        for (cat_idx, category) in self.categories.iter().enumerate() {
            if !self.show_all && cat_idx != self.selected_category {
                continue;
            }

            for (bind_idx, binding) in category.bindings.iter().enumerate() {
                if query.is_empty() {
                    self.filtered.push((cat_idx, bind_idx));
                } else {
                    let key_match = binding.key.to_lowercase().contains(&query);
                    let action_match = binding.action.to_lowercase().contains(&query);
                    let context_match = binding
                        .context
                        .as_ref()
                        .map(|c| c.to_lowercase().contains(&query))
                        .unwrap_or(false);

                    if key_match || action_match || context_match {
                        self.filtered.push((cat_idx, bind_idx));
                    }
                }
            }
        }

        if self.selected_binding >= self.filtered.len() {
            self.selected_binding = 0;
        }
        self.update_list_state();
    }

    /// Render the help viewer
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Full screen
        frame.render_widget(Clear, area);

        // Main block
        let block = Block::default()
            .title(Span::styled(
                format!(" {} ", self.title),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Layout: tabs + search + content + status
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Tabs
                Constraint::Length(3),  // Search
                Constraint::Min(1),     // Content
                Constraint::Length(1),  // Status
            ])
            .split(inner);

        // Tabs
        self.render_tabs(frame, chunks[0]);

        // Search
        self.render_search(frame, chunks[1]);

        // Content
        self.render_bindings(frame, chunks[2]);

        // Status
        self.render_status(frame, chunks[3]);
    }

    /// Render category tabs
    fn render_tabs(&self, frame: &mut Frame, area: Rect) {
        if self.categories.is_empty() {
            return;
        }

        let titles: Vec<Line> = self
            .categories
            .iter()
            .map(|c| Line::from(c.name.clone()))
            .collect();

        let tabs = Tabs::new(titles)
            .select(self.selected_category)
            .style(Style::default().fg(Color::Rgb(86, 95, 137)))
            .highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .divider(Span::raw(" | "));

        frame.render_widget(tabs, area);
    }

    /// Render search input
    fn render_search(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(86, 95, 137)));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let content = if self.query.is_empty() {
            if self.search_mode {
                Line::from(vec![
                    Span::styled("/ ", Style::default().fg(Color::Cyan)),
                    Span::styled("█", Style::default().fg(Color::Cyan)),
                ])
            } else {
                Line::from(vec![
                    Span::styled("/ ", Style::default().fg(Color::Rgb(86, 95, 137))),
                    Span::styled(
                        "Press / to search...",
                        Style::default().fg(Color::Rgb(86, 95, 137)),
                    ),
                ])
            }
        } else {
            Line::from(vec![
                Span::styled("/ ", Style::default().fg(Color::Cyan)),
                Span::raw(&self.query),
                if self.search_mode {
                    Span::styled("█", Style::default().fg(Color::Cyan))
                } else {
                    Span::raw("")
                },
            ])
        };

        let search = Paragraph::new(content);
        frame.render_widget(search, inner);
    }

    /// Render bindings list
    fn render_bindings(&mut self, frame: &mut Frame, area: Rect) {
        if self.filtered.is_empty() {
            let empty = Paragraph::new(Line::from(Span::styled(
                "No bindings found",
                Style::default().fg(Color::Rgb(86, 95, 137)),
            )))
            .alignment(Alignment::Center);
            frame.render_widget(empty, area);
            return;
        }

        let items: Vec<ListItem> = self
            .filtered
            .iter()
            .enumerate()
            .map(|(i, &(cat_idx, bind_idx))| {
                let binding = &self.categories[cat_idx].bindings[bind_idx];
                let is_selected = i == self.selected_binding;

                let mut spans = vec![];

                // Key
                spans.push(Span::styled(
                    format!("{:>15}", binding.key),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(if is_selected {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ));

                // Separator
                spans.push(Span::styled(
                    "  │  ",
                    Style::default().fg(Color::Rgb(86, 95, 137)),
                ));

                // Action
                let action_style = if is_selected {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Rgb(192, 202, 245))
                };
                spans.push(Span::styled(&binding.action, action_style));

                // Context (if any)
                if let Some(ctx) = &binding.context {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(
                        format!("({})", ctx),
                        Style::default().fg(Color::Rgb(86, 95, 137)),
                    ));
                }

                let line = Line::from(spans);

                let style = if is_selected {
                    Style::default().bg(Color::Rgb(41, 46, 66))
                } else {
                    Style::default()
                };

                ListItem::new(line).style(style)
            })
            .collect();

        let list = List::new(items).highlight_symbol("");
        frame.render_stateful_widget(list, area, &mut self.list_state);
    }

    /// Render status bar
    fn render_status(&self, frame: &mut Frame, area: Rect) {
        let status = Line::from(vec![
            Span::styled(
                format!(" {} bindings ", self.filtered.len()),
                Style::default().fg(Color::Rgb(86, 95, 137)),
            ),
            Span::raw("│"),
            Span::styled(
                " Tab: Switch category ",
                Style::default().fg(Color::Rgb(86, 95, 137)),
            ),
            Span::styled(
                " a: Show all ",
                Style::default().fg(Color::Rgb(86, 95, 137)),
            ),
            Span::styled(
                " q: Close ",
                Style::default().fg(Color::Rgb(86, 95, 137)),
            ),
        ]);

        let paragraph = Paragraph::new(status);
        frame.render_widget(paragraph, area);
    }

    /// Create default help with common bindings
    pub fn with_defaults() -> Self {
        let mut viewer = Self::new();

        // Navigation category
        viewer.add_category(
            HelpCategory::new("Navigation")
                .description("Movement and navigation")
                .bindings(vec![
                    KeyBinding::new("↑/k", "Move up"),
                    KeyBinding::new("↓/j", "Move down"),
                    KeyBinding::new("←/h", "Move left"),
                    KeyBinding::new("→/l", "Move right"),
                    KeyBinding::new("Page Up", "Page up"),
                    KeyBinding::new("Page Down", "Page down"),
                    KeyBinding::new("Home", "Go to start"),
                    KeyBinding::new("End", "Go to end"),
                    KeyBinding::new("Ctrl+U", "Half page up"),
                    KeyBinding::new("Ctrl+D", "Half page down"),
                ]),
        );

        // Editing category
        viewer.add_category(
            HelpCategory::new("Editing")
                .description("Text editing commands")
                .bindings(vec![
                    KeyBinding::new("Ctrl+A", "Select all"),
                    KeyBinding::new("Ctrl+C", "Copy"),
                    KeyBinding::new("Ctrl+X", "Cut"),
                    KeyBinding::new("Ctrl+V", "Paste"),
                    KeyBinding::new("Ctrl+Z", "Undo"),
                    KeyBinding::new("Ctrl+Y", "Redo"),
                    KeyBinding::new("Ctrl+W", "Delete word backward"),
                    KeyBinding::new("Ctrl+K", "Kill to end of line"),
                    KeyBinding::new("Ctrl+U", "Clear line"),
                    KeyBinding::new("Alt+D", "Delete word forward"),
                ]),
        );

        // Chat category
        viewer.add_category(
            HelpCategory::new("Chat")
                .description("Chat interaction")
                .bindings(vec![
                    KeyBinding::new("Enter", "Send message"),
                    KeyBinding::new("Shift+Enter", "New line"),
                    KeyBinding::new("↑", "Previous history").context("empty input"),
                    KeyBinding::new("↓", "Next history").context("in history"),
                    KeyBinding::new("Tab", "Auto-complete"),
                    KeyBinding::new("Esc", "Cancel"),
                    KeyBinding::new("Ctrl+L", "Clear screen"),
                    KeyBinding::new("/", "Command mode"),
                ]),
        );

        // General category
        viewer.add_category(
            HelpCategory::new("General")
                .description("General actions")
                .bindings(vec![
                    KeyBinding::new("?", "Show help"),
                    KeyBinding::new("Ctrl+Q", "Quit"),
                    KeyBinding::new("Ctrl+S", "Save"),
                    KeyBinding::new("Ctrl+N", "New session"),
                    KeyBinding::new("Ctrl+O", "Open file"),
                    KeyBinding::new("Ctrl+R", "Resume session"),
                    KeyBinding::new("Ctrl+T", "New tab"),
                    KeyBinding::new("Ctrl+W", "Close tab"),
                ]),
        );

        // Tools category
        viewer.add_category(
            HelpCategory::new("Tools")
                .description("Tool interactions")
                .bindings(vec![
                    KeyBinding::new("y", "Approve action"),
                    KeyBinding::new("n", "Reject action"),
                    KeyBinding::new("d", "Show details"),
                    KeyBinding::new("Ctrl+A", "Approve all"),
                    KeyBinding::new("Tab", "Next option"),
                    KeyBinding::new("Shift+Tab", "Previous option"),
                ]),
        );

        viewer.update_filtered();
        viewer
    }
}

impl Default for HelpViewer {
    fn default() -> Self {
        Self::new()
    }
}

/// Action returned by help viewer
#[derive(Debug, Clone, PartialEq)]
pub enum HelpAction {
    /// No action
    None,
    /// Close help
    Close,
}

/// Cheat sheet widget
pub struct CheatSheet {
    /// Title
    title: String,
    /// Sections
    sections: Vec<CheatSection>,
}

/// Cheat sheet section
#[derive(Clone)]
pub struct CheatSection {
    /// Section title
    title: String,
    /// Items
    items: Vec<(String, String)>,
}

impl CheatSection {
    /// Create a new section
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            items: Vec::new(),
        }
    }

    /// Add item
    pub fn item(mut self, key: impl Into<String>, desc: impl Into<String>) -> Self {
        self.items.push((key.into(), desc.into()));
        self
    }
}

impl CheatSheet {
    /// Create a new cheat sheet
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            sections: Vec::new(),
        }
    }

    /// Add section
    pub fn section(mut self, section: CheatSection) -> Self {
        self.sections.push(section);
        self
    }

    /// Render the cheat sheet
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(Span::styled(
                format!(" {} ", self.title),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Split into columns for sections
        let num_sections = self.sections.len();
        if num_sections == 0 {
            return;
        }

        let col_width = 100 / num_sections as u16;
        let constraints: Vec<Constraint> = (0..num_sections)
            .map(|_| Constraint::Percentage(col_width))
            .collect();

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(inner);

        for (i, section) in self.sections.iter().enumerate() {
            self.render_section(frame, chunks[i], section);
        }
    }

    /// Render a single section
    fn render_section(&self, frame: &mut Frame, area: Rect, section: &CheatSection) {
        let mut lines = vec![
            Line::from(Span::styled(
                &section.title,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        for (key, desc) in &section.items {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:>10}", key),
                    Style::default().fg(Color::Rgb(192, 202, 245)),
                ),
                Span::styled("  ", Style::default()),
                Span::styled(desc, Style::default().fg(Color::Rgb(169, 177, 214))),
            ]));
        }

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, area);
    }

    /// Create a default cheat sheet
    pub fn default_vim() -> Self {
        CheatSheet::new("Vim Cheat Sheet")
            .section(
                CheatSection::new("Movement")
                    .item("h/l", "Left/Right")
                    .item("j/k", "Down/Up")
                    .item("w/b", "Word forward/back")
                    .item("0/$", "Line start/end")
                    .item("gg/G", "File start/end"),
            )
            .section(
                CheatSection::new("Editing")
                    .item("i/a", "Insert/Append")
                    .item("o/O", "New line below/above")
                    .item("dd", "Delete line")
                    .item("yy", "Yank line")
                    .item("p/P", "Paste after/before"),
            )
            .section(
                CheatSection::new("Other")
                    .item("u", "Undo")
                    .item("Ctrl+R", "Redo")
                    .item("/", "Search")
                    .item("n/N", "Next/prev match")
                    .item(":w", "Save"),
            )
    }
}

/// Quick reference popup
pub struct QuickRef {
    /// Items
    items: Vec<(String, String)>,
    /// Visible
    visible: bool,
}

impl QuickRef {
    /// Create a new quick reference
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            visible: false,
        }
    }

    /// Add item
    pub fn item(mut self, key: impl Into<String>, desc: impl Into<String>) -> Self {
        self.items.push((key.into(), desc.into()));
        self
    }

    /// Show
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Toggle
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Render
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible || self.items.is_empty() {
            return;
        }

        let width = self
            .items
            .iter()
            .map(|(k, d)| k.len() + d.len() + 4)
            .max()
            .unwrap_or(20)
            .max(20) as u16;

        let height = (self.items.len() + 2) as u16;

        let popup_area = Rect {
            x: area.x + area.width - width - 2,
            y: area.y + 1,
            width: width.min(area.width - 4),
            height: height.min(area.height - 2),
        };

        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .title(" Quick Ref ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        let lines: Vec<Line> = self
            .items
            .iter()
            .map(|(key, desc)| {
                Line::from(vec![
                    Span::styled(
                        format!("{:>8}", key),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::raw("  "),
                    Span::styled(desc, Style::default().fg(Color::Rgb(192, 202, 245))),
                ])
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }
}

impl Default for QuickRef {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    // HelpCategory tests
    #[test]
    fn test_category_creation() {
        let cat = HelpCategory::new("Navigation");
        assert_eq!(cat.name, "Navigation");
        assert!(cat.description.is_empty());
        assert!(cat.bindings.is_empty());
    }

    #[test]
    fn test_category_description() {
        let cat = HelpCategory::new("Nav").description("Movement commands");
        assert_eq!(cat.description, "Movement commands");
    }

    #[test]
    fn test_category_binding() {
        let cat = HelpCategory::new("Nav")
            .binding(KeyBinding::new("j", "Down"))
            .binding(KeyBinding::new("k", "Up"));
        assert_eq!(cat.bindings.len(), 2);
    }

    #[test]
    fn test_category_bindings() {
        let cat = HelpCategory::new("Nav").bindings(vec![
            KeyBinding::new("j", "Down"),
            KeyBinding::new("k", "Up"),
        ]);
        assert_eq!(cat.bindings.len(), 2);
    }

    // KeyBinding tests
    #[test]
    fn test_keybinding_creation() {
        let binding = KeyBinding::new("Ctrl+C", "Copy");
        assert_eq!(binding.key, "Ctrl+C");
        assert_eq!(binding.action, "Copy");
        assert!(binding.context.is_none());
        assert!(!binding.is_chord);
    }

    #[test]
    fn test_keybinding_context() {
        let binding = KeyBinding::new("Enter", "Submit").context("chat input");
        assert_eq!(binding.context, Some("chat input".to_string()));
    }

    #[test]
    fn test_keybinding_chord() {
        let binding = KeyBinding::new("dd", "Delete line").chord();
        assert!(binding.is_chord);
    }

    // HelpViewer tests
    #[test]
    fn test_viewer_creation() {
        let viewer = HelpViewer::new();
        assert!(!viewer.is_visible());
        assert!(viewer.categories.is_empty());
    }

    #[test]
    fn test_viewer_categories() {
        let viewer = HelpViewer::new().categories(vec![
            HelpCategory::new("Cat1"),
            HelpCategory::new("Cat2"),
        ]);
        assert_eq!(viewer.categories.len(), 2);
    }

    #[test]
    fn test_viewer_add_category() {
        let mut viewer = HelpViewer::new();
        viewer.add_category(HelpCategory::new("Test"));
        assert_eq!(viewer.categories.len(), 1);
    }

    #[test]
    fn test_viewer_title() {
        let viewer = HelpViewer::new().title("My Help");
        assert_eq!(viewer.title, "My Help");
    }

    #[test]
    fn test_viewer_show_hide() {
        let mut viewer = HelpViewer::new();
        viewer.show();
        assert!(viewer.is_visible());
        viewer.hide();
        assert!(!viewer.is_visible());
    }

    #[test]
    fn test_viewer_show_resets_state() {
        let mut viewer = HelpViewer::new().categories(vec![
            HelpCategory::new("Cat").binding(KeyBinding::new("a", "Action")),
        ]);
        viewer.show();
        viewer.selected_binding = 5;
        viewer.query = "test".to_string();
        viewer.search_mode = true;

        viewer.show();
        assert_eq!(viewer.selected_binding, 0);
        assert!(viewer.query.is_empty());
        assert!(!viewer.search_mode);
    }

    #[test]
    fn test_viewer_toggle_show_all() {
        let mut viewer = HelpViewer::new();
        assert!(!viewer.show_all);
        viewer.toggle_show_all();
        assert!(viewer.show_all);
        viewer.toggle_show_all();
        assert!(!viewer.show_all);
    }

    #[test]
    fn test_viewer_handle_key_escape() {
        let mut viewer = HelpViewer::new();
        viewer.show();

        let action = viewer.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert_eq!(action, HelpAction::Close);
        assert!(!viewer.is_visible());
    }

    #[test]
    fn test_viewer_handle_key_q() {
        let mut viewer = HelpViewer::new();
        viewer.show();

        let action = viewer.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty()));
        assert_eq!(action, HelpAction::Close);
    }

    #[test]
    fn test_viewer_handle_key_search_mode() {
        let mut viewer = HelpViewer::new();
        viewer.show();

        viewer.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()));
        assert!(viewer.search_mode);
    }

    #[test]
    fn test_viewer_handle_key_search_input() {
        let mut viewer = HelpViewer::new();
        viewer.show();
        viewer.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()));

        viewer.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::empty()));
        viewer.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty()));
        assert_eq!(viewer.query, "te");
    }

    #[test]
    fn test_viewer_handle_key_search_backspace() {
        let mut viewer = HelpViewer::new();
        viewer.show();
        viewer.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()));

        viewer.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()));
        viewer.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::empty()));
        viewer.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty()));
        assert_eq!(viewer.query, "a");
    }

    #[test]
    fn test_viewer_handle_key_search_escape() {
        let mut viewer = HelpViewer::new();
        viewer.show();
        viewer.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()));

        viewer.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert!(!viewer.search_mode);
    }

    #[test]
    fn test_viewer_handle_key_search_enter() {
        let mut viewer = HelpViewer::new();
        viewer.show();
        viewer.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()));

        viewer.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
        assert!(!viewer.search_mode);
    }

    #[test]
    fn test_viewer_handle_key_navigation() {
        let mut viewer = HelpViewer::new().categories(vec![
            HelpCategory::new("Cat").bindings(vec![
                KeyBinding::new("a", "Action A"),
                KeyBinding::new("b", "Action B"),
                KeyBinding::new("c", "Action C"),
            ]),
        ]);
        viewer.show();

        viewer.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
        assert_eq!(viewer.selected_binding, 1);

        viewer.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
        assert_eq!(viewer.selected_binding, 0);

        viewer.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty()));
        assert_eq!(viewer.selected_binding, 1);

        viewer.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::empty()));
        assert_eq!(viewer.selected_binding, 0);
    }

    #[test]
    fn test_viewer_handle_key_home_end() {
        let mut viewer = HelpViewer::new().categories(vec![
            HelpCategory::new("Cat").bindings(vec![
                KeyBinding::new("a", "A"),
                KeyBinding::new("b", "B"),
                KeyBinding::new("c", "C"),
            ]),
        ]);
        viewer.show();

        viewer.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::empty()));
        assert_eq!(viewer.selected_binding, 2);

        viewer.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::empty()));
        assert_eq!(viewer.selected_binding, 0);
    }

    #[test]
    fn test_viewer_handle_key_tab() {
        let mut viewer = HelpViewer::new().categories(vec![
            HelpCategory::new("Cat1").binding(KeyBinding::new("a", "A")),
            HelpCategory::new("Cat2").binding(KeyBinding::new("b", "B")),
        ]);
        viewer.show();

        viewer.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()));
        assert_eq!(viewer.selected_category, 1);

        viewer.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()));
        assert_eq!(viewer.selected_category, 0);
    }

    #[test]
    fn test_viewer_handle_key_backtab() {
        let mut viewer = HelpViewer::new().categories(vec![
            HelpCategory::new("Cat1").binding(KeyBinding::new("a", "A")),
            HelpCategory::new("Cat2").binding(KeyBinding::new("b", "B")),
        ]);
        viewer.show();

        viewer.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::empty()));
        assert_eq!(viewer.selected_category, 1);
    }

    #[test]
    fn test_viewer_handle_key_toggle_all() {
        let mut viewer = HelpViewer::new();
        viewer.show();

        viewer.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()));
        assert!(viewer.show_all);
    }

    #[test]
    fn test_viewer_handle_key_not_visible() {
        let mut viewer = HelpViewer::new();
        let action = viewer.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty()));
        assert_eq!(action, HelpAction::None);
    }

    #[test]
    fn test_viewer_with_defaults() {
        let viewer = HelpViewer::with_defaults();
        assert!(!viewer.categories.is_empty());
        assert!(viewer.categories.len() >= 4);
    }

    #[test]
    fn test_viewer_filtering() {
        let mut viewer = HelpViewer::new().categories(vec![
            HelpCategory::new("Cat").bindings(vec![
                KeyBinding::new("a", "Alpha"),
                KeyBinding::new("b", "Beta"),
                KeyBinding::new("c", "Copy"),
            ]),
        ]);
        viewer.show();
        assert_eq!(viewer.filtered.len(), 3);

        // Search for "beta"
        viewer.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()));
        viewer.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::empty()));
        viewer.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty()));
        viewer.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::empty()));
        viewer.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()));

        assert_eq!(viewer.filtered.len(), 1);
    }

    // HelpAction tests
    #[test]
    fn test_help_action_enum() {
        let _none = HelpAction::None;
        let _close = HelpAction::Close;
    }

    // CheatSection tests
    #[test]
    fn test_cheat_section_creation() {
        let section = CheatSection::new("Movement");
        assert_eq!(section.title, "Movement");
        assert!(section.items.is_empty());
    }

    #[test]
    fn test_cheat_section_item() {
        let section = CheatSection::new("Nav")
            .item("j", "Down")
            .item("k", "Up");
        assert_eq!(section.items.len(), 2);
    }

    // CheatSheet tests
    #[test]
    fn test_cheatsheet_creation() {
        let sheet = CheatSheet::new("My Sheet");
        assert_eq!(sheet.title, "My Sheet");
        assert!(sheet.sections.is_empty());
    }

    #[test]
    fn test_cheatsheet_section() {
        let sheet = CheatSheet::new("Sheet")
            .section(CheatSection::new("Section 1"))
            .section(CheatSection::new("Section 2"));
        assert_eq!(sheet.sections.len(), 2);
    }

    #[test]
    fn test_cheatsheet_default_vim() {
        let sheet = CheatSheet::default_vim();
        assert_eq!(sheet.title, "Vim Cheat Sheet");
        assert!(sheet.sections.len() >= 3);
    }

    // QuickRef tests
    #[test]
    fn test_quickref_creation() {
        let qr = QuickRef::new();
        assert!(!qr.is_visible());
        assert!(qr.items.is_empty());
    }

    #[test]
    fn test_quickref_item() {
        let qr = QuickRef::new()
            .item("q", "Quit")
            .item("?", "Help");
        assert_eq!(qr.items.len(), 2);
    }

    #[test]
    fn test_quickref_show_hide() {
        let mut qr = QuickRef::new();
        qr.show();
        assert!(qr.is_visible());
        qr.hide();
        assert!(!qr.is_visible());
    }

    #[test]
    fn test_quickref_toggle() {
        let mut qr = QuickRef::new();
        qr.toggle();
        assert!(qr.is_visible());
        qr.toggle();
        assert!(!qr.is_visible());
    }
}

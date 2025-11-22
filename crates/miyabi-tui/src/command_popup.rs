//! Command Popup - Command palette/popup component
//!
//! Following Codex patterns for command input UI.
//! Features:
//! - Fuzzy search/filtering
//! - Keyboard navigation
//! - Command categorization
//! - Keyboard shortcut display

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::wrapping::display_width;

/// Command definition
#[derive(Debug, Clone)]
pub struct Command {
    /// Command name/identifier
    pub name: String,
    /// Display label
    pub label: String,
    /// Description
    pub description: String,
    /// Category for grouping
    pub category: String,
    /// Keyboard shortcut (for display)
    pub shortcut: Option<String>,
    /// Whether command is enabled
    pub enabled: bool,
}

impl Command {
    /// Create a new command
    pub fn new(name: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            description: String::new(),
            category: "General".to_string(),
            shortcut: None,
            enabled: true,
        }
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set category
    pub fn category(mut self, cat: impl Into<String>) -> Self {
        self.category = cat.into();
        self
    }

    /// Set shortcut
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Set enabled state
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Command popup state
pub struct CommandPopup {
    /// All available commands
    commands: Vec<Command>,
    /// Filtered commands (indices into commands)
    filtered: Vec<usize>,
    /// Search query
    query: String,
    /// Selected index in filtered list
    selected: usize,
    /// List state for scrolling
    list_state: ListState,
    /// Whether popup is visible
    visible: bool,
    /// Title
    title: String,
    /// Placeholder text
    placeholder: String,
}

impl CommandPopup {
    /// Create a new command popup
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            filtered: Vec::new(),
            query: String::new(),
            selected: 0,
            list_state: ListState::default(),
            visible: false,
            title: "Commands".to_string(),
            placeholder: "Type to search...".to_string(),
        }
    }

    /// Set commands
    pub fn commands(mut self, commands: Vec<Command>) -> Self {
        self.commands = commands;
        self.update_filtered();
        self
    }

    /// Set title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set placeholder
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Show the popup
    pub fn show(&mut self) {
        self.visible = true;
        self.query.clear();
        self.selected = 0;
        self.update_filtered();
    }

    /// Hide the popup
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Get selected command
    pub fn selected_command(&self) -> Option<&Command> {
        self.filtered.get(self.selected).map(|&idx| &self.commands[idx])
    }

    /// Handle key event
    pub fn handle_key(&mut self, key: KeyEvent) -> CommandPopupAction {
        if !self.visible {
            return CommandPopupAction::None;
        }

        match key.code {
            KeyCode::Esc => {
                self.hide();
                CommandPopupAction::Cancel
            }
            KeyCode::Enter => {
                if let Some(cmd) = self.selected_command() {
                    if cmd.enabled {
                        let name = cmd.name.clone();
                        self.hide();
                        CommandPopupAction::Execute(name)
                    } else {
                        CommandPopupAction::None
                    }
                } else {
                    CommandPopupAction::None
                }
            }
            KeyCode::Up | KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_up();
                CommandPopupAction::None
            }
            KeyCode::Down | KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_down();
                CommandPopupAction::None
            }
            KeyCode::Up => {
                self.move_up();
                CommandPopupAction::None
            }
            KeyCode::Down | KeyCode::Tab => {
                self.move_down();
                CommandPopupAction::None
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_down();
                CommandPopupAction::None
            }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_up();
                CommandPopupAction::None
            }
            KeyCode::PageUp => {
                for _ in 0..10 {
                    self.move_up();
                }
                CommandPopupAction::None
            }
            KeyCode::PageDown => {
                for _ in 0..10 {
                    self.move_down();
                }
                CommandPopupAction::None
            }
            KeyCode::Home => {
                self.selected = 0;
                self.update_list_state();
                CommandPopupAction::None
            }
            KeyCode::End => {
                if !self.filtered.is_empty() {
                    self.selected = self.filtered.len() - 1;
                    self.update_list_state();
                }
                CommandPopupAction::None
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.query.clear();
                self.update_filtered();
                CommandPopupAction::None
            }
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Delete word backward
                while !self.query.is_empty() && self.query.ends_with(' ') {
                    self.query.pop();
                }
                while !self.query.is_empty() && !self.query.ends_with(' ') {
                    self.query.pop();
                }
                self.update_filtered();
                CommandPopupAction::None
            }
            KeyCode::Char(c) => {
                self.query.push(c);
                self.update_filtered();
                CommandPopupAction::None
            }
            KeyCode::Backspace => {
                self.query.pop();
                self.update_filtered();
                CommandPopupAction::None
            }
            _ => CommandPopupAction::None,
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

    /// Update filtered commands
    fn update_filtered(&mut self) {
        let query = self.query.to_lowercase();

        if query.is_empty() {
            // Show all commands
            self.filtered = (0..self.commands.len()).collect();
        } else {
            // Fuzzy filter
            self.filtered = self
                .commands
                .iter()
                .enumerate()
                .filter(|(_, cmd)| {
                    let name = cmd.name.to_lowercase();
                    let label = cmd.label.to_lowercase();
                    let desc = cmd.description.to_lowercase();
                    let cat = cmd.category.to_lowercase();

                    // Simple fuzzy match
                    fuzzy_match(&query, &name)
                        || fuzzy_match(&query, &label)
                        || desc.contains(&query)
                        || cat.contains(&query)
                })
                .map(|(i, _)| i)
                .collect();
        }

        // Reset selection if out of bounds
        if self.selected >= self.filtered.len() {
            self.selected = 0;
        }
        self.update_list_state();
    }

    /// Render the popup
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Calculate popup dimensions
        let popup_width = area.width.min(60);
        let popup_height = area.height.min(20);

        let popup_area = Rect {
            x: (area.width - popup_width) / 2 + area.x,
            y: (area.height - popup_height) / 3 + area.y, // Slightly towards top
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

        // Layout: search input + list
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(inner);

        // Search input
        self.render_search(frame, chunks[0]);

        // Command list
        self.render_list(frame, chunks[1]);
    }

    /// Render search input
    fn render_search(&self, frame: &mut Frame, area: Rect) {
        let search_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(86, 95, 137)));

        let inner = search_block.inner(area);
        frame.render_widget(search_block, area);

        let content = if self.query.is_empty() {
            Line::from(vec![
                Span::styled("› ", Style::default().fg(Color::Cyan)),
                Span::styled(&self.placeholder, Style::default().fg(Color::Rgb(86, 95, 137))),
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

    /// Render command list
    fn render_list(&mut self, frame: &mut Frame, area: Rect) {
        if self.filtered.is_empty() {
            let no_results = Paragraph::new(Line::from(Span::styled(
                "No commands found",
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
            .map(|(i, &cmd_idx)| {
                let cmd = &self.commands[cmd_idx];
                let is_selected = i == self.selected;

                let mut spans = vec![];

                // Icon
                let icon = if cmd.enabled { "  " } else { "  " };
                spans.push(Span::styled(
                    icon,
                    Style::default().fg(if cmd.enabled {
                        Color::Green
                    } else {
                        Color::Rgb(86, 95, 137)
                    }),
                ));

                // Label
                let label_style = if is_selected {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else if cmd.enabled {
                    Style::default().fg(Color::Rgb(192, 202, 245))
                } else {
                    Style::default().fg(Color::Rgb(86, 95, 137))
                };
                spans.push(Span::styled(&cmd.label, label_style));

                // Shortcut (if any)
                if let Some(shortcut) = &cmd.shortcut {
                    spans.push(Span::raw("  "));
                    spans.push(Span::styled(
                        shortcut,
                        Style::default().fg(Color::Rgb(86, 95, 137)),
                    ));
                }

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

    /// Add default commands
    pub fn with_default_commands(mut self) -> Self {
        self.commands = vec![
            Command::new("help", "Help")
                .description("Show help information")
                .shortcut("?")
                .category("General"),
            Command::new("clear", "Clear Chat")
                .description("Clear conversation history")
                .shortcut("Ctrl+L")
                .category("Chat"),
            Command::new("new", "New Conversation")
                .description("Start a new conversation")
                .shortcut("Ctrl+N")
                .category("Chat"),
            Command::new("history", "History")
                .description("View conversation history")
                .shortcut("Ctrl+H")
                .category("Chat"),
            Command::new("model", "Change Model")
                .description("Select AI model")
                .category("Settings"),
            Command::new("temperature", "Temperature")
                .description("Adjust response creativity")
                .category("Settings"),
            Command::new("tools", "Manage Tools")
                .description("Enable/disable tools")
                .category("Settings"),
            Command::new("context", "Context")
                .description("View current context")
                .category("Debug"),
            Command::new("export", "Export Chat")
                .description("Export conversation to file")
                .category("File"),
            Command::new("import", "Import Chat")
                .description("Import conversation from file")
                .category("File"),
            Command::new("theme", "Theme")
                .description("Change color theme")
                .category("Appearance"),
            Command::new("quit", "Quit")
                .description("Exit the application")
                .shortcut("Ctrl+Q")
                .category("General"),
        ];
        self.update_filtered();
        self
    }
}

impl Default for CommandPopup {
    fn default() -> Self {
        Self::new()
    }
}

/// Action returned by command popup
#[derive(Debug, Clone, PartialEq)]
pub enum CommandPopupAction {
    /// No action
    None,
    /// Execute command
    Execute(String),
    /// Cancel/close popup
    Cancel,
}

/// Simple fuzzy match
fn fuzzy_match(query: &str, target: &str) -> bool {
    let mut query_chars = query.chars().peekable();
    let mut target_chars = target.chars();

    while let Some(&q) = query_chars.peek() {
        match target_chars.next() {
            Some(t) if t.eq_ignore_ascii_case(&q) => {
                query_chars.next();
            }
            Some(_) => continue,
            None => return false,
        }
    }

    true
}

/// Command category for grouping
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandCategory {
    pub name: String,
    pub order: usize,
}

impl CommandCategory {
    pub fn new(name: impl Into<String>, order: usize) -> Self {
        Self {
            name: name.into(),
            order,
        }
    }
}

/// Helper to build command list
pub struct CommandBuilder {
    commands: Vec<Command>,
}

impl CommandBuilder {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    pub fn add(mut self, command: Command) -> Self {
        self.commands.push(command);
        self
    }

    pub fn build(self) -> Vec<Command> {
        self.commands
    }
}

impl Default for CommandBuilder {
    fn default() -> Self {
        Self::new()
    }
}

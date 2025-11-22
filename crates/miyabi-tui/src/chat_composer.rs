//! Chat Composer - Input handling component
//!
//! Separates input handling from the main app following Codex patterns.
//! Features:
//! - Multi-line input support
//! - Command history navigation
//! - Auto-completion suggestions
//! - Vim-style keybindings (optional)

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::wrapping::display_width;

/// Cursor position in the text
#[derive(Debug, Clone, Copy, Default)]
pub struct CursorPos {
    /// Line index (0-based)
    pub line: usize,
    /// Column index (0-based, in characters not bytes)
    pub col: usize,
}

/// Input mode for the composer
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    /// Normal text input
    Normal,
    /// Command mode (after /)
    Command,
    /// Search mode
    Search,
}

/// Selection range
#[derive(Debug, Clone, Copy)]
pub struct Selection {
    pub start: CursorPos,
    pub end: CursorPos,
}

/// History entry
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Chat composer state
pub struct ChatComposer {
    /// Input lines
    lines: Vec<String>,
    /// Cursor position
    cursor: CursorPos,
    /// Current input mode
    mode: InputMode,
    /// Selection (if any)
    selection: Option<Selection>,
    /// Command history
    history: Vec<HistoryEntry>,
    /// Current history index (for navigation)
    history_index: Option<usize>,
    /// Temporary buffer when navigating history
    temp_buffer: Option<String>,
    /// Scroll offset for multi-line input
    scroll_offset: usize,
    /// Whether the composer is focused
    focused: bool,
    /// Auto-complete suggestions
    suggestions: Vec<String>,
    /// Selected suggestion index
    suggestion_index: usize,
    /// Show suggestions popup
    show_suggestions: bool,
    /// Placeholder text
    placeholder: String,
    /// Maximum history size
    max_history: usize,
}

impl ChatComposer {
    /// Create a new chat composer
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor: CursorPos::default(),
            mode: InputMode::Normal,
            selection: None,
            history: Vec::new(),
            history_index: None,
            temp_buffer: None,
            scroll_offset: 0,
            focused: true,
            suggestions: Vec::new(),
            suggestion_index: 0,
            show_suggestions: false,
            placeholder: "Type your message...".to_string(),
            max_history: 100,
        }
    }

    /// Set placeholder text
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Set maximum history size
    pub fn max_history(mut self, size: usize) -> Self {
        self.max_history = size;
        self
    }

    /// Get current input as a single string
    pub fn get_input(&self) -> String {
        self.lines.join("\n")
    }

    /// Check if input is empty
    pub fn is_empty(&self) -> bool {
        self.lines.len() == 1 && self.lines[0].is_empty()
    }

    /// Clear input
    pub fn clear(&mut self) {
        self.lines = vec![String::new()];
        self.cursor = CursorPos::default();
        self.selection = None;
        self.scroll_offset = 0;
    }

    /// Submit current input and add to history
    pub fn submit(&mut self) -> String {
        let content = self.get_input();

        if !content.trim().is_empty() {
            // Add to history
            self.history.push(HistoryEntry {
                content: content.clone(),
                timestamp: chrono::Utc::now(),
            });

            // Trim history if needed
            if self.history.len() > self.max_history {
                self.history.remove(0);
            }
        }

        // Reset state
        self.clear();
        self.history_index = None;
        self.temp_buffer = None;

        content
    }

    /// Handle key event
    pub fn handle_key(&mut self, key: KeyEvent) -> ComposerAction {
        // Handle suggestions first
        if self.show_suggestions {
            match key.code {
                KeyCode::Tab | KeyCode::Down => {
                    self.suggestion_index = (self.suggestion_index + 1) % self.suggestions.len().max(1);
                    return ComposerAction::None;
                }
                KeyCode::Up => {
                    self.suggestion_index = self.suggestion_index.saturating_sub(1);
                    return ComposerAction::None;
                }
                KeyCode::Enter => {
                    if let Some(suggestion) = self.suggestions.get(self.suggestion_index) {
                        self.insert_suggestion(suggestion.clone());
                    }
                    self.show_suggestions = false;
                    return ComposerAction::None;
                }
                KeyCode::Esc => {
                    self.show_suggestions = false;
                    return ComposerAction::None;
                }
                _ => {
                    self.show_suggestions = false;
                }
            }
        }

        // Handle modifiers
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return self.handle_ctrl_key(key.code);
        }

        if key.modifiers.contains(KeyModifiers::ALT) {
            return self.handle_alt_key(key.code);
        }

        // Handle normal keys
        match key.code {
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    // Shift+Enter: new line
                    self.insert_newline();
                    ComposerAction::None
                } else {
                    // Enter: submit
                    if !self.is_empty() {
                        ComposerAction::Submit
                    } else {
                        ComposerAction::None
                    }
                }
            }
            KeyCode::Char(c) => {
                self.insert_char(c);
                self.update_suggestions();
                ComposerAction::None
            }
            KeyCode::Backspace => {
                self.backspace();
                self.update_suggestions();
                ComposerAction::None
            }
            KeyCode::Delete => {
                self.delete();
                ComposerAction::None
            }
            KeyCode::Left => {
                self.move_cursor_left();
                ComposerAction::None
            }
            KeyCode::Right => {
                self.move_cursor_right();
                ComposerAction::None
            }
            KeyCode::Up => {
                if self.lines.len() > 1 && self.cursor.line > 0 {
                    self.move_cursor_up();
                } else {
                    self.history_prev();
                }
                ComposerAction::None
            }
            KeyCode::Down => {
                if self.cursor.line < self.lines.len() - 1 {
                    self.move_cursor_down();
                } else {
                    self.history_next();
                }
                ComposerAction::None
            }
            KeyCode::Home => {
                self.cursor.col = 0;
                ComposerAction::None
            }
            KeyCode::End => {
                self.cursor.col = self.current_line().chars().count();
                ComposerAction::None
            }
            KeyCode::Tab => {
                if self.mode == InputMode::Command {
                    self.show_suggestions = true;
                } else {
                    self.insert_char('\t');
                }
                ComposerAction::None
            }
            KeyCode::Esc => {
                ComposerAction::Cancel
            }
            _ => ComposerAction::None,
        }
    }

    fn handle_ctrl_key(&mut self, code: KeyCode) -> ComposerAction {
        match code {
            KeyCode::Char('a') => {
                // Select all
                self.select_all();
                ComposerAction::None
            }
            KeyCode::Char('c') => {
                // Copy (would need clipboard integration)
                ComposerAction::None
            }
            KeyCode::Char('v') => {
                // Paste (would need clipboard integration)
                ComposerAction::None
            }
            KeyCode::Char('z') => {
                // Undo (would need undo stack)
                ComposerAction::None
            }
            KeyCode::Char('y') => {
                // Redo
                ComposerAction::None
            }
            KeyCode::Char('w') => {
                // Delete word backward
                self.delete_word_backward();
                ComposerAction::None
            }
            KeyCode::Char('u') => {
                // Clear line
                self.clear_line();
                ComposerAction::None
            }
            KeyCode::Char('k') => {
                // Kill to end of line
                self.kill_to_end();
                ComposerAction::None
            }
            KeyCode::Left => {
                // Move word left
                self.move_word_left();
                ComposerAction::None
            }
            KeyCode::Right => {
                // Move word right
                self.move_word_right();
                ComposerAction::None
            }
            KeyCode::Backspace => {
                // Delete word backward
                self.delete_word_backward();
                ComposerAction::None
            }
            _ => ComposerAction::None,
        }
    }

    fn handle_alt_key(&mut self, code: KeyCode) -> ComposerAction {
        match code {
            KeyCode::Char('b') => {
                // Move word backward
                self.move_word_left();
                ComposerAction::None
            }
            KeyCode::Char('f') => {
                // Move word forward
                self.move_word_right();
                ComposerAction::None
            }
            KeyCode::Char('d') => {
                // Delete word forward
                self.delete_word_forward();
                ComposerAction::None
            }
            KeyCode::Backspace => {
                // Delete word backward
                self.delete_word_backward();
                ComposerAction::None
            }
            _ => ComposerAction::None,
        }
    }

    /// Insert a character at cursor
    fn insert_char(&mut self, c: char) {
        // Compute byte index before mutable borrow
        let byte_idx = self.char_to_byte_index(&self.lines[self.cursor.line], self.cursor.col);
        self.lines[self.cursor.line].insert(byte_idx, c);
        self.cursor.col += 1;

        // Check for command mode
        if c == '/' && self.cursor.col == 1 && self.cursor.line == 0 {
            self.mode = InputMode::Command;
        }
    }

    /// Insert newline at cursor
    fn insert_newline(&mut self) {
        // Compute byte index before mutable borrow
        let byte_idx = self.char_to_byte_index(&self.lines[self.cursor.line], self.cursor.col);
        let rest = self.lines[self.cursor.line][byte_idx..].to_string();
        self.lines[self.cursor.line].truncate(byte_idx);

        self.cursor.line += 1;
        self.cursor.col = 0;
        self.lines.insert(self.cursor.line, rest);
    }

    /// Delete character before cursor
    fn backspace(&mut self) {
        if self.cursor.col > 0 {
            // Compute byte indices before mutable borrow
            let byte_idx = self.char_to_byte_index(&self.lines[self.cursor.line], self.cursor.col - 1);
            let next_byte_idx = self.char_to_byte_index(&self.lines[self.cursor.line], self.cursor.col);
            self.lines[self.cursor.line].replace_range(byte_idx..next_byte_idx, "");
            self.cursor.col -= 1;
        } else if self.cursor.line > 0 {
            // Merge with previous line
            let current = self.lines.remove(self.cursor.line);
            self.cursor.line -= 1;
            self.cursor.col = self.lines[self.cursor.line].chars().count();
            self.lines[self.cursor.line].push_str(&current);
        }

        // Check for mode change
        if self.is_empty() || !self.lines[0].starts_with('/') {
            self.mode = InputMode::Normal;
        }
    }

    /// Delete character at cursor
    fn delete(&mut self) {
        let char_count = self.lines[self.cursor.line].chars().count();

        if self.cursor.col < char_count {
            // Compute byte indices before mutable borrow
            let byte_idx = self.char_to_byte_index(&self.lines[self.cursor.line], self.cursor.col);
            let next_byte_idx = self.char_to_byte_index(&self.lines[self.cursor.line], self.cursor.col + 1);
            self.lines[self.cursor.line].replace_range(byte_idx..next_byte_idx, "");
        } else if self.cursor.line < self.lines.len() - 1 {
            // Merge with next line
            let next = self.lines.remove(self.cursor.line + 1);
            self.lines[self.cursor.line].push_str(&next);
        }
    }

    /// Move cursor left
    fn move_cursor_left(&mut self) {
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
        } else if self.cursor.line > 0 {
            self.cursor.line -= 1;
            self.cursor.col = self.lines[self.cursor.line].chars().count();
        }
    }

    /// Move cursor right
    fn move_cursor_right(&mut self) {
        let char_count = self.lines[self.cursor.line].chars().count();
        if self.cursor.col < char_count {
            self.cursor.col += 1;
        } else if self.cursor.line < self.lines.len() - 1 {
            self.cursor.line += 1;
            self.cursor.col = 0;
        }
    }

    /// Move cursor up
    fn move_cursor_up(&mut self) {
        if self.cursor.line > 0 {
            self.cursor.line -= 1;
            let max_col = self.lines[self.cursor.line].chars().count();
            self.cursor.col = self.cursor.col.min(max_col);
        }
    }

    /// Move cursor down
    fn move_cursor_down(&mut self) {
        if self.cursor.line < self.lines.len() - 1 {
            self.cursor.line += 1;
            let max_col = self.lines[self.cursor.line].chars().count();
            self.cursor.col = self.cursor.col.min(max_col);
        }
    }

    /// Move to previous word
    fn move_word_left(&mut self) {
        let line = &self.lines[self.cursor.line];
        let chars: Vec<char> = line.chars().collect();

        if self.cursor.col == 0 {
            if self.cursor.line > 0 {
                self.cursor.line -= 1;
                self.cursor.col = self.lines[self.cursor.line].chars().count();
            }
            return;
        }

        let mut col = self.cursor.col - 1;

        // Skip whitespace
        while col > 0 && chars[col].is_whitespace() {
            col -= 1;
        }

        // Skip word
        while col > 0 && !chars[col - 1].is_whitespace() {
            col -= 1;
        }

        self.cursor.col = col;
    }

    /// Move to next word
    fn move_word_right(&mut self) {
        let line = &self.lines[self.cursor.line];
        let chars: Vec<char> = line.chars().collect();
        let len = chars.len();

        if self.cursor.col >= len {
            if self.cursor.line < self.lines.len() - 1 {
                self.cursor.line += 1;
                self.cursor.col = 0;
            }
            return;
        }

        let mut col = self.cursor.col;

        // Skip current word
        while col < len && !chars[col].is_whitespace() {
            col += 1;
        }

        // Skip whitespace
        while col < len && chars[col].is_whitespace() {
            col += 1;
        }

        self.cursor.col = col;
    }

    /// Delete word backward
    fn delete_word_backward(&mut self) {
        let start_col = self.cursor.col;
        self.move_word_left();

        if self.cursor.col < start_col {
            // Compute byte indices before mutable borrow
            let line_idx = self.cursor.line;
            let start_byte = self.char_to_byte_index(&self.lines[line_idx], self.cursor.col);
            let end_byte = self.char_to_byte_index(&self.lines[line_idx], start_col);
            self.lines[line_idx].replace_range(start_byte..end_byte, "");
        }
    }

    /// Delete word forward
    fn delete_word_forward(&mut self) {
        let start_col = self.cursor.col;
        self.move_word_right();

        if self.cursor.col > start_col {
            // Compute byte indices before mutable borrow
            let line_idx = self.cursor.line;
            let start_byte = self.char_to_byte_index(&self.lines[line_idx], start_col);
            let end_byte = self.char_to_byte_index(&self.lines[line_idx], self.cursor.col);
            self.lines[line_idx].replace_range(start_byte..end_byte, "");
            self.cursor.col = start_col;
        }
    }

    /// Clear current line
    fn clear_line(&mut self) {
        self.lines[self.cursor.line].clear();
        self.cursor.col = 0;
    }

    /// Kill to end of line
    fn kill_to_end(&mut self) {
        // Compute byte index before mutable borrow
        let byte_idx = self.char_to_byte_index(&self.lines[self.cursor.line], self.cursor.col);
        self.lines[self.cursor.line].truncate(byte_idx);
    }

    /// Select all text
    fn select_all(&mut self) {
        self.selection = Some(Selection {
            start: CursorPos { line: 0, col: 0 },
            end: CursorPos {
                line: self.lines.len() - 1,
                col: self.lines.last().map(|l| l.chars().count()).unwrap_or(0),
            },
        });
    }

    /// Navigate to previous history entry
    fn history_prev(&mut self) {
        if self.history.is_empty() {
            return;
        }

        match self.history_index {
            None => {
                // Save current input
                self.temp_buffer = Some(self.get_input());
                self.history_index = Some(self.history.len() - 1);
            }
            Some(idx) if idx > 0 => {
                self.history_index = Some(idx - 1);
            }
            _ => return,
        }

        if let Some(idx) = self.history_index {
            let content = self.history[idx].content.clone();
            self.set_input(&content);
        }
    }

    /// Navigate to next history entry
    fn history_next(&mut self) {
        match self.history_index {
            Some(idx) if idx < self.history.len() - 1 => {
                self.history_index = Some(idx + 1);
                let content = self.history[idx + 1].content.clone();
                self.set_input(&content);
            }
            Some(_) => {
                // Return to temp buffer
                self.history_index = None;
                if let Some(content) = self.temp_buffer.take() {
                    self.set_input(&content);
                }
            }
            None => {}
        }
    }

    /// Set input content
    fn set_input(&mut self, content: &str) {
        self.lines = content.lines().map(String::from).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.cursor.line = self.lines.len() - 1;
        self.cursor.col = self.lines[self.cursor.line].chars().count();
    }

    /// Update auto-complete suggestions
    fn update_suggestions(&mut self) {
        if self.mode != InputMode::Command {
            self.suggestions.clear();
            return;
        }

        let input = self.get_input();
        if let Some(cmd) = input.strip_prefix('/') {
            // Built-in commands
            let commands = vec![
                "help", "clear", "history", "exit", "quit",
                "model", "temperature", "tools", "context",
            ];

            self.suggestions = commands
                .into_iter()
                .filter(|c| c.starts_with(cmd))
                .map(String::from)
                .collect();
        }
    }

    /// Insert selected suggestion
    fn insert_suggestion(&mut self, suggestion: String) {
        if self.mode == InputMode::Command {
            self.lines = vec![format!("/{}", suggestion)];
            self.cursor.line = 0;
            self.cursor.col = self.lines[0].chars().count();
        }
    }

    /// Get current line
    fn current_line(&self) -> &str {
        &self.lines[self.cursor.line]
    }

    /// Convert character index to byte index
    fn char_to_byte_index(&self, s: &str, char_idx: usize) -> usize {
        s.char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or(s.len())
    }

    /// Render the composer
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if self.focused {
                Color::Cyan
            } else {
                Color::Rgb(86, 95, 137)
            }))
            .title(Span::styled(
                " Input ",
                Style::default().fg(Color::Rgb(192, 202, 245)),
            ));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Render input lines
        let mut lines: Vec<Line> = Vec::new();

        if self.is_empty() {
            // Show placeholder
            lines.push(Line::from(vec![
                Span::styled("› ", Style::default().fg(Color::Cyan)),
                Span::styled(&self.placeholder, Style::default().fg(Color::Rgb(86, 95, 137))),
            ]));
        } else {
            for (i, line) in self.lines.iter().enumerate() {
                let prefix = if i == 0 { "› " } else { "  " };
                let mut spans = vec![
                    Span::styled(prefix, Style::default().fg(Color::Cyan)),
                ];

                // Render line with cursor
                if i == self.cursor.line && self.focused {
                    let chars: Vec<char> = line.chars().collect();
                    let before: String = chars[..self.cursor.col].iter().collect();
                    let cursor_char = chars.get(self.cursor.col).copied().unwrap_or(' ');
                    let after: String = chars[self.cursor.col.saturating_add(1)..].iter().collect();

                    spans.push(Span::raw(before));
                    spans.push(Span::styled(
                        cursor_char.to_string(),
                        Style::default().bg(Color::Cyan).fg(Color::Black),
                    ));
                    spans.push(Span::raw(after));
                } else {
                    spans.push(Span::raw(line.clone()));
                }

                lines.push(Line::from(spans));
            }
        }

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);

        // Render suggestions popup
        if self.show_suggestions && !self.suggestions.is_empty() {
            self.render_suggestions(frame, area);
        }
    }

    /// Render suggestions popup
    fn render_suggestions(&self, frame: &mut Frame, area: Rect) {
        let popup_height = (self.suggestions.len() + 2).min(10) as u16;
        let popup_width = self.suggestions.iter()
            .map(|s| display_width(s))
            .max()
            .unwrap_or(20)
            .max(20) as u16 + 4;

        let popup_area = Rect {
            x: area.x + 2,
            y: area.y.saturating_sub(popup_height),
            width: popup_width.min(area.width - 4),
            height: popup_height,
        };

        frame.render_widget(Clear, popup_area);

        let items: Vec<Line> = self.suggestions
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let style = if i == self.suggestion_index {
                    Style::default().bg(Color::Rgb(86, 95, 137)).fg(Color::White)
                } else {
                    Style::default().fg(Color::Rgb(192, 202, 245))
                };
                Line::from(Span::styled(format!(" {} ", s), style))
            })
            .collect();

        let popup = Paragraph::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
                    .title(" Commands "),
            );

        frame.render_widget(popup, popup_area);
    }

    /// Set focus state
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Get line count
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
}

impl Default for ChatComposer {
    fn default() -> Self {
        Self::new()
    }
}

/// Action returned by composer after handling input
#[derive(Debug, Clone, PartialEq)]
pub enum ComposerAction {
    /// No action needed
    None,
    /// Submit the input
    Submit,
    /// Cancel input
    Cancel,
}

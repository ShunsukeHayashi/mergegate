//! TextArea - Advanced multi-line text input widget
//!
//! Following Codex patterns for production-quality text input.
//! Features:
//! - Syntax highlighting support
//! - Line numbers
//! - Selection and clipboard
//! - Undo/redo stack
//! - Vim-like keybindings (optional)

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
    Frame,
};

use crate::wrapping::display_width;

/// Operation for undo/redo
#[derive(Debug, Clone)]
enum EditOp {
    Insert {
        pos: usize,
        text: String,
    },
    Delete {
        pos: usize,
        text: String,
    },
    Replace {
        pos: usize,
        old_text: String,
        new_text: String,
    },
}

/// Selection range in the text
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}

impl TextRange {
    pub fn new(start: usize, end: usize) -> Self {
        if start <= end {
            Self { start, end }
        } else {
            Self { start: end, end: start }
        }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

/// Cursor position (line, column)
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TextCursor {
    pub line: usize,
    pub col: usize,
}

/// TextArea configuration
#[derive(Debug, Clone)]
pub struct TextAreaConfig {
    /// Show line numbers
    pub show_line_numbers: bool,
    /// Tab width in spaces
    pub tab_width: usize,
    /// Enable soft wrap
    pub soft_wrap: bool,
    /// Enable syntax highlighting
    pub syntax_highlight: bool,
    /// Maximum undo stack size
    pub max_undo: usize,
    /// Placeholder text
    pub placeholder: String,
}

impl Default for TextAreaConfig {
    fn default() -> Self {
        Self {
            show_line_numbers: true,
            tab_width: 4,
            soft_wrap: true,
            syntax_highlight: false,
            max_undo: 100,
            placeholder: String::new(),
        }
    }
}

/// Advanced text area widget
pub struct TextArea {
    /// Text content as lines
    lines: Vec<String>,
    /// Cursor position
    cursor: TextCursor,
    /// Selection anchor (if selecting)
    anchor: Option<TextCursor>,
    /// Scroll offset
    scroll: (usize, usize), // (vertical, horizontal)
    /// Configuration
    config: TextAreaConfig,
    /// Undo stack
    undo_stack: Vec<EditOp>,
    /// Redo stack
    redo_stack: Vec<EditOp>,
    /// Clipboard content
    clipboard: String,
    /// Whether focused
    focused: bool,
    /// Viewport size
    viewport: (u16, u16),
    /// Preferred column (for up/down movement)
    preferred_col: Option<usize>,
}

impl TextArea {
    /// Create a new text area
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor: TextCursor::default(),
            anchor: None,
            scroll: (0, 0),
            config: TextAreaConfig::default(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            clipboard: String::new(),
            focused: true,
            viewport: (80, 24),
            preferred_col: None,
        }
    }

    /// Set configuration
    pub fn config(mut self, config: TextAreaConfig) -> Self {
        self.config = config;
        self
    }

    /// Set placeholder text
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.config.placeholder = text.into();
        self
    }

    /// Get text content
    pub fn get_text(&self) -> String {
        self.lines.join("\n")
    }

    /// Set text content
    pub fn set_text(&mut self, text: &str) {
        self.lines = text.lines().map(String::from).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.cursor = TextCursor::default();
        self.anchor = None;
        self.scroll = (0, 0);
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.lines.len() == 1 && self.lines[0].is_empty()
    }

    /// Clear content
    pub fn clear(&mut self) {
        self.set_text("");
    }

    /// Get selection range
    pub fn selection(&self) -> Option<TextRange> {
        self.anchor.map(|anchor| {
            let start = self.pos_to_offset(anchor);
            let end = self.pos_to_offset(self.cursor);
            TextRange::new(start, end)
        })
    }

    /// Get selected text
    pub fn selected_text(&self) -> Option<String> {
        self.selection().map(|range| {
            let text = self.get_text();
            text[range.start..range.end].to_string()
        })
    }

    /// Handle key event
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        let alt = key.modifiers.contains(KeyModifiers::ALT);

        // Update selection anchor if shift is held
        if shift && self.anchor.is_none() {
            self.anchor = Some(self.cursor);
        } else if !shift && !matches!(key.code, KeyCode::Char('a') if ctrl) {
            self.anchor = None;
        }

        match key.code {
            // Navigation
            KeyCode::Left if ctrl => self.move_word_left(),
            KeyCode::Right if ctrl => self.move_word_right(),
            KeyCode::Left => self.move_left(),
            KeyCode::Right => self.move_right(),
            KeyCode::Up if ctrl => self.scroll_up(),
            KeyCode::Down if ctrl => self.scroll_down(),
            KeyCode::Up => self.move_up(),
            KeyCode::Down => self.move_down(),
            KeyCode::Home if ctrl => self.move_to_start(),
            KeyCode::End if ctrl => self.move_to_end(),
            KeyCode::Home => self.move_to_line_start(),
            KeyCode::End => self.move_to_line_end(),
            KeyCode::PageUp => self.page_up(),
            KeyCode::PageDown => self.page_down(),

            // Editing
            KeyCode::Char('a') if ctrl => {
                self.select_all();
            }
            KeyCode::Char('c') if ctrl => {
                self.copy();
            }
            KeyCode::Char('x') if ctrl => {
                self.cut();
            }
            KeyCode::Char('v') if ctrl => {
                self.paste();
            }
            KeyCode::Char('z') if ctrl && shift => {
                self.redo();
            }
            KeyCode::Char('z') if ctrl => {
                self.undo();
            }
            KeyCode::Char('y') if ctrl => {
                self.redo();
            }
            KeyCode::Char('w') if ctrl => {
                self.delete_word_backward();
            }
            KeyCode::Char('u') if ctrl => {
                self.delete_to_line_start();
            }
            KeyCode::Char('k') if ctrl => {
                self.delete_to_line_end();
            }
            KeyCode::Char('d') if ctrl => {
                self.delete_line();
            }

            // Alt shortcuts
            KeyCode::Char('b') if alt => self.move_word_left(),
            KeyCode::Char('f') if alt => self.move_word_right(),
            KeyCode::Char('d') if alt => self.delete_word_forward(),
            KeyCode::Backspace if alt => self.delete_word_backward(),

            // Input
            KeyCode::Char(c) => {
                self.insert_char(c);
            }
            KeyCode::Tab => {
                self.insert_tab();
            }
            KeyCode::Enter => {
                self.insert_newline();
            }
            KeyCode::Backspace if ctrl => {
                self.delete_word_backward();
            }
            KeyCode::Backspace => {
                self.backspace();
            }
            KeyCode::Delete if ctrl => {
                self.delete_word_forward();
            }
            KeyCode::Delete => {
                self.delete();
            }

            _ => return false,
        }

        self.ensure_cursor_visible();
        true
    }

    /// Insert character at cursor
    fn insert_char(&mut self, c: char) {
        self.delete_selection();

        let byte_idx = self.cursor_byte_index();
        self.lines[self.cursor.line].insert(byte_idx, c);

        self.push_undo(EditOp::Insert {
            pos: self.cursor_offset(),
            text: c.to_string(),
        });

        self.cursor.col += 1;
        self.preferred_col = None;
    }

    /// Insert tab
    fn insert_tab(&mut self) {
        let spaces = " ".repeat(self.config.tab_width);
        for c in spaces.chars() {
            self.insert_char(c);
        }
    }

    /// Insert newline
    fn insert_newline(&mut self) {
        self.delete_selection();

        let byte_idx = self.cursor_byte_index();
        let rest = self.lines[self.cursor.line][byte_idx..].to_string();
        self.lines[self.cursor.line].truncate(byte_idx);

        self.cursor.line += 1;
        self.cursor.col = 0;
        self.lines.insert(self.cursor.line, rest);

        self.push_undo(EditOp::Insert {
            pos: self.cursor_offset() - 1,
            text: "\n".to_string(),
        });

        self.preferred_col = None;
    }

    /// Delete character before cursor
    fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }

        if self.cursor.col > 0 {
            let byte_idx = self.char_to_byte(&self.lines[self.cursor.line], self.cursor.col - 1);
            let next_byte = self.char_to_byte(&self.lines[self.cursor.line], self.cursor.col);
            let deleted = self.lines[self.cursor.line][byte_idx..next_byte].to_string();
            self.lines[self.cursor.line].replace_range(byte_idx..next_byte, "");

            self.push_undo(EditOp::Delete {
                pos: self.cursor_offset() - 1,
                text: deleted,
            });

            self.cursor.col -= 1;
        } else if self.cursor.line > 0 {
            let current = self.lines.remove(self.cursor.line);
            self.cursor.line -= 1;
            self.cursor.col = self.lines[self.cursor.line].chars().count();
            self.lines[self.cursor.line].push_str(&current);

            self.push_undo(EditOp::Delete {
                pos: self.cursor_offset(),
                text: "\n".to_string(),
            });
        }

        self.preferred_col = None;
    }

    /// Delete character at cursor
    fn delete(&mut self) {
        if self.delete_selection() {
            return;
        }

        let char_count = self.lines[self.cursor.line].chars().count();

        if self.cursor.col < char_count {
            let byte_idx = self.cursor_byte_index();
            let next_byte = self.char_to_byte(&self.lines[self.cursor.line], self.cursor.col + 1);
            let deleted = self.lines[self.cursor.line][byte_idx..next_byte].to_string();
            self.lines[self.cursor.line].replace_range(byte_idx..next_byte, "");

            self.push_undo(EditOp::Delete {
                pos: self.cursor_offset(),
                text: deleted,
            });
        } else if self.cursor.line < self.lines.len() - 1 {
            let next = self.lines.remove(self.cursor.line + 1);
            self.lines[self.cursor.line].push_str(&next);

            self.push_undo(EditOp::Delete {
                pos: self.cursor_offset(),
                text: "\n".to_string(),
            });
        }
    }

    /// Delete selection if any
    fn delete_selection(&mut self) -> bool {
        if let Some(range) = self.selection() {
            if !range.is_empty() {
                let text = self.get_text();
                let deleted = text[range.start..range.end].to_string();

                // Calculate cursor position from start
                let new_cursor = self.offset_to_pos(range.start);
                self.set_text(&format!("{}{}", &text[..range.start], &text[range.end..]));
                self.cursor = new_cursor;

                self.push_undo(EditOp::Delete {
                    pos: range.start,
                    text: deleted,
                });

                self.anchor = None;
                return true;
            }
        }
        self.anchor = None;
        false
    }

    /// Delete word backward
    fn delete_word_backward(&mut self) {
        if self.delete_selection() {
            return;
        }

        let start_col = self.cursor.col;
        self.move_word_left();

        if self.cursor.col < start_col {
            let line_idx = self.cursor.line;
            let start_byte = self.char_to_byte(&self.lines[line_idx], self.cursor.col);
            let end_byte = self.char_to_byte(&self.lines[line_idx], start_col);
            let deleted = self.lines[line_idx][start_byte..end_byte].to_string();
            self.lines[line_idx].replace_range(start_byte..end_byte, "");

            self.push_undo(EditOp::Delete {
                pos: self.cursor_offset(),
                text: deleted,
            });
        }
    }

    /// Delete word forward
    fn delete_word_forward(&mut self) {
        if self.delete_selection() {
            return;
        }

        let start_col = self.cursor.col;
        self.move_word_right();

        if self.cursor.col > start_col {
            let line_idx = self.cursor.line;
            let start_byte = self.char_to_byte(&self.lines[line_idx], start_col);
            let end_byte = self.char_to_byte(&self.lines[line_idx], self.cursor.col);
            let deleted = self.lines[line_idx][start_byte..end_byte].to_string();
            self.lines[line_idx].replace_range(start_byte..end_byte, "");

            self.push_undo(EditOp::Delete {
                pos: self.cursor_offset() - deleted.len(),
                text: deleted,
            });

            self.cursor.col = start_col;
        }
    }

    /// Delete to line start
    fn delete_to_line_start(&mut self) {
        if self.cursor.col == 0 {
            return;
        }

        let byte_idx = self.cursor_byte_index();
        let deleted = self.lines[self.cursor.line][..byte_idx].to_string();
        self.lines[self.cursor.line] = self.lines[self.cursor.line][byte_idx..].to_string();

        self.push_undo(EditOp::Delete {
            pos: self.cursor_offset() - deleted.len(),
            text: deleted,
        });

        self.cursor.col = 0;
    }

    /// Delete to line end
    fn delete_to_line_end(&mut self) {
        let byte_idx = self.cursor_byte_index();
        let deleted = self.lines[self.cursor.line][byte_idx..].to_string();
        self.lines[self.cursor.line].truncate(byte_idx);

        self.push_undo(EditOp::Delete {
            pos: self.cursor_offset(),
            text: deleted,
        });
    }

    /// Delete entire line
    fn delete_line(&mut self) {
        if self.lines.len() == 1 {
            let deleted = self.lines[0].clone();
            self.lines[0].clear();
            self.cursor.col = 0;

            self.push_undo(EditOp::Delete {
                pos: 0,
                text: deleted,
            });
        } else {
            let deleted = format!("{}\n", self.lines.remove(self.cursor.line));
            if self.cursor.line >= self.lines.len() {
                self.cursor.line = self.lines.len() - 1;
            }
            self.cursor.col = 0;

            self.push_undo(EditOp::Delete {
                pos: self.cursor_offset(),
                text: deleted,
            });
        }
    }

    /// Move cursor left
    fn move_left(&mut self) {
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
        } else if self.cursor.line > 0 {
            self.cursor.line -= 1;
            self.cursor.col = self.lines[self.cursor.line].chars().count();
        }
        self.preferred_col = None;
    }

    /// Move cursor right
    fn move_right(&mut self) {
        let char_count = self.lines[self.cursor.line].chars().count();
        if self.cursor.col < char_count {
            self.cursor.col += 1;
        } else if self.cursor.line < self.lines.len() - 1 {
            self.cursor.line += 1;
            self.cursor.col = 0;
        }
        self.preferred_col = None;
    }

    /// Move cursor up
    fn move_up(&mut self) {
        if self.cursor.line > 0 {
            self.cursor.line -= 1;
            let target_col = self.preferred_col.unwrap_or(self.cursor.col);
            let max_col = self.lines[self.cursor.line].chars().count();
            self.cursor.col = target_col.min(max_col);
            if self.preferred_col.is_none() {
                self.preferred_col = Some(target_col);
            }
        }
    }

    /// Move cursor down
    fn move_down(&mut self) {
        if self.cursor.line < self.lines.len() - 1 {
            self.cursor.line += 1;
            let target_col = self.preferred_col.unwrap_or(self.cursor.col);
            let max_col = self.lines[self.cursor.line].chars().count();
            self.cursor.col = target_col.min(max_col);
            if self.preferred_col.is_none() {
                self.preferred_col = Some(target_col);
            }
        }
    }

    /// Move to word left
    fn move_word_left(&mut self) {
        let chars: Vec<char> = self.lines[self.cursor.line].chars().collect();

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
        self.preferred_col = None;
    }

    /// Move to word right
    fn move_word_right(&mut self) {
        let chars: Vec<char> = self.lines[self.cursor.line].chars().collect();
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
        self.preferred_col = None;
    }

    /// Move to document start
    fn move_to_start(&mut self) {
        self.cursor = TextCursor::default();
        self.preferred_col = None;
    }

    /// Move to document end
    fn move_to_end(&mut self) {
        self.cursor.line = self.lines.len() - 1;
        self.cursor.col = self.lines[self.cursor.line].chars().count();
        self.preferred_col = None;
    }

    /// Move to line start
    fn move_to_line_start(&mut self) {
        self.cursor.col = 0;
        self.preferred_col = None;
    }

    /// Move to line end
    fn move_to_line_end(&mut self) {
        self.cursor.col = self.lines[self.cursor.line].chars().count();
        self.preferred_col = None;
    }

    /// Page up
    fn page_up(&mut self) {
        let page_size = self.viewport.1.saturating_sub(2) as usize;
        for _ in 0..page_size {
            if self.cursor.line == 0 {
                break;
            }
            self.move_up();
        }
    }

    /// Page down
    fn page_down(&mut self) {
        let page_size = self.viewport.1.saturating_sub(2) as usize;
        for _ in 0..page_size {
            if self.cursor.line >= self.lines.len() - 1 {
                break;
            }
            self.move_down();
        }
    }

    /// Scroll up
    fn scroll_up(&mut self) {
        self.scroll.0 = self.scroll.0.saturating_sub(1);
    }

    /// Scroll down
    fn scroll_down(&mut self) {
        if self.scroll.0 < self.lines.len().saturating_sub(1) {
            self.scroll.0 += 1;
        }
    }

    /// Select all text
    fn select_all(&mut self) {
        self.anchor = Some(TextCursor::default());
        self.cursor.line = self.lines.len() - 1;
        self.cursor.col = self.lines[self.cursor.line].chars().count();
    }

    /// Copy selection to clipboard
    fn copy(&mut self) {
        if let Some(text) = self.selected_text() {
            self.clipboard = text;
        }
    }

    /// Cut selection to clipboard
    fn cut(&mut self) {
        if let Some(text) = self.selected_text() {
            self.clipboard = text;
            self.delete_selection();
        }
    }

    /// Paste from clipboard
    fn paste(&mut self) {
        if !self.clipboard.is_empty() {
            self.delete_selection();

            let clipboard = self.clipboard.clone();
            for c in clipboard.chars() {
                if c == '\n' {
                    self.insert_newline();
                } else {
                    self.insert_char(c);
                }
            }
        }
    }

    /// Undo last operation
    fn undo(&mut self) {
        if let Some(op) = self.undo_stack.pop() {
            match &op {
                EditOp::Insert { pos, text } => {
                    let cursor = self.offset_to_pos(*pos);
                    let full_text = self.get_text();
                    self.set_text(&format!(
                        "{}{}",
                        &full_text[..*pos],
                        &full_text[*pos + text.len()..]
                    ));
                    self.cursor = cursor;
                }
                EditOp::Delete { pos, text } => {
                    let full_text = self.get_text();
                    self.set_text(&format!(
                        "{}{}{}",
                        &full_text[..*pos],
                        text,
                        &full_text[*pos..]
                    ));
                    self.cursor = self.offset_to_pos(*pos + text.len());
                }
                EditOp::Replace { pos, old_text, new_text } => {
                    let full_text = self.get_text();
                    self.set_text(&format!(
                        "{}{}{}",
                        &full_text[..*pos],
                        old_text,
                        &full_text[*pos + new_text.len()..]
                    ));
                    self.cursor = self.offset_to_pos(*pos + old_text.len());
                }
            }
            self.redo_stack.push(op);
        }
    }

    /// Redo last undone operation
    fn redo(&mut self) {
        if let Some(op) = self.redo_stack.pop() {
            match &op {
                EditOp::Insert { pos, text } => {
                    let full_text = self.get_text();
                    self.set_text(&format!(
                        "{}{}{}",
                        &full_text[..*pos],
                        text,
                        &full_text[*pos..]
                    ));
                    self.cursor = self.offset_to_pos(*pos + text.len());
                }
                EditOp::Delete { pos, text } => {
                    let cursor = self.offset_to_pos(*pos);
                    let full_text = self.get_text();
                    self.set_text(&format!(
                        "{}{}",
                        &full_text[..*pos],
                        &full_text[*pos + text.len()..]
                    ));
                    self.cursor = cursor;
                }
                EditOp::Replace { pos, old_text, new_text } => {
                    let full_text = self.get_text();
                    self.set_text(&format!(
                        "{}{}{}",
                        &full_text[..*pos],
                        new_text,
                        &full_text[*pos + old_text.len()..]
                    ));
                    self.cursor = self.offset_to_pos(*pos + new_text.len());
                }
            }
            self.undo_stack.push(op);
        }
    }

    /// Push operation to undo stack
    fn push_undo(&mut self, op: EditOp) {
        self.undo_stack.push(op);
        if self.undo_stack.len() > self.config.max_undo {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    /// Ensure cursor is visible
    fn ensure_cursor_visible(&mut self) {
        let height = self.viewport.1.saturating_sub(2) as usize;

        if self.cursor.line < self.scroll.0 {
            self.scroll.0 = self.cursor.line;
        } else if self.cursor.line >= self.scroll.0 + height {
            self.scroll.0 = self.cursor.line.saturating_sub(height - 1);
        }
    }

    /// Convert cursor position to byte offset
    fn cursor_byte_index(&self) -> usize {
        self.char_to_byte(&self.lines[self.cursor.line], self.cursor.col)
    }

    /// Convert cursor to absolute offset
    fn cursor_offset(&self) -> usize {
        self.pos_to_offset(self.cursor)
    }

    /// Convert position to offset
    fn pos_to_offset(&self, pos: TextCursor) -> usize {
        let mut offset = 0;
        for i in 0..pos.line {
            offset += self.lines[i].len() + 1; // +1 for newline
        }
        offset += self.char_to_byte(&self.lines[pos.line], pos.col);
        offset
    }

    /// Convert offset to position
    fn offset_to_pos(&self, offset: usize) -> TextCursor {
        let mut remaining = offset;
        for (line, content) in self.lines.iter().enumerate() {
            let line_len = content.len() + 1; // +1 for newline
            if remaining < line_len || line == self.lines.len() - 1 {
                let col = content
                    .char_indices()
                    .position(|(i, _)| i >= remaining)
                    .unwrap_or_else(|| content.chars().count());
                return TextCursor { line, col };
            }
            remaining -= line_len;
        }
        TextCursor::default()
    }

    /// Convert character index to byte index
    fn char_to_byte(&self, s: &str, char_idx: usize) -> usize {
        s.char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or(s.len())
    }

    /// Set viewport size
    pub fn set_viewport(&mut self, width: u16, height: u16) {
        self.viewport = (width, height);
    }

    /// Set focused state
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Get line count
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Render the text area
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if self.focused {
                Color::Cyan
            } else {
                Color::Rgb(86, 95, 137)
            }));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut render_lines: Vec<Line> = Vec::new();
        let height = inner.height as usize;
        let line_num_width = if self.config.show_line_numbers {
            self.lines.len().to_string().len() + 1
        } else {
            0
        };

        if self.is_empty() && !self.config.placeholder.is_empty() {
            // Show placeholder
            let placeholder_spans = vec![
                Span::styled(
                    " ".repeat(line_num_width),
                    Style::default().fg(Color::Rgb(86, 95, 137)),
                ),
                Span::styled(
                    &self.config.placeholder,
                    Style::default().fg(Color::Rgb(86, 95, 137)),
                ),
            ];
            render_lines.push(Line::from(placeholder_spans));
        } else {
            // Render visible lines
            let visible_end = (self.scroll.0 + height).min(self.lines.len());

            for i in self.scroll.0..visible_end {
                let mut spans = Vec::new();

                // Line number
                if self.config.show_line_numbers {
                    let num_str = format!("{:>width$} ", i + 1, width = line_num_width - 1);
                    spans.push(Span::styled(
                        num_str,
                        Style::default().fg(Color::Rgb(86, 95, 137)),
                    ));
                }

                let line = &self.lines[i];

                // Render line with cursor and selection
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

                render_lines.push(Line::from(spans));
            }
        }

        let paragraph = Paragraph::new(render_lines);
        frame.render_widget(paragraph, inner);
    }
}

impl Default for TextArea {
    fn default() -> Self {
        Self::new()
    }
}

/// Action returned by textarea
#[derive(Debug, Clone, PartialEq)]
pub enum TextAreaAction {
    /// No action
    None,
    /// Text changed
    Changed,
    /// Submit (Ctrl+Enter typically)
    Submit,
}

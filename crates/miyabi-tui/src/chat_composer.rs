//! Chat Composer - Input handling component
//!
//! Separates input handling from the main app following Codex patterns.
//! Features:
//! - Multi-line input support
//! - Command history navigation
//! - Auto-completion suggestions
//! - Vim-style keybindings (optional)
//! - Emacs keybindings
//! - Undo/Redo support
//! - Clipboard integration
//! - Syntax highlighting
//! - Search mode

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use std::collections::VecDeque;

use crate::wrapping::display_width;

/// Vim editing mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VimMode {
    /// Normal mode (navigation)
    Normal,
    /// Insert mode (typing)
    Insert,
    /// Visual mode (selection)
    Visual,
    /// Visual line mode
    VisualLine,
    /// Command-line mode (:)
    Command,
}

/// Keybinding style preference
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum KeybindingStyle {
    /// Standard keybindings
    #[default]
    Standard,
    /// Vim-style keybindings
    Vim,
    /// Emacs-style keybindings
    Emacs,
}


/// Edit operation for undo/redo
#[derive(Debug, Clone)]
pub enum EditOperation {
    /// Insert text at position
    Insert { pos: CursorPos, text: String },
    /// Delete text range
    Delete {
        start: CursorPos,
        end: CursorPos,
        text: String,
    },
    /// Replace text range
    Replace {
        start: CursorPos,
        end: CursorPos,
        old_text: String,
        new_text: String,
    },
    /// Batch operation (multiple operations as one undo unit)
    Batch(Vec<EditOperation>),
}

/// Undo/Redo stack
#[derive(Debug, Clone)]
pub struct UndoStack {
    /// Past operations (for undo)
    undo_stack: VecDeque<EditOperation>,
    /// Future operations (for redo)
    redo_stack: VecDeque<EditOperation>,
    /// Maximum stack size
    max_size: usize,
}

impl UndoStack {
    /// Create a new undo stack
    pub fn new(max_size: usize) -> Self {
        Self {
            undo_stack: VecDeque::with_capacity(max_size),
            redo_stack: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    /// Push an operation to the undo stack
    pub fn push(&mut self, op: EditOperation) {
        // Clear redo stack on new operation
        self.redo_stack.clear();

        // Add to undo stack
        if self.undo_stack.len() >= self.max_size {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(op);
    }

    /// Pop an operation for undo
    pub fn pop_undo(&mut self) -> Option<EditOperation> {
        self.undo_stack.pop_back()
    }

    /// Pop an operation for redo
    pub fn pop_redo(&mut self) -> Option<EditOperation> {
        self.redo_stack.pop_back()
    }

    /// Push to redo stack after undo
    pub fn push_redo(&mut self, op: EditOperation) {
        if self.redo_stack.len() >= self.max_size {
            self.redo_stack.pop_front();
        }
        self.redo_stack.push_back(op);
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new(100)
    }
}

/// Auto-completion suggestion
#[derive(Debug, Clone)]
pub struct Suggestion {
    /// The suggestion text
    pub text: String,
    /// Description or documentation
    pub description: Option<String>,
    /// Category/type
    pub category: SuggestionCategory,
    /// Match score (higher is better)
    pub score: i32,
}

/// Suggestion category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionCategory {
    /// Command (starts with /)
    Command,
    /// File path
    FilePath,
    /// Variable name
    Variable,
    /// Keyword
    Keyword,
    /// History item
    History,
}

/// Search state
#[derive(Debug, Clone)]
pub struct SearchState {
    /// Search query
    pub query: String,
    /// Current match index
    pub current_match: usize,
    /// All match positions
    pub matches: Vec<(CursorPos, CursorPos)>,
    /// Search direction
    pub direction: SearchDirection,
    /// Case sensitive
    pub case_sensitive: bool,
    /// Regex mode
    pub regex: bool,
}

impl Default for SearchState {
    fn default() -> Self {
        Self {
            query: String::new(),
            current_match: 0,
            matches: Vec::new(),
            direction: SearchDirection::Forward,
            case_sensitive: false,
            regex: false,
        }
    }
}

/// Search direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchDirection {
    Forward,
    Backward,
}

/// Clipboard manager
#[derive(Debug, Clone, Default)]
pub struct Clipboard {
    /// Internal clipboard content
    content: String,
}

impl Clipboard {
    /// Create a new clipboard
    pub fn new() -> Self {
        Self {
            content: String::new(),
        }
    }

    /// Copy text to clipboard
    pub fn copy(&mut self, text: String) {
        self.content = text.clone();
        // Try system clipboard
        #[cfg(feature = "clipboard")]
        {
            if let Ok(mut ctx) = arboard::Clipboard::new() {
                let _ = ctx.set_text(text);
            }
        }
    }

    /// Paste from clipboard
    pub fn paste(&mut self) -> String {
        // Try system clipboard first
        #[cfg(feature = "clipboard")]
        {
            if let Ok(mut ctx) = arboard::Clipboard::new() {
                if let Ok(text) = ctx.get_text() {
                    self.content = text.clone();
                    return text;
                }
            }
        }
        self.content.clone()
    }

    /// Get clipboard content without system access
    pub fn get(&self) -> &str {
        &self.content
    }
}

/// Register for vim-style yanking
#[derive(Debug, Clone, Default)]
pub struct VimRegisters {
    /// Unnamed register (default)
    unnamed: String,
    /// Named registers (a-z)
    named: [String; 26],
    /// Small delete register
    _small_delete: String,
    /// Numbered registers (0-9)
    numbered: [String; 10],
    /// Last search register
    search: String,
}

impl VimRegisters {
    /// Create new registers
    pub fn new() -> Self {
        Self::default()
    }

    /// Set unnamed register
    pub fn set_unnamed(&mut self, text: String) {
        // Shift numbered registers
        for i in (1..10).rev() {
            self.numbered[i] = self.numbered[i - 1].clone();
        }
        self.numbered[0] = text.clone();
        self.unnamed = text;
    }

    /// Get unnamed register
    pub fn get_unnamed(&self) -> &str {
        &self.unnamed
    }

    /// Set named register
    pub fn set_named(&mut self, reg: char, text: String) {
        if let Some(idx) = (reg as usize).checked_sub('a' as usize) {
            if idx < 26 {
                self.named[idx] = text;
            }
        }
    }

    /// Get named register
    pub fn get_named(&self, reg: char) -> Option<&str> {
        (reg as usize)
            .checked_sub('a' as usize)
            .filter(|&idx| idx < 26)
            .map(|idx| self.named[idx].as_str())
    }

    /// Set search register
    pub fn set_search(&mut self, text: String) {
        self.search = text;
    }

    /// Get search register
    pub fn get_search(&self) -> &str {
        &self.search
    }
}

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
    /// Keybinding style
    keybinding_style: KeybindingStyle,
    /// Vim mode (when using Vim keybindings)
    vim_mode: VimMode,
    /// Vim registers
    vim_registers: VimRegisters,
    /// Vim command buffer (for multi-key commands like "dd")
    vim_command_buffer: String,
    /// Vim repeat count (e.g., 3dw)
    vim_count: Option<usize>,
    /// Undo/Redo stack
    undo_stack: UndoStack,
    /// Clipboard manager
    clipboard: Clipboard,
    /// Search state
    search: SearchState,
    /// Rich suggestions with metadata
    rich_suggestions: Vec<Suggestion>,
    /// Show line numbers
    show_line_numbers: bool,
    /// Highlight current line
    highlight_current_line: bool,
    /// Auto-indent on newline
    auto_indent: bool,
    /// Bracket matching
    _bracket_matching: bool,
    /// Last matched bracket position
    matched_bracket: Option<CursorPos>,
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
            keybinding_style: KeybindingStyle::Standard,
            vim_mode: VimMode::Insert,
            vim_registers: VimRegisters::new(),
            vim_command_buffer: String::new(),
            vim_count: None,
            undo_stack: UndoStack::default(),
            clipboard: Clipboard::new(),
            search: SearchState::default(),
            rich_suggestions: Vec::new(),
            show_line_numbers: false,
            highlight_current_line: true,
            auto_indent: true,
            _bracket_matching: true,
            matched_bracket: None,
        }
    }

    /// Enable Vim keybindings
    pub fn with_vim_keybindings(mut self) -> Self {
        self.keybinding_style = KeybindingStyle::Vim;
        self.vim_mode = VimMode::Normal;
        self
    }

    /// Enable Emacs keybindings
    pub fn with_emacs_keybindings(mut self) -> Self {
        self.keybinding_style = KeybindingStyle::Emacs;
        self
    }

    /// Enable line numbers
    pub fn with_line_numbers(mut self) -> Self {
        self.show_line_numbers = true;
        self
    }

    /// Disable auto-indent
    pub fn without_auto_indent(mut self) -> Self {
        self.auto_indent = false;
        self
    }

    /// Get current vim mode
    pub fn vim_mode(&self) -> VimMode {
        self.vim_mode
    }

    /// Get keybinding style
    pub fn keybinding_style(&self) -> KeybindingStyle {
        self.keybinding_style
    }

    /// Check if in vim normal mode
    pub fn is_vim_normal(&self) -> bool {
        self.keybinding_style == KeybindingStyle::Vim && self.vim_mode == VimMode::Normal
    }

    /// Check if can undo
    pub fn can_undo(&self) -> bool {
        self.undo_stack.can_undo()
    }

    /// Check if can redo
    pub fn can_redo(&self) -> bool {
        self.undo_stack.can_redo()
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
        // Handle search mode
        if self.mode == InputMode::Search {
            return self.handle_search_key(key);
        }

        // Route based on keybinding style
        match self.keybinding_style {
            KeybindingStyle::Vim => self.handle_vim_key(key),
            KeybindingStyle::Emacs => self.handle_emacs_key(key),
            KeybindingStyle::Standard => self.handle_standard_key(key),
        }
    }

    /// Handle Vim keybinding mode
    fn handle_vim_key(&mut self, key: KeyEvent) -> ComposerAction {
        match self.vim_mode {
            VimMode::Normal | VimMode::Command => self.handle_vim_normal_key(key),
            VimMode::Visual | VimMode::VisualLine => self.handle_vim_visual_key(key),
            VimMode::Insert => {
                // Escape to normal mode
                if key.code == KeyCode::Esc {
                    self.vim_mode = VimMode::Normal;
                    return ComposerAction::None;
                }
                // Otherwise use standard handling
                self.handle_standard_key(key)
            }
        }
    }

    /// Handle Emacs keybindings
    fn handle_emacs_key(&mut self, key: KeyEvent) -> ComposerAction {
        // Emacs uses more control sequences
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('a') => {
                    self.cursor.col = 0;
                    return ComposerAction::None;
                }
                KeyCode::Char('e') => {
                    self.cursor.col = self.current_line().chars().count();
                    return ComposerAction::None;
                }
                KeyCode::Char('f') => {
                    self.move_cursor_right();
                    return ComposerAction::None;
                }
                KeyCode::Char('b') => {
                    self.move_cursor_left();
                    return ComposerAction::None;
                }
                KeyCode::Char('n') => {
                    self.move_cursor_down();
                    return ComposerAction::None;
                }
                KeyCode::Char('p') => {
                    self.move_cursor_up();
                    return ComposerAction::None;
                }
                KeyCode::Char('d') => {
                    self.delete();
                    return ComposerAction::None;
                }
                KeyCode::Char('h') => {
                    self.backspace();
                    return ComposerAction::None;
                }
                KeyCode::Char('k') => {
                    self.kill_to_end();
                    return ComposerAction::None;
                }
                KeyCode::Char('u') => {
                    self.clear_line();
                    return ComposerAction::None;
                }
                KeyCode::Char('w') => {
                    self.delete_word_backward();
                    return ComposerAction::None;
                }
                KeyCode::Char('y') => {
                    self.paste();
                    return ComposerAction::None;
                }
                KeyCode::Char('/') => {
                    self.undo();
                    return ComposerAction::None;
                }
                KeyCode::Char('g') => {
                    // Ctrl+G: cancel
                    return ComposerAction::Cancel;
                }
                KeyCode::Char('s') => {
                    // Ctrl+S: incremental search
                    self.start_search(SearchDirection::Forward);
                    return ComposerAction::None;
                }
                KeyCode::Char('r') => {
                    // Ctrl+R: reverse search
                    self.start_search(SearchDirection::Backward);
                    return ComposerAction::None;
                }
                _ => {}
            }
        }

        if key.modifiers.contains(KeyModifiers::ALT) {
            match key.code {
                KeyCode::Char('f') => {
                    self.move_word_right();
                    return ComposerAction::None;
                }
                KeyCode::Char('b') => {
                    self.move_word_left();
                    return ComposerAction::None;
                }
                KeyCode::Char('d') => {
                    self.delete_word_forward();
                    return ComposerAction::None;
                }
                KeyCode::Char('<') => {
                    self.cursor = CursorPos { line: 0, col: 0 };
                    return ComposerAction::None;
                }
                KeyCode::Char('>') => {
                    self.cursor.line = self.lines.len() - 1;
                    self.cursor.col = self.lines[self.cursor.line].chars().count();
                    return ComposerAction::None;
                }
                _ => {}
            }
        }

        // Fall through to standard handling
        self.handle_standard_key(key)
    }

    /// Handle search mode keys
    fn handle_search_key(&mut self, key: KeyEvent) -> ComposerAction {
        match key.code {
            KeyCode::Esc => {
                self.mode = InputMode::Normal;
                self.search.query.clear();
            }
            KeyCode::Enter => {
                self.perform_search();
                self.mode = InputMode::Normal;
                // Store in search register for vim
                self.vim_registers.set_search(self.search.query.clone());
            }
            KeyCode::Backspace => {
                self.search.query.pop();
            }
            KeyCode::Char(c) => {
                self.search.query.push(c);
                // Incremental search
                self.perform_search();
            }
            _ => {}
        }
        ComposerAction::None
    }

    /// Handle standard keybindings
    fn handle_standard_key(&mut self, key: KeyEvent) -> ComposerAction {
        // Handle suggestions first
        if self.show_suggestions {
            match key.code {
                KeyCode::Tab | KeyCode::Down => {
                    self.suggestion_index =
                        (self.suggestion_index + 1) % self.suggestions.len().max(1);
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
            KeyCode::Esc => ComposerAction::Cancel,
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
            let byte_idx =
                self.char_to_byte_index(&self.lines[self.cursor.line], self.cursor.col - 1);
            let next_byte_idx =
                self.char_to_byte_index(&self.lines[self.cursor.line], self.cursor.col);
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
            let next_byte_idx =
                self.char_to_byte_index(&self.lines[self.cursor.line], self.cursor.col + 1);
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
                "help",
                "clear",
                "history",
                "exit",
                "quit",
                "model",
                "temperature",
                "tools",
                "context",
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
        // Determine title based on mode
        let title = self.get_mode_title();
        let border_color = self.get_border_color();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(Span::styled(
                title,
                Style::default().fg(Color::Rgb(192, 202, 245)),
            ));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Calculate line number width if showing
        let line_num_width = if self.show_line_numbers {
            self.lines.len().to_string().len() + 1
        } else {
            0
        };

        // Render input lines
        let mut lines: Vec<Line> = Vec::new();

        if self.is_empty() && self.mode != InputMode::Search {
            // Show placeholder with mode indicator
            let mode_indicator = self.get_mode_indicator();
            lines.push(Line::from(vec![
                Span::styled(mode_indicator, Style::default().fg(Color::Cyan)),
                Span::styled(
                    &self.placeholder,
                    Style::default().fg(Color::Rgb(86, 95, 137)),
                ),
            ]));
        } else if self.mode == InputMode::Search {
            // Show search prompt
            let search_prefix = if self.search.direction == SearchDirection::Forward {
                "/"
            } else {
                "?"
            };
            lines.push(Line::from(vec![
                Span::styled(
                    search_prefix,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(&self.search.query, Style::default().fg(Color::White)),
                Span::styled("█", Style::default().fg(Color::Yellow)),
            ]));

            // Show match info
            if let Some(info) = self.search_info() {
                lines.push(Line::from(vec![Span::styled(
                    format!("  {} matches", info),
                    Style::default().fg(Color::Rgb(86, 95, 137)),
                )]));
            }
        } else {
            for (i, line) in self.lines.iter().enumerate() {
                let mut spans = Vec::new();

                // Line numbers
                if self.show_line_numbers {
                    let line_num = format!("{:>width$} ", i + 1, width = line_num_width - 1);
                    let num_style = if i == self.cursor.line {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Rgb(86, 95, 137))
                    };
                    spans.push(Span::styled(line_num, num_style));
                }

                // Mode indicator for first line
                if i == 0 {
                    spans.push(Span::styled(
                        self.get_mode_indicator(),
                        Style::default().fg(Color::Cyan),
                    ));
                } else {
                    spans.push(Span::styled("  ", Style::default()));
                }

                // Render line content with highlighting
                let rendered_spans = self.render_line_content(i, line);
                spans.extend(rendered_spans);

                // Add line with potential current line highlight
                let line_style = if i == self.cursor.line && self.highlight_current_line {
                    Style::default().bg(Color::Rgb(40, 42, 54))
                } else {
                    Style::default()
                };

                lines.push(Line::from(spans).style(line_style));
            }
        }

        let paragraph = Paragraph::new(lines).scroll((self.scroll_offset as u16, 0));
        frame.render_widget(paragraph, inner);

        // Render mode line at bottom
        if self.keybinding_style == KeybindingStyle::Vim {
            self.render_vim_status(frame, area);
        }

        // Render suggestions popup
        if self.show_suggestions && !self.suggestions.is_empty() {
            self.render_suggestions(frame, area);
        }

        // Render rich suggestions if available
        if !self.rich_suggestions.is_empty() && self.show_suggestions {
            self.render_rich_suggestions(frame, area);
        }
    }

    /// Get mode title for the input box
    fn get_mode_title(&self) -> String {
        match self.keybinding_style {
            KeybindingStyle::Vim => {
                let mode = match self.vim_mode {
                    VimMode::Normal => "NORMAL",
                    VimMode::Insert => "INSERT",
                    VimMode::Visual => "VISUAL",
                    VimMode::VisualLine => "V-LINE",
                    VimMode::Command => "COMMAND",
                };
                format!(" Input [{}] ", mode)
            }
            KeybindingStyle::Emacs => " Input [Emacs] ".to_string(),
            KeybindingStyle::Standard => " Input ".to_string(),
        }
    }

    /// Get border color based on mode
    fn get_border_color(&self) -> Color {
        if !self.focused {
            return Color::Rgb(86, 95, 137);
        }

        match self.keybinding_style {
            KeybindingStyle::Vim => match self.vim_mode {
                VimMode::Normal => Color::Blue,
                VimMode::Insert => Color::Green,
                VimMode::Visual | VimMode::VisualLine => Color::Magenta,
                VimMode::Command => Color::Yellow,
            },
            _ => Color::Cyan,
        }
    }

    /// Get mode indicator character
    fn get_mode_indicator(&self) -> &'static str {
        match self.keybinding_style {
            KeybindingStyle::Vim => match self.vim_mode {
                VimMode::Normal => "● ",
                VimMode::Insert => "› ",
                VimMode::Visual | VimMode::VisualLine => "▌ ",
                VimMode::Command => ": ",
            },
            _ => "› ",
        }
    }

    /// Render line content with syntax highlighting and decorations
    fn render_line_content(&self, line_idx: usize, line: &str) -> Vec<Span<'static>> {
        let chars: Vec<char> = line.chars().collect();
        let mut spans = Vec::new();

        if chars.is_empty() {
            // Empty line with cursor
            if line_idx == self.cursor.line && self.focused {
                spans.push(Span::styled(
                    " ",
                    Style::default().bg(Color::Cyan).fg(Color::Black),
                ));
            }
            return spans;
        }

        let mut current_pos = 0;

        while current_pos < chars.len() {
            let char_style = self.get_char_style(line_idx, current_pos, chars[current_pos]);

            // Check if this is cursor position
            let is_cursor =
                line_idx == self.cursor.line && current_pos == self.cursor.col && self.focused;

            let style = if is_cursor {
                Style::default().bg(Color::Cyan).fg(Color::Black)
            } else {
                char_style
            };

            spans.push(Span::styled(chars[current_pos].to_string(), style));
            current_pos += 1;
        }

        // Add cursor at end of line if needed
        if line_idx == self.cursor.line && self.cursor.col >= chars.len() && self.focused {
            spans.push(Span::styled(
                " ",
                Style::default().bg(Color::Cyan).fg(Color::Black),
            ));
        }

        spans
    }

    /// Get style for a character (syntax highlighting)
    fn get_char_style(&self, line_idx: usize, col: usize, ch: char) -> Style {
        // Check if in selection
        if let Some(ref sel) = self.selection {
            if self.is_in_selection(line_idx, col, sel) {
                return Style::default().bg(Color::Rgb(68, 71, 90)).fg(Color::White);
            }
        }

        // Check if in search match
        for (start, end) in &self.search.matches {
            if line_idx == start.line && col >= start.col && col < end.col {
                return Style::default().bg(Color::Yellow).fg(Color::Black);
            }
        }

        // Check for bracket matching
        if let Some(matched) = self.matched_bracket {
            if line_idx == matched.line && col == matched.col {
                return Style::default().bg(Color::Rgb(68, 71, 90)).fg(Color::Cyan);
            }
        }
        if line_idx == self.cursor.line && col == self.cursor.col
            && matches!(ch, '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>') {
                return Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD);
            }

        // Basic syntax highlighting
        match ch {
            // Brackets
            '(' | ')' | '[' | ']' | '{' | '}' => Style::default().fg(Color::Rgb(189, 147, 249)),
            // Operators
            '+' | '-' | '*' | '/' | '=' | '<' | '>' | '!' | '&' | '|' | '^' | '%' => {
                Style::default().fg(Color::Rgb(255, 121, 198))
            }
            // Punctuation
            '.' | ',' | ':' | ';' | '@' | '#' => Style::default().fg(Color::Rgb(139, 233, 253)),
            // Quotes
            '"' | '\'' | '`' => Style::default().fg(Color::Rgb(241, 250, 140)),
            // Numbers
            c if c.is_ascii_digit() => Style::default().fg(Color::Rgb(189, 147, 249)),
            // Default
            _ => Style::default().fg(Color::Rgb(248, 248, 242)),
        }
    }

    /// Check if position is in selection
    fn is_in_selection(&self, line: usize, col: usize, sel: &Selection) -> bool {
        let (start, end) = if sel.start.line < sel.end.line
            || (sel.start.line == sel.end.line && sel.start.col <= sel.end.col)
        {
            (sel.start, sel.end)
        } else {
            (sel.end, sel.start)
        };

        if line < start.line || line > end.line {
            return false;
        }

        if line == start.line && line == end.line {
            col >= start.col && col < end.col
        } else if line == start.line {
            col >= start.col
        } else if line == end.line {
            col < end.col
        } else {
            true
        }
    }

    /// Render Vim status line
    fn render_vim_status(&self, frame: &mut Frame, area: Rect) {
        if area.height < 2 {
            return;
        }

        let status_area = Rect {
            x: area.x + 1,
            y: area.y + area.height - 1,
            width: area.width.saturating_sub(2),
            height: 1,
        };

        // Mode indicator
        let (mode_text, mode_color) = match self.vim_mode {
            VimMode::Normal => ("-- NORMAL --", Color::Blue),
            VimMode::Insert => ("-- INSERT --", Color::Green),
            VimMode::Visual => ("-- VISUAL --", Color::Magenta),
            VimMode::VisualLine => ("-- VISUAL LINE --", Color::Magenta),
            VimMode::Command => (":", Color::Yellow),
        };

        // Position info
        let pos_info = format!("{}:{} ", self.cursor.line + 1, self.cursor.col + 1);

        // Command buffer display
        let cmd_display = if !self.vim_command_buffer.is_empty() {
            format!(" {}", self.vim_command_buffer)
        } else if let Some(count) = self.vim_count {
            format!(" {}", count)
        } else {
            String::new()
        };

        let status = Line::from(vec![
            Span::styled(
                mode_text,
                Style::default().fg(mode_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(cmd_display, Style::default().fg(Color::Yellow)),
            Span::raw(" "),
            Span::styled(pos_info, Style::default().fg(Color::Rgb(86, 95, 137))),
        ]);

        frame.render_widget(Paragraph::new(status), status_area);
    }

    /// Render rich suggestions with descriptions
    fn render_rich_suggestions(&self, frame: &mut Frame, area: Rect) {
        let popup_height = (self.rich_suggestions.len() + 2).min(12) as u16;
        let popup_width = self
            .rich_suggestions
            .iter()
            .map(|s| {
                let desc_len = s.description.as_ref().map(|d| d.len()).unwrap_or(0);
                s.text.len() + desc_len + 10
            })
            .max()
            .unwrap_or(30)
            .max(30) as u16;

        let popup_area = Rect {
            x: area.x + 2,
            y: area.y.saturating_sub(popup_height),
            width: popup_width.min(area.width - 4),
            height: popup_height,
        };

        frame.render_widget(Clear, popup_area);

        let items: Vec<Line> = self
            .rich_suggestions
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let is_selected = i == self.suggestion_index;

                // Category icon
                let icon = match s.category {
                    SuggestionCategory::Command => "⌘ ",
                    SuggestionCategory::FilePath => "📁 ",
                    SuggestionCategory::Variable => "λ ",
                    SuggestionCategory::Keyword => "◆ ",
                    SuggestionCategory::History => "⏱ ",
                };

                let base_style = if is_selected {
                    Style::default().bg(Color::Rgb(68, 71, 90))
                } else {
                    Style::default()
                };

                let mut spans = vec![
                    Span::styled(icon, base_style.fg(Color::Cyan)),
                    Span::styled(
                        &s.text,
                        base_style
                            .fg(if is_selected {
                                Color::White
                            } else {
                                Color::Rgb(248, 248, 242)
                            })
                            .add_modifier(if is_selected {
                                Modifier::BOLD
                            } else {
                                Modifier::empty()
                            }),
                    ),
                ];

                if let Some(desc) = &s.description {
                    spans.push(Span::styled(
                        format!("  {}", desc),
                        base_style.fg(Color::Rgb(98, 114, 164)),
                    ));
                }

                Line::from(spans)
            })
            .collect();

        let title = format!(" Suggestions ({}) ", self.rich_suggestions.len());
        let popup = Paragraph::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(98, 114, 164)))
                .title(Span::styled(title, Style::default().fg(Color::Cyan))),
        );

        frame.render_widget(popup, popup_area);
    }

    /// Render suggestions popup
    fn render_suggestions(&self, frame: &mut Frame, area: Rect) {
        let popup_height = (self.suggestions.len() + 2).min(10) as u16;
        let popup_width = self
            .suggestions
            .iter()
            .map(|s| display_width(s))
            .max()
            .unwrap_or(20)
            .max(20) as u16
            + 4;

        let popup_area = Rect {
            x: area.x + 2,
            y: area.y.saturating_sub(popup_height),
            width: popup_width.min(area.width - 4),
            height: popup_height,
        };

        frame.render_widget(Clear, popup_area);

        let items: Vec<Line> = self
            .suggestions
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let style = if i == self.suggestion_index {
                    Style::default()
                        .bg(Color::Rgb(86, 95, 137))
                        .fg(Color::White)
                } else {
                    Style::default().fg(Color::Rgb(192, 202, 245))
                };
                Line::from(Span::styled(format!(" {} ", s), style))
            })
            .collect();

        let popup = Paragraph::new(items).block(
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

    /// Perform undo
    pub fn undo(&mut self) -> bool {
        if let Some(op) = self.undo_stack.pop_undo() {
            self.apply_undo_operation(&op);
            self.undo_stack.push_redo(op);
            true
        } else {
            false
        }
    }

    /// Perform redo
    pub fn redo(&mut self) -> bool {
        if let Some(op) = self.undo_stack.pop_redo() {
            self.apply_redo_operation(&op);
            self.undo_stack.push(op);
            true
        } else {
            false
        }
    }

    /// Apply undo operation (reverse the operation)
    fn apply_undo_operation(&mut self, op: &EditOperation) {
        match op {
            EditOperation::Insert { pos, text } => {
                // Undo insert by deleting
                self.cursor = *pos;
                for _ in 0..text.chars().count() {
                    self.delete();
                }
            }
            EditOperation::Delete { start, text, .. } => {
                // Undo delete by inserting
                self.cursor = *start;
                for c in text.chars() {
                    if c == '\n' {
                        self.insert_newline();
                    } else {
                        self.insert_char(c);
                    }
                }
            }
            EditOperation::Replace {
                start, old_text, ..
            } => {
                // Undo replace by replacing with old text
                self.cursor = *start;
                self.set_input(old_text);
            }
            EditOperation::Batch(ops) => {
                // Undo in reverse order
                for op in ops.iter().rev() {
                    self.apply_undo_operation(op);
                }
            }
        }
    }

    /// Apply redo operation (reapply the operation)
    fn apply_redo_operation(&mut self, op: &EditOperation) {
        match op {
            EditOperation::Insert { pos, text } => {
                self.cursor = *pos;
                for c in text.chars() {
                    if c == '\n' {
                        self.insert_newline();
                    } else {
                        self.insert_char(c);
                    }
                }
            }
            EditOperation::Delete { start, end, .. } => {
                self.cursor = *start;
                // Delete from start to end
                while self.cursor.line < end.line
                    || (self.cursor.line == end.line && self.cursor.col < end.col)
                {
                    self.delete();
                }
            }
            EditOperation::Replace {
                start, new_text, ..
            } => {
                self.cursor = *start;
                self.set_input(new_text);
            }
            EditOperation::Batch(ops) => {
                for op in ops {
                    self.apply_redo_operation(op);
                }
            }
        }
    }

    /// Copy selection to clipboard
    pub fn copy_selection(&mut self) -> Option<String> {
        if let Some(sel) = &self.selection {
            let text = self.get_selected_text(sel);
            self.clipboard.copy(text.clone());
            self.vim_registers.set_unnamed(text.clone());
            Some(text)
        } else {
            None
        }
    }

    /// Cut selection to clipboard
    pub fn cut_selection(&mut self) -> Option<String> {
        if let Some(sel) = self.selection.take() {
            let text = self.get_selected_text(&sel);
            self.clipboard.copy(text.clone());
            self.vim_registers.set_unnamed(text.clone());
            self.delete_selection(&sel);
            Some(text)
        } else {
            None
        }
    }

    /// Paste from clipboard
    pub fn paste(&mut self) {
        let text = self.clipboard.paste();
        for c in text.chars() {
            if c == '\n' {
                self.insert_newline();
            } else {
                self.insert_char(c);
            }
        }
    }

    /// Get selected text
    fn get_selected_text(&self, sel: &Selection) -> String {
        let (start, end) = if sel.start.line < sel.end.line
            || (sel.start.line == sel.end.line && sel.start.col <= sel.end.col)
        {
            (sel.start, sel.end)
        } else {
            (sel.end, sel.start)
        };

        if start.line == end.line {
            let line = &self.lines[start.line];
            let chars: Vec<char> = line.chars().collect();
            chars[start.col..end.col].iter().collect()
        } else {
            let mut result = String::new();
            // First line
            let first_line = &self.lines[start.line];
            let chars: Vec<char> = first_line.chars().collect();
            result.push_str(&chars[start.col..].iter().collect::<String>());
            result.push('\n');
            // Middle lines
            for line_idx in (start.line + 1)..end.line {
                result.push_str(&self.lines[line_idx]);
                result.push('\n');
            }
            // Last line
            let last_line = &self.lines[end.line];
            let chars: Vec<char> = last_line.chars().collect();
            result.push_str(&chars[..end.col].iter().collect::<String>());
            result
        }
    }

    /// Delete selected text
    fn delete_selection(&mut self, sel: &Selection) {
        let (start, end) = if sel.start.line < sel.end.line
            || (sel.start.line == sel.end.line && sel.start.col <= sel.end.col)
        {
            (sel.start, sel.end)
        } else {
            (sel.end, sel.start)
        };

        if start.line == end.line {
            let byte_start = self.char_to_byte_index(&self.lines[start.line], start.col);
            let byte_end = self.char_to_byte_index(&self.lines[start.line], end.col);
            self.lines[start.line].replace_range(byte_start..byte_end, "");
        } else {
            // Get the parts to keep
            let first_part: String = self.lines[start.line].chars().take(start.col).collect();
            let last_part: String = self.lines[end.line].chars().skip(end.col).collect();

            // Remove lines between
            for _ in start.line..=end.line {
                if start.line < self.lines.len() {
                    self.lines.remove(start.line);
                }
            }

            // Insert merged line
            self.lines.insert(start.line, first_part + &last_part);
        }

        self.cursor = start;
    }

    /// Handle Vim normal mode keys
    pub fn handle_vim_normal_key(&mut self, key: KeyEvent) -> ComposerAction {
        // Build repeat count
        if let KeyCode::Char(c) = key.code {
            if c.is_ascii_digit() && (self.vim_count.is_some() || c != '0') {
                let digit = c.to_digit(10).unwrap() as usize;
                self.vim_count = Some(self.vim_count.unwrap_or(0) * 10 + digit);
                return ComposerAction::None;
            }
        }

        let count = self.vim_count.take().unwrap_or(1);

        match key.code {
            // Mode switches
            KeyCode::Char('i') => {
                self.vim_mode = VimMode::Insert;
            }
            KeyCode::Char('I') => {
                self.cursor.col = 0;
                self.vim_mode = VimMode::Insert;
            }
            KeyCode::Char('a') => {
                self.move_cursor_right();
                self.vim_mode = VimMode::Insert;
            }
            KeyCode::Char('A') => {
                self.cursor.col = self.current_line().chars().count();
                self.vim_mode = VimMode::Insert;
            }
            KeyCode::Char('o') => {
                self.cursor.col = self.current_line().chars().count();
                self.insert_newline();
                self.vim_mode = VimMode::Insert;
            }
            KeyCode::Char('O') => {
                self.cursor.col = 0;
                self.insert_newline();
                self.move_cursor_up();
                self.vim_mode = VimMode::Insert;
            }
            KeyCode::Char('v') => {
                self.vim_mode = VimMode::Visual;
                self.selection = Some(Selection {
                    start: self.cursor,
                    end: self.cursor,
                });
            }
            KeyCode::Char('V') => {
                self.vim_mode = VimMode::VisualLine;
                self.selection = Some(Selection {
                    start: CursorPos {
                        line: self.cursor.line,
                        col: 0,
                    },
                    end: CursorPos {
                        line: self.cursor.line,
                        col: self.current_line().chars().count(),
                    },
                });
            }

            // Navigation
            KeyCode::Char('h') | KeyCode::Left => {
                for _ in 0..count {
                    self.move_cursor_left();
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                for _ in 0..count {
                    self.move_cursor_right();
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                for _ in 0..count {
                    self.move_cursor_down();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                for _ in 0..count {
                    self.move_cursor_up();
                }
            }
            KeyCode::Char('w') => {
                for _ in 0..count {
                    self.move_word_right();
                }
            }
            KeyCode::Char('b') => {
                for _ in 0..count {
                    self.move_word_left();
                }
            }
            KeyCode::Char('0') | KeyCode::Home => {
                self.cursor.col = 0;
            }
            KeyCode::Char('$') | KeyCode::End => {
                self.cursor.col = self.current_line().chars().count();
            }
            KeyCode::Char('^') => {
                // First non-whitespace
                let line = self.current_line();
                self.cursor.col = line.chars().position(|c| !c.is_whitespace()).unwrap_or(0);
            }
            KeyCode::Char('g') => {
                // gg - go to beginning
                self.vim_command_buffer.push('g');
                if self.vim_command_buffer == "gg" {
                    self.cursor = CursorPos { line: 0, col: 0 };
                    self.vim_command_buffer.clear();
                }
            }
            KeyCode::Char('G') => {
                // Go to end
                self.cursor.line = self.lines.len() - 1;
                self.cursor.col = 0;
            }

            // Editing
            KeyCode::Char('x') => {
                for _ in 0..count {
                    self.delete();
                }
            }
            KeyCode::Char('X') => {
                for _ in 0..count {
                    self.backspace();
                }
            }
            KeyCode::Char('d') => {
                self.vim_command_buffer.push('d');
                if self.vim_command_buffer == "dd" {
                    // Delete line
                    for _ in 0..count {
                        if self.lines.len() > 1 {
                            let text = self.lines.remove(self.cursor.line);
                            self.vim_registers.set_unnamed(text);
                            if self.cursor.line >= self.lines.len() {
                                self.cursor.line = self.lines.len() - 1;
                            }
                        } else {
                            let text = std::mem::take(&mut self.lines[0]);
                            self.vim_registers.set_unnamed(text);
                        }
                    }
                    self.vim_command_buffer.clear();
                }
            }
            KeyCode::Char('y') => {
                self.vim_command_buffer.push('y');
                if self.vim_command_buffer == "yy" {
                    // Yank line
                    let text = self.lines[self.cursor.line].clone();
                    self.vim_registers.set_unnamed(text);
                    self.vim_command_buffer.clear();
                }
            }
            KeyCode::Char('p') => {
                // Paste after
                let text = self.vim_registers.get_unnamed().to_string();
                self.move_cursor_right();
                for c in text.chars() {
                    if c == '\n' {
                        self.insert_newline();
                    } else {
                        self.insert_char(c);
                    }
                }
            }
            KeyCode::Char('P') => {
                // Paste before
                let text = self.vim_registers.get_unnamed().to_string();
                for c in text.chars() {
                    if c == '\n' {
                        self.insert_newline();
                    } else {
                        self.insert_char(c);
                    }
                }
            }
            KeyCode::Char('u') => {
                for _ in 0..count {
                    self.undo();
                }
            }
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                for _ in 0..count {
                    self.redo();
                }
            }

            // Search
            KeyCode::Char('/') => {
                self.mode = InputMode::Search;
                self.search.direction = SearchDirection::Forward;
                self.search.query.clear();
            }
            KeyCode::Char('?') => {
                self.mode = InputMode::Search;
                self.search.direction = SearchDirection::Backward;
                self.search.query.clear();
            }
            KeyCode::Char('n') => {
                self.find_next_match();
            }
            KeyCode::Char('N') => {
                self.find_prev_match();
            }

            // Other
            KeyCode::Esc => {
                self.vim_command_buffer.clear();
                self.vim_count = None;
            }
            KeyCode::Enter => {
                if !self.is_empty() {
                    return ComposerAction::Submit;
                }
            }

            _ => {
                self.vim_command_buffer.clear();
            }
        }

        ComposerAction::None
    }

    /// Handle Vim visual mode keys
    pub fn handle_vim_visual_key(&mut self, key: KeyEvent) -> ComposerAction {
        match key.code {
            KeyCode::Esc => {
                self.vim_mode = VimMode::Normal;
                self.selection = None;
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.move_cursor_left();
                self.update_visual_selection();
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.move_cursor_right();
                self.update_visual_selection();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_cursor_down();
                self.update_visual_selection();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_cursor_up();
                self.update_visual_selection();
            }
            KeyCode::Char('y') => {
                // Yank selection
                self.copy_selection();
                self.vim_mode = VimMode::Normal;
                self.selection = None;
            }
            KeyCode::Char('d') | KeyCode::Char('x') => {
                // Delete selection
                self.cut_selection();
                self.vim_mode = VimMode::Normal;
            }
            KeyCode::Char('c') => {
                // Change selection
                self.cut_selection();
                self.vim_mode = VimMode::Insert;
            }
            _ => {}
        }
        ComposerAction::None
    }

    /// Update visual selection end position
    fn update_visual_selection(&mut self) {
        if let Some(ref mut sel) = self.selection {
            sel.end = self.cursor;
        }
    }

    /// Start search
    pub fn start_search(&mut self, direction: SearchDirection) {
        self.mode = InputMode::Search;
        self.search.direction = direction;
        self.search.query.clear();
        self.search.matches.clear();
    }

    /// Perform search
    pub fn perform_search(&mut self) {
        self.search.matches.clear();

        if self.search.query.is_empty() {
            return;
        }

        let query = if self.search.case_sensitive {
            self.search.query.clone()
        } else {
            self.search.query.to_lowercase()
        };

        for (line_idx, line) in self.lines.iter().enumerate() {
            let search_line = if self.search.case_sensitive {
                line.clone()
            } else {
                line.to_lowercase()
            };

            let mut start = 0;
            while let Some(pos) = search_line[start..].find(&query) {
                let col_start = search_line[..start + pos].chars().count();
                let col_end = col_start + query.chars().count();

                self.search.matches.push((
                    CursorPos {
                        line: line_idx,
                        col: col_start,
                    },
                    CursorPos {
                        line: line_idx,
                        col: col_end,
                    },
                ));

                start += pos + 1;
            }
        }

        // Jump to first match
        if !self.search.matches.is_empty() {
            self.search.current_match = 0;
            self.cursor = self.search.matches[0].0;
        }
    }

    /// Find next search match
    pub fn find_next_match(&mut self) {
        if self.search.matches.is_empty() {
            return;
        }

        self.search.current_match = (self.search.current_match + 1) % self.search.matches.len();
        self.cursor = self.search.matches[self.search.current_match].0;
    }

    /// Find previous search match
    pub fn find_prev_match(&mut self) {
        if self.search.matches.is_empty() {
            return;
        }

        if self.search.current_match == 0 {
            self.search.current_match = self.search.matches.len() - 1;
        } else {
            self.search.current_match -= 1;
        }
        self.cursor = self.search.matches[self.search.current_match].0;
    }

    /// Find matching bracket
    pub fn find_matching_bracket(&mut self) {
        let line = &self.lines[self.cursor.line];
        let chars: Vec<char> = line.chars().collect();

        if self.cursor.col >= chars.len() {
            self.matched_bracket = None;
            return;
        }

        let current = chars[self.cursor.col];
        let (target, forward) = match current {
            '(' => (')', true),
            ')' => ('(', false),
            '[' => (']', true),
            ']' => ('[', false),
            '{' => ('}', true),
            '}' => ('{', false),
            '<' => ('>', true),
            '>' => ('<', false),
            _ => {
                self.matched_bracket = None;
                return;
            }
        };

        let mut depth = 1;
        let mut pos = self.cursor;

        if forward {
            // Search forward
            pos.col += 1;
            while pos.line < self.lines.len() {
                let line_chars: Vec<char> = self.lines[pos.line].chars().collect();
                while pos.col < line_chars.len() {
                    if line_chars[pos.col] == current {
                        depth += 1;
                    } else if line_chars[pos.col] == target {
                        depth -= 1;
                        if depth == 0 {
                            self.matched_bracket = Some(pos);
                            return;
                        }
                    }
                    pos.col += 1;
                }
                pos.line += 1;
                pos.col = 0;
            }
        } else {
            // Search backward
            if pos.col > 0 {
                pos.col -= 1;
            } else if pos.line > 0 {
                pos.line -= 1;
                pos.col = self.lines[pos.line].chars().count().saturating_sub(1);
            } else {
                self.matched_bracket = None;
                return;
            }

            loop {
                let line_chars: Vec<char> = self.lines[pos.line].chars().collect();
                while pos.col < line_chars.len() {
                    if line_chars[pos.col] == current {
                        depth += 1;
                    } else if line_chars[pos.col] == target {
                        depth -= 1;
                        if depth == 0 {
                            self.matched_bracket = Some(pos);
                            return;
                        }
                    }
                    if pos.col == 0 {
                        break;
                    }
                    pos.col -= 1;
                }
                if pos.line == 0 {
                    break;
                }
                pos.line -= 1;
                pos.col = self.lines[pos.line].chars().count().saturating_sub(1);
            }
        }

        self.matched_bracket = None;
    }

    /// Get search match count info
    pub fn search_info(&self) -> Option<String> {
        if self.search.matches.is_empty() {
            None
        } else {
            Some(format!(
                "{}/{}",
                self.search.current_match + 1,
                self.search.matches.len()
            ))
        }
    }

    /// Add rich suggestion
    pub fn add_suggestion(
        &mut self,
        text: impl Into<String>,
        category: SuggestionCategory,
        description: Option<String>,
    ) {
        self.rich_suggestions.push(Suggestion {
            text: text.into(),
            description,
            category,
            score: 0,
        });
    }

    /// Clear suggestions
    pub fn clear_suggestions(&mut self) {
        self.suggestions.clear();
        self.rich_suggestions.clear();
        self.show_suggestions = false;
    }

    /// Get auto-indent string for new line
    #[allow(dead_code)]
    fn get_auto_indent(&self) -> String {
        if !self.auto_indent {
            return String::new();
        }

        let line = &self.lines[self.cursor.line];
        let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();

        // Check for additional indent after { or :
        if let Some(last_char) = line.trim_end().chars().last() {
            if last_char == '{' || last_char == ':' {
                return indent + "    ";
            }
        }

        indent
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_composer_creation() {
        let composer = ChatComposer::new();
        assert!(composer.is_empty());
        assert_eq!(composer.line_count(), 1);
    }

    #[test]
    fn test_insert_char() {
        let mut composer = ChatComposer::new();
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        composer.handle_key(key);
        assert_eq!(composer.get_input(), "a");
    }

    #[test]
    fn test_insert_multiple_chars() {
        let mut composer = ChatComposer::new();
        for c in "hello".chars() {
            let key = KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
            composer.handle_key(key);
        }
        assert_eq!(composer.get_input(), "hello");
    }

    #[test]
    fn test_backspace() {
        let mut composer = ChatComposer::new();
        for c in "ab".chars() {
            let key = KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
            composer.handle_key(key);
        }
        let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        composer.handle_key(key);
        assert_eq!(composer.get_input(), "a");
    }

    #[test]
    fn test_submit() {
        let mut composer = ChatComposer::new();
        for c in "test".chars() {
            let key = KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
            composer.handle_key(key);
        }
        let action = composer.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(action, ComposerAction::Submit);
    }

    #[test]
    fn test_submit_empty() {
        let mut composer = ChatComposer::new();
        let action = composer.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(action, ComposerAction::None);
    }

    #[test]
    fn test_cancel() {
        let mut composer = ChatComposer::new();
        let action = composer.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(action, ComposerAction::Cancel);
    }

    #[test]
    fn test_multiline_input() {
        let mut composer = ChatComposer::new();
        for c in "line1".chars() {
            composer.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        composer.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));
        for c in "line2".chars() {
            composer.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        assert_eq!(composer.get_input(), "line1\nline2");
        assert_eq!(composer.line_count(), 2);
    }

    #[test]
    fn test_cursor_movement_left_right() {
        let mut composer = ChatComposer::new();
        for c in "abc".chars() {
            composer.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        // Cursor at position 3
        composer.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        // Cursor at position 2
        composer.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        assert_eq!(composer.get_input(), "abxc");
    }

    #[test]
    fn test_cursor_home_end() {
        let mut composer = ChatComposer::new();
        for c in "hello".chars() {
            composer.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        composer.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
        composer.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        assert_eq!(composer.get_input(), "xhello");
    }

    #[test]
    fn test_history() {
        let mut composer = ChatComposer::new();

        // Submit first message
        for c in "msg1".chars() {
            composer.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        composer.submit();

        // Submit second message
        for c in "msg2".chars() {
            composer.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        composer.submit();

        // Navigate back
        composer.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(composer.get_input(), "msg2");

        composer.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(composer.get_input(), "msg1");
    }

    #[test]
    fn test_command_mode() {
        let mut composer = ChatComposer::new();
        composer.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        assert_eq!(composer.mode, InputMode::Command);
    }

    #[test]
    fn test_clear() {
        let mut composer = ChatComposer::new();
        for c in "test".chars() {
            composer.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        composer.clear();
        assert!(composer.is_empty());
    }

    #[test]
    fn test_ctrl_u_clear_line() {
        let mut composer = ChatComposer::new();
        for c in "test".chars() {
            composer.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        composer.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert_eq!(composer.get_input(), "");
    }

    #[test]
    fn test_ctrl_k_kill_to_end() {
        let mut composer = ChatComposer::new();
        for c in "hello".chars() {
            composer.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        // Move to position 2
        composer.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
        composer.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        composer.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        // Kill to end
        composer.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL));
        assert_eq!(composer.get_input(), "he");
    }

    #[test]
    fn test_placeholder() {
        let composer = ChatComposer::new().placeholder("Custom placeholder");
        assert_eq!(composer.placeholder, "Custom placeholder");
    }

    #[test]
    fn test_max_history() {
        let mut composer = ChatComposer::new().max_history(2);

        for i in 0..5 {
            for c in format!("msg{}", i).chars() {
                composer.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
            }
            composer.submit();
        }

        assert_eq!(composer.history.len(), 2);
    }

    #[test]
    fn test_delete_key() {
        let mut composer = ChatComposer::new();
        for c in "ab".chars() {
            composer.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        composer.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
        composer.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
        assert_eq!(composer.get_input(), "b");
    }

    #[test]
    fn test_unicode_support() {
        let mut composer = ChatComposer::new();
        for c in "日本語".chars() {
            composer.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        assert_eq!(composer.get_input(), "日本語");
    }

    #[test]
    fn test_cursor_position_struct() {
        let pos = CursorPos { line: 1, col: 5 };
        assert_eq!(pos.line, 1);
        assert_eq!(pos.col, 5);
    }

    #[test]
    fn test_input_mode_enum() {
        assert_ne!(InputMode::Normal, InputMode::Command);
        assert_ne!(InputMode::Command, InputMode::Search);
    }

    #[test]
    fn test_composer_action_enum() {
        assert_ne!(ComposerAction::None, ComposerAction::Submit);
        assert_ne!(ComposerAction::Submit, ComposerAction::Cancel);
    }

    #[test]
    fn test_focused_state() {
        let mut composer = ChatComposer::new();
        assert!(composer.focused);
        composer.set_focused(false);
        assert!(!composer.focused);
    }
}

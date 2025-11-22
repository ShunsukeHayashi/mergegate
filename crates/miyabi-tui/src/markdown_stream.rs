//! Streaming Markdown Renderer
//!
//! This module provides a streaming markdown renderer that can display
//! partial content as it arrives from an LLM response.

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// State of the markdown stream
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamState {
    /// No content yet
    Idle,
    /// Actively receiving content
    Streaming,
    /// All content received
    Complete,
}

impl Default for StreamState {
    fn default() -> Self {
        Self::Idle
    }
}

/// Buffer for accumulating streamed content
#[derive(Debug, Clone, Default)]
pub struct StreamBuffer {
    /// Raw text content
    content: String,
    /// Whether the content has changed since last render
    dirty: bool,
}

impl StreamBuffer {
    /// Create a new empty buffer
    pub fn new() -> Self {
        Self::default()
    }

    /// Push new content to the buffer
    pub fn push_str(&mut self, s: &str) {
        self.content.push_str(s);
        self.dirty = true;
    }

    /// Get the current content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Check if buffer has changed
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark buffer as clean (rendered)
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.content.clear();
        self.dirty = true;
    }

    /// Get content length
    pub fn len(&self) -> usize {
        self.content.len()
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}

/// Cursor position in the content
#[derive(Debug, Clone, Copy, Default)]
pub struct CursorPosition {
    /// Line number (0-indexed)
    pub line: usize,
    /// Column position (0-indexed)
    pub column: usize,
}

impl CursorPosition {
    /// Create new position
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// Scroll state for viewport management
#[derive(Debug, Clone)]
pub struct ScrollState {
    /// Current scroll offset (line number at top of viewport)
    pub offset: usize,
    /// Total number of content lines
    pub total_lines: usize,
    /// Viewport height in lines
    pub viewport_height: usize,
    /// Whether auto-scroll is enabled
    pub auto_scroll: bool,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self {
            offset: 0,
            total_lines: 0,
            viewport_height: 20,
            auto_scroll: true,
        }
    }
}

impl ScrollState {
    /// Create new scroll state
    pub fn new() -> Self {
        Self::default()
    }

    /// Set viewport height
    pub fn set_viewport_height(&mut self, height: usize) {
        self.viewport_height = height.max(1);
    }

    /// Get maximum scroll offset
    pub fn max_offset(&self) -> usize {
        self.total_lines.saturating_sub(self.viewport_height)
    }

    /// Check if we can scroll up
    pub fn can_scroll_up(&self) -> bool {
        self.offset > 0
    }

    /// Check if we can scroll down
    pub fn can_scroll_down(&self) -> bool {
        self.offset < self.max_offset()
    }

    /// Scroll up by n lines
    pub fn scroll_up(&mut self, n: usize) {
        self.offset = self.offset.saturating_sub(n);
        self.auto_scroll = false;
    }

    /// Scroll down by n lines
    pub fn scroll_down(&mut self, n: usize) {
        self.offset = (self.offset + n).min(self.max_offset());
        self.auto_scroll = false;
    }

    /// Scroll to top
    pub fn scroll_to_top(&mut self) {
        self.offset = 0;
        self.auto_scroll = false;
    }

    /// Scroll to bottom
    pub fn scroll_to_bottom(&mut self) {
        self.offset = self.max_offset();
        self.auto_scroll = true;
    }

    /// Page up
    pub fn page_up(&mut self) {
        self.scroll_up(self.viewport_height.saturating_sub(2));
    }

    /// Page down
    pub fn page_down(&mut self) {
        self.scroll_down(self.viewport_height.saturating_sub(2));
    }

    /// Update total lines and auto-scroll if enabled
    pub fn update_total_lines(&mut self, total: usize) {
        self.total_lines = total;
        if self.auto_scroll {
            self.offset = self.max_offset();
        }
    }

    /// Get scroll percentage (0.0 - 1.0)
    pub fn scroll_percentage(&self) -> f32 {
        if self.max_offset() == 0 {
            1.0
        } else {
            self.offset as f32 / self.max_offset() as f32
        }
    }
}

/// Streaming markdown renderer
#[derive(Debug, Clone)]
pub struct MarkdownStream {
    /// Current state
    state: StreamState,
    /// Content buffer
    buffer: StreamBuffer,
    /// Scroll state
    scroll: ScrollState,
    /// Cursor position
    cursor: CursorPosition,
}

impl Default for MarkdownStream {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownStream {
    /// Create a new markdown stream
    pub fn new() -> Self {
        Self {
            state: StreamState::Idle,
            buffer: StreamBuffer::new(),
            scroll: ScrollState::new(),
            cursor: CursorPosition::default(),
        }
    }

    /// Get current state
    pub fn state(&self) -> &StreamState {
        &self.state
    }

    /// Check if streaming is active
    pub fn is_streaming(&self) -> bool {
        self.state == StreamState::Streaming
    }

    /// Check if stream is complete
    pub fn is_complete(&self) -> bool {
        self.state == StreamState::Complete
    }

    /// Push new content chunk
    pub fn push_str(&mut self, s: &str) {
        if self.state == StreamState::Idle {
            self.state = StreamState::Streaming;
        }
        self.buffer.push_str(s);

        // Update cursor position to end of content
        let content = self.buffer.content();
        let line_count = content.lines().count();
        let last_line_len = content.lines().last().map(|l| l.len()).unwrap_or(0);
        self.cursor = CursorPosition::new(
            line_count.saturating_sub(1),
            last_line_len,
        );
    }

    /// Mark stream as complete
    pub fn complete(&mut self) {
        self.state = StreamState::Complete;
    }

    /// Reset the stream
    pub fn reset(&mut self) {
        self.state = StreamState::Idle;
        self.buffer.clear();
        self.scroll = ScrollState::new();
        self.cursor = CursorPosition::default();
    }

    /// Get current content
    pub fn content(&self) -> &str {
        self.buffer.content()
    }

    /// Get content length
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Get scroll state reference
    pub fn scroll(&self) -> &ScrollState {
        &self.scroll
    }

    /// Get mutable scroll state
    pub fn scroll_mut(&mut self) -> &mut ScrollState {
        &mut self.scroll
    }

    /// Get cursor position
    pub fn cursor(&self) -> CursorPosition {
        self.cursor
    }

    /// Set viewport height
    pub fn set_viewport_height(&mut self, height: usize) {
        self.scroll.set_viewport_height(height);
    }

    /// Set scroll offset (legacy API compatibility)
    pub fn set_scroll(&mut self, offset: usize) {
        self.scroll.offset = offset;
        self.scroll.auto_scroll = false;
    }

    /// Enable auto-scroll
    pub fn enable_auto_scroll(&mut self) {
        self.scroll.auto_scroll = true;
    }

    /// Get scroll offset (legacy API compatibility)
    pub fn scroll_offset(&self) -> usize {
        self.scroll.offset
    }

    /// Scroll up by n lines
    pub fn scroll_up(&mut self, n: usize) {
        self.scroll.scroll_up(n);
    }

    /// Scroll down by n lines
    pub fn scroll_down(&mut self, n: usize) {
        self.scroll.scroll_down(n);
    }

    /// Scroll to top
    pub fn scroll_to_top(&mut self) {
        self.scroll.scroll_to_top();
    }

    /// Scroll to bottom
    pub fn scroll_to_bottom(&mut self) {
        self.scroll.scroll_to_bottom();
    }

    /// Page up
    pub fn page_up(&mut self) {
        self.scroll.page_up();
    }

    /// Page down
    pub fn page_down(&mut self) {
        self.scroll.page_down();
    }

    /// Get total line count
    pub fn line_count(&self) -> usize {
        self.scroll.total_lines
    }

    /// Render content to Ratatui lines
    ///
    /// This is a basic implementation that converts markdown to styled spans.
    /// For full markdown support, use pulldown-cmark in the incremental parser.
    pub fn render(&mut self) -> Vec<Line<'static>> {
        let content = self.buffer.content().to_string();
        self.buffer.mark_clean();

        if content.is_empty() {
            return vec![Line::from(Span::styled(
                "Waiting for response...",
                Style::default().fg(Color::DarkGray),
            ))];
        }

        let mut lines = Vec::new();
        let mut in_code_block = false;
        let mut code_lang = String::new();

        for line in content.lines() {
            // Handle code blocks
            if line.starts_with("```") {
                if in_code_block {
                    // End code block
                    in_code_block = false;
                    lines.push(Line::from(Span::styled(
                        "```",
                        Style::default().fg(Color::DarkGray),
                    )));
                } else {
                    // Start code block
                    in_code_block = true;
                    code_lang = line.trim_start_matches('`').to_string();
                    let display = if code_lang.is_empty() {
                        "```".to_string()
                    } else {
                        format!("```{}", code_lang)
                    };
                    lines.push(Line::from(Span::styled(
                        display,
                        Style::default().fg(Color::DarkGray),
                    )));
                }
                continue;
            }

            if in_code_block {
                // Code content
                lines.push(Line::from(Span::styled(
                    line.to_string(),
                    Style::default().fg(Color::Green),
                )));
                continue;
            }

            // Headers
            if line.starts_with("# ") {
                lines.push(Line::from(Span::styled(
                    line.to_string(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )));
            } else if line.starts_with("## ") {
                lines.push(Line::from(Span::styled(
                    line.to_string(),
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                )));
            } else if line.starts_with("### ") {
                lines.push(Line::from(Span::styled(
                    line.to_string(),
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                )));
            }
            // Lists
            else if line.starts_with("- ") || line.starts_with("* ") {
                lines.push(Line::from(Span::styled(
                    line.to_string(),
                    Style::default().fg(Color::Yellow),
                )));
            }
            // Numbered lists
            else if line.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
                && line.contains(". ")
            {
                lines.push(Line::from(Span::styled(
                    line.to_string(),
                    Style::default().fg(Color::Yellow),
                )));
            }
            // Regular text
            else {
                lines.push(Line::from(Span::raw(line.to_string())));
            }
        }

        // Add cursor if streaming
        if self.state == StreamState::Streaming {
            if let Some(last) = lines.last_mut() {
                last.spans.push(Span::styled(
                    "▌",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::SLOW_BLINK),
                ));
            }
        }

        // Update scroll state with total line count
        self.scroll.update_total_lines(lines.len());

        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_state_transitions() {
        let mut stream = MarkdownStream::new();
        assert_eq!(stream.state(), &StreamState::Idle);

        stream.push_str("Hello");
        assert_eq!(stream.state(), &StreamState::Streaming);

        stream.complete();
        assert_eq!(stream.state(), &StreamState::Complete);
    }

    #[test]
    fn test_buffer_operations() {
        let mut buffer = StreamBuffer::new();
        assert!(buffer.is_empty());

        buffer.push_str("Hello");
        assert!(!buffer.is_empty());
        assert_eq!(buffer.len(), 5);
        assert!(buffer.is_dirty());

        buffer.mark_clean();
        assert!(!buffer.is_dirty());

        buffer.push_str(" World");
        assert!(buffer.is_dirty());
        assert_eq!(buffer.content(), "Hello World");
    }

    #[test]
    fn test_render_basic() {
        let mut stream = MarkdownStream::new();
        stream.push_str("# Hello\n\nThis is text.");

        let lines = stream.render();
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_render_code_block() {
        let mut stream = MarkdownStream::new();
        stream.push_str("```rust\nfn main() {}\n```");

        let lines = stream.render();
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_reset() {
        let mut stream = MarkdownStream::new();
        stream.push_str("Content");
        stream.complete();

        stream.reset();
        assert_eq!(stream.state(), &StreamState::Idle);
        assert!(stream.is_empty());
    }

    #[test]
    fn test_scroll_state() {
        let mut scroll = ScrollState::new();
        scroll.total_lines = 100;
        scroll.viewport_height = 20;

        assert_eq!(scroll.max_offset(), 80);
        assert!(scroll.can_scroll_down());
        assert!(!scroll.can_scroll_up());

        scroll.scroll_down(10);
        assert_eq!(scroll.offset, 10);
        assert!(scroll.can_scroll_up());
        assert!(!scroll.auto_scroll);

        scroll.scroll_up(5);
        assert_eq!(scroll.offset, 5);

        scroll.scroll_to_bottom();
        assert_eq!(scroll.offset, 80);
        assert!(scroll.auto_scroll);

        scroll.scroll_to_top();
        assert_eq!(scroll.offset, 0);
        assert!(!scroll.auto_scroll);
    }

    #[test]
    fn test_page_navigation() {
        let mut scroll = ScrollState::new();
        scroll.total_lines = 100;
        scroll.viewport_height = 20;

        scroll.page_down();
        assert_eq!(scroll.offset, 18); // viewport_height - 2

        scroll.page_up();
        assert_eq!(scroll.offset, 0);
    }

    #[test]
    fn test_scroll_percentage() {
        let mut scroll = ScrollState::new();
        scroll.total_lines = 100;
        scroll.viewport_height = 20;

        assert_eq!(scroll.scroll_percentage(), 0.0);

        scroll.scroll_to_bottom();
        assert_eq!(scroll.scroll_percentage(), 1.0);

        scroll.offset = 40;
        assert_eq!(scroll.scroll_percentage(), 0.5);
    }

    #[test]
    fn test_cursor_position_tracking() {
        let mut stream = MarkdownStream::new();

        stream.push_str("Line 1");
        assert_eq!(stream.cursor().line, 0);
        assert_eq!(stream.cursor().column, 6);

        stream.push_str("\nLine 2\nLine 3");
        assert_eq!(stream.cursor().line, 2);
        assert_eq!(stream.cursor().column, 6);
    }

    #[test]
    fn test_stream_scroll_methods() {
        let mut stream = MarkdownStream::new();
        stream.push_str("Line 1\nLine 2\nLine 3\nLine 4\nLine 5");
        stream.render(); // Updates total_lines
        stream.set_viewport_height(3);

        assert_eq!(stream.line_count(), 5);
        assert!(stream.scroll().can_scroll_down());

        stream.scroll_down(2);
        assert_eq!(stream.scroll_offset(), 2);

        stream.scroll_up(1);
        assert_eq!(stream.scroll_offset(), 1);

        stream.scroll_to_top();
        assert_eq!(stream.scroll_offset(), 0);

        stream.scroll_to_bottom();
        assert_eq!(stream.scroll_offset(), 2); // 5 - 3 = 2
    }

    #[test]
    fn test_auto_scroll_on_new_content() {
        let mut stream = MarkdownStream::new();
        stream.set_viewport_height(3);

        // Add content and render
        stream.push_str("Line 1\nLine 2\nLine 3\nLine 4\nLine 5");
        let lines = stream.render();

        // Auto-scroll should put us at bottom
        assert_eq!(lines.len(), 5);
        assert_eq!(stream.scroll_offset(), 2); // 5 - 3

        // Add more content
        stream.push_str("\nLine 6");
        let lines = stream.render();

        // Should auto-scroll to show new content
        assert_eq!(lines.len(), 6);
        assert_eq!(stream.scroll_offset(), 3); // 6 - 3
    }
}

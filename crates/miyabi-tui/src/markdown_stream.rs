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

/// Streaming markdown renderer
#[derive(Debug, Clone)]
pub struct MarkdownStream {
    /// Current state
    state: StreamState,
    /// Content buffer
    buffer: StreamBuffer,
    /// Scroll offset (for auto-scroll)
    scroll_offset: usize,
    /// Whether to auto-scroll on new content
    auto_scroll: bool,
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
            scroll_offset: 0,
            auto_scroll: true,
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
    }

    /// Mark stream as complete
    pub fn complete(&mut self) {
        self.state = StreamState::Complete;
    }

    /// Reset the stream
    pub fn reset(&mut self) {
        self.state = StreamState::Idle;
        self.buffer.clear();
        self.scroll_offset = 0;
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

    /// Set scroll offset
    pub fn set_scroll(&mut self, offset: usize) {
        self.scroll_offset = offset;
        self.auto_scroll = false;
    }

    /// Enable auto-scroll
    pub fn enable_auto_scroll(&mut self) {
        self.auto_scroll = true;
    }

    /// Get scroll offset
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
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
}

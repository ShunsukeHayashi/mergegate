//! History Cell Abstraction - Premium Card Design
//!
//! Following OpenAI Codex TUI patterns with functional colors:
//! - Cyan: User input, tool names
//! - Green: Success
//! - Red: Errors
//! - Magenta: Assistant/Miyabi brand
//! - Dim: Secondary, timestamps

use std::any::Any;
use std::sync::Mutex;

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::markdown_stream::MarkdownStream;
use crate::wrapping::wrap_text;

/// Trait for renderable history items
pub trait HistoryCell: Send + Sync {
    fn render(&self, width: u16) -> Vec<Line<'static>>;
    fn timestamp(&self) -> &str;
    fn is_streaming(&self) -> bool {
        false
    }
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// User message cell - Cyan accented card
pub struct UserMessageCell {
    pub content: String,
    pub timestamp: String,
}

impl HistoryCell for UserMessageCell {
    fn render(&self, width: u16) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let content_width = (width as usize).saturating_sub(4);

        // Header line with role and timestamp
        lines.push(Line::from(vec![
            Span::styled(
                "You ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                self.timestamp.clone(),
                Style::default().add_modifier(Modifier::DIM),
            ),
        ]));

        // Content with proper text wrapping
        for line in self.content.lines() {
            let wrapped = wrap_text(line, content_width);
            for wrapped_line in wrapped {
                lines.push(wrapped_line);
            }
        }

        lines
    }

    fn timestamp(&self) -> &str {
        &self.timestamp
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Assistant message cell - Magenta accented card with markdown
pub struct AssistantMessageCell {
    stream: Mutex<MarkdownStream>,
    pub timestamp: String,
    pub streaming: bool,
}

impl AssistantMessageCell {
    /// Create a new assistant message cell
    pub fn new(timestamp: String) -> Self {
        Self {
            stream: Mutex::new(MarkdownStream::new()),
            timestamp,
            streaming: true,
        }
    }

    /// Push content to the stream
    pub fn push_str(&self, s: &str) {
        if let Ok(mut stream) = self.stream.lock() {
            stream.push_str(s);
        }
    }

    /// Mark streaming as complete
    pub fn set_complete(&mut self) {
        self.streaming = false;
        if let Ok(mut stream) = self.stream.lock() {
            stream.complete();
        }
    }

    /// Get the current content as string
    pub fn content(&self) -> String {
        self.stream
            .lock()
            .map(|s| s.content().to_string())
            .unwrap_or_default()
    }

    /// Set content directly (for non-streaming messages)
    pub fn set_content(&self, content: &str) {
        if let Ok(mut stream) = self.stream.lock() {
            stream.push_str(content);
        }
    }

    /// Check if content is empty
    pub fn is_empty(&self) -> bool {
        self.stream
            .lock()
            .map(|s| s.content().is_empty())
            .unwrap_or(true)
    }
}

impl HistoryCell for AssistantMessageCell {
    fn render(&self, _width: u16) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        // Header with streaming indicator
        let header_text = if self.streaming {
            "Assistant ●"
        } else {
            "Assistant"
        };
        let header_style = Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD);

        lines.push(Line::from(vec![
            Span::styled(header_text, header_style),
            Span::styled(" ", Style::default()),
            Span::styled(
                self.timestamp.clone(),
                Style::default().add_modifier(Modifier::DIM),
            ),
        ]));

        // Markdown rendered content using MarkdownStream
        let md_lines = if let Ok(mut stream) = self.stream.lock() {
            stream.render()
        } else {
            Vec::new()
        };

        if md_lines.is_empty() && self.streaming {
            lines.push(Line::from(Span::styled(
                "...",
                Style::default().add_modifier(Modifier::DIM),
            )));
        } else {
            for md_line in md_lines {
                lines.push(md_line);
            }
        }

        lines
    }

    fn timestamp(&self) -> &str {
        &self.timestamp
    }

    fn is_streaming(&self) -> bool {
        self.streaming
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Tool result cell - Green/Red based on success
pub struct ToolResultCell {
    pub tool_name: String,
    pub content: String,
    pub timestamp: String,
    pub execution_time_ms: u64,
    pub success: bool,
    /// Optional truncated input preview for debugging
    pub input_preview: Option<String>,
}

impl ToolResultCell {
    /// Create a new tool result cell with input preview
    pub fn new(
        tool_name: String,
        content: String,
        timestamp: String,
        execution_time_ms: u64,
        success: bool,
        input: Option<&serde_json::Value>,
    ) -> Self {
        let input_preview = input.map(|v| {
            let s = serde_json::to_string(v).unwrap_or_default();
            if s.len() > 100 {
                format!("{}...", &s[..97])
            } else {
                s
            }
        });

        Self {
            tool_name,
            content,
            timestamp,
            execution_time_ms,
            success,
            input_preview,
        }
    }
}

impl HistoryCell for ToolResultCell {
    fn render(&self, width: u16) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let content_width = (width as usize).saturating_sub(4);
        let status_color = if self.success {
            Color::Green
        } else {
            Color::Red
        };

        // Header with status icon
        let icon = if self.success { "✔" } else { "✗" };
        let time_str = format!("{}ms", self.execution_time_ms);

        lines.push(Line::from(vec![
            Span::styled(format!("{} ", icon), Style::default().fg(status_color)),
            Span::styled(
                self.tool_name.clone(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {}", time_str),
                Style::default().add_modifier(Modifier::DIM),
            ),
        ]));

        // Input preview (if available)
        if let Some(input) = &self.input_preview {
            let preview = format!("→ {}", input);
            let truncated = if preview.len() > content_width {
                format!("{}...", &preview[..content_width.saturating_sub(3)])
            } else {
                preview
            };
            lines.push(Line::from(Span::styled(
                truncated,
                Style::default().fg(Color::DarkGray),
            )));
        }

        // Content with proper text wrapping
        for line in self.content.lines() {
            let wrapped = wrap_text(line, content_width);
            for wrapped_line in wrapped {
                lines.push(wrapped_line);
            }
        }

        lines
    }

    fn timestamp(&self) -> &str {
        &self.timestamp
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// System message cell
pub struct SystemMessageCell {
    pub content: String,
    pub timestamp: String,
    pub message_type: SystemMessageType,
}

#[derive(Clone, Copy)]
pub enum SystemMessageType {
    Info,
    Warning,
    Error,
    Success,
}

impl HistoryCell for SystemMessageCell {
    fn render(&self, _width: u16) -> Vec<Line<'static>> {
        let (icon, color) = match self.message_type {
            SystemMessageType::Info => ("ℹ", Color::Cyan),
            SystemMessageType::Warning => ("⚠", Color::Yellow),
            SystemMessageType::Error => ("✗", Color::Red),
            SystemMessageType::Success => ("✔", Color::Green),
        };

        vec![Line::from(vec![
            Span::styled(format!("{} ", icon), Style::default().fg(color)),
            Span::styled(
                self.content.clone(),
                Style::default().add_modifier(Modifier::DIM),
            ),
        ])]
    }

    fn timestamp(&self) -> &str {
        &self.timestamp
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

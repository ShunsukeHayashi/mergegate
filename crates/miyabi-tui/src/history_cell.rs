//! History Cell Abstraction - Premium Card Design
//!
//! Following OpenAI Codex TUI patterns with functional colors:
//! - Cyan: User input, tool names
//! - Green: Success
//! - Red: Errors
//! - Magenta: Assistant/Miyabi brand
//! - Dim: Secondary, timestamps

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::markdown_render::MarkdownRenderer;
use crate::wrapping::wrap_text;

/// Trait for renderable history items
pub trait HistoryCell: Send + Sync {
    fn render(&self, width: u16) -> Vec<Line<'static>>;
    fn timestamp(&self) -> &str;
    fn is_streaming(&self) -> bool {
        false
    }
}

/// User message cell - Cyan accented card
pub struct UserMessageCell {
    pub content: String,
    pub timestamp: String,
}

impl HistoryCell for UserMessageCell {
    fn render(&self, width: u16) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let inner_width = (width as usize).saturating_sub(6).min(70);
        let border = "─".repeat(inner_width);

        // Top border
        lines.push(Line::from(vec![
            Span::styled("┌", Style::default().fg(Color::Cyan)),
            Span::styled(border.clone(), Style::default().fg(Color::Cyan)),
            Span::styled("┐", Style::default().fg(Color::Cyan)),
        ]));

        // Header
        lines.push(Line::from(vec![
            Span::styled("│ ", Style::default().fg(Color::Cyan)),
            Span::styled("You", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("{:>width$}", self.timestamp, width = inner_width - 4),
                Style::default().add_modifier(Modifier::DIM)
            ),
            Span::styled(" │", Style::default().fg(Color::Cyan)),
        ]));

        // Content with proper text wrapping
        let content_width = inner_width.saturating_sub(2);
        for line in self.content.lines() {
            let wrapped = wrap_text(line, content_width);
            for wrapped_line in wrapped {
                let content_str: String = wrapped_line.spans.iter()
                    .map(|s| s.content.as_ref())
                    .collect();
                let padded = format!("{:<width$}", content_str, width = content_width);
                lines.push(Line::from(vec![
                    Span::styled("│ ", Style::default().fg(Color::Cyan)),
                    Span::raw(padded),
                    Span::styled(" │", Style::default().fg(Color::Cyan)),
                ]));
            }
        }

        // Bottom border
        lines.push(Line::from(vec![
            Span::styled("└", Style::default().fg(Color::Cyan)),
            Span::styled(border, Style::default().fg(Color::Cyan)),
            Span::styled("┘", Style::default().fg(Color::Cyan)),
        ]));

        lines
    }

    fn timestamp(&self) -> &str {
        &self.timestamp
    }
}

/// Assistant message cell - Magenta accented card with markdown
pub struct AssistantMessageCell {
    pub content: String,
    pub timestamp: String,
    pub streaming: bool,
}

impl HistoryCell for AssistantMessageCell {
    fn render(&self, width: u16) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let inner_width = (width as usize).saturating_sub(6).min(70);
        let border = "─".repeat(inner_width);

        // Top border
        lines.push(Line::from(vec![
            Span::styled("┌", Style::default().fg(Color::Magenta)),
            Span::styled(border.clone(), Style::default().fg(Color::Magenta)),
            Span::styled("┐", Style::default().fg(Color::Magenta)),
        ]));

        // Header with streaming indicator
        let header_text = if self.streaming { "Assistant ●" } else { "Assistant" };
        let header_style = Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD);

        lines.push(Line::from(vec![
            Span::styled("│ ", Style::default().fg(Color::Magenta)),
            Span::styled(header_text, header_style),
            Span::styled(
                format!("{:>width$}", self.timestamp, width = inner_width - header_text.len() - 1),
                Style::default().add_modifier(Modifier::DIM)
            ),
            Span::styled(" │", Style::default().fg(Color::Magenta)),
        ]));

        // Markdown rendered content
        let renderer = MarkdownRenderer::new();
        let md_lines = renderer.render(&self.content);

        if md_lines.is_empty() && self.streaming {
            lines.push(Line::from(vec![
                Span::styled("│ ", Style::default().fg(Color::Magenta)),
                Span::styled("...", Style::default().add_modifier(Modifier::DIM)),
                Span::styled(
                    format!("{:>width$}", "", width = inner_width - 5),
                    Style::default()
                ),
                Span::styled(" │", Style::default().fg(Color::Magenta)),
            ]));
        } else {
            for md_line in md_lines {
                let mut content_spans = vec![
                    Span::styled("│ ", Style::default().fg(Color::Magenta)),
                ];
                content_spans.extend(md_line.spans);
                content_spans.push(Span::styled(" │", Style::default().fg(Color::Magenta)));
                lines.push(Line::from(content_spans));
            }
        }

        // Bottom border
        lines.push(Line::from(vec![
            Span::styled("└", Style::default().fg(Color::Magenta)),
            Span::styled(border, Style::default().fg(Color::Magenta)),
            Span::styled("┘", Style::default().fg(Color::Magenta)),
        ]));

        lines
    }

    fn timestamp(&self) -> &str {
        &self.timestamp
    }

    fn is_streaming(&self) -> bool {
        self.streaming
    }
}

/// Tool result cell - Green/Red based on success
pub struct ToolResultCell {
    pub tool_name: String,
    pub content: String,
    pub timestamp: String,
    pub execution_time_ms: u64,
    pub success: bool,
}

impl HistoryCell for ToolResultCell {
    fn render(&self, width: u16) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let inner_width = (width as usize).saturating_sub(8).min(68);
        let border = "═".repeat(inner_width);
        let border_color = if self.success { Color::Green } else { Color::Red };

        // Top border (double line for tool)
        lines.push(Line::from(vec![
            Span::styled("  ╔", Style::default().fg(border_color)),
            Span::styled(border.clone(), Style::default().fg(border_color)),
            Span::styled("╗", Style::default().fg(border_color)),
        ]));

        // Header with status icon
        let icon = if self.success { "✔" } else { "✗" };
        let time_str = format!("{}ms", self.execution_time_ms);

        lines.push(Line::from(vec![
            Span::styled("  ║ ", Style::default().fg(border_color)),
            Span::styled(format!("{} ", icon), Style::default().fg(border_color)),
            Span::styled(self.tool_name.clone(), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("{:>width$}", time_str, width = inner_width - self.tool_name.len() - 4),
                Style::default().add_modifier(Modifier::DIM)
            ),
            Span::styled(" ║", Style::default().fg(border_color)),
        ]));

        // Content with proper text wrapping
        let content_width = inner_width.saturating_sub(2);
        for line in self.content.lines() {
            let wrapped = wrap_text(line, content_width);
            for wrapped_line in wrapped {
                let content_str: String = wrapped_line.spans.iter()
                    .map(|s| s.content.as_ref())
                    .collect();
                let padded = format!("{:<width$}", content_str, width = content_width);
                lines.push(Line::from(vec![
                    Span::styled("  ║ ", Style::default().fg(border_color)),
                    Span::styled(padded, Style::default().add_modifier(Modifier::DIM)),
                    Span::styled(" ║", Style::default().fg(border_color)),
                ]));
            }
        }

        // Bottom border
        lines.push(Line::from(vec![
            Span::styled("  ╚", Style::default().fg(border_color)),
            Span::styled(border, Style::default().fg(border_color)),
            Span::styled("╝", Style::default().fg(border_color)),
        ]));

        lines
    }

    fn timestamp(&self) -> &str {
        &self.timestamp
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

        vec![
            Line::from(vec![
                Span::styled(format!("{} ", icon), Style::default().fg(color)),
                Span::styled(self.content.clone(), Style::default().add_modifier(Modifier::DIM)),
            ]),
        ]
    }

    fn timestamp(&self) -> &str {
        &self.timestamp
    }
}

//! Streaming markdown widget placeholder.
//!
//! This file is the landing zone for `markdown_stream` integration to keep
//! rendering logic out of `history_cell.rs`.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::markdown_render::MarkdownRenderer;
use crate::markdown_stream::MarkdownStream;
use crate::ui::colors;

/// Render a markdown stream into the provided frame area.
pub fn render_stream(frame: &mut Frame, area: Rect, stream: &MarkdownStream) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors::BORDER))
        .title("Markdown");

    let inner_height = area.height.saturating_sub(2) as usize;

    let mut lines = if stream.is_empty() {
        vec![Line::from(Span::styled(
            "Waiting for response...",
            Style::default().fg(colors::COMMENT),
        ))]
    } else {
        let renderer = MarkdownRenderer::default();
        renderer.render(stream.content())
    };

    if stream.is_streaming() {
        if let Some(last) = lines.last_mut() {
            last.spans.push(Span::styled(
                "▌",
                Style::default()
                    .fg(colors::FG)
                    .add_modifier(Modifier::SLOW_BLINK),
            ));
        } else {
            lines.push(Line::from(Span::styled(
                "▌",
                Style::default()
                    .fg(colors::FG)
                    .add_modifier(Modifier::SLOW_BLINK),
            )));
        }
    }

    let max_offset = lines.len().saturating_sub(inner_height.max(1));
    let offset = stream.scroll_offset().min(max_offset);

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((offset as u16, 0));

    frame.render_widget(paragraph, area);
}

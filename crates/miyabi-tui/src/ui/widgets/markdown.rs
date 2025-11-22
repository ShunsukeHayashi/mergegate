//! Streaming markdown widget placeholder.
//!
//! This file is the landing zone for `markdown_stream` integration to keep
//! rendering logic out of `history_cell.rs`.

use ratatui::layout::Rect;
use ratatui::Frame;

use crate::markdown_stream::MarkdownStream;

/// Render a markdown stream into the provided frame area.
pub fn render_stream(frame: &mut Frame, area: Rect, stream: &MarkdownStream) {
    let _ = (frame, area, stream);
    // TODO: reuse MarkdownRenderer and add code block scroll support.
}

//! Diff viewer widget for displaying file changes.
//!
//! Wraps `diff_render.rs` for reuse across overlays with scrolling support.

use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::diff_render::{DiffRender, FileDiff};

/// Properties for rendering a diff widget
pub struct DiffWidgetProps {
    /// The file diff to display
    pub diff: FileDiff,
    /// Current scroll offset
    pub scroll: u16,
    /// Optional title for the block
    pub title: Option<String>,
}

/// Render a diff within the given area with scrolling support.
pub fn render_diff_widget(frame: &mut Frame, area: Rect, diff: &FileDiff) {
    render_diff_widget_with_scroll(frame, area, diff, 0, None);
}

/// Render a diff with scroll offset and optional title.
pub fn render_diff_widget_with_scroll(
    frame: &mut Frame,
    area: Rect,
    diff: &FileDiff,
    scroll: u16,
    title: Option<&str>,
) {
    // Create a DiffRender with the single file
    let mut renderer = DiffRender::new();
    renderer.files.push(diff.clone());

    // Get rendered lines
    let lines = renderer.render();

    // Create block with optional title
    let block = if let Some(t) = title {
        Block::default().borders(Borders::ALL).title(t.to_string())
    } else {
        Block::default()
            .borders(Borders::ALL)
            .title(format!("{} → {}", diff.old_path, diff.new_path))
    };

    // Create paragraph with scroll
    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));

    frame.render_widget(paragraph, area);
}

/// Calculate the total number of lines in a diff for scroll bounds
pub fn diff_line_count(diff: &FileDiff) -> usize {
    let mut count = 1; // File header
    for hunk in &diff.hunks {
        count += hunk.lines.len();
    }
    count
}

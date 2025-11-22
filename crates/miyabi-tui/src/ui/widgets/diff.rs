//! Diff viewer widget placeholder.
//!
//! This will eventually wrap `diff_render.rs` for reuse across overlays.

use ratatui::layout::Rect;
use ratatui::Frame;

use crate::diff_render::FileDiff;

/// Render a diff within the given area.
pub fn render_diff_widget(frame: &mut Frame, area: Rect, diff: &FileDiff) {
    let _ = (frame, area, diff);
    // TODO: integrate syntax highlighting and line numbers.
}

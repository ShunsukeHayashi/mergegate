//! History list widget placeholder.
//!
//! Intended to wrap `HistoryCell` rendering with consistent padding and theming.

use ratatui::{
    layout::Rect,
    widgets::{Block, Borders},
    Frame,
};

use crate::history_cell::HistoryCell;

/// Properties required to render the history list.
pub struct HistoryListProps<'a> {
    pub items: &'a [Box<dyn HistoryCell>],
    pub scroll: u16,
}

/// Render conversation history as a list.
pub struct HistoryList;

impl HistoryList {
    pub fn render(frame: &mut Frame, area: Rect, props: HistoryListProps<'_>) {
        let block = Block::default().borders(Borders::ALL);
        frame.render_widget(block, area);

        let _ = props;
        // TODO: integrate with virtualized list + markdown rendering.
    }
}

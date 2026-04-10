//! History list widget placeholder.
//!
//! Intended to wrap `HistoryCell` rendering with consistent padding and theming.

use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::history_cell::HistoryCell;
use crate::ui::colors;

/// Properties required to render the history list.
pub struct HistoryListProps<'a> {
    pub items: &'a [Box<dyn HistoryCell>],
    pub scroll: u16,
}

/// Render conversation history as a list.
pub struct HistoryList;

impl HistoryList {
    pub fn render(frame: &mut Frame, area: Rect, props: HistoryListProps<'_>) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(colors::BORDER))
            .title("History");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        if props.items.is_empty() {
            let empty = Paragraph::new("No history yet")
                .style(Style::default().fg(colors::COMMENT))
                .alignment(Alignment::Center);
            frame.render_widget(empty, inner);
            return;
        }

        let scroll = props.scroll as usize;
        let max_lines = inner.height as usize;
        let mut visible_lines: Vec<Line> = Vec::with_capacity(max_lines);
        let mut line_index = 0usize;

        'outer: for (idx, cell) in props.items.iter().enumerate() {
            let mut rendered = cell.render(inner.width);
            if idx + 1 < props.items.len() {
                rendered.push(Line::from(""));
            }

            for line in rendered {
                if line_index >= scroll {
                    visible_lines.push(line);
                    if visible_lines.len() >= max_lines {
                        break 'outer;
                    }
                }
                line_index += 1;
            }
        }

        let list_items: Vec<ListItem> = visible_lines.into_iter().map(ListItem::new).collect();

        let list = List::new(list_items).style(Style::default().fg(colors::FG));
        frame.render_widget(list, inner);
    }
}

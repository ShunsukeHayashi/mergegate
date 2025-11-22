//! Theme container for sharing palette and typography choices.

use ratatui::style::{Modifier, Style};

use crate::ui::colors;

/// Centralized theme values used across widgets.
#[derive(Debug, Clone, Copy)]
pub struct Theme;

impl Theme {
    pub fn primary() -> Style {
        Style::default().fg(colors::CYAN)
    }

    pub fn muted() -> Style {
        Style::default().fg(colors::FG_GUTTER)
    }

    pub fn accent() -> Style {
        Style::default()
            .fg(colors::MAGENTA)
            .add_modifier(Modifier::BOLD)
    }

    pub fn danger() -> Style {
        Style::default().fg(colors::RED)
    }

    pub fn success() -> Style {
        Style::default().fg(colors::GREEN)
    }
}

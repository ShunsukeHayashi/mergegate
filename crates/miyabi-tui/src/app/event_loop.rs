//! Event loop helpers for the TUI.
//!
//! This module isolates Crossterm-driven events from application actions
//! so the run loop in `app.rs` can stay focused on orchestration.

use crate::domain::actions::AppAction;
use crate::event::Event;

/// Map a raw event into a high-level application action.
pub fn map_event(event: Event) -> Option<AppAction> {
    match event {
        Event::Key(key) => Some(AppAction::KeyPressed(key)),
        Event::Resize(w, h) => Some(AppAction::Resize {
            width: w,
            height: h,
        }),
        Event::Tick => Some(AppAction::Tick),
        Event::Mouse(_) => None,
    }
}

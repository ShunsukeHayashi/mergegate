//! Shared application state container for the TUI.
//!
//! This module is a staging area for gradually moving state out of `app.rs`
//! into a testable structure.

use miyabi_core::session::Session;

/// High-level application state kept in the TUI loop.
#[derive(Debug, Clone)]
pub struct AppState {
    /// Whether the UI should exit.
    pub should_quit: bool,
    /// Whether a streaming response is active.
    pub is_streaming: bool,
    /// Active session being displayed.
    pub session: Session,
}

impl AppState {
    /// Create a new state with the given session.
    pub fn new(session: Session) -> Self {
        Self {
            should_quit: false,
            is_streaming: false,
            session,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(Session::new("New Session"))
    }
}

//! Key binding definitions for the TUI.

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::domain::actions::AppAction;

/// Individual key binding entry.
#[derive(Debug, Clone)]
pub struct KeyBinding {
    pub key: KeyEvent,
    pub action: AppAction,
}

/// Default keymap mapping keys to actions.
pub fn default_keymap() -> HashMap<KeyEvent, AppAction> {
    let mut map = HashMap::new();

    map.insert(
        KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL),
        AppAction::Quit,
    );
    map.insert(
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        AppAction::CancelStreaming,
    );
    map.insert(
        KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL),
        AppAction::ToggleSidebar,
    );
    map.insert(
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::ALT),
        AppAction::ToggleAgentMode,
    );

    map
}

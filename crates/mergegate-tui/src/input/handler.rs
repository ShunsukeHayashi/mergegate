//! Maps key events to application actions.

use crossterm::event::KeyEvent;

use crate::domain::actions::AppAction;
use crate::input::keymap::default_keymap;

/// Convert a key event into a high-level action using the default keymap.
pub fn handle_key_event(event: KeyEvent) -> Option<AppAction> {
    let map = default_keymap();
    map.get(&event)
        .cloned()
        .or(Some(AppAction::KeyPressed(event)))
}

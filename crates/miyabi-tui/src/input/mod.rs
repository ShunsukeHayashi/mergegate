//! Input handling layer.

pub mod handler;
pub mod keymap;

pub use handler::handle_key_event;
pub use keymap::{default_keymap, KeyBinding};

//! Miyabi TUI - Terminal User Interface
//!
//! A premium TUI following the OpenAI Codex patterns for clean,
//! functional design with proper text wrapping and markdown rendering.

pub mod app;
pub mod event;
pub mod wrapping;
pub mod history_cell;
pub mod markdown_render;

pub use app::App;
pub use event::{Event, EventHandler};
pub use wrapping::{word_wrap_line, wrap_text, display_width, WrapOptions};
pub use history_cell::{HistoryCell, UserMessageCell, AssistantMessageCell, ToolResultCell, SystemMessageCell};
pub use markdown_render::{MarkdownRenderer, MarkdownStyles};

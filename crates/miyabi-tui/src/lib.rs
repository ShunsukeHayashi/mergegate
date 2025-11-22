//! Miyabi TUI - Terminal User Interface
//!
//! A premium TUI following the OpenAI Codex patterns for clean,
//! functional design with proper text wrapping and markdown rendering.

pub mod app;
pub mod event;
pub mod wrapping;
pub mod history_cell;
pub mod markdown_render;
pub mod markdown_stream;
pub mod diff_render;
pub mod diff_viewer;
pub mod markdown_parser;
pub mod syntax;

pub use app::App;
pub use event::{Event, EventHandler};
pub use wrapping::{word_wrap_line, wrap_text, display_width, WrapOptions};
pub use history_cell::{HistoryCell, UserMessageCell, AssistantMessageCell, ToolResultCell, SystemMessageCell};
pub use markdown_render::{MarkdownRenderer, MarkdownStyles};
pub use markdown_stream::{MarkdownStream, StreamState, StreamBuffer};
pub use diff_render::{DiffRender, DiffLine, DiffLineType, DiffHunk, FileDiff};
pub use diff_viewer::{DiffViewer, DiffViewerOptions, DiffColors, render_diff, render_diff_minimal};
pub use markdown_parser::MarkdownParser;
pub use syntax::{SyntaxHighlighter, highlight_code, render_code_block, normalize_language};

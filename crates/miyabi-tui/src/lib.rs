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
pub mod chat_composer;
pub mod textarea;
pub mod command_popup;
pub mod approval_overlay;
pub mod resume_picker;
pub mod pager_overlay;

pub use app::App;
pub use event::{Event, EventHandler};
pub use wrapping::{word_wrap_line, wrap_text, display_width, WrapOptions};
pub use history_cell::{HistoryCell, UserMessageCell, AssistantMessageCell, ToolResultCell, SystemMessageCell};
pub use markdown_render::{MarkdownRenderer, MarkdownStyles};
pub use markdown_stream::{MarkdownStream, StreamState, StreamBuffer, ScrollState, CursorPosition};
pub use diff_render::{DiffRender, DiffLine, DiffLineType, DiffHunk, FileDiff};
pub use diff_viewer::{DiffViewer, DiffViewerOptions, DiffColors, render_diff, render_diff_minimal};
pub use markdown_parser::MarkdownParser;
pub use syntax::{SyntaxHighlighter, highlight_code, render_code_block, normalize_language};
pub use chat_composer::{ChatComposer, ComposerAction, InputMode, CursorPos};
pub use textarea::{TextArea, TextAreaConfig, TextAreaAction, TextCursor, TextRange};
pub use command_popup::{CommandPopup, CommandPopupAction, Command, CommandBuilder, CommandCategory};
pub use approval_overlay::{ApprovalOverlay, ApprovalAction, ApprovalRequest, ApprovalBuilder, RiskLevel, BatchApproval};
pub use resume_picker::{ResumePicker, ResumePickerAction, SessionEntry, SessionSortOrder, SessionManager};
pub use pager_overlay::{PagerOverlay, PagerAction, PagerContent, PagerBuilder};

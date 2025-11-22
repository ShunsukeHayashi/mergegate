//! Miyabi TUI - Terminal User Interface
//!
//! A premium TUI following the OpenAI Codex patterns for clean,
//! functional design with proper text wrapping and markdown rendering.

pub mod app;
pub mod approval_overlay;
pub mod chat_composer;
pub mod command_popup;
pub mod diff_render;
pub mod diff_viewer;
pub mod domain;
pub mod event;
pub mod help;
pub mod history_cell;
pub mod input;
pub mod markdown_parser;
pub mod markdown_render;
pub mod markdown_stream;
pub mod notification;
pub mod pager_overlay;
pub mod resume_picker;
pub mod shimmer;
pub mod syntax;
pub mod textarea;
pub mod ui;
pub mod update;
pub mod views;
pub mod wrapping;

pub use app::App;
pub use approval_overlay::{
    ApprovalAction, ApprovalBuilder, ApprovalOverlay, ApprovalRequest, BatchApproval, RiskLevel,
};
pub use chat_composer::{ChatComposer, ComposerAction, CursorPos, InputMode};
pub use command_popup::{
    Command, CommandBuilder, CommandCategory, CommandPopup, CommandPopupAction,
};
pub use diff_render::{DiffHunk, DiffLine, DiffLineType, DiffRender, FileDiff};
pub use diff_viewer::{
    render_diff, render_diff_minimal, DiffColors, DiffViewer, DiffViewerOptions,
};
pub use domain::{AppAction, ConversationEntry, SessionSummary};
pub use event::{Event, EventHandler};
pub use help::{
    CheatSection, CheatSheet, HelpAction, HelpCategory, HelpViewer, KeyBinding, QuickRef,
};
pub use history_cell::{
    AssistantMessageCell, HistoryCell, SystemMessageCell, ToolResultCell, UserMessageCell,
};
pub use input::{default_keymap, handle_key_event, KeyBinding as InputKeyBinding};
pub use markdown_parser::MarkdownParser;
pub use markdown_render::{MarkdownRenderer, MarkdownStyles};
pub use markdown_stream::{CursorPosition, MarkdownStream, ScrollState, StreamBuffer, StreamState};
pub use notification::{
    Alert, AlertAction, AlertButton, AlertType, Banner, Notification, NotificationAction,
    NotificationCenter, NotificationPanel, NotificationPanelAction, NotificationPriority,
};
pub use pager_overlay::{PagerAction, PagerBuilder, PagerContent, PagerOverlay};
pub use resume_picker::{
    ResumePicker, ResumePickerAction, SessionEntry, SessionManager, SessionSortOrder,
};
pub use shimmer::{
    Countdown, LoadingOverlay, LoadingState, ProgressBar, ShimmerEffect, ShimmerState,
    SkeletonLoader, Spinner, SpinnerStyle, TypingIndicator,
};
pub use syntax::{highlight_code, normalize_language, render_code_block, SyntaxHighlighter};
pub use textarea::{TextArea, TextAreaAction, TextAreaConfig, TextCursor, TextRange};
pub use ui::{
    colors, layout, styles, Badge, Breadcrumb, Divider, EmptyState, KeyHint, KeyHints, Modal,
    StatusBar, StatusItem, Toast, ToastManager, ToastType,
};
pub use update::reduce;
pub use views::{
    ActiveOverlay, AppMode, FocusArea, LayoutConfig, MainView, ViewAction, ViewBuilder,
};
pub use wrapping::{display_width, word_wrap_line, wrap_text, WrapOptions};

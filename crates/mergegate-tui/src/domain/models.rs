//! Domain structs used by the TUI layer.

use miyabi_core::anthropic::Message;

/// Summary of a session for list views.
#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub id: String,
    pub title: String,
    pub updated_at: String,
    pub tokens_used: usize,
}

/// Minimal representation of a message for UI rendering.
#[derive(Debug, Clone)]
pub struct ConversationEntry {
    pub role: String,
    pub message: Message,
}

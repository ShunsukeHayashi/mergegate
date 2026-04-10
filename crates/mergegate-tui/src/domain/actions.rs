//! Application-level actions produced from input handling.

use crossterm::event::KeyEvent;

/// High-level actions the TUI can perform.
#[derive(Debug, Clone)]
pub enum AppAction {
    /// Exit the application loop.
    Quit,
    /// Send a chat message.
    SendMessage { text: String },
    /// Execute a command string.
    ExecuteCommand { command: String },
    /// Approve or reject a pending tool use.
    ApproveTool { id: String, approved: bool },
    /// Cancel current streaming response.
    CancelStreaming,
    /// Toggle agent/chat mode.
    ToggleAgentMode,
    /// Toggle sidebar visibility.
    ToggleSidebar,
    /// Generic key press for downstream handlers.
    KeyPressed(KeyEvent),
    /// Terminal resize.
    Resize { width: u16, height: u16 },
    /// Periodic tick for animations.
    Tick,
}

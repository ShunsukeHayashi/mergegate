//! Main TUI Application

use futures::StreamExt;

use crate::event::{Event, EventHandler};
use crate::history_cell::{
    UserMessageCell, AssistantMessageCell, SystemMessageCell, SystemMessageType,
};
use crate::views::{MainView, ViewAction};
use miyabi_core::anthropic::{AnthropicClient, Message, StreamEvent};

/// Main application state
pub struct App {
    /// Whether the app should quit
    pub should_quit: bool,
    /// Main view with all UI components
    pub view: MainView,
    /// Anthropic API client
    client: Option<AnthropicClient>,
    /// Conversation history for API calls
    conversation: Vec<Message>,
    /// Whether currently streaming a response
    is_streaming: bool,
}

impl App {
    /// Create a new app
    pub fn new() -> Self {
        let timestamp = chrono::Local::now().format("%H:%M").to_string();

        // Try to get API key from environment
        let client = std::env::var("ANTHROPIC_API_KEY")
            .ok()
            .and_then(|key| AnthropicClient::new(key).ok())
            .map(|c| c.with_max_tokens(8192));

        let welcome_message = if client.is_some() {
            "Welcome to Miyabi CLI! Type your message and press Enter. Press Ctrl+P for commands, F1 for help."
        } else {
            "⚠ ANTHROPIC_API_KEY not set. Running in demo mode. Press Ctrl+P for commands."
        };

        let mut view = MainView::new();

        // Add welcome message
        view.push_message(Box::new(SystemMessageCell {
            content: welcome_message.to_string(),
            timestamp: timestamp.clone(),
            message_type: if client.is_some() { SystemMessageType::Info } else { SystemMessageType::Warning },
        }));

        // Set model name if client available
        if client.is_some() {
            view = view.with_model("claude-sonnet-4-20250514");
        }

        Self {
            should_quit: false,
            view,
            client,
            conversation: Vec::new(),
            is_streaming: false,
        }
    }

    /// Run the main app loop
    pub async fn run(
        &mut self,
        terminal: &mut ratatui::Terminal<impl ratatui::backend::Backend>,
    ) -> anyhow::Result<()> {
        let mut events = EventHandler::new(100);

        loop {
            terminal.draw(|f| self.view.render(f))?;

            if let Some(event) = events.next().await {
                match event {
                    Event::Key(key) => {
                        let action = self.view.handle_key(key);
                        match action {
                            ViewAction::Quit => {
                                self.should_quit = true;
                            }
                            ViewAction::SendMessage(message) => {
                                if !self.is_streaming {
                                    self.send_message(message).await;
                                }
                            }
                            ViewAction::ExecuteCommand(cmd) => {
                                self.execute_command(&cmd).await;
                            }
                            ViewAction::Cancel => {
                                // Cancel current streaming
                                self.is_streaming = false;
                                self.view.set_streaming(false);
                            }
                            _ => {}
                        }
                    }
                    Event::Resize(_, _) => {}
                    Event::Tick => {
                        self.view.tick();
                    }
                    Event::Mouse(_) => {}
                }
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Execute a command
    async fn execute_command(&mut self, cmd: &str) {
        match cmd {
            "quit" | "exit" => self.should_quit = true,
            "clear" => {
                self.view.history.clear();
                self.conversation.clear();
            }
            "help" => self.view.show_help(),
            _ => {}
        }
    }

    /// Send a message
    async fn send_message(&mut self, message: String) {
        let timestamp = chrono::Local::now().format("%H:%M").to_string();

        // Add user message to UI
        self.view.push_message(Box::new(UserMessageCell {
            content: message.clone(),
            timestamp: timestamp.clone(),
        }));

        // Add to conversation history
        self.conversation.push(Message::user(&message));

        // Call API if client is available
        if let Some(client) = &self.client {
            self.is_streaming = true;
            self.view.set_streaming(true);

            // Add streaming placeholder
            let cell_index = self.view.history.len();
            self.view.push_message(Box::new(AssistantMessageCell {
                content: String::new(),
                timestamp: timestamp.clone(),
                streaming: true,
            }));

            // Start streaming
            match client.message_stream(
                self.conversation.clone(),
                Some("You are a helpful AI assistant. Be concise and clear.".to_string()),
                None,
                None,
            ).await {
                Ok(mut stream) => {
                    let mut response_text = String::new();

                    while let Some(event) = stream.next().await {
                        match event {
                            Ok(StreamEvent::ContentBlockDelta { delta, .. }) => {
                                response_text.push_str(&delta.text);
                                // Update the cell content
                                if let Some(cell) = self.view.history.get_mut(cell_index) {
                                    if let Some(assistant_cell) = (**cell).as_any_mut().downcast_mut::<AssistantMessageCell>() {
                                        assistant_cell.content = response_text.clone();
                                    }
                                }
                            }
                            Ok(StreamEvent::MessageStop) => {
                                break;
                            }
                            Ok(StreamEvent::Error { error }) => {
                                response_text = format!("Error: {}", error);
                                break;
                            }
                            _ => {}
                        }
                    }

                    // Mark as done streaming
                    if let Some(cell) = self.view.history.get_mut(cell_index) {
                        if let Some(assistant_cell) = (**cell).as_any_mut().downcast_mut::<AssistantMessageCell>() {
                            assistant_cell.streaming = false;
                            if response_text.is_empty() {
                                assistant_cell.content = "(No response)".to_string();
                            }
                        }
                    }

                    // Add to conversation history
                    if !response_text.is_empty() {
                        self.conversation.push(Message::assistant(&response_text));
                    }
                }
                Err(e) => {
                    // Replace with error message
                    if let Some(cell) = self.view.history.get_mut(cell_index) {
                        if let Some(assistant_cell) = (**cell).as_any_mut().downcast_mut::<AssistantMessageCell>() {
                            assistant_cell.content = format!("Error: {}", e);
                            assistant_cell.streaming = false;
                        }
                    }
                }
            }

            self.is_streaming = false;
            self.view.set_streaming(false);
        } else {
            // Demo mode - no API key
            let response = format!("You said: {}\n\nThis is a **demo response** with `markdown` support!\n\nSet ANTHROPIC_API_KEY to enable real responses.", message);
            self.view.push_message(Box::new(AssistantMessageCell {
                content: response,
                timestamp,
                streaming: false,
            }));
        }

        // Auto-scroll to bottom
        self.view.history_scroll = self.view.max_scroll;
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

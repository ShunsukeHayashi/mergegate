//! Main TUI Application

pub mod event_loop;
pub mod state;

use std::time::Instant;

use futures::StreamExt;

use crate::approval_overlay::{ApprovalRequest, RiskLevel};
use crate::event::{Event, EventHandler};
use crate::history_cell::{
    AssistantMessageCell, SystemMessageCell, SystemMessageType, ToolResultCell, UserMessageCell,
};
use crate::views::{MainView, ViewAction};
use miyabi_core::anthropic::{AnthropicClient, ContentBlock, Message, StreamEvent};
use miyabi_core::config::Config;
use miyabi_core::session::{Session, SessionStorage};
use miyabi_core::tool::ToolRegistry;
use miyabi_core::tools::create_standard_tool_registry;
use miyabi_core::{Agent, AgentConfig, AgentEvent, ExecutorRegistry};
use tokio::sync::mpsc;

const MODEL_PRESETS: &[&str] = &[
    "claude-sonnet-4-5-20250929",
    "claude-haiku-4-5-20251001",
    "claude-sonnet-4-20250514",
];
const THINKING_ON: &str = "thinking:on";
const THINKING_OFF: &str = "thinking:off";

/// Pending tool request awaiting approval
#[derive(Debug, Clone)]
pub struct PendingTool {
    /// Tool use ID from Claude
    pub id: String,
    /// Tool name
    pub name: String,
    /// Tool input
    pub input: serde_json::Value,
}

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
    /// Tool registry for executing tools
    tool_registry: ToolRegistry,
    /// Pending tools awaiting approval
    pending_tools: Vec<PendingTool>,
    /// Current session
    session: Session,
    /// Session storage for persistence
    storage: SessionStorage,
    /// System prompt for API requests
    system_prompt: Option<String>,
    /// Agent mode (autonomous execution)
    agent_mode: bool,
    /// API key for agent mode
    api_key: Option<String>,
    /// Model name for agent mode
    model_name: String,
    /// Max tokens for agent mode
    max_tokens: u32,
    /// Whether to request extended thinking
    thinking: bool,
}

impl App {
    /// Create a new app with default configuration
    pub fn new() -> Self {
        let config = Config::load().unwrap_or_default();
        Self::with_config(config)
    }

    /// Create a new app with specific configuration
    pub fn with_config(config: Config) -> Self {
        let timestamp = chrono::Local::now().format("%H:%M").to_string();

        // Create Anthropic client from config
        let client = config
            .api
            .api_key
            .as_ref()
            .and_then(|key| AnthropicClient::new(key).ok())
            .map(|c| {
                c.with_model(&config.api.model)
                    .with_max_tokens(config.api.max_tokens)
                    .with_thinking(config.api.thinking)
            });

        let welcome_message = if client.is_some() {
            "Welcome to Miyabi CLI! Type your message and press Enter. Press Ctrl+P for commands, F1 for help."
        } else {
            "⚠ ANTHROPIC_API_KEY not set. Please set it in config or environment to use Claude API."
        };

        let mut view = MainView::new();

        // Add welcome message
        view.push_message(Box::new(SystemMessageCell {
            content: welcome_message.to_string(),
            timestamp: timestamp.clone(),
            message_type: if client.is_some() {
                SystemMessageType::Info
            } else {
                SystemMessageType::Warning
            },
        }));

        // Get model name from config
        let model_name = &config.api.model;
        view = view.with_model(model_name);

        // Create session with model from config
        let session = Session::new("New Session").model(model_name);

        // Create storage using config sessions directory
        let storage = SessionStorage::new(config.sessions_dir());

        // Get system prompt from config
        let system_prompt = config.api.system_prompt.clone();

        // Store config values for agent mode
        let api_key = config.api.api_key.clone();
        let model_name = config.api.model.clone();
        let max_tokens = config.api.max_tokens;
        let thinking = config.api.thinking;

        Self {
            should_quit: false,
            view,
            client,
            conversation: Vec::new(),
            is_streaming: false,
            tool_registry: create_standard_tool_registry(),
            pending_tools: Vec::new(),
            session,
            storage,
            system_prompt,
            agent_mode: false,
            api_key,
            model_name,
            max_tokens,
            thinking,
        }
    }

    /// Toggle agent mode
    pub fn toggle_agent_mode(&mut self) {
        self.agent_mode = !self.agent_mode;
        let mode_str = if self.agent_mode { "Agent" } else { "Chat" };
        self.view
            .notifications
            .info("Mode Changed", format!("Switched to {} mode", mode_str));

        // Update view mode indicator
        if self.agent_mode {
            self.view.set_mode_indicator("🤖 AGENT");
        } else {
            self.view.set_mode_indicator("");
        }
    }

    /// Save current session to disk
    pub fn save_session(&self) -> anyhow::Result<()> {
        self.storage.save(&self.session)
    }

    /// Load a session by ID
    pub fn load_session(&mut self, id: &str) -> anyhow::Result<()> {
        let session = self.storage.load(id)?;
        self.conversation = session.messages.clone();
        self.view.tokens_used = session.tokens_used;
        self.session = session;
        Ok(())
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session.id
    }

    /// Get a mutable reference to the tool registry for registration
    pub fn tool_registry_mut(&mut self) -> &mut ToolRegistry {
        &mut self.tool_registry
    }

    /// Run the main app loop
    pub async fn run<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut ratatui::Terminal<B>,
    ) -> anyhow::Result<()> {
        let mut events = EventHandler::new(100);

        loop {
            terminal.draw(|f| self.view.render(f))?;

            if let Some(event) = events.next().await {
                match event {
                    Event::Key(key) => {
                        let action = self.view.handle_key(key);
                        match action {
                            ViewAction::None => {}
                            ViewAction::Quit => {
                                self.should_quit = true;
                            }
                            ViewAction::SendMessage(message) => {
                                if !self.is_streaming {
                                    self.send_message(message, terminal).await;
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
                            ViewAction::Approve {
                                request_id,
                                approved,
                            } => {
                                self.handle_tool_approval(&request_id, approved, terminal)
                                    .await;
                            }
                            ViewAction::ToggleSidebar => {
                                // Sidebar already toggled in views.rs handle_key()
                                // No additional action needed here
                            }
                            ViewAction::Notify(notification) => {
                                self.view.notifications.panel.push(notification);
                            }
                            ViewAction::Copy(text) => {
                                // TODO: Implement clipboard support
                                self.view
                                    .notifications
                                    .info("Copied", format!("{} chars", text.len()));
                            }
                            ViewAction::OpenFile(path) => {
                                // TODO: Implement file opening
                                self.view.notifications.info("Open File", &path);
                            }
                            ViewAction::ResumeSession(session_id) => {
                                // TODO: Implement session resume
                                self.view.notifications.info("Resume Session", &session_id);
                            }
                            ViewAction::ToggleAgentMode => {
                                self.toggle_agent_mode();
                            }
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

        // Auto-save session on exit if there are messages
        if !self.conversation.is_empty() {
            // Update session title from first user message if still default
            if self.session.title == "New Session" {
                if let Some(first_msg) = self.conversation.first() {
                    if let Some(ContentBlock::Text { text }) = first_msg.content.first() {
                        // Take first 50 chars as title
                        let title: String = text.chars().take(50).collect();
                        self.session.title = if title.len() < text.len() {
                            format!("{}...", title)
                        } else {
                            title
                        };
                    }
                }
            }

            // Update session with conversation
            self.session.messages = self.conversation.clone();
            self.session.tokens_used = self.view.tokens_used;

            // Save session
            if let Err(e) = self.save_session() {
                eprintln!("Failed to save session: {}", e);
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
            "model" => self.cycle_model(),
            THINKING_ON => self.set_thinking(true),
            THINKING_OFF => self.set_thinking(false),
            _ => {}
        }
    }

    fn set_model(&mut self, model: &str) {
        self.model_name = model.to_string();
        self.view.model_name = model.to_string();
        self.session.model = model.to_string();

        if let Some(ref api_key) = self.api_key {
            // Recreate client with new model if API key exists
            self.client = AnthropicClient::new(api_key.clone()).ok().map(|c| {
                c.with_model(model)
                    .with_max_tokens(self.max_tokens)
                    .with_thinking(self.thinking)
            });
        }
    }

    fn cycle_model(&mut self) {
        let current_index = MODEL_PRESETS
            .iter()
            .position(|m| *m == self.model_name)
            .unwrap_or(0);
        let next_index = (current_index + 1) % MODEL_PRESETS.len();
        let next_model = MODEL_PRESETS[next_index];
        self.set_model(next_model);
        self.view.notifications.info("Model changed", next_model);
    }

    fn set_thinking(&mut self, enabled: bool) {
        self.thinking = enabled;
        if let Some(ref api_key) = self.api_key {
            // Recreate client with updated thinking flag
            self.client = AnthropicClient::new(api_key.clone()).ok().map(|c| {
                c.with_model(&self.model_name)
                    .with_max_tokens(self.max_tokens)
                    .with_thinking(self.thinking)
            });
        }
        let status = if enabled {
            "Extended Thinking ON"
        } else {
            "Extended Thinking OFF"
        };
        self.view.notifications.info("Thinking", status);
    }

    /// Send a message
    async fn send_message<B: ratatui::backend::Backend>(
        &mut self,
        message: String,
        terminal: &mut ratatui::Terminal<B>,
    ) {
        // Use agent mode if enabled
        if self.agent_mode {
            self.send_message_agent(message, terminal).await;
            return;
        }

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
            self.view
                .push_message(Box::new(AssistantMessageCell::new(timestamp.clone())));

            // Start streaming
            match client
                .message_stream(
                    self.conversation.clone(),
                    self.system_prompt.clone(),
                    None,
                    None,
                )
                .await
            {
                Ok(mut stream) => {
                    while let Some(event) = stream.next().await {
                        match event {
                            Ok(StreamEvent::ContentBlockStart {
                                content_block: ContentBlock::ToolUse { id, name, input },
                                ..
                            }) => {
                                // Store pending tool
                                self.pending_tools.push(PendingTool {
                                    id: id.clone(),
                                    name: name.clone(),
                                    input: input.clone(),
                                });

                                // Determine risk level based on tool
                                let risk = if name.contains("write")
                                    || name.contains("delete")
                                    || name.contains("execute")
                                {
                                    RiskLevel::High
                                } else if name.contains("read") || name.contains("search") {
                                    RiskLevel::Low
                                } else {
                                    RiskLevel::Medium
                                };

                                // Show approval overlay
                                let request = ApprovalRequest::new(id, &name)
                                    .risk_level(risk)
                                    .description(format!("Execute tool: {}", name));
                                self.view.show_approval(request);
                            }
                            Ok(StreamEvent::ContentBlockStart { .. }) => {
                                // Other content block types - ignore
                            }
                            Ok(StreamEvent::ContentBlockDelta { delta, .. }) => {
                                // Push delta to the stream directly
                                if let Some(cell) = self.view.history.get(cell_index) {
                                    if let Some(assistant_cell) =
                                        cell.as_ref()
                                            .as_any()
                                            .downcast_ref::<AssistantMessageCell>()
                                    {
                                        assistant_cell.push_str(&delta.text);
                                    }
                                }
                                // Redraw terminal to show streaming content
                                let _ = terminal.draw(|f| self.view.render(f));
                            }
                            Ok(StreamEvent::MessageDelta { usage, .. }) => {
                                // Track token usage
                                self.view.tokens_used += usage.output_tokens as usize;
                            }
                            Ok(StreamEvent::MessageStop) => {
                                break;
                            }
                            Ok(StreamEvent::Error { error }) => {
                                // Show error notification
                                self.view.notifications.error("API Error", &error);
                                if let Some(cell) = self.view.history.get(cell_index) {
                                    if let Some(assistant_cell) =
                                        cell.as_ref()
                                            .as_any()
                                            .downcast_ref::<AssistantMessageCell>()
                                    {
                                        assistant_cell.push_str(&format!("Error: {}", error));
                                    }
                                }
                                break;
                            }
                            _ => {}
                        }
                    }

                    // Mark as done streaming and get content for conversation history
                    let response_text = if let Some(cell) = self.view.history.get_mut(cell_index) {
                        if let Some(assistant_cell) =
                            (**cell).as_any_mut().downcast_mut::<AssistantMessageCell>()
                        {
                            assistant_cell.set_complete();
                            if assistant_cell.is_empty() {
                                assistant_cell.set_content("(No response)");
                            }
                            assistant_cell.content()
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };

                    // Add to conversation history
                    if !response_text.is_empty() {
                        self.conversation.push(Message::assistant(&response_text));
                    }
                }
                Err(e) => {
                    // Show error notification
                    self.view
                        .notifications
                        .error("Connection Error", e.to_string());
                    // Replace with error message
                    if let Some(cell) = self.view.history.get_mut(cell_index) {
                        if let Some(assistant_cell) =
                            (**cell).as_any_mut().downcast_mut::<AssistantMessageCell>()
                        {
                            assistant_cell.set_content(&format!("Error: {}", e));
                            assistant_cell.set_complete();
                        }
                    }
                }
            }

            self.is_streaming = false;
            self.view.set_streaming(false);
        } else {
            // Demo mode - no API key
            let response = format!("You said: {}\n\nThis is a **demo response** with `markdown` support!\n\nSet ANTHROPIC_API_KEY to enable real responses.", message);
            let mut cell = AssistantMessageCell::new(timestamp);
            cell.set_content(&response);
            cell.set_complete();
            self.view.push_message(Box::new(cell));
        }

        // Auto-scroll to bottom
        self.view.history_scroll = self.view.max_scroll;
    }

    /// Send a message in agent mode
    async fn send_message_agent<B: ratatui::backend::Backend>(
        &mut self,
        message: String,
        terminal: &mut ratatui::Terminal<B>,
    ) {
        let timestamp = chrono::Local::now().format("%H:%M").to_string();

        // Add user message to UI
        self.view.push_message(Box::new(UserMessageCell {
            content: message.clone(),
            timestamp: timestamp.clone(),
        }));

        // Check for API key
        let Some(api_key) = self.api_key.clone() else {
            self.view
                .notifications
                .error("Agent Error", "No API key available");
            return;
        };

        // Create client
        let client = match AnthropicClient::new(api_key) {
            Ok(c) => c
                .with_model(&self.model_name)
                .with_max_tokens(self.max_tokens)
                .with_thinking(self.thinking),
            Err(e) => {
                self.view.notifications.error("Agent Error", e.to_string());
                return;
            }
        };

        // Create executor registry
        let registry = ExecutorRegistry::with_standard_tools();

        // Configure agent
        let agent_config = AgentConfig {
            max_iterations: 10,
            max_tokens_per_turn: self.max_tokens,
            require_approval: false, // Auto-approve in TUI agent mode
            auto_approve_patterns: vec![
                "read".to_string(),
                "glob".to_string(),
                "grep".to_string(),
                "write".to_string(),
                "edit".to_string(),
                "bash".to_string(),
            ],
            ..Default::default()
        };

        // Create agent
        let mut agent = Agent::new(client, registry).with_config(agent_config);
        if let Some(sys) = &self.system_prompt {
            agent = agent.with_system_prompt(sys.clone());
        }

        // Create event channel
        let (tx, mut rx) = mpsc::channel(100);
        let agent = agent.with_event_channel(tx);

        // Start streaming indicator
        self.is_streaming = true;
        self.view.set_streaming(true);

        // Add placeholder for response
        let cell_index = self.view.history.len();
        self.view
            .push_message(Box::new(AssistantMessageCell::new(timestamp.clone())));

        // Spawn agent
        let prompt = message.clone();
        let agent_handle = tokio::spawn(async move { agent.run(&prompt).await });

        // Process events
        while let Some(event) = rx.recv().await {
            match event {
                AgentEvent::Thinking { iteration } => {
                    if let Some(cell) = self.view.history.get(cell_index) {
                        if let Some(assistant_cell) = cell
                            .as_ref()
                            .as_any()
                            .downcast_ref::<AssistantMessageCell>()
                        {
                            assistant_cell
                                .push_str(&format!("💭 Iteration {}...\n", iteration + 1));
                        }
                    }
                    let _ = terminal.draw(|f| self.view.render(f));
                }
                AgentEvent::ToolExecuting { name, .. } => {
                    if let Some(cell) = self.view.history.get(cell_index) {
                        if let Some(assistant_cell) = cell
                            .as_ref()
                            .as_any()
                            .downcast_ref::<AssistantMessageCell>()
                        {
                            assistant_cell.push_str(&format!("⚡ Executing: {}\n", name));
                        }
                    }
                    let _ = terminal.draw(|f| self.view.render(f));
                }
                AgentEvent::ToolCompleted { name, .. } => {
                    if let Some(cell) = self.view.history.get(cell_index) {
                        if let Some(assistant_cell) = cell
                            .as_ref()
                            .as_any()
                            .downcast_ref::<AssistantMessageCell>()
                        {
                            assistant_cell.push_str(&format!("✅ {}\n", name));
                        }
                    }
                    let _ = terminal.draw(|f| self.view.render(f));
                }
                AgentEvent::ToolFailed { name, error } => {
                    if let Some(cell) = self.view.history.get(cell_index) {
                        if let Some(assistant_cell) = cell
                            .as_ref()
                            .as_any()
                            .downcast_ref::<AssistantMessageCell>()
                        {
                            assistant_cell.push_str(&format!("❌ {}: {}\n", name, error));
                        }
                    }
                    let _ = terminal.draw(|f| self.view.render(f));
                }
                AgentEvent::Completed { result } => {
                    if let Some(cell) = self.view.history.get_mut(cell_index) {
                        if let Some(assistant_cell) =
                            (**cell).as_any_mut().downcast_mut::<AssistantMessageCell>()
                        {
                            assistant_cell.set_content(&format!(
                                "{}\n\n---\n📊 {} iterations, {} tool calls, {} tokens",
                                result.output,
                                result.iterations,
                                result.tool_calls,
                                result.total_tokens
                            ));
                            assistant_cell.set_complete();
                        }
                    }
                    self.view.tokens_used += result.total_tokens;
                    break;
                }
                AgentEvent::Failed { error } => {
                    if let Some(cell) = self.view.history.get_mut(cell_index) {
                        if let Some(assistant_cell) =
                            (**cell).as_any_mut().downcast_mut::<AssistantMessageCell>()
                        {
                            assistant_cell.set_content(&format!("❌ Agent failed: {}", error));
                            assistant_cell.set_complete();
                        }
                    }
                    self.view.notifications.error("Agent Error", &error);
                    break;
                }
                _ => {}
            }
        }

        // Wait for agent to complete
        let _ = agent_handle.await;

        self.is_streaming = false;
        self.view.set_streaming(false);

        // Auto-scroll to bottom
        self.view.history_scroll = self.view.max_scroll;
    }

    /// Handle tool approval/rejection
    async fn handle_tool_approval<B: ratatui::backend::Backend>(
        &mut self,
        request_id: &str,
        approved: bool,
        terminal: &mut ratatui::Terminal<B>,
    ) {
        let timestamp = chrono::Local::now().format("%H:%M").to_string();

        // Find the pending tool
        let tool_index = self.pending_tools.iter().position(|t| t.id == request_id);
        let Some(tool_index) = tool_index else {
            return;
        };

        let tool = self.pending_tools.remove(tool_index);

        if approved {
            // Execute the tool with timing
            let start = Instant::now();
            let result = self
                .tool_registry
                .execute(&tool.name, tool.input.clone())
                .await;
            let execution_time_ms = start.elapsed().as_millis() as u64;

            let (content, is_error) = match result {
                Ok(output) => {
                    // Convert Value to String for display
                    let content_str = if let serde_json::Value::String(s) = output.content {
                        s
                    } else {
                        serde_json::to_string_pretty(&output.content).unwrap_or_default()
                    };
                    (content_str, false)
                }
                Err(e) => (format!("Error: {}", e), true),
            };

            // Add tool result to UI
            self.view.push_message(Box::new(ToolResultCell::new(
                tool.name.clone(),
                content.clone(),
                timestamp.clone(),
                execution_time_ms,
                !is_error,
                Some(&tool.input),
            )));

            // Add tool_result to conversation for Claude
            let tool_result = ContentBlock::ToolResult {
                tool_use_id: tool.id,
                content,
            };
            self.conversation.push(Message {
                role: miyabi_core::anthropic::Role::User,
                content: vec![tool_result],
            });

            // Continue the conversation with tool result
            // This will trigger another API call with the tool result
            if let Some(client) = &self.client {
                self.continue_with_tool_result(client.clone(), terminal)
                    .await;
            }
        } else {
            // Tool was rejected
            let error_content = format!("Tool '{}' was rejected by user", tool.name);

            self.view.push_message(Box::new(ToolResultCell::new(
                tool.name.clone(),
                error_content.clone(),
                timestamp.clone(),
                0,
                false,
                Some(&tool.input),
            )));

            // Send rejection as tool_result
            let tool_result = ContentBlock::ToolResult {
                tool_use_id: tool.id,
                content: error_content,
            };
            self.conversation.push(Message {
                role: miyabi_core::anthropic::Role::User,
                content: vec![tool_result],
            });
        }
    }

    /// Continue conversation after tool execution
    async fn continue_with_tool_result<B: ratatui::backend::Backend>(
        &mut self,
        client: AnthropicClient,
        terminal: &mut ratatui::Terminal<B>,
    ) {
        let timestamp = chrono::Local::now().format("%H:%M").to_string();

        self.is_streaming = true;
        self.view.set_streaming(true);

        // Add streaming placeholder
        let cell_index = self.view.history.len();
        self.view
            .push_message(Box::new(AssistantMessageCell::new(timestamp.clone())));

        // Continue the conversation
        match client
            .message_stream(
                self.conversation.clone(),
                Some("You are a helpful AI assistant. Be concise and clear.".to_string()),
                None,
                None,
            )
            .await
        {
            Ok(mut stream) => {
                while let Some(event) = stream.next().await {
                    match event {
                        Ok(StreamEvent::ContentBlockStart {
                            content_block: ContentBlock::ToolUse { id, name, input },
                            ..
                        }) => {
                            self.pending_tools.push(PendingTool {
                                id: id.clone(),
                                name: name.clone(),
                                input: input.clone(),
                            });

                            let risk = if name.contains("write")
                                || name.contains("delete")
                                || name.contains("execute")
                            {
                                RiskLevel::High
                            } else if name.contains("read") || name.contains("search") {
                                RiskLevel::Low
                            } else {
                                RiskLevel::Medium
                            };

                            let request = ApprovalRequest::new(id, &name)
                                .risk_level(risk)
                                .description(format!("Execute tool: {}", name));
                            self.view.show_approval(request);
                        }
                        Ok(StreamEvent::ContentBlockStart { .. }) => {
                            // Other content block types - ignore
                        }
                        Ok(StreamEvent::ContentBlockDelta { delta, .. }) => {
                            // Push delta to the stream directly
                            if let Some(cell) = self.view.history.get(cell_index) {
                                if let Some(assistant_cell) =
                                    cell.as_ref()
                                        .as_any()
                                        .downcast_ref::<AssistantMessageCell>()
                                {
                                    assistant_cell.push_str(&delta.text);
                                }
                            }
                            // Redraw terminal to show streaming content
                            let _ = terminal.draw(|f| self.view.render(f));
                        }
                        Ok(StreamEvent::MessageDelta { usage, .. }) => {
                            self.view.tokens_used += usage.output_tokens as usize;
                        }
                        Ok(StreamEvent::MessageStop) => {
                            break;
                        }
                        Ok(StreamEvent::Error { error }) => {
                            self.view.notifications.error("API Error", &error);
                            if let Some(cell) = self.view.history.get(cell_index) {
                                if let Some(assistant_cell) =
                                    cell.as_ref()
                                        .as_any()
                                        .downcast_ref::<AssistantMessageCell>()
                                {
                                    assistant_cell.push_str(&format!("Error: {}", error));
                                }
                            }
                            break;
                        }
                        _ => {}
                    }
                }

                // Mark as done streaming and get content for conversation history
                let response_text = if let Some(cell) = self.view.history.get_mut(cell_index) {
                    if let Some(assistant_cell) =
                        (**cell).as_any_mut().downcast_mut::<AssistantMessageCell>()
                    {
                        assistant_cell.set_complete();
                        if assistant_cell.is_empty() {
                            assistant_cell.set_content("(No response)");
                        }
                        assistant_cell.content()
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };

                // Add to conversation history
                if !response_text.is_empty() {
                    self.conversation.push(Message::assistant(&response_text));
                }
            }
            Err(e) => {
                self.view
                    .notifications
                    .error("Connection Error", e.to_string());
                if let Some(cell) = self.view.history.get_mut(cell_index) {
                    if let Some(assistant_cell) =
                        (**cell).as_any_mut().downcast_mut::<AssistantMessageCell>()
                    {
                        assistant_cell.set_content(&format!("Error: {}", e));
                        assistant_cell.set_complete();
                    }
                }
            }
        }

        self.is_streaming = false;
        self.view.set_streaming(false);
        self.view.history_scroll = self.view.max_scroll;
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

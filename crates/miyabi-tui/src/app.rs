//! Main TUI Application

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use crossterm::event::{KeyCode, KeyModifiers};
use futures::StreamExt;

use crate::event::{Event, EventHandler};
use crate::history_cell::{
    HistoryCell, UserMessageCell, AssistantMessageCell, SystemMessageCell, SystemMessageType,
};
use miyabi_core::anthropic::{AnthropicClient, Message, StreamEvent};

/// Main application state
pub struct App {
    /// Whether the app should quit
    pub should_quit: bool,
    /// Current input buffer
    pub input: String,
    /// Chat history cells
    pub cells: Vec<Box<dyn HistoryCell>>,
    /// Scroll position
    pub scroll: u16,
    /// Maximum scroll
    pub max_scroll: u16,
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
            "Welcome to Miyabi CLI! Type your message and press Enter."
        } else {
            "⚠ ANTHROPIC_API_KEY not set. Running in demo mode."
        };

        let welcome_cell: Box<dyn HistoryCell> = Box::new(SystemMessageCell {
            content: welcome_message.to_string(),
            timestamp: timestamp.clone(),
            message_type: if client.is_some() { SystemMessageType::Info } else { SystemMessageType::Warning },
        });

        Self {
            should_quit: false,
            input: String::new(),
            cells: vec![welcome_cell],
            scroll: 0,
            max_scroll: 0,
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
            terminal.draw(|f| self.draw(f))?;

            if let Some(event) = events.next().await {
                match event {
                    Event::Key(key) => {
                        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                            self.should_quit = true;
                        } else {
                            match key.code {
                                KeyCode::Enter => {
                                    if !self.input.is_empty() && !self.is_streaming {
                                        self.send_message().await;
                                    }
                                }
                                KeyCode::Char(c) => {
                                    self.input.push(c);
                                }
                                KeyCode::Backspace => {
                                    self.input.pop();
                                }
                                KeyCode::Esc => {
                                    self.should_quit = true;
                                }
                                KeyCode::Up => {
                                    self.scroll = self.scroll.saturating_sub(1);
                                }
                                KeyCode::Down => {
                                    if self.scroll < self.max_scroll {
                                        self.scroll += 1;
                                    }
                                }
                                KeyCode::PageUp => {
                                    self.scroll = self.scroll.saturating_sub(10);
                                }
                                KeyCode::PageDown => {
                                    self.scroll = (self.scroll + 10).min(self.max_scroll);
                                }
                                _ => {}
                            }
                        }
                    }
                    Event::Resize(_, _) => {}
                    Event::Tick => {}
                    Event::Mouse(_) => {}
                }
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Draw the UI
    fn draw(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),     // Header
                Constraint::Min(1),        // Chat area
                Constraint::Length(3),     // Input
            ])
            .split(frame.area());

        self.draw_header(frame, chunks[0]);
        self.draw_chat(frame, chunks[1]);
        self.draw_input(frame, chunks[2]);
    }

    fn draw_header(&self, frame: &mut Frame, area: Rect) {
        let title = Paragraph::new(Line::from(vec![
            Span::styled(" Miyabi ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            Span::styled("CLI", Style::default().fg(Color::Cyan)),
        ]))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Rgb(86, 95, 137))));

        frame.render_widget(title, area);
    }

    fn draw_chat(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(86, 95, 137)))
            .title(Span::styled(" Chat ", Style::default().fg(Color::Rgb(192, 202, 245))));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Render cells
        let mut lines: Vec<Line<'static>> = Vec::new();
        let width = inner.width.saturating_sub(4);

        for cell in &self.cells {
            lines.push(Line::from(""));
            lines.extend(cell.render(width));
        }

        let total_lines = lines.len() as u16;
        self.max_scroll = total_lines.saturating_sub(inner.height);

        let paragraph = Paragraph::new(lines)
            .scroll((self.scroll, 0));

        frame.render_widget(paragraph, inner);

        // Scrollbar
        if self.max_scroll > 0 {
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .thumb_style(Style::default().fg(Color::Magenta));

            let mut scrollbar_state = ScrollbarState::new(self.max_scroll as usize)
                .position(self.scroll as usize);

            frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }
    }

    fn draw_input(&self, frame: &mut Frame, area: Rect) {
        let input = Paragraph::new(Line::from(vec![
            Span::styled("› ", Style::default().fg(Color::Cyan)),
            Span::raw(self.input.clone()),
            Span::styled("█", Style::default().fg(Color::Cyan)),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(86, 95, 137)))
                .title(Span::styled(" Input ", Style::default().fg(Color::Rgb(192, 202, 245)))),
        );

        frame.render_widget(input, area);
    }

    /// Send a message
    async fn send_message(&mut self) {
        let message = std::mem::take(&mut self.input);
        let timestamp = chrono::Local::now().format("%H:%M").to_string();

        // Add user message to UI
        self.cells.push(Box::new(UserMessageCell {
            content: message.clone(),
            timestamp: timestamp.clone(),
        }));

        // Add to conversation history
        self.conversation.push(Message::user(&message));

        // Call API if client is available
        if let Some(client) = &self.client {
            self.is_streaming = true;

            // Add streaming placeholder
            let cell_index = self.cells.len();
            self.cells.push(Box::new(AssistantMessageCell {
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
                                if let Some(cell) = self.cells.get_mut(cell_index) {
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
                    if let Some(cell) = self.cells.get_mut(cell_index) {
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
                    if let Some(cell) = self.cells.get_mut(cell_index) {
                        if let Some(assistant_cell) = (**cell).as_any_mut().downcast_mut::<AssistantMessageCell>() {
                            assistant_cell.content = format!("Error: {}", e);
                            assistant_cell.streaming = false;
                        }
                    }
                }
            }

            self.is_streaming = false;
        } else {
            // Demo mode - no API key
            let response = format!("You said: {}\n\nThis is a **demo response** with `markdown` support!\n\nSet ANTHROPIC_API_KEY to enable real responses.", message);
            self.cells.push(Box::new(AssistantMessageCell {
                content: response,
                timestamp,
                streaming: false,
            }));
        }

        // Auto-scroll to bottom
        self.scroll = self.max_scroll;
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

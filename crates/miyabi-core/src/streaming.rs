//! Streaming Support for Agent
//!
//! Provides real-time streaming of agent responses and tool executions.

use futures::stream::{Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc;

use crate::agent::{Agent, AgentConfig, AgentError, AgentResult, ExecutorRegistry};
use crate::anthropic::{AnthropicClient, StreamEvent};

/// Simple ReceiverStream wrapper
struct ReceiverStream<T> {
    inner: mpsc::Receiver<T>,
}

impl<T> ReceiverStream<T> {
    fn new(inner: mpsc::Receiver<T>) -> Self {
        Self { inner }
    }
}

impl<T> Stream for ReceiverStream<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_recv(cx)
    }
}

/// Events emitted during streaming agent execution
#[derive(Debug, Clone)]
pub enum AgentStreamEvent {
    /// Agent started processing
    Started,
    /// Text chunk received from LLM
    TextChunk { text: String },
    /// Full text response completed
    TextComplete { text: String },
    /// Tool use detected
    ToolDetected {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// Tool execution started
    ToolStarted { name: String },
    /// Tool execution completed
    ToolCompleted { name: String, output: String },
    /// Tool execution failed
    ToolFailed { name: String, error: String },
    /// Iteration completed
    IterationComplete { iteration: usize },
    /// Token usage update
    TokenUsage { input: usize, output: usize },
    /// Agent completed successfully
    Completed { result: AgentResult },
    /// Agent encountered an error
    Error { error: String },
}

/// Configuration for streaming
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    /// Buffer size for the event channel
    pub buffer_size: usize,
    /// Whether to emit text chunks or wait for complete text
    pub emit_chunks: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            buffer_size: 100,
            emit_chunks: true,
        }
    }
}

/// Streaming agent that emits events during execution
pub struct StreamingAgent {
    client: AnthropicClient,
    registry: ExecutorRegistry,
    agent_config: AgentConfig,
    stream_config: StreamingConfig,
    system_prompt: Option<String>,
}

impl StreamingAgent {
    /// Create a new streaming agent
    pub fn new(client: AnthropicClient, registry: ExecutorRegistry) -> Self {
        Self {
            client,
            registry,
            agent_config: AgentConfig::default(),
            stream_config: StreamingConfig::default(),
            system_prompt: None,
        }
    }

    /// Set agent configuration
    pub fn with_agent_config(mut self, config: AgentConfig) -> Self {
        self.agent_config = config;
        self
    }

    /// Set streaming configuration
    pub fn with_stream_config(mut self, config: StreamingConfig) -> Self {
        self.stream_config = config;
        self
    }

    /// Set system prompt
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Run the agent and return a stream of events
    pub fn run_stream(
        self,
        prompt: String,
    ) -> Pin<Box<dyn Stream<Item = AgentStreamEvent> + Send>> {
        let (tx, rx) = mpsc::channel(self.stream_config.buffer_size);

        // Spawn the agent execution in a separate task
        tokio::spawn(async move {
            let _ = tx.send(AgentStreamEvent::Started).await;

            // Create the underlying agent with event channel
            let (event_tx, mut event_rx) = mpsc::channel(100);
            let agent = Agent::new(self.client, self.registry)
                .with_config(self.agent_config)
                .with_event_channel(event_tx);

            let agent = if let Some(prompt) = self.system_prompt {
                agent.with_system_prompt(prompt)
            } else {
                agent
            };

            // Spawn event forwarder
            let tx_clone = tx.clone();
            let forward_handle = tokio::spawn(async move {
                while let Some(event) = event_rx.recv().await {
                    let stream_event = convert_agent_event(event);
                    if tx_clone.send(stream_event).await.is_err() {
                        break;
                    }
                }
            });

            // Run the agent
            match agent.run(&prompt).await {
                Ok(result) => {
                    let _ = tx.send(AgentStreamEvent::Completed { result }).await;
                }
                Err(e) => {
                    let _ = tx
                        .send(AgentStreamEvent::Error {
                            error: e.to_string(),
                        })
                        .await;
                }
            }

            // Wait for event forwarder to finish
            let _ = forward_handle.await;
        });

        Box::pin(ReceiverStream::new(rx))
    }

    /// Run the agent with a callback for each event
    pub async fn run_with_callback<F>(
        self,
        prompt: String,
        mut callback: F,
    ) -> Result<AgentResult, AgentError>
    where
        F: FnMut(AgentStreamEvent) + Send + 'static,
    {
        let mut stream = self.run_stream(prompt);
        let mut result = None;
        let mut error = None;

        while let Some(event) = stream.next().await {
            match &event {
                AgentStreamEvent::Completed { result: r } => {
                    result = Some(r.clone());
                }
                AgentStreamEvent::Error { error: e } => {
                    error = Some(e.clone());
                }
                _ => {}
            }
            callback(event);
        }

        if let Some(r) = result {
            Ok(r)
        } else if let Some(e) = error {
            Err(AgentError::ToolExecutionFailed(e))
        } else {
            Err(AgentError::ToolExecutionFailed(
                "Stream ended without result".to_string(),
            ))
        }
    }
}

/// Convert internal agent events to stream events
fn convert_agent_event(event: crate::agent::AgentEvent) -> AgentStreamEvent {
    match event {
        crate::agent::AgentEvent::Started { .. } => AgentStreamEvent::Started,
        crate::agent::AgentEvent::Thinking { iteration } => {
            AgentStreamEvent::IterationComplete { iteration }
        }
        crate::agent::AgentEvent::ToolDetected { id, name, input } => {
            AgentStreamEvent::ToolDetected { id, name, input }
        }
        crate::agent::AgentEvent::AwaitingApproval { name, .. } => {
            AgentStreamEvent::ToolStarted { name }
        }
        crate::agent::AgentEvent::ToolExecuting { name, .. } => {
            AgentStreamEvent::ToolStarted { name }
        }
        crate::agent::AgentEvent::ToolCompleted { name, output } => {
            AgentStreamEvent::ToolCompleted {
                name,
                output: output.content.to_string(),
            }
        }
        crate::agent::AgentEvent::ToolFailed { name, error } => {
            AgentStreamEvent::ToolFailed { name, error }
        }
        crate::agent::AgentEvent::TokenUsage { input, output } => {
            AgentStreamEvent::TokenUsage { input, output }
        }
        crate::agent::AgentEvent::Completed { result } => AgentStreamEvent::Completed { result },
        crate::agent::AgentEvent::Failed { error } => AgentStreamEvent::Error { error },
        crate::agent::AgentEvent::ToolApproved { .. } => {
            // Approval events are internal, map to Started
            AgentStreamEvent::Started
        }
        crate::agent::AgentEvent::ToolDenied { id: _, reason } => {
            AgentStreamEvent::Error { error: reason }
        }
        crate::agent::AgentEvent::ToolProgress { name, .. } => {
            AgentStreamEvent::ToolStarted { name }
        }
        crate::agent::AgentEvent::TextDelta { text } => AgentStreamEvent::TextChunk { text },
    }
}

/// Stream processor for handling LLM streaming responses
pub struct StreamProcessor {
    accumulated_text: String,
    emit_chunks: bool,
}

impl StreamProcessor {
    /// Create a new stream processor
    pub fn new(emit_chunks: bool) -> Self {
        Self {
            accumulated_text: String::new(),
            emit_chunks,
        }
    }

    /// Process a streaming event from the API
    pub fn process(&mut self, event: StreamEvent) -> Vec<AgentStreamEvent> {
        let mut events = Vec::new();

        match event {
            StreamEvent::ContentBlockDelta { delta, .. } => {
                self.accumulated_text.push_str(&delta.text);
                if self.emit_chunks {
                    events.push(AgentStreamEvent::TextChunk { text: delta.text });
                }
            }
            StreamEvent::ContentBlockStop { .. } => {
                if !self.accumulated_text.is_empty() {
                    events.push(AgentStreamEvent::TextComplete {
                        text: self.accumulated_text.clone(),
                    });
                    self.accumulated_text.clear();
                }
            }
            StreamEvent::MessageDelta { usage, .. } => {
                events.push(AgentStreamEvent::TokenUsage {
                    input: usage.input_tokens as usize,
                    output: usage.output_tokens as usize,
                });
            }
            StreamEvent::Error { error } => {
                events.push(AgentStreamEvent::Error { error });
            }
            _ => {}
        }

        events
    }

    /// Get accumulated text
    pub fn get_text(&self) -> &str {
        &self.accumulated_text
    }

    /// Clear accumulated text
    pub fn clear(&mut self) {
        self.accumulated_text.clear();
    }
}

/// Create a streaming agent with default configuration
pub fn create_streaming_agent(
    client: AnthropicClient,
    registry: ExecutorRegistry,
) -> StreamingAgent {
    StreamingAgent::new(client, registry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_config_default() {
        let config = StreamingConfig::default();
        assert_eq!(config.buffer_size, 100);
        assert!(config.emit_chunks);
    }

    #[test]
    fn test_streaming_agent_creation() {
        let client = AnthropicClient::new("test-key".to_string()).unwrap();
        let registry = ExecutorRegistry::with_standard_tools();

        let agent = StreamingAgent::new(client, registry);
        assert!(agent.system_prompt.is_none());
    }

    #[test]
    fn test_streaming_agent_with_system_prompt() {
        let client = AnthropicClient::new("test-key".to_string()).unwrap();
        let registry = ExecutorRegistry::with_standard_tools();

        let agent = StreamingAgent::new(client, registry).with_system_prompt("You are helpful");
        assert_eq!(agent.system_prompt, Some("You are helpful".to_string()));
    }

    #[test]
    fn test_stream_processor_text_accumulation() {
        let mut processor = StreamProcessor::new(true);

        let events = processor.process(StreamEvent::ContentBlockDelta {
            index: 0,
            delta: crate::anthropic::TextDelta {
                delta_type: "text_delta".to_string(),
                text: "Hello".to_string(),
            },
        });

        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentStreamEvent::TextChunk { text } => assert_eq!(text, "Hello"),
            _ => panic!("Expected TextChunk"),
        }
        assert_eq!(processor.get_text(), "Hello");
    }

    #[test]
    fn test_stream_processor_text_complete() {
        let mut processor = StreamProcessor::new(false);

        // Add some text
        processor.process(StreamEvent::ContentBlockDelta {
            index: 0,
            delta: crate::anthropic::TextDelta {
                delta_type: "text_delta".to_string(),
                text: "Hello World".to_string(),
            },
        });

        // Complete the block
        let events = processor.process(StreamEvent::ContentBlockStop { index: 0 });

        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentStreamEvent::TextComplete { text } => assert_eq!(text, "Hello World"),
            _ => panic!("Expected TextComplete"),
        }
        assert!(processor.get_text().is_empty());
    }

    #[test]
    fn test_stream_processor_no_chunks() {
        let mut processor = StreamProcessor::new(false);

        let events = processor.process(StreamEvent::ContentBlockDelta {
            index: 0,
            delta: crate::anthropic::TextDelta {
                delta_type: "text_delta".to_string(),
                text: "Hello".to_string(),
            },
        });

        // When emit_chunks is false, no events are emitted for deltas
        assert!(events.is_empty());
        assert_eq!(processor.get_text(), "Hello");
    }

    #[test]
    fn test_stream_processor_error() {
        let mut processor = StreamProcessor::new(true);

        let events = processor.process(StreamEvent::Error {
            error: "Test error".to_string(),
        });

        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentStreamEvent::Error { error } => assert_eq!(error, "Test error"),
            _ => panic!("Expected Error"),
        }
    }

    #[test]
    fn test_stream_processor_clear() {
        let mut processor = StreamProcessor::new(true);

        processor.process(StreamEvent::ContentBlockDelta {
            index: 0,
            delta: crate::anthropic::TextDelta {
                delta_type: "text_delta".to_string(),
                text: "Hello".to_string(),
            },
        });

        assert!(!processor.get_text().is_empty());
        processor.clear();
        assert!(processor.get_text().is_empty());
    }

    #[test]
    fn test_create_streaming_agent_helper() {
        let client = AnthropicClient::new("test-key".to_string()).unwrap();
        let registry = ExecutorRegistry::with_standard_tools();

        let agent = create_streaming_agent(client, registry);
        assert!(agent.system_prompt.is_none());
    }

    #[test]
    fn test_agent_stream_event_variants() {
        let event = AgentStreamEvent::Started;
        assert!(matches!(event, AgentStreamEvent::Started));

        let event = AgentStreamEvent::TextChunk {
            text: "test".to_string(),
        };
        match event {
            AgentStreamEvent::TextChunk { text } => assert_eq!(text, "test"),
            _ => panic!("Expected TextChunk"),
        }

        let event = AgentStreamEvent::Error {
            error: "test error".to_string(),
        };
        match event {
            AgentStreamEvent::Error { error } => assert_eq!(error, "test error"),
            _ => panic!("Expected Error"),
        }
    }
}

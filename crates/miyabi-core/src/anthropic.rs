//! Anthropic API Client
//!
//! This module provides a client for the Anthropic Messages API
//! with streaming support for real-time LLM responses.

use futures::stream::{Stream, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, error, warn};

/// Anthropic API base URL
const API_BASE_URL: &str = "https://api.anthropic.com";

/// Default model to use
pub const DEFAULT_MODEL: &str = "claude-sonnet-4-5-20250929";

/// Maximum retry attempts for transient errors
const MAX_RETRIES: u32 = 3;

/// Base delay for exponential backoff (in milliseconds)
const RETRY_BASE_DELAY_MS: u64 = 1000;

/// Retry configuration for API requests
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Base delay for exponential backoff (milliseconds)
    pub base_delay_ms: u64,
    /// Maximum delay cap (milliseconds)
    pub max_delay_ms: u64,
    /// Whether to retry on network errors
    pub retry_on_network_error: bool,
    /// Whether to retry on rate limits
    pub retry_on_rate_limit: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: MAX_RETRIES,
            base_delay_ms: RETRY_BASE_DELAY_MS,
            max_delay_ms: 60_000, // 1 minute max
            retry_on_network_error: true,
            retry_on_rate_limit: true,
        }
    }
}

impl RetryConfig {
    /// Create a new retry config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum retries
    pub fn with_max_retries(mut self, max: u32) -> Self {
        self.max_retries = max;
        self
    }

    /// Set base delay
    pub fn with_base_delay_ms(mut self, delay: u64) -> Self {
        self.base_delay_ms = delay;
        self
    }

    /// Set maximum delay cap
    pub fn with_max_delay_ms(mut self, delay: u64) -> Self {
        self.max_delay_ms = delay;
        self
    }

    /// Disable retries
    pub fn no_retries(mut self) -> Self {
        self.max_retries = 0;
        self
    }

    /// Calculate delay for attempt (exponential backoff)
    pub fn delay_for_attempt(&self, attempt: u32) -> u64 {
        let delay = self.base_delay_ms * 2u64.pow(attempt);
        delay.min(self.max_delay_ms)
    }
}

/// Anthropic API errors
#[derive(Error, Debug)]
pub enum AnthropicError {
    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Rate limit exceeded: retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },

    #[error("API error ({status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Stream error: {0}")]
    StreamError(String),

    #[error("Invalid configuration: {0}")]
    ConfigError(String),
}

/// Result type for Anthropic operations
pub type Result<T> = std::result::Result<T, AnthropicError>;

/// Message role
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

/// Content block types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

/// A message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

impl Message {
    /// Create a user message with text content
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: vec![ContentBlock::Text { text: text.into() }],
        }
    }

    /// Create an assistant message with text content
    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: vec![ContentBlock::Text { text: text.into() }],
        }
    }
}

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Request to the Messages API
#[derive(Debug, Serialize)]
pub struct MessagesRequest {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<bool>,
    pub stream: bool,
}

/// Stop reason for a response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    StopSequence,
    ToolUse,
    ModelContextWindowExceeded,
}

/// Usage statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Response from the Messages API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagesResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub response_type: String,
    pub role: Role,
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub stop_reason: Option<StopReason>,
    pub usage: Usage,
}

/// Streaming event types
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Message started
    MessageStart { message: MessagesResponse },
    /// Content block started
    ContentBlockStart {
        index: usize,
        content_block: ContentBlock,
    },
    /// Text delta in content
    ContentBlockDelta { index: usize, delta: TextDelta },
    /// Content block finished
    ContentBlockStop { index: usize },
    /// Message delta (stop reason, usage)
    MessageDelta { delta: MessageDelta, usage: Usage },
    /// Message completed
    MessageStop,
    /// Ping event
    Ping,
    /// Error event
    Error { error: String },
}

/// Text delta for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDelta {
    #[serde(rename = "type")]
    pub delta_type: String,
    pub text: String,
}

/// Message delta for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDelta {
    pub stop_reason: Option<StopReason>,
}

/// Anthropic API client
#[derive(Clone)]
pub struct AnthropicClient {
    client: reqwest::Client,
    api_key: String,
    model: String,
    max_tokens: u32,
    thinking: bool,
}

impl AnthropicClient {
    /// Create a new Anthropic client
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        let api_key = api_key.into();
        if api_key.is_empty() {
            return Err(AnthropicError::ConfigError(
                "API key cannot be empty".to_string(),
            ));
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .map_err(AnthropicError::NetworkError)?;

        Ok(Self {
            client,
            api_key,
            model: DEFAULT_MODEL.to_string(),
            max_tokens: 4096,
            thinking: false,
        })
    }

    /// Set the model to use
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set the maximum tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Enable or disable extended thinking
    pub fn with_thinking(mut self, thinking: bool) -> Self {
        self.thinking = thinking;
        self
    }

    /// Build request headers
    fn build_headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&self.api_key)
                .map_err(|_| AnthropicError::ConfigError("Invalid API key format".to_string()))?,
        );
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        Ok(headers)
    }

    /// Send a non-streaming message request
    pub async fn message(
        &self,
        messages: Vec<Message>,
        system: Option<String>,
        tools: Option<Vec<Tool>>,
        temperature: Option<f32>,
    ) -> Result<MessagesResponse> {
        let request = MessagesRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            messages,
            system,
            tools,
            temperature,
            thinking: self.thinking.then_some(true),
            stream: false,
        };

        self.send_with_retry(&request).await
    }

    /// Send request with retry logic
    async fn send_with_retry(&self, request: &MessagesRequest) -> Result<MessagesResponse> {
        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            match self.send_request(request).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    match &e {
                        AnthropicError::RateLimited { retry_after_ms } => {
                            if attempt < MAX_RETRIES - 1 {
                                warn!("Rate limited, retrying after {}ms", retry_after_ms);
                                tokio::time::sleep(Duration::from_millis(*retry_after_ms)).await;
                            }
                        }
                        AnthropicError::NetworkError(_) => {
                            if attempt < MAX_RETRIES - 1 {
                                let delay = RETRY_BASE_DELAY_MS * 2u64.pow(attempt);
                                warn!("Network error, retrying after {}ms", delay);
                                tokio::time::sleep(Duration::from_millis(delay)).await;
                            }
                        }
                        _ => return Err(e),
                    }
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or(AnthropicError::StreamError(
            "Max retries exceeded".to_string(),
        )))
    }

    /// Send a single request
    async fn send_request(&self, request: &MessagesRequest) -> Result<MessagesResponse> {
        let url = format!("{}/v1/messages", API_BASE_URL);
        let headers = self.build_headers()?;

        debug!("Sending request to {}", url);

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(request)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            return Err(self.handle_error_response(response).await);
        }

        let body = response.json().await?;
        Ok(body)
    }

    /// Handle error response
    async fn handle_error_response(&self, response: reqwest::Response) -> AnthropicError {
        let status = response.status().as_u16();

        // Check for rate limit headers
        if status == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(60)
                * 1000;

            return AnthropicError::RateLimited {
                retry_after_ms: retry_after,
            };
        }

        // Try to parse error body
        let message = match response.text().await {
            Ok(body) => {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                    json.get("error")
                        .and_then(|e| e.get("message"))
                        .and_then(|m| m.as_str())
                        .unwrap_or(&body)
                        .to_string()
                } else {
                    body
                }
            }
            Err(_) => "Unknown error".to_string(),
        };

        match status {
            401 => AnthropicError::AuthError(message),
            _ => AnthropicError::ApiError { status, message },
        }
    }

    /// Send a streaming message request
    pub async fn message_stream(
        &self,
        messages: Vec<Message>,
        system: Option<String>,
        tools: Option<Vec<Tool>>,
        temperature: Option<f32>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>> {
        let request = MessagesRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            messages,
            system,
            tools,
            temperature,
            thinking: self.thinking.then_some(true),
            stream: true,
        };

        let url = format!("{}/v1/messages", API_BASE_URL);
        let headers = self.build_headers()?;

        debug!("Starting stream request to {}", url);

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&request)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            return Err(self.handle_error_response(response).await);
        }

        let stream = response.bytes_stream();

        Ok(Box::pin(
            stream
                .scan(String::new(), |buffer, chunk| {
                    let result = match chunk {
                        Ok(bytes) => {
                            buffer.push_str(&String::from_utf8_lossy(&bytes));

                            let mut events = Vec::new();

                            // Parse SSE events from buffer
                            while let Some(event_end) = buffer.find("\n\n") {
                                let event_data = buffer[..event_end].to_string();
                                *buffer = buffer[event_end + 2..].to_string();

                                if let Some(event) = parse_sse_event(&event_data) {
                                    events.push(Ok(event));
                                }
                            }

                            Some(futures::stream::iter(events))
                        }
                        Err(e) => Some(futures::stream::iter(vec![Err(
                            AnthropicError::NetworkError(e),
                        )])),
                    };
                    async move { result }
                })
                .flatten(),
        ))
    }
}

/// Parse a single SSE event
fn parse_sse_event(event_data: &str) -> Option<StreamEvent> {
    let mut event_type = None;
    let mut data = None;

    for line in event_data.lines() {
        if let Some(value) = line.strip_prefix("event: ") {
            event_type = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("data: ") {
            data = Some(value.trim().to_string());
        }
    }

    let event_type = event_type?;
    let data = data?;

    match event_type.as_str() {
        "message_start" => {
            let parsed: serde_json::Value = serde_json::from_str(&data).ok()?;
            let message: MessagesResponse =
                serde_json::from_value(parsed.get("message")?.clone()).ok()?;
            Some(StreamEvent::MessageStart { message })
        }
        "content_block_start" => {
            let parsed: serde_json::Value = serde_json::from_str(&data).ok()?;
            let index = parsed.get("index")?.as_u64()? as usize;
            let content_block: ContentBlock =
                serde_json::from_value(parsed.get("content_block")?.clone()).ok()?;
            Some(StreamEvent::ContentBlockStart {
                index,
                content_block,
            })
        }
        "content_block_delta" => {
            let parsed: serde_json::Value = serde_json::from_str(&data).ok()?;
            let index = parsed.get("index")?.as_u64()? as usize;
            let delta: TextDelta = serde_json::from_value(parsed.get("delta")?.clone()).ok()?;
            Some(StreamEvent::ContentBlockDelta { index, delta })
        }
        "content_block_stop" => {
            let parsed: serde_json::Value = serde_json::from_str(&data).ok()?;
            let index = parsed.get("index")?.as_u64()? as usize;
            Some(StreamEvent::ContentBlockStop { index })
        }
        "message_delta" => {
            let parsed: serde_json::Value = serde_json::from_str(&data).ok()?;
            let delta: MessageDelta = serde_json::from_value(parsed.get("delta")?.clone()).ok()?;
            let usage: Usage = serde_json::from_value(parsed.get("usage")?.clone()).ok()?;
            Some(StreamEvent::MessageDelta { delta, usage })
        }
        "message_stop" => Some(StreamEvent::MessageStop),
        "ping" => Some(StreamEvent::Ping),
        "error" => {
            let parsed: serde_json::Value = serde_json::from_str(&data).ok()?;
            let error = parsed.get("error")?.get("message")?.as_str()?.to_string();
            Some(StreamEvent::Error { error })
        }
        _ => {
            debug!("Unknown event type: {}", event_type);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = AnthropicClient::new("test-api-key");
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_empty_key() {
        let client = AnthropicClient::new("");
        assert!(matches!(client, Err(AnthropicError::ConfigError(_))));
    }

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content.len(), 1);
    }

    #[test]
    fn test_client_builder() {
        let client = AnthropicClient::new("test-key")
            .unwrap()
            .with_model("claude-3-opus")
            .with_max_tokens(8192);

        assert_eq!(client.model, "claude-3-opus");
        assert_eq!(client.max_tokens, 8192);
    }

    #[test]
    fn test_parse_sse_text_delta() {
        let event_data = "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}";
        let event = parse_sse_event(event_data);

        assert!(matches!(event, Some(StreamEvent::ContentBlockDelta { .. })));
    }

    #[test]
    fn test_parse_sse_message_stop() {
        let event_data = "event: message_stop\ndata: {}";
        let event = parse_sse_event(event_data);

        assert!(matches!(event, Some(StreamEvent::MessageStop)));
    }

    #[test]
    fn test_role_serialization() {
        let user = Role::User;
        let json = serde_json::to_string(&user).unwrap();
        assert_eq!(json, "\"user\"");

        let assistant = Role::Assistant;
        let json = serde_json::to_string(&assistant).unwrap();
        assert_eq!(json, "\"assistant\"");
    }

    #[test]
    fn test_content_block_text() {
        let block = ContentBlock::Text {
            text: "Hello".to_string(),
        };
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"text\":\"Hello\""));
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.base_delay_ms, 1000);
        assert_eq!(config.max_delay_ms, 60_000);
        assert!(config.retry_on_network_error);
        assert!(config.retry_on_rate_limit);
    }

    #[test]
    fn test_retry_config_builder() {
        let config = RetryConfig::new()
            .with_max_retries(5)
            .with_base_delay_ms(500);

        assert_eq!(config.max_retries, 5);
        assert_eq!(config.base_delay_ms, 500);
    }

    #[test]
    fn test_retry_config_no_retries() {
        let config = RetryConfig::new().no_retries();
        assert_eq!(config.max_retries, 0);
    }

    #[test]
    fn test_retry_delay_calculation() {
        let config = RetryConfig::new().with_base_delay_ms(1000);

        // Exponential backoff: 1000, 2000, 4000, 8000...
        assert_eq!(config.delay_for_attempt(0), 1000);
        assert_eq!(config.delay_for_attempt(1), 2000);
        assert_eq!(config.delay_for_attempt(2), 4000);
        assert_eq!(config.delay_for_attempt(3), 8000);
    }

    #[test]
    fn test_retry_delay_capped_at_max() {
        let config = RetryConfig::new()
            .with_base_delay_ms(1000)
            .with_max_delay_ms(5000);

        // Should be capped at 5000ms
        assert_eq!(config.delay_for_attempt(0), 1000);
        assert_eq!(config.delay_for_attempt(1), 2000);
        assert_eq!(config.delay_for_attempt(2), 4000);
        assert_eq!(config.delay_for_attempt(3), 5000); // capped
        assert_eq!(config.delay_for_attempt(10), 5000); // still capped
    }

    #[test]
    fn test_error_types() {
        let auth_err = AnthropicError::AuthError("invalid key".to_string());
        assert!(auth_err.to_string().contains("invalid key"));

        let rate_err = AnthropicError::RateLimited {
            retry_after_ms: 5000,
        };
        assert!(rate_err.to_string().contains("5000"));

        let api_err = AnthropicError::ApiError {
            status: 500,
            message: "Internal error".to_string(),
        };
        assert!(api_err.to_string().contains("500"));
        assert!(api_err.to_string().contains("Internal error"));
    }

    #[test]
    fn test_tool_definition() {
        let tool = Tool {
            name: "read".to_string(),
            description: "Read a file".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            }),
        };

        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("\"name\":\"read\""));
        assert!(json.contains("\"description\":\"Read a file\""));
    }
}

//! Token Counting and Context Window Management
//!
//! This module provides token estimation and context window management
//! for Claude API conversations.

use crate::anthropic::{ContentBlock, Message, DEFAULT_MODEL};
use crate::conversation::{Conversation, ConversationMessage};
use serde::{Deserialize, Serialize};

/// Claude model context limits (in tokens)
#[derive(Debug, Clone, Copy)]
pub struct ModelLimits {
    /// Maximum context window size
    pub context_window: usize,
    /// Maximum output tokens
    pub max_output: usize,
}

impl ModelLimits {
    /// Claude 4.5 Sonnet limits
    pub const CLAUDE_4_5_SONNET: Self = Self {
        context_window: 200_000,
        max_output: 4_096,
    };

    /// Claude 4.5 Haiku limits
    pub const CLAUDE_4_5_HAIKU: Self = Self {
        context_window: 200_000,
        max_output: 4_096,
    };

    /// Claude 3 Opus limits
    pub const CLAUDE_3_OPUS: Self = Self {
        context_window: 200_000,
        max_output: 4_096,
    };

    /// Claude 3 Sonnet limits
    pub const CLAUDE_3_SONNET: Self = Self {
        context_window: 200_000,
        max_output: 4_096,
    };

    /// Claude 3 Haiku limits
    pub const CLAUDE_3_HAIKU: Self = Self {
        context_window: 200_000,
        max_output: 4_096,
    };

    /// Get limits for a model name
    pub fn for_model(model: &str) -> Self {
        let lower = model.to_lowercase();

        if lower.contains("sonnet-4-5") {
            Self::CLAUDE_4_5_SONNET
        } else if lower.contains("haiku-4-5") {
            Self::CLAUDE_4_5_HAIKU
        } else if lower.contains("opus") {
            Self::CLAUDE_3_OPUS
        } else if lower.contains("haiku") {
            Self::CLAUDE_3_HAIKU
        } else {
            Self::CLAUDE_3_SONNET
        }
    }
}

/// Token usage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Estimated input tokens
    pub input_tokens: usize,
    /// Output tokens (from API response)
    pub output_tokens: usize,
    /// Total tokens used
    pub total_tokens: usize,
}

impl TokenUsage {
    /// Create new usage
    pub fn new(input: usize, output: usize) -> Self {
        Self {
            input_tokens: input,
            output_tokens: output,
            total_tokens: input + output,
        }
    }

    /// Add usage
    pub fn add(&mut self, other: &TokenUsage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.total_tokens += other.total_tokens;
    }
}

/// Token counter for estimating token usage
#[derive(Debug, Clone)]
pub struct TokenCounter {
    /// Model limits
    pub limits: ModelLimits,
    /// Characters per token estimate (Claude uses ~4 chars/token on average)
    pub chars_per_token: f32,
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenCounter {
    /// Create a new token counter with default settings
    pub fn new() -> Self {
        Self::with_model(DEFAULT_MODEL)
    }

    /// Create with specific model limits
    pub fn with_model(model: &str) -> Self {
        Self {
            limits: ModelLimits::for_model(model),
            chars_per_token: 4.0,
        }
    }

    /// Estimate tokens for text
    pub fn estimate_text(&self, text: &str) -> usize {
        (text.len() as f32 / self.chars_per_token).ceil() as usize
    }

    /// Estimate tokens for a content block
    pub fn estimate_content_block(&self, block: &ContentBlock) -> usize {
        match block {
            ContentBlock::Text { text } => self.estimate_text(text),
            ContentBlock::ToolUse { name, input, .. } => {
                let input_str = serde_json::to_string(input).unwrap_or_default();
                self.estimate_text(name) + self.estimate_text(&input_str) + 20
            }
            ContentBlock::ToolResult { content, .. } => self.estimate_text(content) + 20,
        }
    }

    /// Estimate tokens for a message
    pub fn estimate_message(&self, message: &Message) -> usize {
        let content_tokens: usize = message
            .content
            .iter()
            .map(|b| self.estimate_content_block(b))
            .sum();

        // Add overhead for message structure
        content_tokens + 10
    }

    /// Estimate tokens for a conversation message
    pub fn estimate_conversation_message(&self, message: &ConversationMessage) -> usize {
        let content_tokens: usize = message
            .content
            .iter()
            .map(|b| self.estimate_content_block(b))
            .sum();

        content_tokens + 10
    }

    /// Estimate tokens for a conversation
    pub fn estimate_conversation(&self, conversation: &Conversation) -> usize {
        let mut total = 0;

        // System prompt
        if let Some(ref prompt) = conversation.system_prompt {
            total += self.estimate_text(prompt) + 10;
        }

        // Messages
        for message in &conversation.messages {
            total += self.estimate_conversation_message(message);
        }

        total
    }

    /// Get available tokens for output
    pub fn available_output_tokens(&self, conversation: &Conversation) -> usize {
        let input = self.estimate_conversation(conversation);
        let available = self.limits.context_window.saturating_sub(input);
        available.min(self.limits.max_output)
    }

    /// Check if conversation is within limits
    pub fn within_limits(&self, conversation: &Conversation) -> bool {
        let tokens = self.estimate_conversation(conversation);
        tokens < self.limits.context_window - self.limits.max_output
    }

    /// Get context window size
    pub fn context_window(&self) -> usize {
        self.limits.context_window
    }

    /// Get max output tokens
    pub fn max_output(&self) -> usize {
        self.limits.max_output
    }
}

/// Context window manager for automatic pruning
#[derive(Debug, Clone)]
pub struct ContextManager {
    /// Token counter
    pub counter: TokenCounter,
    /// Target utilization (0.0 - 1.0)
    pub target_utilization: f32,
    /// Minimum messages to keep
    pub min_messages: usize,
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextManager {
    /// Create a new context manager
    pub fn new() -> Self {
        Self {
            counter: TokenCounter::new(),
            target_utilization: 0.8, // Keep 80% of context for input
            min_messages: 2,         // Keep at least 2 messages
        }
    }

    /// Create with specific model
    pub fn with_model(model: &str) -> Self {
        Self {
            counter: TokenCounter::with_model(model),
            ..Default::default()
        }
    }

    /// Set target utilization
    pub fn with_target_utilization(mut self, utilization: f32) -> Self {
        self.target_utilization = utilization.clamp(0.1, 0.95);
        self
    }

    /// Set minimum messages to keep
    pub fn with_min_messages(mut self, min: usize) -> Self {
        self.min_messages = min.max(1);
        self
    }

    /// Get current token estimate
    pub fn estimate_tokens(&self, conversation: &Conversation) -> usize {
        self.counter.estimate_conversation(conversation)
    }

    /// Get target token limit
    pub fn target_limit(&self) -> usize {
        (self.counter.context_window() as f32 * self.target_utilization) as usize
    }

    /// Check if pruning is needed
    pub fn needs_pruning(&self, conversation: &Conversation) -> bool {
        self.estimate_tokens(conversation) > self.target_limit()
    }

    /// Prune conversation to fit within limits
    ///
    /// Removes oldest messages while keeping system prompt and minimum messages
    pub fn prune(&self, conversation: &mut Conversation) -> usize {
        let target = self.target_limit();
        let mut removed = 0;

        while self.estimate_tokens(conversation) > target
            && conversation.message_count() > self.min_messages
        {
            conversation.messages.remove(0);
            removed += 1;
        }

        removed
    }

    /// Get usage statistics
    pub fn get_usage(&self, conversation: &Conversation) -> ContextUsage {
        let tokens = self.estimate_tokens(conversation);
        let limit = self.counter.context_window();
        let available = self.counter.available_output_tokens(conversation);

        ContextUsage {
            used_tokens: tokens,
            total_tokens: limit,
            available_output: available,
            utilization: tokens as f32 / limit as f32,
            message_count: conversation.message_count(),
        }
    }
}

/// Context usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextUsage {
    /// Tokens used by current context
    pub used_tokens: usize,
    /// Total context window size
    pub total_tokens: usize,
    /// Available tokens for output
    pub available_output: usize,
    /// Utilization percentage (0.0 - 1.0)
    pub utilization: f32,
    /// Number of messages
    pub message_count: usize,
}

impl ContextUsage {
    /// Format as display string
    pub fn display(&self) -> String {
        format!(
            "{}/{} tokens ({:.1}%) | {} messages",
            self.used_tokens,
            self.total_tokens,
            self.utilization * 100.0,
            self.message_count
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_estimation() {
        let counter = TokenCounter::new();

        // ~4 chars per token
        let tokens = counter.estimate_text("Hello, World!"); // 13 chars
        assert!((3..=5).contains(&tokens));
    }

    #[test]
    fn test_model_limits() {
        let limits = ModelLimits::for_model("claude-3-opus");
        assert_eq!(limits.context_window, 200_000);

        let limits = ModelLimits::for_model("claude-3-sonnet");
        assert_eq!(limits.context_window, 200_000);

        let limits = ModelLimits::for_model("claude-sonnet-4-5-20250929");
        assert_eq!(limits.context_window, 200_000);

        let limits = ModelLimits::for_model("claude-haiku-4-5-20251001");
        assert_eq!(limits.context_window, 200_000);
    }

    #[test]
    fn test_conversation_estimation() {
        let counter = TokenCounter::new();
        let mut conv = Conversation::new().with_system_prompt("You are a helpful assistant");

        conv.add_user_message("Hello");
        conv.add_assistant_message("Hi there!");

        let tokens = counter.estimate_conversation(&conv);
        assert!(tokens > 0);
    }

    #[test]
    fn test_within_limits() {
        let counter = TokenCounter::new();
        let conv = Conversation::new().with_system_prompt("Test");

        assert!(counter.within_limits(&conv));
    }

    #[test]
    fn test_context_manager_creation() {
        let manager = ContextManager::new();
        assert_eq!(manager.target_utilization, 0.8);
    }

    #[test]
    fn test_context_manager_pruning() {
        // Create manager with small model limits for testing
        let mut manager = ContextManager::new();
        manager.counter.limits = ModelLimits {
            context_window: 100, // Very small for testing
            max_output: 10,
        };
        manager.target_utilization = 0.5; // 50 token target

        let mut conv = Conversation::new();
        // Add messages that will exceed the limit
        for i in 0..10 {
            // Each message is roughly 30+ tokens
            conv.add_user_message(format!(
                "This is message number {} with substantial content that uses many tokens",
                i
            ));
        }

        let initial = conv.message_count();
        let removed = manager.prune(&mut conv);

        assert!(
            removed > 0,
            "Expected messages to be removed, tokens: {}",
            manager.estimate_tokens(&conv)
        );
        assert!(conv.message_count() < initial);
    }

    #[test]
    fn test_context_usage_display() {
        let usage = ContextUsage {
            used_tokens: 1000,
            total_tokens: 200_000,
            available_output: 4096,
            utilization: 0.005,
            message_count: 5,
        };

        let display = usage.display();
        assert!(display.contains("1000"));
        assert!(display.contains("200000"));
    }

    #[test]
    fn test_available_output_tokens() {
        let counter = TokenCounter::new();
        let conv = Conversation::new();

        let available = counter.available_output_tokens(&conv);
        assert_eq!(available, counter.max_output());
    }

    #[test]
    fn test_token_usage() {
        let mut usage = TokenUsage::new(100, 50);
        assert_eq!(usage.total_tokens, 150);

        usage.add(&TokenUsage::new(50, 25));
        assert_eq!(usage.total_tokens, 225);
    }

    #[test]
    fn test_min_messages_preserved() {
        let manager = ContextManager::new()
            .with_target_utilization(0.001)
            .with_min_messages(5);

        let mut conv = Conversation::new();
        for i in 0..10 {
            conv.add_user_message(format!("Message {}", i));
        }

        manager.prune(&mut conv);
        assert!(conv.message_count() >= 5);
    }
}

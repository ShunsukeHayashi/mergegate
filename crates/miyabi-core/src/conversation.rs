//! Conversation State and Message History
//!
//! This module provides conversation management for maintaining context
//! across multiple interactions with the Claude API.

use crate::anthropic::{ContentBlock, Message, Role};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;
use uuid::Uuid;

/// Conversation errors
#[derive(Error, Debug)]
pub enum ConversationError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Conversation not found: {0}")]
    NotFound(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),
}

/// Result type for conversation operations
pub type Result<T> = std::result::Result<T, ConversationError>;

/// A timestamped message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    /// Message ID
    pub id: String,
    /// Role (user/assistant)
    pub role: Role,
    /// Message content
    pub content: Vec<ContentBlock>,
    /// When the message was created
    pub timestamp: DateTime<Utc>,
    /// Optional metadata
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl ConversationMessage {
    /// Create a new user message
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: Role::User,
            content: vec![ContentBlock::Text { text: text.into() }],
            timestamp: Utc::now(),
            metadata: serde_json::Value::Null,
        }
    }

    /// Create a new assistant message
    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: Role::Assistant,
            content: vec![ContentBlock::Text { text: text.into() }],
            timestamp: Utc::now(),
            metadata: serde_json::Value::Null,
        }
    }

    /// Create from an API message
    pub fn from_message(message: Message) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: message.role,
            content: message.content,
            timestamp: Utc::now(),
            metadata: serde_json::Value::Null,
        }
    }

    /// Convert to API message
    pub fn to_message(&self) -> Message {
        Message {
            role: self.role.clone(),
            content: self.content.clone(),
        }
    }

    /// Get text content
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|block| match block {
                ContentBlock::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Add metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Conversation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMetadata {
    /// Conversation title
    pub title: Option<String>,
    /// Tags for organization
    #[serde(default)]
    pub tags: Vec<String>,
    /// Model used
    pub model: Option<String>,
    /// Total tokens used
    #[serde(default)]
    pub total_tokens: u64,
    /// Custom metadata
    #[serde(default)]
    pub custom: serde_json::Value,
}

impl Default for ConversationMetadata {
    fn default() -> Self {
        Self {
            title: None,
            tags: Vec::new(),
            model: None,
            total_tokens: 0,
            custom: serde_json::Value::Null,
        }
    }
}

/// A conversation with message history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    /// Unique conversation ID
    pub id: String,
    /// When the conversation was created
    pub created_at: DateTime<Utc>,
    /// When the conversation was last updated
    pub updated_at: DateTime<Utc>,
    /// System prompt
    pub system_prompt: Option<String>,
    /// Message history
    pub messages: Vec<ConversationMessage>,
    /// Conversation metadata
    pub metadata: ConversationMetadata,
}

impl Default for Conversation {
    fn default() -> Self {
        Self::new()
    }
}

impl Conversation {
    /// Create a new conversation
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            created_at: now,
            updated_at: now,
            system_prompt: None,
            messages: Vec::new(),
            metadata: ConversationMetadata::default(),
        }
    }

    /// Create with a specific ID
    pub fn with_id(id: impl Into<String>) -> Self {
        let mut conv = Self::new();
        conv.id = id.into();
        conv
    }

    /// Set system prompt
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set title
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.metadata.title = Some(title.into());
        self
    }

    /// Set model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.metadata.model = Some(model.into());
        self
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        self.metadata.tags.push(tag.into());
    }

    /// Add a user message
    pub fn add_user_message(&mut self, text: impl Into<String>) -> &mut Self {
        self.messages.push(ConversationMessage::user(text));
        self.updated_at = Utc::now();
        self
    }

    /// Add an assistant message
    pub fn add_assistant_message(&mut self, text: impl Into<String>) -> &mut Self {
        self.messages.push(ConversationMessage::assistant(text));
        self.updated_at = Utc::now();
        self
    }

    /// Add a message
    pub fn add_message(&mut self, message: ConversationMessage) -> &mut Self {
        self.messages.push(message);
        self.updated_at = Utc::now();
        self
    }

    /// Get message count
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Get last message
    pub fn last_message(&self) -> Option<&ConversationMessage> {
        self.messages.last()
    }

    /// Get messages for API request
    pub fn to_api_messages(&self) -> Vec<Message> {
        self.messages.iter().map(|m| m.to_message()).collect()
    }

    /// Get system prompt for API
    pub fn get_system_prompt(&self) -> Option<String> {
        self.system_prompt.clone()
    }

    /// Update token count
    pub fn update_tokens(&mut self, tokens: u64) {
        self.metadata.total_tokens += tokens;
    }

    /// Clear message history
    pub fn clear(&mut self) {
        self.messages.clear();
        self.updated_at = Utc::now();
    }

    /// Truncate to last N messages
    pub fn truncate(&mut self, max_messages: usize) {
        if self.messages.len() > max_messages {
            let start = self.messages.len() - max_messages;
            self.messages = self.messages[start..].to_vec();
            self.updated_at = Utc::now();
        }
    }

    /// Save to file
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load from file
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let conv = serde_json::from_str(&json)?;
        Ok(conv)
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
}

/// Conversation manager for handling multiple conversations
#[derive(Debug, Clone, Default)]
pub struct ConversationManager {
    /// Active conversations
    conversations: std::collections::HashMap<String, Conversation>,
    /// Current conversation ID
    current_id: Option<String>,
}

impl ConversationManager {
    /// Create a new manager
    pub fn new() -> Self {
        Self {
            conversations: std::collections::HashMap::new(),
            current_id: None,
        }
    }

    /// Create a new conversation and make it current
    pub fn new_conversation(&mut self) -> &mut Conversation {
        let conv = Conversation::new();
        let id = conv.id.clone();
        self.conversations.insert(id.clone(), conv);
        self.current_id = Some(id.clone());
        self.conversations.get_mut(&id).unwrap()
    }

    /// Get current conversation
    pub fn current(&self) -> Option<&Conversation> {
        self.current_id
            .as_ref()
            .and_then(|id| self.conversations.get(id))
    }

    /// Get current conversation mutably
    pub fn current_mut(&mut self) -> Option<&mut Conversation> {
        let id = self.current_id.clone()?;
        self.conversations.get_mut(&id)
    }

    /// Set current conversation by ID
    pub fn set_current(&mut self, id: &str) -> Result<()> {
        if self.conversations.contains_key(id) {
            self.current_id = Some(id.to_string());
            Ok(())
        } else {
            Err(ConversationError::NotFound(id.to_string()))
        }
    }

    /// Get a conversation by ID
    pub fn get(&self, id: &str) -> Option<&Conversation> {
        self.conversations.get(id)
    }

    /// Get a conversation mutably by ID
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Conversation> {
        self.conversations.get_mut(id)
    }

    /// Add an existing conversation
    pub fn add(&mut self, conv: Conversation) {
        let id = conv.id.clone();
        self.conversations.insert(id, conv);
    }

    /// Remove a conversation
    pub fn remove(&mut self, id: &str) -> Option<Conversation> {
        if self.current_id.as_deref() == Some(id) {
            self.current_id = None;
        }
        self.conversations.remove(id)
    }

    /// List all conversation IDs
    pub fn list(&self) -> Vec<String> {
        self.conversations.keys().cloned().collect()
    }

    /// Get conversation count
    pub fn len(&self) -> usize {
        self.conversations.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.conversations.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_creation() {
        let conv = Conversation::new();
        assert!(conv.is_empty());
        assert!(conv.system_prompt.is_none());
    }

    #[test]
    fn test_add_messages() {
        let mut conv = Conversation::new();
        conv.add_user_message("Hello");
        conv.add_assistant_message("Hi there!");

        assert_eq!(conv.message_count(), 2);
        assert!(!conv.is_empty());
    }

    #[test]
    fn test_system_prompt() {
        let conv = Conversation::new().with_system_prompt("You are a helpful assistant");

        assert_eq!(
            conv.get_system_prompt(),
            Some("You are a helpful assistant".to_string())
        );
    }

    #[test]
    fn test_message_text() {
        let msg = ConversationMessage::user("Hello, world!");
        assert_eq!(msg.text(), "Hello, world!");
    }

    #[test]
    fn test_to_api_messages() {
        let mut conv = Conversation::new();
        conv.add_user_message("Hello");
        conv.add_assistant_message("Hi!");

        let messages = conv.to_api_messages();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(messages[1].role, Role::Assistant);
    }

    #[test]
    fn test_truncate() {
        let mut conv = Conversation::new();
        for i in 0..10 {
            conv.add_user_message(format!("Message {}", i));
        }

        conv.truncate(5);
        assert_eq!(conv.message_count(), 5);
    }

    #[test]
    fn test_clear() {
        let mut conv = Conversation::new();
        conv.add_user_message("Hello");
        conv.clear();

        assert!(conv.is_empty());
    }

    #[test]
    fn test_serialization() {
        let mut conv = Conversation::new()
            .with_system_prompt("Test prompt")
            .with_title("Test conversation");
        conv.add_user_message("Hello");

        let json = conv.to_json().unwrap();
        let restored = Conversation::from_json(&json).unwrap();

        assert_eq!(restored.id, conv.id);
        assert_eq!(restored.system_prompt, conv.system_prompt);
        assert_eq!(restored.message_count(), 1);
    }

    #[test]
    fn test_metadata() {
        let mut conv = Conversation::new()
            .with_title("Test")
            .with_model("claude-sonnet-4-20250514");
        conv.add_tag("important");
        conv.update_tokens(100);

        assert_eq!(conv.metadata.title, Some("Test".to_string()));
        assert_eq!(conv.metadata.tags, vec!["important"]);
        assert_eq!(conv.metadata.total_tokens, 100);
    }

    #[test]
    fn test_manager_creation() {
        let manager = ConversationManager::new();
        assert!(manager.is_empty());
    }

    #[test]
    fn test_manager_new_conversation() {
        let mut manager = ConversationManager::new();
        let conv = manager.new_conversation();
        conv.add_user_message("Hello");

        assert_eq!(manager.len(), 1);
        assert!(manager.current().is_some());
    }

    #[test]
    fn test_manager_multiple_conversations() {
        let mut manager = ConversationManager::new();

        let id1 = manager.new_conversation().id.clone();
        let id2 = manager.new_conversation().id.clone();

        assert_eq!(manager.len(), 2);
        assert_eq!(manager.current().unwrap().id, id2);

        manager.set_current(&id1).unwrap();
        assert_eq!(manager.current().unwrap().id, id1);
    }

    #[test]
    fn test_manager_remove() {
        let mut manager = ConversationManager::new();
        let id = manager.new_conversation().id.clone();

        let removed = manager.remove(&id);
        assert!(removed.is_some());
        assert!(manager.is_empty());
        assert!(manager.current().is_none());
    }

    #[test]
    fn test_last_message() {
        let mut conv = Conversation::new();
        conv.add_user_message("First");
        conv.add_assistant_message("Second");

        let last = conv.last_message().unwrap();
        assert_eq!(last.role, Role::Assistant);
    }
}

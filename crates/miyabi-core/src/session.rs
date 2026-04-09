//! Session Persistence
//!
//! Handles saving and loading conversation sessions to disk.

use crate::anthropic::Message;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

/// A complete session with metadata and conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session ID
    pub id: String,
    /// Session title
    pub title: String,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Model used
    pub model: String,
    /// Token usage
    pub tokens_used: usize,
    /// Session tags
    pub tags: Vec<String>,
    /// Conversation history
    pub messages: Vec<Message>,
    /// System prompt used
    pub system_prompt: Option<String>,
}

impl Session {
    /// Create a new session
    pub fn new(title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            title: title.into(),
            created_at: now,
            updated_at: now,
            model: String::new(),
            tokens_used: 0,
            tags: Vec::new(),
            messages: Vec::new(),
            system_prompt: None,
        }
    }

    /// Create with a specific ID
    pub fn with_id(id: impl Into<String>, title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            title: title.into(),
            created_at: now,
            updated_at: now,
            model: String::new(),
            tokens_used: 0,
            tags: Vec::new(),
            messages: Vec::new(),
            system_prompt: None,
        }
    }

    /// Set model name
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set system prompt
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Add a message to the conversation
    pub fn push_message(&mut self, message: Message) {
        self.messages.push(message);
        self.updated_at = Utc::now();
    }

    /// Update token usage
    pub fn add_tokens(&mut self, tokens: usize) {
        self.tokens_used += tokens;
        self.updated_at = Utc::now();
    }

    /// Get message count
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Get last message preview
    pub fn preview(&self) -> String {
        self.messages
            .last()
            .and_then(|m| {
                m.content.first().and_then(|c| match c {
                    crate::anthropic::ContentBlock::Text { text } => Some(text.clone()),
                    _ => None,
                })
            })
            .unwrap_or_default()
            .chars()
            .take(100)
            .collect()
    }
}

/// Session storage manager
#[derive(Debug, Clone)]
pub struct SessionStorage {
    /// Directory for storing sessions
    sessions_dir: PathBuf,
}

impl SessionStorage {
    /// Create a new session storage
    pub fn new(sessions_dir: impl Into<PathBuf>) -> Self {
        Self {
            sessions_dir: sessions_dir.into(),
        }
    }

    /// Create with default directory (~/.miyabi/sessions)
    pub fn default_dir() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Self::new(home.join(".miyabi").join("sessions"))
    }

    /// Ensure the sessions directory exists
    pub fn ensure_dir(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.sessions_dir)
    }

    /// Get path for a session file
    fn session_path(&self, id: &str) -> PathBuf {
        self.sessions_dir.join(format!("{}.json", id))
    }

    /// Save a session to disk
    pub fn save(&self, session: &Session) -> anyhow::Result<()> {
        self.ensure_dir()?;
        let path = self.session_path(&session.id);
        let json = serde_json::to_string_pretty(session)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Load a session from disk
    pub fn load(&self, id: &str) -> anyhow::Result<Session> {
        let path = self.session_path(id);
        let json = fs::read_to_string(path)?;
        let session: Session = serde_json::from_str(&json)?;
        Ok(session)
    }

    /// Delete a session
    pub fn delete(&self, id: &str) -> std::io::Result<()> {
        let path = self.session_path(id);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    /// List all sessions
    pub fn list(&self) -> anyhow::Result<Vec<Session>> {
        self.ensure_dir()?;
        let mut sessions = Vec::new();

        for entry in fs::read_dir(&self.sessions_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(json) = fs::read_to_string(&path) {
                    if let Ok(session) = serde_json::from_str::<Session>(&json) {
                        sessions.push(session);
                    }
                }
            }
        }

        // Sort by updated_at descending
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(sessions)
    }

    /// List session metadata only (for picker)
    pub fn list_metadata(&self) -> anyhow::Result<Vec<SessionMetadata>> {
        let sessions = self.list()?;
        Ok(sessions.into_iter().map(|s| s.into()).collect())
    }

    /// Check if a session exists
    pub fn exists(&self, id: &str) -> bool {
        self.session_path(id).exists()
    }
}

/// Lightweight session metadata for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub model: String,
    pub tokens_used: usize,
    pub message_count: usize,
    pub preview: String,
    pub tags: Vec<String>,
}

impl From<Session> for SessionMetadata {
    fn from(session: Session) -> Self {
        let preview = session.preview();
        let message_count = session.messages.len();
        Self {
            id: session.id,
            title: session.title,
            created_at: session.created_at,
            updated_at: session.updated_at,
            model: session.model,
            tokens_used: session.tokens_used,
            message_count,
            preview,
            tags: session.tags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_session_creation() {
        let session = Session::new("Test Session");
        assert!(!session.id.is_empty());
        assert_eq!(session.title, "Test Session");
        assert_eq!(session.messages.len(), 0);
    }

    #[test]
    fn test_session_with_model() {
        let session = Session::new("Test").model("claude-3-sonnet");
        assert_eq!(session.model, "claude-3-sonnet");
    }

    #[test]
    fn test_session_push_message() {
        let mut session = Session::new("Test");
        session.push_message(Message::user("Hello"));
        assert_eq!(session.message_count(), 1);
    }

    #[test]
    fn test_session_preview() {
        let mut session = Session::new("Test");
        session.push_message(Message::user("Hello, world!"));
        assert_eq!(session.preview(), "Hello, world!");
    }

    #[test]
    fn test_session_tokens() {
        let mut session = Session::new("Test");
        session.add_tokens(100);
        session.add_tokens(50);
        assert_eq!(session.tokens_used, 150);
    }

    #[test]
    fn test_storage_save_load() {
        let dir = TempDir::new().unwrap();
        let storage = SessionStorage::new(dir.path());

        let mut session = Session::new("Test Session");
        session.push_message(Message::user("Hello"));

        // Save
        storage.save(&session).unwrap();

        // Load
        let loaded = storage.load(&session.id).unwrap();
        assert_eq!(loaded.title, "Test Session");
        assert_eq!(loaded.message_count(), 1);
    }

    #[test]
    fn test_storage_list() {
        let dir = TempDir::new().unwrap();
        let storage = SessionStorage::new(dir.path());

        // Create multiple sessions
        let session1 = Session::new("Session 1");
        let session2 = Session::new("Session 2");

        storage.save(&session1).unwrap();
        storage.save(&session2).unwrap();

        let sessions = storage.list().unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn test_storage_delete() {
        let dir = TempDir::new().unwrap();
        let storage = SessionStorage::new(dir.path());

        let session = Session::new("Test");
        storage.save(&session).unwrap();

        assert!(storage.exists(&session.id));
        storage.delete(&session.id).unwrap();
        assert!(!storage.exists(&session.id));
    }

    #[test]
    fn test_storage_list_empty() {
        let dir = TempDir::new().unwrap();
        let storage = SessionStorage::new(dir.path());

        let sessions = storage.list().unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_metadata_conversion() {
        let mut session = Session::new("Test");
        session.push_message(Message::user("Hello"));
        session.tokens_used = 100;

        let metadata: SessionMetadata = session.into();
        assert_eq!(metadata.title, "Test");
        assert_eq!(metadata.message_count, 1);
        assert_eq!(metadata.tokens_used, 100);
    }

    #[test]
    fn test_session_with_id() {
        let session = Session::with_id("custom-id", "Custom Session");
        assert_eq!(session.id, "custom-id");
        assert_eq!(session.title, "Custom Session");
    }

    #[test]
    fn test_session_with_system_prompt() {
        let session = Session::new("Test").system_prompt("You are a helpful assistant");
        assert_eq!(
            session.system_prompt,
            Some("You are a helpful assistant".to_string())
        );
    }

    #[test]
    fn test_session_tags() {
        let mut session = Session::new("Test");
        session.tags.push("development".to_string());
        session.tags.push("rust".to_string());
        assert_eq!(session.tags.len(), 2);
    }

    #[test]
    fn test_session_preview_truncation() {
        let mut session = Session::new("Test");
        let long_message = "A".repeat(200);
        session.push_message(Message::user(&long_message));
        let preview = session.preview();
        assert_eq!(preview.len(), 100);
    }

    #[test]
    fn test_session_preview_empty() {
        let session = Session::new("Test");
        assert!(session.preview().is_empty());
    }

    #[test]
    fn test_storage_default_dir() {
        let storage = SessionStorage::default_dir();
        // Should create storage without panic
        assert!(storage.sessions_dir.ends_with("sessions"));
    }

    #[test]
    fn test_storage_exists() {
        let dir = TempDir::new().unwrap();
        let storage = SessionStorage::new(dir.path());

        let session = Session::new("Test");
        assert!(!storage.exists(&session.id));

        storage.save(&session).unwrap();
        assert!(storage.exists(&session.id));
    }

    #[test]
    fn test_storage_list_metadata() {
        let dir = TempDir::new().unwrap();
        let storage = SessionStorage::new(dir.path());

        let mut session = Session::new("Test");
        session.push_message(Message::user("Hello"));
        storage.save(&session).unwrap();

        let metadata_list = storage.list_metadata().unwrap();
        assert_eq!(metadata_list.len(), 1);
        assert_eq!(metadata_list[0].message_count, 1);
    }

    #[test]
    fn test_session_builder_pattern() {
        let session = Session::new("Builder Test")
            .model("claude-3-opus")
            .system_prompt("Be concise");

        assert_eq!(session.title, "Builder Test");
        assert_eq!(session.model, "claude-3-opus");
        assert_eq!(session.system_prompt, Some("Be concise".to_string()));
    }

    #[test]
    fn test_storage_load_nonexistent() {
        let dir = TempDir::new().unwrap();
        let storage = SessionStorage::new(dir.path());

        let result = storage.load("nonexistent-id");
        assert!(result.is_err());
    }

    #[test]
    fn test_storage_delete_nonexistent() {
        let dir = TempDir::new().unwrap();
        let storage = SessionStorage::new(dir.path());

        // Should not error when deleting nonexistent
        let result = storage.delete("nonexistent-id");
        assert!(result.is_ok());
    }

    #[test]
    fn test_metadata_preview() {
        let mut session = Session::new("Test");
        session.push_message(Message::user("Preview text"));

        let metadata: SessionMetadata = session.into();
        assert_eq!(metadata.preview, "Preview text");
    }

    #[test]
    fn test_session_update_timestamp() {
        let mut session = Session::new("Test");
        let initial_updated = session.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        session.push_message(Message::user("Update"));

        assert!(session.updated_at > initial_updated);
    }
}

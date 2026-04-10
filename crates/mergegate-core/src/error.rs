//! Error types for Miyabi

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Git error: {0}")]
    Git(String),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Timeout after {0}ms")]
    Timeout(u64),

    #[error("GitHub error: {0}")]
    GitHub(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Tool error: {0}")]
    Tool(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Check if an error is retryable (transient)
impl Error {
    pub fn is_retryable(&self) -> bool {
        match self {
            // Always retryable
            Error::Timeout(_) => true,

            // HTTP errors - check message for transient patterns
            Error::Http(msg) => {
                let lower = msg.to_lowercase();
                lower.contains("timeout")
                    || lower.contains("connection")
                    || lower.contains("temporarily")
            }

            // GitHub errors - check for rate limiting
            Error::GitHub(msg) => {
                let lower = msg.to_lowercase();
                lower.contains("rate limit")
                    || lower.contains("retry")
                    || lower.contains("temporarily unavailable")
            }

            // IO errors - some kinds are retryable
            Error::Io(io_error) => matches!(
                io_error.kind(),
                std::io::ErrorKind::ConnectionRefused
                    | std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::TimedOut
                    | std::io::ErrorKind::Interrupted
                    | std::io::ErrorKind::WouldBlock
            ),

            // Git errors - check message for lock conflicts
            Error::Git(msg) => {
                let lower = msg.to_lowercase();
                lower.contains("lock") || lower.contains("unable to create")
            }

            // Never retryable
            Error::Agent(_) => false,
            Error::Auth(_) => false,
            Error::Config(_) => false,
            Error::Validation(_) => false,
            Error::Json(_) => false,
            Error::Tool(_) => false,
            Error::PermissionDenied(_) => false,
            Error::Other(_) => false,
        }
    }
}

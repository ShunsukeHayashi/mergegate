//! Retry logic with exponential backoff
//!
//! Provides utilities for retrying transient failures with exponential backoff.
//! Useful for network operations, external API calls, and transient errors.

use crate::error::{Error, Result};
use std::future::Future;
use std::time::Duration;

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (excluding the first attempt)
    pub max_attempts: u32,
    /// Initial delay before first retry (milliseconds)
    pub initial_delay_ms: u64,
    /// Maximum delay between retries (milliseconds)
    pub max_delay_ms: u64,
    /// Backoff multiplier (typically 2.0 for exponential backoff)
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 100,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Create a new retry configuration
    pub fn new(max_attempts: u32, initial_delay_ms: u64, max_delay_ms: u64) -> Self {
        Self {
            max_attempts,
            initial_delay_ms,
            max_delay_ms,
            backoff_multiplier: 2.0,
        }
    }

    /// Create aggressive retry config (5 attempts, 50ms initial, 10s max)
    pub fn aggressive() -> Self {
        Self {
            max_attempts: 5,
            initial_delay_ms: 50,
            max_delay_ms: 10000,
            backoff_multiplier: 2.0,
        }
    }

    /// Create conservative retry config (2 attempts, 500ms initial, 60s max)
    pub fn conservative() -> Self {
        Self {
            max_attempts: 2,
            initial_delay_ms: 500,
            max_delay_ms: 60000,
            backoff_multiplier: 2.0,
        }
    }

    /// Calculate delay for a given attempt number (0-indexed)
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay_ms = (self.initial_delay_ms as f64 * self.backoff_multiplier.powi(attempt as i32))
            .min(self.max_delay_ms as f64) as u64;
        Duration::from_millis(delay_ms)
    }
}

/// Retry a future with exponential backoff
///
/// # Arguments
/// * `config` - Retry configuration
/// * `operation` - Async operation to retry (must be Fn, not FnOnce)
///
/// # Returns
/// * `Ok(T)` - Operation succeeded
/// * `Err(Error)` - All retry attempts failed
///
/// # Example
/// ```ignore
/// let result = retry_with_backoff(
///     RetryConfig::default(),
///     || async { fetch_data().await }
/// ).await?;
/// ```
pub async fn retry_with_backoff<F, Fut, T>(config: RetryConfig, operation: F) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut last_error = None;

    for attempt in 0..=config.max_attempts {
        match operation().await {
            Ok(result) => {
                return Ok(result);
            }
            Err(error) => {
                // Check if error is retryable
                if !error.is_retryable() {
                    return Err(error);
                }

                last_error = Some(error);

                // Don't sleep after the last attempt
                if attempt < config.max_attempts {
                    let delay = config.calculate_delay(attempt);
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    // All attempts failed
    Err(last_error.unwrap_or_else(|| Error::Other("No error captured".to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.initial_delay_ms, 100);
        assert_eq!(config.max_delay_ms, 30000);
        assert_eq!(config.backoff_multiplier, 2.0);
    }

    #[tokio::test]
    async fn test_calculate_delay_exponential() {
        let config = RetryConfig::new(3, 100, 10000);

        assert_eq!(config.calculate_delay(0), Duration::from_millis(100));
        assert_eq!(config.calculate_delay(1), Duration::from_millis(200));
        assert_eq!(config.calculate_delay(2), Duration::from_millis(400));
        assert_eq!(config.calculate_delay(3), Duration::from_millis(800));
    }

    #[tokio::test]
    async fn test_retry_succeeds_immediately() {
        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = Arc::clone(&attempt_count);

        let config = RetryConfig::new(3, 10, 1000);

        let result = retry_with_backoff(config, || {
            let count = Arc::clone(&attempt_count_clone);
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Ok::<String, Error>("success".to_string())
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(attempt_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_succeeds_after_failures() {
        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = Arc::clone(&attempt_count);

        let config = RetryConfig::new(3, 10, 1000);

        let result = retry_with_backoff(config, || {
            let count = Arc::clone(&attempt_count_clone);
            async move {
                let current = count.fetch_add(1, Ordering::SeqCst);
                if current < 2 {
                    Err(Error::Timeout(1000))
                } else {
                    Ok::<String, Error>("success".to_string())
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_non_retryable_fails_immediately() {
        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = Arc::clone(&attempt_count);

        let config = RetryConfig::new(3, 10, 1000);

        let result = retry_with_backoff(config, || {
            let count = Arc::clone(&attempt_count_clone);
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Err::<String, Error>(Error::Validation("Invalid".to_string()))
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(attempt_count.load(Ordering::SeqCst), 1);
    }
}

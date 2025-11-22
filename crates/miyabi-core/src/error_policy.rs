//! Error handling policies for Miyabi system
//!
//! This module provides advanced error handling strategies:
//! - Circuit Breaker pattern for preventing cascading failures
//! - Fallback strategies for graceful degradation

use crate::error::Error;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Fallback strategy when execution fails
#[derive(Debug, Clone)]
pub enum FallbackStrategy {
    /// Accept partial success
    AcceptPartialSuccess {
        /// Minimum number of successful operations required
        min_successful: usize,
    },
    /// Retry with lower temperature
    RetryWithLowerTemperature {
        /// Amount to reduce temperature by
        temperature_reduction: f64,
    },
    /// Switch to a different LLM model
    SwitchModel {
        /// Fallback model name
        fallback_model: String,
    },
    /// Wait for human intervention
    WaitForHumanIntervention {
        /// Timeout before giving up
        timeout: Duration,
    },
    /// Skip the task entirely
    SkipTask,
}

impl Default for FallbackStrategy {
    fn default() -> Self {
        Self::AcceptPartialSuccess { min_successful: 1 }
    }
}

impl FallbackStrategy {
    /// Creates a partial success strategy with default threshold
    pub fn partial_success() -> Self {
        Self::AcceptPartialSuccess { min_successful: 1 }
    }

    /// Creates a temperature reduction strategy
    pub fn lower_temperature() -> Self {
        Self::RetryWithLowerTemperature {
            temperature_reduction: 0.2,
        }
    }

    /// Creates a model switch strategy
    pub fn switch_to_claude() -> Self {
        Self::SwitchModel {
            fallback_model: "claude-sonnet-4-5-20250929".to_string(),
        }
    }

    /// Creates a human intervention strategy
    pub fn wait_for_human() -> Self {
        Self::WaitForHumanIntervention {
            timeout: Duration::from_secs(24 * 60 * 60),
        }
    }
}

/// State of the circuit breaker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed - requests flow normally
    Closed,
    /// Circuit is open - requests are blocked
    Open,
    /// Circuit is half-open - testing if service recovered
    HalfOpen,
}

/// Circuit breaker for preventing cascading failures
///
/// The circuit breaker pattern prevents repeated attempts to execute
/// operations that are likely to fail, allowing the system to recover.
pub struct CircuitBreaker {
    /// Number of consecutive failures before opening circuit
    failure_threshold: usize,
    /// Number of consecutive successes before closing circuit
    success_threshold: usize,
    /// Duration to wait before transitioning from Open to HalfOpen
    timeout: Duration,
    /// Current circuit state
    state: Arc<Mutex<CircuitState>>,
    /// Count of consecutive failures
    consecutive_failures: Arc<Mutex<usize>>,
    /// Count of consecutive successes
    consecutive_successes: Arc<Mutex<usize>>,
    /// Time when circuit was opened
    opened_at: Arc<Mutex<Option<Instant>>>,
}

impl CircuitBreaker {
    /// Creates a new CircuitBreaker
    pub fn new(failure_threshold: usize, success_threshold: usize, timeout: Duration) -> Self {
        Self {
            failure_threshold,
            success_threshold,
            timeout,
            state: Arc::new(Mutex::new(CircuitState::Closed)),
            consecutive_failures: Arc::new(Mutex::new(0)),
            consecutive_successes: Arc::new(Mutex::new(0)),
            opened_at: Arc::new(Mutex::new(None)),
        }
    }

    /// Creates a CircuitBreaker with default settings
    pub fn default_config() -> Self {
        Self::new(5, 2, Duration::from_secs(60))
    }

    /// Executes the given operation through the circuit breaker
    pub async fn call<F, T, E>(&self, operation: F) -> Result<T, Error>
    where
        F: FnOnce() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send>>,
        E: std::error::Error + Send + Sync + 'static,
    {
        // Check if we should attempt reset
        if self.should_attempt_reset().await {
            *self.state.lock().await = CircuitState::HalfOpen;
        }

        let current_state = *self.state.lock().await;

        match current_state {
            CircuitState::Open => Err(Error::Other("Circuit breaker is open".to_string())),
            CircuitState::Closed | CircuitState::HalfOpen => match operation().await {
                Ok(result) => {
                    self.on_success().await;
                    Ok(result)
                }
                Err(e) => {
                    self.on_failure().await;
                    Err(Error::Other(e.to_string()))
                }
            },
        }
    }

    /// Records a successful operation
    async fn on_success(&self) {
        let mut successes = self.consecutive_successes.lock().await;
        *successes += 1;
        *self.consecutive_failures.lock().await = 0;

        if *successes >= self.success_threshold {
            let mut state = self.state.lock().await;
            if *state != CircuitState::Closed {
                *state = CircuitState::Closed;
                *self.opened_at.lock().await = None;
            }
            *successes = 0;
        }
    }

    /// Records a failed operation
    async fn on_failure(&self) {
        let mut failures = self.consecutive_failures.lock().await;
        *failures += 1;
        *self.consecutive_successes.lock().await = 0;

        if *failures >= self.failure_threshold {
            let mut state = self.state.lock().await;
            if *state == CircuitState::Closed {
                *state = CircuitState::Open;
                *self.opened_at.lock().await = Some(Instant::now());
            }
        }
    }

    /// Checks if circuit should attempt reset
    async fn should_attempt_reset(&self) -> bool {
        let state = *self.state.lock().await;
        if state != CircuitState::Open {
            return false;
        }

        let opened_at = self.opened_at.lock().await;
        if let Some(opened_time) = *opened_at {
            opened_time.elapsed() >= self.timeout
        } else {
            false
        }
    }

    /// Gets the current circuit state
    pub async fn state(&self) -> CircuitState {
        *self.state.lock().await
    }

    /// Gets the number of consecutive failures
    pub async fn consecutive_failures(&self) -> usize {
        *self.consecutive_failures.lock().await
    }

    /// Gets the number of consecutive successes
    pub async fn consecutive_successes(&self) -> usize {
        *self.consecutive_successes.lock().await
    }

    /// Resets the circuit breaker to closed state
    pub async fn reset(&self) {
        *self.state.lock().await = CircuitState::Closed;
        *self.consecutive_failures.lock().await = 0;
        *self.consecutive_successes.lock().await = 0;
        *self.opened_at.lock().await = None;
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::default_config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_failures() {
        let breaker = CircuitBreaker::new(3, 2, Duration::from_millis(100));
        assert_eq!(breaker.state().await, CircuitState::Closed);

        for _ in 0..3 {
            let result = breaker
                .call(|| {
                    Box::pin(async {
                        Result::<(), std::io::Error>::Err(std::io::Error::other("test error"))
                    })
                })
                .await;
            assert!(result.is_err());
        }

        assert_eq!(breaker.state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_blocks_when_open() {
        let breaker = CircuitBreaker::new(2, 2, Duration::from_secs(60));

        for _ in 0..2 {
            let _ = breaker
                .call(|| {
                    Box::pin(async {
                        Result::<(), std::io::Error>::Err(std::io::Error::other("error"))
                    })
                })
                .await;
        }

        assert_eq!(breaker.state().await, CircuitState::Open);

        let result = breaker
            .call(|| Box::pin(async { Ok::<(), std::io::Error>(()) }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_circuit_breaker_reset() {
        let breaker = CircuitBreaker::new(2, 2, Duration::from_secs(60));

        for _ in 0..2 {
            let _ = breaker
                .call(|| {
                    Box::pin(async {
                        Result::<(), std::io::Error>::Err(std::io::Error::other("error"))
                    })
                })
                .await;
        }

        assert_eq!(breaker.state().await, CircuitState::Open);
        breaker.reset().await;
        assert_eq!(breaker.state().await, CircuitState::Closed);
    }
}

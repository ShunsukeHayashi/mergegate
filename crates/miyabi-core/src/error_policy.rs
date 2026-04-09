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

    #[test]
    fn test_fallback_strategy_default() {
        let strategy = FallbackStrategy::default();
        match strategy {
            FallbackStrategy::AcceptPartialSuccess { min_successful } => {
                assert_eq!(min_successful, 1);
            }
            _ => panic!("Expected AcceptPartialSuccess"),
        }
    }

    #[test]
    fn test_fallback_strategy_partial_success() {
        let strategy = FallbackStrategy::partial_success();
        match strategy {
            FallbackStrategy::AcceptPartialSuccess { min_successful } => {
                assert_eq!(min_successful, 1);
            }
            _ => panic!("Expected AcceptPartialSuccess"),
        }
    }

    #[test]
    fn test_fallback_strategy_lower_temperature() {
        let strategy = FallbackStrategy::lower_temperature();
        match strategy {
            FallbackStrategy::RetryWithLowerTemperature {
                temperature_reduction,
            } => {
                assert_eq!(temperature_reduction, 0.2);
            }
            _ => panic!("Expected RetryWithLowerTemperature"),
        }
    }

    #[test]
    fn test_fallback_strategy_switch_model() {
        let strategy = FallbackStrategy::switch_to_claude();
        match strategy {
            FallbackStrategy::SwitchModel { fallback_model } => {
                assert_eq!(fallback_model, "claude-sonnet-4-5-20250929");
            }
            _ => panic!("Expected SwitchModel"),
        }
    }

    #[test]
    fn test_fallback_strategy_wait_for_human() {
        let strategy = FallbackStrategy::wait_for_human();
        match strategy {
            FallbackStrategy::WaitForHumanIntervention { timeout } => {
                assert_eq!(timeout, Duration::from_secs(24 * 60 * 60));
            }
            _ => panic!("Expected WaitForHumanIntervention"),
        }
    }

    #[test]
    fn test_fallback_strategy_skip_task() {
        let strategy = FallbackStrategy::SkipTask;
        assert!(matches!(strategy, FallbackStrategy::SkipTask));
    }

    #[test]
    fn test_circuit_state_equality() {
        assert_eq!(CircuitState::Closed, CircuitState::Closed);
        assert_eq!(CircuitState::Open, CircuitState::Open);
        assert_eq!(CircuitState::HalfOpen, CircuitState::HalfOpen);
        assert_ne!(CircuitState::Closed, CircuitState::Open);
        assert_ne!(CircuitState::Open, CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn test_circuit_breaker_default() {
        let breaker = CircuitBreaker::default();
        assert_eq!(breaker.state().await, CircuitState::Closed);
        assert_eq!(breaker.failure_threshold, 5);
        assert_eq!(breaker.success_threshold, 2);
        assert_eq!(breaker.timeout, Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_circuit_breaker_default_config() {
        let breaker = CircuitBreaker::default_config();
        assert_eq!(breaker.failure_threshold, 5);
        assert_eq!(breaker.success_threshold, 2);
        assert_eq!(breaker.timeout, Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_circuit_breaker_success_closes_circuit() {
        let breaker = CircuitBreaker::new(2, 2, Duration::from_millis(10));

        // Open the circuit
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

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(20)).await;

        // First success puts it in half-open
        let result = breaker
            .call(|| Box::pin(async { Ok::<i32, std::io::Error>(42) }))
            .await;
        assert!(result.is_ok());

        // Second success should close
        let result = breaker
            .call(|| Box::pin(async { Ok::<i32, std::io::Error>(42) }))
            .await;
        assert!(result.is_ok());

        assert_eq!(breaker.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_consecutive_failures() {
        let breaker = CircuitBreaker::new(3, 2, Duration::from_secs(60));

        // Record one failure
        let _ = breaker
            .call(|| {
                Box::pin(async {
                    Result::<(), std::io::Error>::Err(std::io::Error::other("error"))
                })
            })
            .await;
        assert_eq!(breaker.consecutive_failures().await, 1);

        // Record success - should reset failures
        let _ = breaker
            .call(|| Box::pin(async { Ok::<(), std::io::Error>(()) }))
            .await;
        assert_eq!(breaker.consecutive_failures().await, 0);
    }

    #[tokio::test]
    async fn test_circuit_breaker_consecutive_successes() {
        let breaker = CircuitBreaker::new(3, 2, Duration::from_secs(60));

        let _ = breaker
            .call(|| Box::pin(async { Ok::<(), std::io::Error>(()) }))
            .await;
        assert_eq!(breaker.consecutive_successes().await, 1);

        let _ = breaker
            .call(|| Box::pin(async { Ok::<(), std::io::Error>(()) }))
            .await;
        // After reaching success_threshold, successes is reset
        assert_eq!(breaker.consecutive_successes().await, 0);
    }

    #[tokio::test]
    async fn test_circuit_breaker_passes_result_through() {
        let breaker = CircuitBreaker::new(3, 2, Duration::from_secs(60));

        let result = breaker
            .call(|| Box::pin(async { Ok::<i32, std::io::Error>(42) }))
            .await;

        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_circuit_breaker_custom_thresholds() {
        let breaker = CircuitBreaker::new(1, 1, Duration::from_millis(10));

        // Single failure opens circuit
        let _ = breaker
            .call(|| {
                Box::pin(async {
                    Result::<(), std::io::Error>::Err(std::io::Error::other("error"))
                })
            })
            .await;
        assert_eq!(breaker.state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_reset_clears_counters() {
        let breaker = CircuitBreaker::new(3, 3, Duration::from_secs(60));

        // Record some failures
        for _ in 0..2 {
            let _ = breaker
                .call(|| {
                    Box::pin(async {
                        Result::<(), std::io::Error>::Err(std::io::Error::other("error"))
                    })
                })
                .await;
        }

        assert_eq!(breaker.consecutive_failures().await, 2);

        breaker.reset().await;

        assert_eq!(breaker.consecutive_failures().await, 0);
        assert_eq!(breaker.consecutive_successes().await, 0);
        assert_eq!(breaker.state().await, CircuitState::Closed);
    }

    #[test]
    fn test_fallback_strategy_custom_partial_success() {
        let strategy = FallbackStrategy::AcceptPartialSuccess { min_successful: 5 };
        match strategy {
            FallbackStrategy::AcceptPartialSuccess { min_successful } => {
                assert_eq!(min_successful, 5);
            }
            _ => panic!("Expected AcceptPartialSuccess"),
        }
    }

    #[test]
    fn test_fallback_strategy_custom_model() {
        let strategy = FallbackStrategy::SwitchModel {
            fallback_model: "gpt-4".to_string(),
        };
        match strategy {
            FallbackStrategy::SwitchModel { fallback_model } => {
                assert_eq!(fallback_model, "gpt-4");
            }
            _ => panic!("Expected SwitchModel"),
        }
    }

    #[test]
    fn test_fallback_strategy_custom_timeout() {
        let strategy = FallbackStrategy::WaitForHumanIntervention {
            timeout: Duration::from_secs(3600),
        };
        match strategy {
            FallbackStrategy::WaitForHumanIntervention { timeout } => {
                assert_eq!(timeout, Duration::from_secs(3600));
            }
            _ => panic!("Expected WaitForHumanIntervention"),
        }
    }
}

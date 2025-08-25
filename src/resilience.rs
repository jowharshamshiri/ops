// Advanced Resilience Patterns - Error Recovery Strategies
// Enhanced operations framework with retry, circuit breaker, and fallback patterns

use crate::operation::Operation;
use crate::context::OperationalContext;
use crate::error::OperationError;
use async_trait::async_trait;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use log::{warn, info, error};

/// Retry strategy configuration
#[derive(Debug, Clone)]
pub enum RetryStrategy {
    /// Fixed delay between retries
    FixedDelay { delay: Duration, max_attempts: u32 },
    /// Exponential backoff with jitter
    ExponentialBackoff { initial_delay: Duration, max_delay: Duration, multiplier: f64, max_attempts: u32 },
    /// Custom delay function
    Custom { delays: Vec<Duration> },
}

impl RetryStrategy {
    pub fn fixed(delay: Duration, max_attempts: u32) -> Self {
        Self::FixedDelay { delay, max_attempts }
    }

    pub fn exponential(initial_delay: Duration, max_attempts: u32) -> Self {
        Self::ExponentialBackoff {
            initial_delay,
            max_delay: Duration::from_secs(60),
            multiplier: 2.0,
            max_attempts,
        }
    }

    pub fn custom(delays: Vec<Duration>) -> Self {
        Self::Custom { delays }
    }

    pub fn get_delay(&self, attempt: u32) -> Option<Duration> {
        match self {
            Self::FixedDelay { delay, max_attempts } => {
                if attempt < *max_attempts {
                    Some(*delay)
                } else {
                    None
                }
            },
            Self::ExponentialBackoff { initial_delay, max_delay, multiplier, max_attempts } => {
                if attempt < *max_attempts {
                    let delay = Duration::from_millis(
                        (initial_delay.as_millis() as f64 * multiplier.powi(attempt as i32)).min(max_delay.as_millis() as f64) as u64
                    );
                    Some(delay)
                } else {
                    None
                }
            },
            Self::Custom { delays } => {
                delays.get(attempt as usize).copied()
            }
        }
    }

    pub fn max_attempts(&self) -> u32 {
        match self {
            Self::FixedDelay { max_attempts, .. } => *max_attempts,
            Self::ExponentialBackoff { max_attempts, .. } => *max_attempts,
            Self::Custom { delays } => delays.len() as u32,
        }
    }
}

/// Retry operation wrapper with configurable strategy
pub struct RetryOperation<T> {
    operation: Box<dyn Operation<T>>,
    strategy: RetryStrategy,
    retry_predicate: Option<Box<dyn Fn(&OperationError) -> bool + Send + Sync>>,
    operation_name: String,
}

impl<T> RetryOperation<T>
where
    T: Send + 'static,
{
    pub fn new(operation: Box<dyn Operation<T>>, strategy: RetryStrategy) -> Self {
        Self {
            operation,
            strategy,
            retry_predicate: None,
            operation_name: "RetryOperation".to_string(),
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.operation_name = name;
        self
    }

    pub fn with_retry_predicate<F>(mut self, predicate: F) -> Self 
    where 
        F: Fn(&OperationError) -> bool + Send + Sync + 'static,
    {
        self.retry_predicate = Some(Box::new(predicate));
        self
    }

    fn should_retry(&self, error: &OperationError) -> bool {
        if let Some(predicate) = &self.retry_predicate {
            predicate(error)
        } else {
            // Default: retry on execution failures and timeouts
            matches!(error, OperationError::ExecutionFailed(_) | OperationError::Timeout { .. })
        }
    }
}

#[async_trait]
impl<T> Operation<T> for RetryOperation<T>
where
    T: Send + 'static,
{
    async fn perform(&self, context: &mut OperationalContext) -> Result<T, OperationError> {
        let mut attempt = 0;
        let mut last_error = None;

        loop {
            info!("Attempting operation '{}' (attempt {}/{})", 
                self.operation_name, attempt + 1, self.strategy.max_attempts());

            match self.operation.perform(context).await {
                Ok(result) => {
                    if attempt > 0 {
                        info!("Operation '{}' succeeded after {} retries", self.operation_name, attempt);
                    }
                    return Ok(result);
                },
                Err(error) => {
                    last_error = Some(error.clone());
                    
                    if !self.should_retry(&error) {
                        warn!("Operation '{}' failed with non-retryable error: {:?}", self.operation_name, error);
                        return Err(error);
                    }

                    if let Some(delay) = self.strategy.get_delay(attempt) {
                        warn!("Operation '{}' failed on attempt {}, retrying in {:?}: {:?}", 
                            self.operation_name, attempt + 1, delay, error);
                        tokio::time::sleep(delay).await;
                        attempt += 1;
                    } else {
                        error!("Operation '{}' failed after {} attempts, giving up: {:?}", 
                            self.operation_name, attempt + 1, error);
                        return Err(OperationError::ExecutionFailed(
                            format!("Operation failed after {} retries. Last error: {:?}", attempt + 1, error)
                        ));
                    }
                }
            }
        }
    }
}

/// Circuit Breaker state
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,   // Normal operation
    Open,     // Failing fast
    HalfOpen, // Testing if service recovered
}

/// Circuit Breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub recovery_timeout: Duration,
    pub success_threshold: u32, // For half-open state
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(60),
            success_threshold: 3,
        }
    }
}

/// Circuit Breaker state tracking
#[derive(Debug)]
struct CircuitBreakerState {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<Instant>,
}

impl Default for CircuitBreakerState {
    fn default() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure_time: None,
        }
    }
}

/// Circuit Breaker operation wrapper
pub struct CircuitBreakerOperation<T> {
    operation: Box<dyn Operation<T>>,
    config: CircuitBreakerConfig,
    state: Arc<Mutex<CircuitBreakerState>>,
    operation_name: String,
}

impl<T> CircuitBreakerOperation<T>
where
    T: Send + 'static,
{
    pub fn new(operation: Box<dyn Operation<T>>, config: CircuitBreakerConfig) -> Self {
        Self {
            operation,
            config,
            state: Arc::new(Mutex::new(CircuitBreakerState::default())),
            operation_name: "CircuitBreakerOperation".to_string(),
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.operation_name = name;
        self
    }

    fn update_state_on_success(&self) {
        if let Ok(mut state) = self.state.lock() {
            match state.state {
                CircuitState::Closed => {
                    state.failure_count = 0;
                },
                CircuitState::HalfOpen => {
                    state.success_count += 1;
                    if state.success_count >= self.config.success_threshold {
                        info!("Circuit breaker for '{}' closing after {} successes", 
                            self.operation_name, state.success_count);
                        state.state = CircuitState::Closed;
                        state.failure_count = 0;
                        state.success_count = 0;
                        state.last_failure_time = None;
                    }
                },
                CircuitState::Open => {
                    // Should not happen in normal flow
                },
            }
        }
    }

    fn update_state_on_failure(&self) -> CircuitState {
        if let Ok(mut state) = self.state.lock() {
            state.failure_count += 1;
            state.last_failure_time = Some(Instant::now());

            match state.state {
                CircuitState::Closed => {
                    if state.failure_count >= self.config.failure_threshold {
                        warn!("Circuit breaker for '{}' opening after {} failures", 
                            self.operation_name, state.failure_count);
                        state.state = CircuitState::Open;
                    }
                },
                CircuitState::HalfOpen => {
                    warn!("Circuit breaker for '{}' reopening after failure in half-open state", 
                        self.operation_name);
                    state.state = CircuitState::Open;
                    state.success_count = 0;
                },
                CircuitState::Open => {
                    // Already open, stay open
                },
            }

            state.state.clone()
        } else {
            CircuitState::Closed // Fallback if lock fails
        }
    }

    fn should_attempt_call(&self) -> Result<CircuitState, OperationError> {
        if let Ok(mut state) = self.state.lock() {
            match state.state {
                CircuitState::Closed => Ok(CircuitState::Closed),
                CircuitState::HalfOpen => Ok(CircuitState::HalfOpen),
                CircuitState::Open => {
                    if let Some(last_failure) = state.last_failure_time {
                        if last_failure.elapsed() >= self.config.recovery_timeout {
                            info!("Circuit breaker for '{}' transitioning to half-open for testing", 
                                self.operation_name);
                            state.state = CircuitState::HalfOpen;
                            state.success_count = 0;
                            Ok(CircuitState::HalfOpen)
                        } else {
                            Err(OperationError::ExecutionFailed(
                                format!("Circuit breaker is OPEN for '{}'", self.operation_name)
                            ))
                        }
                    } else {
                        Err(OperationError::ExecutionFailed(
                            format!("Circuit breaker is OPEN for '{}'", self.operation_name)
                        ))
                    }
                }
            }
        } else {
            Err(OperationError::ExecutionFailed(
                "Failed to acquire circuit breaker state lock".to_string()
            ))
        }
    }
}

#[async_trait]
impl<T> Operation<T> for CircuitBreakerOperation<T>
where
    T: Send + 'static,
{
    async fn perform(&self, context: &mut OperationalContext) -> Result<T, OperationError> {
        // Check if we should attempt the call
        self.should_attempt_call()?;

        match self.operation.perform(context).await {
            Ok(result) => {
                self.update_state_on_success();
                Ok(result)
            },
            Err(error) => {
                let new_state = self.update_state_on_failure();
                if new_state == CircuitState::Open {
                    warn!("Circuit breaker for '{}' is now OPEN", self.operation_name);
                }
                Err(error)
            }
        }
    }
}

/// Fallback operation wrapper
pub struct FallbackOperation<T> {
    primary: Box<dyn Operation<T>>,
    fallback: Box<dyn Operation<T>>,
    operation_name: String,
    fallback_predicate: Option<Box<dyn Fn(&OperationError) -> bool + Send + Sync>>,
}

impl<T> FallbackOperation<T>
where
    T: Send + 'static,
{
    pub fn new(primary: Box<dyn Operation<T>>, fallback: Box<dyn Operation<T>>) -> Self {
        Self {
            primary,
            fallback,
            operation_name: "FallbackOperation".to_string(),
            fallback_predicate: None,
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.operation_name = name;
        self
    }

    pub fn with_fallback_predicate<F>(mut self, predicate: F) -> Self 
    where 
        F: Fn(&OperationError) -> bool + Send + Sync + 'static,
    {
        self.fallback_predicate = Some(Box::new(predicate));
        self
    }

    fn should_fallback(&self, error: &OperationError) -> bool {
        if let Some(predicate) = &self.fallback_predicate {
            predicate(error)
        } else {
            // Default: fallback on any error except context errors
            !matches!(error, OperationError::Context(_))
        }
    }
}

#[async_trait]
impl<T> Operation<T> for FallbackOperation<T>
where
    T: Send + 'static,
{
    async fn perform(&self, context: &mut OperationalContext) -> Result<T, OperationError> {
        match self.primary.perform(context).await {
            Ok(result) => Ok(result),
            Err(error) => {
                if self.should_fallback(&error) {
                    warn!("Primary operation '{}' failed, attempting fallback: {:?}", 
                        self.operation_name, error);
                    
                    match self.fallback.perform(context).await {
                        Ok(result) => {
                            info!("Fallback operation '{}' succeeded", self.operation_name);
                            Ok(result)
                        },
                        Err(fallback_error) => {
                            error!("Both primary and fallback failed for '{}'. Primary: {:?}, Fallback: {:?}", 
                                self.operation_name, error, fallback_error);
                            Err(OperationError::ExecutionFailed(
                                format!("Primary failed: {:?}, Fallback failed: {:?}", error, fallback_error)
                            ))
                        }
                    }
                } else {
                    warn!("Primary operation '{}' failed with non-fallback error: {:?}", 
                        self.operation_name, error);
                    Err(error)
                }
            }
        }
    }
}

/// Comprehensive resilience wrapper combining retry, circuit breaker, and fallback
pub struct ResilientOperation<T> {
    operation: Box<dyn Operation<T>>,
    retry_strategy: Option<RetryStrategy>,
    circuit_config: Option<CircuitBreakerConfig>,
    fallback: Option<Box<dyn Operation<T>>>,
    operation_name: String,
}

impl<T> ResilientOperation<T>
where
    T: Send + 'static,
{
    pub fn new(operation: Box<dyn Operation<T>>) -> Self {
        Self {
            operation,
            retry_strategy: None,
            circuit_config: None,
            fallback: None,
            operation_name: "ResilientOperation".to_string(),
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.operation_name = name;
        self
    }

    pub fn with_retry(mut self, strategy: RetryStrategy) -> Self {
        self.retry_strategy = Some(strategy);
        self
    }

    pub fn with_circuit_breaker(mut self, config: CircuitBreakerConfig) -> Self {
        self.circuit_config = Some(config);
        self
    }

    pub fn with_fallback(mut self, fallback: Box<dyn Operation<T>>) -> Self {
        self.fallback = Some(fallback);
        self
    }

    pub fn build(self) -> Box<dyn Operation<T>> {
        let mut operation = self.operation;

        // Apply circuit breaker first (innermost)
        if let Some(config) = self.circuit_config {
            operation = Box::new(CircuitBreakerOperation::new(operation, config)
                .with_name(format!("CB[{}]", self.operation_name)));
        }

        // Apply retry
        if let Some(strategy) = self.retry_strategy {
            operation = Box::new(RetryOperation::new(operation, strategy)
                .with_name(format!("Retry[{}]", self.operation_name)));
        }

        // Apply fallback (outermost)
        if let Some(fallback) = self.fallback {
            operation = Box::new(FallbackOperation::new(operation, fallback)
                .with_name(format!("Fallback[{}]", self.operation_name)));
        }

        operation
    }
}

/// Convenience functions for creating resilient operations

pub fn with_retry<T>(operation: Box<dyn Operation<T>>, strategy: RetryStrategy) -> RetryOperation<T>
where
    T: Send + 'static,
{
    RetryOperation::new(operation, strategy)
}

pub fn with_circuit_breaker<T>(operation: Box<dyn Operation<T>>, config: CircuitBreakerConfig) -> CircuitBreakerOperation<T>
where
    T: Send + 'static,
{
    CircuitBreakerOperation::new(operation, config)
}

pub fn with_fallback<T>(primary: Box<dyn Operation<T>>, fallback: Box<dyn Operation<T>>) -> FallbackOperation<T>
where
    T: Send + 'static,
{
    FallbackOperation::new(primary, fallback)
}

pub fn resilient<T>(operation: Box<dyn Operation<T>>) -> ResilientOperation<T>
where
    T: Send + 'static,
{
    ResilientOperation::new(operation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation::ClosureOperation;

    #[test]
    fn test_retry_strategy_fixed() {
        let strategy = RetryStrategy::fixed(Duration::from_millis(100), 3);
        
        assert_eq!(strategy.get_delay(0), Some(Duration::from_millis(100)));
        assert_eq!(strategy.get_delay(1), Some(Duration::from_millis(100)));
        assert_eq!(strategy.get_delay(2), Some(Duration::from_millis(100)));
        assert_eq!(strategy.get_delay(3), None);
        assert_eq!(strategy.max_attempts(), 3);
    }

    #[test]
    fn test_retry_strategy_exponential() {
        let strategy = RetryStrategy::exponential(Duration::from_millis(100), 3);
        
        assert_eq!(strategy.get_delay(0), Some(Duration::from_millis(100)));
        assert_eq!(strategy.get_delay(1), Some(Duration::from_millis(200)));
        assert_eq!(strategy.get_delay(2), Some(Duration::from_millis(400)));
        assert_eq!(strategy.get_delay(3), None);
    }

    #[test]
    fn test_retry_strategy_custom() {
        let delays = vec![
            Duration::from_millis(50),
            Duration::from_millis(200),
            Duration::from_millis(500),
        ];
        let strategy = RetryStrategy::custom(delays.clone());
        
        assert_eq!(strategy.get_delay(0), Some(Duration::from_millis(50)));
        assert_eq!(strategy.get_delay(1), Some(Duration::from_millis(200)));
        assert_eq!(strategy.get_delay(2), Some(Duration::from_millis(500)));
        assert_eq!(strategy.get_delay(3), None);
        assert_eq!(strategy.max_attempts(), 3);
    }

    #[tokio::test]
    async fn test_retry_operation_success_after_failures() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;
        
        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = attempt_count.clone();
        let operation = Box::new(ClosureOperation::new(move |_ctx| {
            let count = attempt_count_clone.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                if count < 2 {
                    Err(OperationError::ExecutionFailed("Simulated failure".to_string()))
                } else {
                    Ok("Success!".to_string())
                }
            })
        }));

        let retry_op = RetryOperation::new(operation, RetryStrategy::fixed(Duration::from_millis(10), 5));
        let mut context = OperationalContext::new();
        
        let result = retry_op.perform(&mut context).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success!");
    }

    #[tokio::test]
    async fn test_retry_operation_max_attempts_exceeded() {
        let operation = Box::new(ClosureOperation::new(|_ctx| {
            Box::pin(async move {
                Err(OperationError::ExecutionFailed("Always fails".to_string()))
            })
        }));

        let retry_op: RetryOperation<String> = RetryOperation::new(operation, RetryStrategy::fixed(Duration::from_millis(1), 2));
        let mut context = OperationalContext::new();
        
        let result = retry_op.perform(&mut context).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            OperationError::ExecutionFailed(msg) => {
                assert!(msg.contains("failed after") && msg.contains("retries"));
            },
            _ => panic!("Expected ExecutionFailed"),
        }
    }

    #[test]
    fn test_circuit_breaker_config() {
        let config = CircuitBreakerConfig::default();
        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.recovery_timeout, Duration::from_secs(60));
        assert_eq!(config.success_threshold, 3);

        let custom_config = CircuitBreakerConfig {
            failure_threshold: 3,
            recovery_timeout: Duration::from_secs(30),
            success_threshold: 2,
        };
        assert_eq!(custom_config.failure_threshold, 3);
    }

    #[tokio::test]
    async fn test_fallback_operation_primary_success() {
        let primary = Box::new(ClosureOperation::new(|_ctx| {
            Box::pin(async move { Ok("Primary result".to_string()) })
        }));
        
        let fallback = Box::new(ClosureOperation::new(|_ctx| {
            Box::pin(async move { Ok("Fallback result".to_string()) })
        }));

        let fallback_op = FallbackOperation::new(primary, fallback);
        let mut context = OperationalContext::new();
        
        let result = fallback_op.perform(&mut context).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Primary result");
    }

    #[tokio::test]
    async fn test_fallback_operation_fallback_success() {
        let primary = Box::new(ClosureOperation::new(|_ctx| {
            Box::pin(async move { 
                Err(OperationError::ExecutionFailed("Primary failed".to_string())) 
            })
        }));
        
        let fallback = Box::new(ClosureOperation::new(|_ctx| {
            Box::pin(async move { Ok("Fallback result".to_string()) })
        }));

        let fallback_op = FallbackOperation::new(primary, fallback);
        let mut context = OperationalContext::new();
        
        let result = fallback_op.perform(&mut context).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Fallback result");
    }

    #[tokio::test] 
    async fn test_resilient_operation_builder() {
        let operation = Box::new(ClosureOperation::new(|_ctx| {
            Box::pin(async move { Ok(42) })
        }));

        let resilient_op = resilient(operation)
            .with_name("TestOp".to_string())
            .with_retry(RetryStrategy::fixed(Duration::from_millis(10), 3))
            .with_circuit_breaker(CircuitBreakerConfig::default())
            .build();

        let mut context = OperationalContext::new();
        let result = resilient_op.perform(&mut context).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }
}
// Advanced Resilience Patterns - Error Recovery Strategies
// Enhanced ops framework with retry, circuit breaker, and fallback patterns

use crate::op::Op;
use crate::context::OpContext;
use crate::error::OpError;
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

/// Retry op wrapper with configurable strategy
pub struct RetryOp<T> {
    op: Box<dyn Op<T>>,
    strategy: RetryStrategy,
    retry_predicate: Option<Box<dyn Fn(&OpError) -> bool + Send + Sync>>,
    op_name: String,
}

impl<T> RetryOp<T>
where
    T: Send + 'static,
{
    pub fn new(op: Box<dyn Op<T>>, strategy: RetryStrategy) -> Self {
        Self {
            op,
            strategy,
            retry_predicate: None,
            op_name: "RetryOp".to_string(),
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.op_name = name;
        self
    }

    pub fn with_retry_predicate<F>(mut self, predicate: F) -> Self 
    where 
        F: Fn(&OpError) -> bool + Send + Sync + 'static,
    {
        self.retry_predicate = Some(Box::new(predicate));
        self
    }

    fn should_retry(&self, error: &OpError) -> bool {
        if let Some(predicate) = &self.retry_predicate {
            predicate(error)
        } else {
            // Default: retry on execution failures and timeouts
            matches!(error, OpError::ExecutionFailed(_) | OpError::Timeout { .. })
        }
    }
}

#[async_trait]
impl<T> Op<T> for RetryOp<T>
where
    T: Send + 'static,
{
    async fn perform(&self, context: &mut OpContext) -> Result<T, OpError> {
        let mut attempt = 0;
        let mut last_error = None;

        loop {
            info!("Attempting op '{}' (attempt {}/{})", 
                self.op_name, attempt + 1, self.strategy.max_attempts());

            match self.op.perform(context).await {
                Ok(result) => {
                    if attempt > 0 {
                        info!("Op '{}' succeeded after {} retries", self.op_name, attempt);
                    }
                    return Ok(result);
                },
                Err(error) => {
                    last_error = Some(error.clone());
                    
                    if !self.should_retry(&error) {
                        warn!("Op '{}' failed with non-retryable error: {:?}", self.op_name, error);
                        return Err(error);
                    }

                    if let Some(delay) = self.strategy.get_delay(attempt) {
                        warn!("Op '{}' failed on attempt {}, retrying in {:?}: {:?}", 
                            self.op_name, attempt + 1, delay, error);
                        tokio::time::sleep(delay).await;
                        attempt += 1;
                    } else {
                        error!("Op '{}' failed after {} attempts, giving up: {:?}", 
                            self.op_name, attempt + 1, error);
                        return Err(OpError::ExecutionFailed(
                            format!("Op failed after {} retries. Last error: {:?}", attempt + 1, error)
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
    Closed,   // Normal op
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

/// Circuit Breaker op wrapper
pub struct CircuitBreakerOp<T> {
    op: Box<dyn Op<T>>,
    config: CircuitBreakerConfig,
    state: Arc<Mutex<CircuitBreakerState>>,
    op_name: String,
}

impl<T> CircuitBreakerOp<T>
where
    T: Send + 'static,
{
    pub fn new(op: Box<dyn Op<T>>, config: CircuitBreakerConfig) -> Self {
        Self {
            op,
            config,
            state: Arc::new(Mutex::new(CircuitBreakerState::default())),
            op_name: "CircuitBreakerOp".to_string(),
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.op_name = name;
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
                            self.op_name, state.success_count);
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
                            self.op_name, state.failure_count);
                        state.state = CircuitState::Open;
                    }
                },
                CircuitState::HalfOpen => {
                    warn!("Circuit breaker for '{}' reopening after failure in half-open state", 
                        self.op_name);
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

    fn should_attempt_call(&self) -> Result<CircuitState, OpError> {
        if let Ok(mut state) = self.state.lock() {
            match state.state {
                CircuitState::Closed => Ok(CircuitState::Closed),
                CircuitState::HalfOpen => Ok(CircuitState::HalfOpen),
                CircuitState::Open => {
                    if let Some(last_failure) = state.last_failure_time {
                        if last_failure.elapsed() >= self.config.recovery_timeout {
                            info!("Circuit breaker for '{}' transitioning to half-open for testing", 
                                self.op_name);
                            state.state = CircuitState::HalfOpen;
                            state.success_count = 0;
                            Ok(CircuitState::HalfOpen)
                        } else {
                            Err(OpError::ExecutionFailed(
                                format!("Circuit breaker is OPEN for '{}'", self.op_name)
                            ))
                        }
                    } else {
                        Err(OpError::ExecutionFailed(
                            format!("Circuit breaker is OPEN for '{}'", self.op_name)
                        ))
                    }
                }
            }
        } else {
            Err(OpError::ExecutionFailed(
                "Failed to acquire circuit breaker state lock".to_string()
            ))
        }
    }
}

#[async_trait]
impl<T> Op<T> for CircuitBreakerOp<T>
where
    T: Send + 'static,
{
    async fn perform(&self, context: &mut OpContext) -> Result<T, OpError> {
        // Check if we should attempt the call
        self.should_attempt_call()?;

        match self.op.perform(context).await {
            Ok(result) => {
                self.update_state_on_success();
                Ok(result)
            },
            Err(error) => {
                let new_state = self.update_state_on_failure();
                if new_state == CircuitState::Open {
                    warn!("Circuit breaker for '{}' is now OPEN", self.op_name);
                }
                Err(error)
            }
        }
    }
}

/// Fallback op wrapper
pub struct FallbackOp<T> {
    primary: Box<dyn Op<T>>,
    fallback: Box<dyn Op<T>>,
    op_name: String,
    fallback_predicate: Option<Box<dyn Fn(&OpError) -> bool + Send + Sync>>,
}

impl<T> FallbackOp<T>
where
    T: Send + 'static,
{
    pub fn new(primary: Box<dyn Op<T>>, fallback: Box<dyn Op<T>>) -> Self {
        Self {
            primary,
            fallback,
            op_name: "FallbackOp".to_string(),
            fallback_predicate: None,
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.op_name = name;
        self
    }

    pub fn with_fallback_predicate<F>(mut self, predicate: F) -> Self 
    where 
        F: Fn(&OpError) -> bool + Send + Sync + 'static,
    {
        self.fallback_predicate = Some(Box::new(predicate));
        self
    }

    fn should_fallback(&self, error: &OpError) -> bool {
        if let Some(predicate) = &self.fallback_predicate {
            predicate(error)
        } else {
            // Default: fallback on any error except context errors
            !matches!(error, OpError::Context(_))
        }
    }
}

#[async_trait]
impl<T> Op<T> for FallbackOp<T>
where
    T: Send + 'static,
{
    async fn perform(&self, context: &mut OpContext) -> Result<T, OpError> {
        match self.primary.perform(context).await {
            Ok(result) => Ok(result),
            Err(error) => {
                if self.should_fallback(&error) {
                    warn!("Primary op '{}' failed, attempting fallback: {:?}", 
                        self.op_name, error);
                    
                    match self.fallback.perform(context).await {
                        Ok(result) => {
                            info!("Fallback op '{}' succeeded", self.op_name);
                            Ok(result)
                        },
                        Err(fallback_error) => {
                            error!("Both primary and fallback failed for '{}'. Primary: {:?}, Fallback: {:?}", 
                                self.op_name, error, fallback_error);
                            Err(OpError::ExecutionFailed(
                                format!("Primary failed: {:?}, Fallback failed: {:?}", error, fallback_error)
                            ))
                        }
                    }
                } else {
                    warn!("Primary op '{}' failed with non-fallback error: {:?}", 
                        self.op_name, error);
                    Err(error)
                }
            }
        }
    }
}

/// Comprehensive resilience wrapper combining retry, circuit breaker, and fallback
pub struct ResilientOp<T> {
    op: Box<dyn Op<T>>,
    retry_strategy: Option<RetryStrategy>,
    circuit_config: Option<CircuitBreakerConfig>,
    fallback: Option<Box<dyn Op<T>>>,
    op_name: String,
}

impl<T> ResilientOp<T>
where
    T: Send + 'static,
{
    pub fn new(op: Box<dyn Op<T>>) -> Self {
        Self {
            op,
            retry_strategy: None,
            circuit_config: None,
            fallback: None,
            op_name: "ResilientOp".to_string(),
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.op_name = name;
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

    pub fn with_fallback(mut self, fallback: Box<dyn Op<T>>) -> Self {
        self.fallback = Some(fallback);
        self
    }

    pub fn build(self) -> Box<dyn Op<T>> {
        let mut op = self.op;

        // Apply circuit breaker first (innermost)
        if let Some(config) = self.circuit_config {
            op = Box::new(CircuitBreakerOp::new(op, config)
                .with_name(format!("CB[{}]", self.op_name)));
        }

        // Apply retry
        if let Some(strategy) = self.retry_strategy {
            op = Box::new(RetryOp::new(op, strategy)
                .with_name(format!("Retry[{}]", self.op_name)));
        }

        // Apply fallback (outermost)
        if let Some(fallback) = self.fallback {
            op = Box::new(FallbackOp::new(op, fallback)
                .with_name(format!("Fallback[{}]", self.op_name)));
        }

        op
    }
}

/// Convenience functions for creating resilient ops

pub fn with_retry<T>(op: Box<dyn Op<T>>, strategy: RetryStrategy) -> RetryOp<T>
where
    T: Send + 'static,
{
    RetryOp::new(op, strategy)
}

pub fn with_circuit_breaker<T>(op: Box<dyn Op<T>>, config: CircuitBreakerConfig) -> CircuitBreakerOp<T>
where
    T: Send + 'static,
{
    CircuitBreakerOp::new(op, config)
}

pub fn with_fallback<T>(primary: Box<dyn Op<T>>, fallback: Box<dyn Op<T>>) -> FallbackOp<T>
where
    T: Send + 'static,
{
    FallbackOp::new(primary, fallback)
}

pub fn resilient<T>(op: Box<dyn Op<T>>) -> ResilientOp<T>
where
    T: Send + 'static,
{
    ResilientOp::new(op)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::op::ClosureOp;

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
    async fn test_retry_op_success_after_failures() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;
        
        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = attempt_count.clone();
        let op = Box::new(ClosureOp::new(move |_ctx| {
            let count = attempt_count_clone.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                if count < 2 {
                    Err(OpError::ExecutionFailed("Simulated failure".to_string()))
                } else {
                    Ok("Success!".to_string())
                }
            })
        }));

        let retry_op = RetryOp::new(op, RetryStrategy::fixed(Duration::from_millis(10), 5));
        let mut context = OpContext::new();
        
        let result = retry_op.perform(&mut context).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success!");
    }

    #[tokio::test]
    async fn test_retry_op_max_attempts_exceeded() {
        let op = Box::new(ClosureOp::new(|_ctx| {
            Box::pin(async move {
                Err(OpError::ExecutionFailed("Always fails".to_string()))
            })
        }));

        let retry_op: RetryOp<String> = RetryOp::new(op, RetryStrategy::fixed(Duration::from_millis(1), 2));
        let mut context = OpContext::new();
        
        let result = retry_op.perform(&mut context).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            OpError::ExecutionFailed(msg) => {
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
    async fn test_fallback_op_primary_success() {
        let primary = Box::new(ClosureOp::new(|_ctx| {
            Box::pin(async move { Ok("Primary result".to_string()) })
        }));
        
        let fallback = Box::new(ClosureOp::new(|_ctx| {
            Box::pin(async move { Ok("Fallback result".to_string()) })
        }));

        let fallback_op = FallbackOp::new(primary, fallback);
        let mut context = OpContext::new();
        
        let result = fallback_op.perform(&mut context).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Primary result");
    }

    #[tokio::test]
    async fn test_fallback_op_fallback_success() {
        let primary = Box::new(ClosureOp::new(|_ctx| {
            Box::pin(async move { 
                Err(OpError::ExecutionFailed("Primary failed".to_string())) 
            })
        }));
        
        let fallback = Box::new(ClosureOp::new(|_ctx| {
            Box::pin(async move { Ok("Fallback result".to_string()) })
        }));

        let fallback_op = FallbackOp::new(primary, fallback);
        let mut context = OpContext::new();
        
        let result = fallback_op.perform(&mut context).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Fallback result");
    }

    #[tokio::test] 
    async fn test_resilient_op_builder() {
        let op = Box::new(ClosureOp::new(|_ctx| {
            Box::pin(async move { Ok(42) })
        }));

        let resilient_op = resilient(op)
            .with_name("TestOp".to_string())
            .with_retry(RetryStrategy::fixed(Duration::from_millis(10), 3))
            .with_circuit_breaker(CircuitBreakerConfig::default())
            .build();

        let mut context = OpContext::new();
        let result = resilient_op.perform(&mut context).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }
}
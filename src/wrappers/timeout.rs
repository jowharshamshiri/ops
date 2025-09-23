use crate::prelude::*;
// TimeBoundWrapper implementation with full Java reference functionality
// Superior implementation using tokio::time::timeout vs Java's unsafe threading

use crate::op::Op;
use crate::context::OpContext;
use crate::error::OpError;
use async_trait::async_trait;
use std::time::{Duration, Instant};
use tracing::{warn, info};

pub struct TimeBoundWrapper<T> {
    wrapped_op: Box<dyn Op<T>>,
    timeout_duration: Duration,
    op_name: Option<String>,
    warn_on_timeout: bool,
}

impl<T> TimeBoundWrapper<T> {
    /// Create new timeout wrapper with duration
    pub fn new(op: Box<dyn Op<T>>, timeout: Duration) -> Self {
        Self {
            wrapped_op: op,
            timeout_duration: timeout,
            op_name: None,
            warn_on_timeout: true,
        }
    }

    /// Create timeout wrapper with op name for better logging
    pub fn with_name(op: Box<dyn Op<T>>, timeout: Duration, name: String) -> Self {
        Self {
            wrapped_op: op,
            timeout_duration: timeout,
            op_name: Some(name),
            warn_on_timeout: true,
        }
    }

    /// Create timeout wrapper with configurable timeout warnings
    pub fn with_warning_control(op: Box<dyn Op<T>>, timeout: Duration, warn: bool) -> Self {
        Self {
            wrapped_op: op,
            timeout_duration: timeout,
            op_name: None,
            warn_on_timeout: warn,
        }
    }

    /// Get op name for logging (fallback to generic name)
    fn get_op_name(&self) -> &str {
        self.op_name.as_deref().unwrap_or("TimeBoundOp")
    }

    /// Log timeout warning (matches Java TimeBoundOpWrapper behavior)
    fn log_timeout_warning(&self) {
        if self.warn_on_timeout {
            warn!(
                "Op '{}' was terminated due to timeout after {:?}",
                self.get_op_name(),
                self.timeout_duration
            );
        }
    }

    /// Log timeout info for successful completion near deadline
    fn log_near_timeout_completion(&self, duration: Duration) {
        let timeout_ratio = duration.as_secs_f64() / self.timeout_duration.as_secs_f64();
        if timeout_ratio > 0.8 {  // Completed using more than 80% of timeout
            info!(
                "Op '{}' completed in {:.3}s ({}% of {:?} timeout)",
                self.get_op_name(),
                duration.as_secs_f64(),
                (timeout_ratio * 100.0) as u32,
                self.timeout_duration
            );
        }
    }
}

#[async_trait]
impl<T> Op<T> for TimeBoundWrapper<T>
where
    T: Send + 'static,
{
    async fn perform(&self, context: &mut OpContext) -> Result<T, OpError> {
        let start_time = Instant::now();
        
        // Use tokio's timeout mechanism (superior to Java's manual threading)
        match tokio::time::timeout(self.timeout_duration, self.wrapped_op.perform(context)).await {
            Ok(result) => {
                // Log near-timeout completions for monitoring
                let duration = start_time.elapsed();
                self.log_near_timeout_completion(duration);
                result
            },
            Err(_timeout_elapsed) => {
                // Log timeout warning (matches Java behavior)
                self.log_timeout_warning();
                
                // Return structured timeout error with context
                Err(OpError::Timeout { 
                    timeout_ms: self.timeout_duration.as_millis() as u64 
                })
            }
        }
    }
}

/// Create timeout wrapper with automatic op name detection
/// Uses caller information for better error reporting
pub fn create_timeout_wrapper_with_caller_name<T>(
    op: Box<dyn Op<T>>, 
    timeout: Duration
) -> TimeBoundWrapper<T> 
where 
    T: Send + 'static,
{
    let caller_name = crate::ops::get_caller_op_name();
    TimeBoundWrapper::with_name(op, timeout, caller_name)
}

/// Create timeout wrapper with both logging and timeout functionality
/// Combines LoggingWrapper and TimeBoundWrapper (composition pattern)
pub fn create_logged_timeout_wrapper<T>(
    op: Box<dyn Op<T>>,
    timeout: Duration,
    op_name: String,
) -> crate::wrappers::logging::LoggingWrapper<T>
where 
    T: Send + 'static,
{
    // First wrap with timeout
    let timeout_wrapper = TimeBoundWrapper::with_name(op, timeout, op_name.clone());
    
    // Then wrap with logging
    crate::wrappers::logging::LoggingWrapper::new(
        Box::new(timeout_wrapper),
        format!("TimeBound[{}]", op_name)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::op::{Op, ClosureOp};
    use crate::context::OpContext;
    use std::time::Duration;

    #[tokio::test]
    async fn test_timeout_wrapper_success() {
        let mut context = OpContext::new();
        
        let op = Box::new(ClosureOp::new(|_ctx| {
            Box::pin(async move { 
                tokio::time::sleep(Duration::from_millis(50)).await;
                Ok(42) 
            })
        }));
        
        let timeout_wrapper = TimeBoundWrapper::new(op, Duration::from_millis(200));
        let result = timeout_wrapper.perform(&mut context).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_timeout_wrapper_timeout() {
        let mut context = OpContext::new();
        
        let op = Box::new(ClosureOp::new(|_ctx| {
            Box::pin(async move { 
                tokio::time::sleep(Duration::from_millis(200)).await;
                Ok(42) 
            })
        }));
        
        let timeout_wrapper = TimeBoundWrapper::new(op, Duration::from_millis(50));
        let result = timeout_wrapper.perform(&mut context).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            OpError::Timeout { timeout_ms } => {
                assert_eq!(timeout_ms, 50);
            },
            _ => panic!("Expected Timeout error"),
        }
    }

    #[tokio::test]
    async fn test_timeout_wrapper_with_name() {
        let mut context = OpContext::new();
        
        let op = Box::new(ClosureOp::new(|_ctx| {
            Box::pin(async move { Ok("success".to_string()) })
        }));
        
        let timeout_wrapper = TimeBoundWrapper::with_name(
            op, 
            Duration::from_millis(100),
            "TestOp".to_string()
        );
        let result = timeout_wrapper.perform(&mut context).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }

    #[tokio::test]
    async fn test_caller_name_wrapper() {
        let mut context = OpContext::new();
        
        let op = Box::new(ClosureOp::new(|_ctx| {
            Box::pin(async move { Ok(100) })
        }));
        
        let timeout_wrapper = create_timeout_wrapper_with_caller_name(op, Duration::from_millis(100));
        let result = timeout_wrapper.perform(&mut context).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 100);
    }

    #[tokio::test]
    async fn test_logged_timeout_wrapper() {
        tracing_subscriber::fmt::try_init().ok();
        let mut context = OpContext::new();
        
        let op = Box::new(ClosureOp::new(|_ctx| {
            Box::pin(async move { Ok("logged and timed".to_string()) })
        }));
        
        let wrapped = create_logged_timeout_wrapper(
            op,
            Duration::from_millis(100), 
            "CompositeOp".to_string()
        );
        let result = wrapped.perform(&mut context).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "logged and timed");
    }
}
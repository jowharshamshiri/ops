use crate::prelude::*;
// LoggingWrapper implementation with full Java reference functionality
// Implements LoggingOpWrapper.java patterns with Rust enhancements

use crate::op::Op;
use crate::context::OpContext;
use crate::error::OpError;
use async_trait::async_trait;
use tracing;
use std::time::Instant;

// ANSI color codes for console output (matches Java CONST class)
const YELLOW: &str = "\x1b[33m";
const GREEN: &str = "\x1b[32m"; 
const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";

pub struct LoggingWrapper<T> {
    wrapped_op: Box<dyn Op<T>>,
    op_name: String,
    logger_name: Option<String>,
}

impl<T> LoggingWrapper<T> {
    /// Create new logging wrapper with op name
    pub fn new(op: Box<dyn Op<T>>, name: String) -> Self {
        Self {
            wrapped_op: op,
            op_name: name,
            logger_name: None,
        }
    }

    /// Create logging wrapper with custom logger name
    /// Equivalent to dynamic logger creation in Java
    pub fn with_logger(op: Box<dyn Op<T>>, name: String, logger_name: String) -> Self {
        Self {
            wrapped_op: op,
            op_name: name,
            logger_name: Some(logger_name),
        }
    }

    /// Get the effective logger name (for context-aware logging)
    fn get_logger_name(&self) -> &str {
        self.logger_name.as_deref().unwrap_or("LoggingWrapper")
    }

    /// Log op start with ANSI colors
    fn log_op_start(&self) {
        tracing::info!(
            logger = self.get_logger_name(),
            "{}Starting op: {}{}", 
            YELLOW, 
            self.op_name,
            RESET
        );
    }

    /// Log op completion with timing
    fn log_op_success(&self, duration: std::time::Duration) {
        let seconds = duration.as_secs_f64();
        tracing::info!(
            logger = self.get_logger_name(),
            "{}Op '{}' completed in {:.3} seconds{}", 
            GREEN,
            self.op_name, 
            seconds,
            RESET
        );
    }

    /// Log op failure with full error context
    fn log_op_failure(&self, error: &OpError, duration: std::time::Duration) {
        let seconds = duration.as_secs_f64();
        tracing::error!(
            logger = self.get_logger_name(),
            "{}Op '{}' failed after {:.3} seconds: {:?}{}", 
            RED,
            self.op_name,
            seconds, 
            error,
            RESET
        );
    }
}

#[async_trait]
impl<T> Op<T> for LoggingWrapper<T>
where
    T: Send + 'static,
{
    async fn perform(&self, context: &mut OpContext) -> Result<T, OpError> {
        let start_time = Instant::now();
        
        // Log op start with yellow color
        self.log_op_start();
        
        // Execute wrapped op
        let result = self.wrapped_op.perform(context).await;
        
        let duration = start_time.elapsed();
        
        // Log result with appropriate color and timing
        match &result {
            Ok(_) => self.log_op_success(duration),
            Err(error) => {
                self.log_op_failure(error, duration);
                // Re-wrap error with op context (matches Java behavior)
                return Err(crate::ops::wrap_nested_op_exception(&self.op_name, 
                    OpError::ExecutionFailed(format!("{:?}", error))));
            }
        }
        
        result
    }
}

/// Create logger with dynamic name based on caller context
/// Equivalent to Java's getCallerCallerClassName() usage
pub fn create_context_aware_logger<T>(op: Box<dyn Op<T>>) -> LoggingWrapper<T> 
where 
    T: Send + 'static,
{
    let caller_name = crate::ops::get_caller_op_name();
    LoggingWrapper::with_logger(op, caller_name.clone(), caller_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::op::{Op, ClosureOp};
    use crate::context::OpContext;

    #[tokio::test]
    async fn test_logging_wrapper_success() {
        tracing_subscriber::fmt::try_init().ok(); // Initialize tracing for tests
        
        let mut context = OpContext::new();
        
        let op = Box::new(ClosureOp::new(|_ctx| {
            Box::pin(async move { Ok(42) })
        }));
        
        let logging_wrapper = LoggingWrapper::new(op, "TestOp".to_string());
        let result = logging_wrapper.perform(&mut context).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_logging_wrapper_failure() {
        tracing_subscriber::fmt::try_init().ok();
        
        let mut context = OpContext::new();
        
        let op = Box::new(ClosureOp::new(|_ctx| {
            Box::pin(async move { 
                Err(OpError::ExecutionFailed("test error".to_string())) 
            })
        }));
        
        let logging_wrapper: LoggingWrapper<i32> = LoggingWrapper::new(op, "FailingOp".to_string());
        let result = logging_wrapper.perform(&mut context).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            OpError::ExecutionFailed(msg) => {
                assert!(msg.contains("FailingOp"));
            },
            _ => panic!("Expected ExecutionFailed error"),
        }
    }

    #[tokio::test]
    async fn test_context_aware_logger() {
        let mut context = OpContext::new();
        
        let op = Box::new(ClosureOp::new(|_ctx| {
            Box::pin(async move { Ok("test".to_string()) })
        }));
        
        let logging_wrapper = create_context_aware_logger(op);
        let result = logging_wrapper.perform(&mut context).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test");
    }

    #[test]
    fn test_ansi_color_constants() {
        assert_eq!(YELLOW, "\x1b[33m");
        assert_eq!(GREEN, "\x1b[32m");
        assert_eq!(RED, "\x1b[31m");
        assert_eq!(RESET, "\x1b[0m");
    }
}
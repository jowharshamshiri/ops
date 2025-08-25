// LoggingWrapper implementation with full Java reference functionality
// Implements LoggingOpWrapper.java patterns with Rust enhancements

use crate::operation::Operation;
use crate::context::OperationalContext;
use crate::error::OperationError;
use async_trait::async_trait;
use log::{info, error};
use std::time::Instant;

// ANSI color codes for console output (matches Java CONST class)
const YELLOW: &str = "\x1b[33m";
const GREEN: &str = "\x1b[32m"; 
const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";

pub struct LoggingWrapper<T> {
    wrapped_operation: Box<dyn Operation<T>>,
    operation_name: String,
    logger_name: Option<String>,
}

impl<T> LoggingWrapper<T> {
    /// Create new logging wrapper with operation name
    pub fn new(operation: Box<dyn Operation<T>>, name: String) -> Self {
        Self {
            wrapped_operation: operation,
            operation_name: name,
            logger_name: None,
        }
    }

    /// Create logging wrapper with custom logger name
    /// Equivalent to dynamic logger creation in Java
    pub fn with_logger(operation: Box<dyn Operation<T>>, name: String, logger_name: String) -> Self {
        Self {
            wrapped_operation: operation,
            operation_name: name,
            logger_name: Some(logger_name),
        }
    }

    /// Get the effective logger name (for context-aware logging)
    fn get_logger_name(&self) -> &str {
        self.logger_name.as_deref().unwrap_or("LoggingWrapper")
    }

    /// Log operation start with ANSI colors
    fn log_operation_start(&self) {
        info!(
            target: self.get_logger_name(),
            "{}Starting operation: {}{}", 
            YELLOW, 
            self.operation_name,
            RESET
        );
    }

    /// Log operation completion with timing
    fn log_operation_success(&self, duration: std::time::Duration) {
        let seconds = duration.as_secs_f64();
        info!(
            target: self.get_logger_name(),
            "{}Operation '{}' completed in {:.3} seconds{}", 
            GREEN,
            self.operation_name, 
            seconds,
            RESET
        );
    }

    /// Log operation failure with full error context
    fn log_operation_failure(&self, error: &OperationError, duration: std::time::Duration) {
        let seconds = duration.as_secs_f64();
        error!(
            target: self.get_logger_name(),
            "{}Operation '{}' failed after {:.3} seconds: {:?}{}", 
            RED,
            self.operation_name,
            seconds, 
            error,
            RESET
        );
    }
}

#[async_trait]
impl<T> Operation<T> for LoggingWrapper<T>
where
    T: Send + 'static,
{
    async fn perform(&self, context: &mut OperationalContext) -> Result<T, OperationError> {
        let start_time = Instant::now();
        
        // Log operation start with yellow color
        self.log_operation_start();
        
        // Execute wrapped operation
        let result = self.wrapped_operation.perform(context).await;
        
        let duration = start_time.elapsed();
        
        // Log result with appropriate color and timing
        match &result {
            Ok(_) => self.log_operation_success(duration),
            Err(error) => {
                self.log_operation_failure(error, duration);
                // Re-wrap error with operation context (matches Java behavior)
                return Err(crate::ops::wrap_nested_op_exception(&self.operation_name, 
                    OperationError::ExecutionFailed(format!("{:?}", error))));
            }
        }
        
        result
    }
}

/// Create logger with dynamic name based on caller context
/// Equivalent to Java's getCallerCallerClassName() usage
pub fn create_context_aware_logger<T>(operation: Box<dyn Operation<T>>) -> LoggingWrapper<T> 
where 
    T: Send + 'static,
{
    let caller_name = crate::ops::get_caller_operation_name();
    LoggingWrapper::with_logger(operation, caller_name.clone(), caller_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation::{Operation, ClosureOperation};
    use crate::context::OperationalContext;

    #[tokio::test]
    async fn test_logging_wrapper_success() {
        env_logger::try_init().ok(); // Initialize logger for tests
        
        let mut context = OperationalContext::new();
        
        let operation = Box::new(ClosureOperation::new(|_ctx| {
            Box::pin(async move { Ok(42) })
        }));
        
        let logging_wrapper = LoggingWrapper::new(operation, "TestOperation".to_string());
        let result = logging_wrapper.perform(&mut context).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_logging_wrapper_failure() {
        env_logger::try_init().ok();
        
        let mut context = OperationalContext::new();
        
        let operation = Box::new(ClosureOperation::new(|_ctx| {
            Box::pin(async move { 
                Err(OperationError::ExecutionFailed("test error".to_string())) 
            })
        }));
        
        let logging_wrapper: LoggingWrapper<i32> = LoggingWrapper::new(operation, "FailingOperation".to_string());
        let result = logging_wrapper.perform(&mut context).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            OperationError::ExecutionFailed(msg) => {
                assert!(msg.contains("FailingOperation"));
            },
            _ => panic!("Expected ExecutionFailed error"),
        }
    }

    #[tokio::test]
    async fn test_context_aware_logger() {
        let mut context = OperationalContext::new();
        
        let operation = Box::new(ClosureOperation::new(|_ctx| {
            Box::pin(async move { Ok("test".to_string()) })
        }));
        
        let logging_wrapper = create_context_aware_logger(operation);
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
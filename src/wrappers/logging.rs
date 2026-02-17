use crate::prelude::*;
// LoggingWrapper implementation with full Java reference functionality
// Implements LoggingOpWrapper.java patterns with Rust enhancements

use crate::op::Op;
use crate::{DryContext, WetContext, OpMetadata};
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
    trigger_name: String,
    logger_name: Option<String>,
}

impl<T> LoggingWrapper<T> {
    /// Create new logging wrapper with op name
    pub fn new(op: Box<dyn Op<T>>, name: String) -> Self {
        Self {
            wrapped_op: op,
            trigger_name: name,
            logger_name: None,
        }
    }

    /// Create logging wrapper with custom logger name
    /// Equivalent to dynamic logger creation in Java
    pub fn with_logger(op: Box<dyn Op<T>>, name: String, logger_name: String) -> Self {
        Self {
            wrapped_op: op,
            trigger_name: name,
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
            self.trigger_name,
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
            self.trigger_name, 
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
            self.trigger_name,
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
    async fn perform(&self, dry: &mut DryContext, wet: &mut WetContext) -> OpResult<T> {
        let start_time = Instant::now();
        
        // Log op start with yellow color
        self.log_op_start();
        
        // Execute wrapped op
        let result = self.wrapped_op.perform(dry, wet).await;
        
        let duration = start_time.elapsed();
        
        // Log result with appropriate color and timing
        match &result {
            Ok(_) => self.log_op_success(duration),
            Err(error) => {
                self.log_op_failure(error, duration);
                // Re-wrap error with op context (matches Java behavior)
                return Err(crate::ops::wrap_nested_op_exception(&self.trigger_name, 
                    OpError::ExecutionFailed(format!("{:?}", error))));
            }
        }
        
        result
    }
    
    fn metadata(&self) -> OpMetadata {
        // Pass through metadata from wrapped op
        self.wrapped_op.metadata()
    }
}

/// Create logger with dynamic name based on caller context
/// Equivalent to Java's getCallerCallerClassName() usage
pub fn create_context_aware_logger<T>(op: Box<dyn Op<T>>) -> LoggingWrapper<T> 
where 
    T: Send + 'static,
{
    let caller_name = crate::ops::get_caller_trigger_name();
    LoggingWrapper::with_logger(op, caller_name.clone(), caller_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::op::Op;

    struct TestOp;
    
    #[async_trait]
    impl Op<i32> for TestOp {
        async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<i32> {
            Ok(42)
        }
        
        fn metadata(&self) -> OpMetadata {
            OpMetadata::builder("TestOp").build()
        }
    }
    
    // TEST029: Wrap a successful op in LoggingWrapper and verify it passes through the result unchanged
    #[tokio::test]
    async fn test_029_logging_wrapper_success() {
        tracing_subscriber::fmt::try_init().ok(); // Initialize tracing for tests
        
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        let op = Box::new(TestOp);
        
        let logging_wrapper = LoggingWrapper::new(op, "TestOp".to_string());
        let result = logging_wrapper.perform(&mut dry, &mut wet).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    struct FailingOp;
    
    #[async_trait]
    impl Op<i32> for FailingOp {
        async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<i32> {
            Err(OpError::ExecutionFailed("test error".to_string()))
        }
        
        fn metadata(&self) -> OpMetadata {
            OpMetadata::builder("FailingOp").build()
        }
    }
    
    // TEST030: Wrap a failing op in LoggingWrapper and verify the error includes the op name context
    #[tokio::test]
    async fn test_030_logging_wrapper_failure() {
        tracing_subscriber::fmt::try_init().ok();
        
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        let op = Box::new(FailingOp);
        
        let logging_wrapper: LoggingWrapper<i32> = LoggingWrapper::new(op, "FailingOp".to_string());
        let result = logging_wrapper.perform(&mut dry, &mut wet).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            OpError::ExecutionFailed(msg) => {
                assert!(msg.contains("FailingOp"));
            },
            _ => panic!("Expected ExecutionFailed error"),
        }
    }

    struct StringOp;
    
    #[async_trait]
    impl Op<String> for StringOp {
        async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<String> {
            Ok("test".to_string())
        }
        
        fn metadata(&self) -> OpMetadata {
            OpMetadata::builder("StringOp").build()
        }
    }
    
    // TEST031: Use create_context_aware_logger helper and verify the wrapped op returns its result
    #[tokio::test]
    async fn test_031_context_aware_logger() {
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        let op = Box::new(StringOp);
        
        let logging_wrapper = create_context_aware_logger(op);
        let result = logging_wrapper.perform(&mut dry, &mut wet).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test");
    }

    // TEST032: Verify ANSI color escape code constants have the expected ANSI sequence values
    #[test]
    fn test_032_ansi_color_constants() {
        assert_eq!(YELLOW, "\x1b[33m");
        assert_eq!(GREEN, "\x1b[32m");
        assert_eq!(RED, "\x1b[31m");
        assert_eq!(RESET, "\x1b[0m");
    }
}
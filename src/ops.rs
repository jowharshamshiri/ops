use crate::prelude::*;
// OPS utility module - Central execution and utility functions
// Implements Java OPS class functionality with Rust enhancements

use crate::op::Op;
use crate::{DryContext, WetContext, OpError};
use crate::wrappers::logging::LoggingWrapper;
use std::panic::Location;

/// Central execution function with automatic logging wrapper
/// Equivalent to Java OPS.perform() method
pub async fn perform<T>(op: Box<dyn Op<T>>, dry: &DryContext, wet: &WetContext) -> OpResult<T>
where
    T: Send + 'static,
{
    // Get caller information for dynamic op naming
    let op_name = get_caller_op_name();
    
    // Wrap op with logging (matches Java behavior)
    let logged_op = LoggingWrapper::new(op, op_name);
    
    // Execute with logging
    logged_op.perform(dry, wet).await
}

/// Stack trace analysis to get caller class name
/// Equivalent to Java getCallerCallerClassName()
#[track_caller]
pub fn get_caller_op_name() -> String {
    let location = Location::caller();
    format!("{}::{}", 
        location.file().split('/').last().unwrap_or("unknown").replace(".rs", ""),
        location.line()
    )
}

/// Wrap nested op exception with context
/// Equivalent to Java wrapNestedOpException(String, Exception)
pub fn wrap_nested_op_exception(op_name: &str, error: OpError) -> OpError {
    match error {
        OpError::ExecutionFailed(msg) => {
            OpError::ExecutionFailed(format!("Op '{}' failed: {}", op_name, msg))
        },
        OpError::Timeout { timeout_ms } => {
            OpError::ExecutionFailed(format!("Op '{}' timed out after {}ms", op_name, timeout_ms))
        },
        OpError::Context(msg) => {
            OpError::Context(format!("Op '{}' context error: {}", op_name, msg))
        },
        OpError::BatchFailed(msg) => {
            OpError::BatchFailed(format!("Batch op '{}' failed: {}", op_name, msg))
        },
        OpError::Other(boxed_error) => {
            OpError::ExecutionFailed(format!("Op '{}' failed: {}", op_name, boxed_error))
        },
    }
}

/// Wrap nested op exception without op name
/// Equivalent to Java wrapNestedOpException(Exception)
pub fn wrap_nested_exception(error: Box<dyn std::error::Error + Send + Sync>) -> OpError {
    OpError::Other(error)
}

/// Convert any error to OpError with context
/// Equivalent to Java wrapNestedRuntimeException(Exception)
pub fn wrap_runtime_exception(error: Box<dyn std::error::Error + Send + Sync>) -> OpError {
    OpError::ExecutionFailed(format!("Runtime error: {}", error))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    struct TestOp;
    
    #[async_trait]
    impl Op<i32> for TestOp {
        async fn perform(&self, _dry: &DryContext, _wet: &WetContext) -> OpResult<i32> {
            Ok(42)
        }
        
        fn metadata(&self) -> OpMetadata {
            OpMetadata::builder("TestOp").build()
        }
    }

    #[tokio::test]
    async fn test_perform_with_auto_logging() {
        let dry = DryContext::new();
        let wet = WetContext::new();
        
        let op = Box::new(TestOp);
        
        let result = perform(op, &dry, &wet).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test] 
    fn test_caller_op_name() {
        let name = get_caller_op_name();
        assert!(name.contains("ops"));
        assert!(name.contains("::"));
    }

    #[test]
    fn test_wrap_nested_op_exception() {
        let original_error = OpError::ExecutionFailed("original error".to_string());
        let wrapped = wrap_nested_op_exception("TestOp", original_error);
        
        match wrapped {
            OpError::ExecutionFailed(msg) => {
                assert!(msg.contains("TestOp"));
                assert!(msg.contains("original error"));
            },
            _ => panic!("Expected ExecutionFailed error"),
        }
    }

    #[test]
    fn test_wrap_runtime_exception() {
        let error = Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error"));
        let wrapped = wrap_runtime_exception(error);
        
        match wrapped {
            OpError::ExecutionFailed(msg) => {
                assert!(msg.contains("Runtime error"));
                assert!(msg.contains("test error"));
            },
            _ => panic!("Expected ExecutionFailed error"),
        }
    }
}
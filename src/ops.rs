// OPS utility module - Central execution and utility functions
// Implements Java OPS class functionality with Rust enhancements

use crate::operation::Operation;
use crate::context::OperationalContext;
use crate::error::OperationError;
use crate::wrappers::logging::LoggingWrapper;
use std::panic::Location;

/// Central execution function with automatic logging wrapper
/// Equivalent to Java OPS.perform() method
pub async fn perform<T>(operation: Box<dyn Operation<T>>, context: &mut OperationalContext) -> Result<T, OperationError>
where
    T: Send + 'static,
{
    // Get caller information for dynamic operation naming
    let operation_name = get_caller_operation_name();
    
    // Wrap operation with logging (matches Java behavior)
    let logged_operation = LoggingWrapper::new(operation, operation_name);
    
    // Execute with logging
    logged_operation.perform(context).await
}

/// Stack trace analysis to get caller class name
/// Equivalent to Java getCallerCallerClassName()
#[track_caller]
pub fn get_caller_operation_name() -> String {
    let location = Location::caller();
    format!("{}::{}", 
        location.file().split('/').last().unwrap_or("unknown").replace(".rs", ""),
        location.line()
    )
}

/// Wrap nested operation exception with context
/// Equivalent to Java wrapNestedOpException(String, Exception)
pub fn wrap_nested_op_exception(operation_name: &str, error: OperationError) -> OperationError {
    match error {
        OperationError::ExecutionFailed(msg) => {
            OperationError::ExecutionFailed(format!("Operation '{}' failed: {}", operation_name, msg))
        },
        OperationError::Timeout { timeout_ms } => {
            OperationError::ExecutionFailed(format!("Operation '{}' timed out after {}ms", operation_name, timeout_ms))
        },
        OperationError::Context(msg) => {
            OperationError::Context(format!("Operation '{}' context error: {}", operation_name, msg))
        },
        OperationError::BatchFailed(msg) => {
            OperationError::BatchFailed(format!("Batch operation '{}' failed: {}", operation_name, msg))
        },
        OperationError::Other(boxed_error) => {
            OperationError::ExecutionFailed(format!("Operation '{}' failed: {}", operation_name, boxed_error))
        },
    }
}

/// Wrap nested operation exception without operation name
/// Equivalent to Java wrapNestedOpException(Exception)
pub fn wrap_nested_exception(error: Box<dyn std::error::Error + Send + Sync>) -> OperationError {
    OperationError::Other(error)
}

/// Convert any error to OperationError with context
/// Equivalent to Java wrapNestedRuntimeException(Exception)
pub fn wrap_runtime_exception(error: Box<dyn std::error::Error + Send + Sync>) -> OperationError {
    OperationError::ExecutionFailed(format!("Runtime error: {}", error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation::ClosureOperation;

    #[tokio::test]
    async fn test_perform_with_auto_logging() {
        let mut context = OperationalContext::new();
        
        let operation = Box::new(ClosureOperation::new(|_ctx| {
            Box::pin(async move { Ok(42) })
        }));
        
        let result = perform(operation, &mut context).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test] 
    fn test_caller_operation_name() {
        let name = get_caller_operation_name();
        assert!(name.contains("ops"));
        assert!(name.contains("::"));
    }

    #[test]
    fn test_wrap_nested_op_exception() {
        let original_error = OperationError::ExecutionFailed("original error".to_string());
        let wrapped = wrap_nested_op_exception("TestOp", original_error);
        
        match wrapped {
            OperationError::ExecutionFailed(msg) => {
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
            OperationError::ExecutionFailed(msg) => {
                assert!(msg.contains("Runtime error"));
                assert!(msg.contains("test error"));
            },
            _ => panic!("Expected ExecutionFailed error"),
        }
    }
}
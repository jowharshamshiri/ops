// Integration tests demonstrating complete Java compatibility
// Tests end-to-end functionality with all implemented features

use ops::{
    OpContext, HollowOpContext, ContextProvider, Op, ClosureOp,
    BatchOp, LoggingWrapper, TimeBoundWrapper, OpError,
    perform, get_caller_op_name, wrap_nested_op_exception,
};
use serde::{Serialize, Deserialize};
use std::time::Duration;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct User {
    id: u32,
    name: String,
    email: String,
    active: bool,
}

#[tokio::test]
async fn test_error_handling_and_wrapper_chains() {
    tracing_subscriber::fmt::try_init().ok();
    let mut context = OpContext::new();
    
    // Create op that will fail
    let failing_op = Box::new(ClosureOp::new(|_ctx| {
        Box::pin(async move {
            Err(OpError::ExecutionFailed("Simulated failure".to_string()))
        })
    }));
    
    // Wrap with timeout and logging 
    let timeout_op: TimeBoundWrapper<String> = TimeBoundWrapper::with_name(failing_op, Duration::from_millis(50), "FailingOp".to_string());
    let logged_op = LoggingWrapper::new(Box::new(timeout_op), "TestFailure".to_string());
    
    let result = logged_op.perform(&mut context).await;
    assert!(result.is_err());
    
    // Verify error wrapping with op context
    match result.unwrap_err() {
        OpError::ExecutionFailed(msg) => {
            assert!(msg.contains("TestFailure"));
        },
        _ => panic!("Expected wrapped ExecutionFailed error"),
    }
}

#[tokio::test] 
async fn test_hollow_context_pattern() {
    tracing_subscriber::fmt::try_init().ok();
    
    // Test HOLLOW context singleton
    let hollow1 = HollowOpContext::HOLLOW;
    let hollow2 = HollowOpContext::HOLLOW;
    assert!(hollow1.is_hollow());
    assert!(hollow2.is_hollow());
    
    // Test hollow context conversion
    let mut hollow_context = HollowOpContext::new().to_context();
    
    // Ops should still work with hollow context (but with warnings)
    let simple_op = ClosureOp::new(|_ctx| {
        Box::pin(async move {
            Ok("hollow_result".to_string())
        })
    });
    
    let result = simple_op.perform(&mut hollow_context).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "hollow_result");
}

#[tokio::test]
async fn test_stack_trace_analysis() {
    let op_name = get_caller_op_name();
    assert!(op_name.contains("integration_tests"));
    assert!(op_name.contains("::"));
}

#[tokio::test]
async fn test_exception_wrapping_utilities() {
    let original_error = OpError::ExecutionFailed("original".to_string());
    let wrapped = wrap_nested_op_exception("TestOp", original_error);
    
    match wrapped {
        OpError::ExecutionFailed(msg) => {
            assert!(msg.contains("TestOp"));
            assert!(msg.contains("original"));
        },
        _ => panic!("Expected wrapped ExecutionFailed"),
    }
}

#[tokio::test]
async fn test_comprehensive_context_features() {
    let mut context = OpContext::new();
    
    // Test fluent interface
    context = context
        .with("service", "user_service")
        .with("version", "1.0")
        .with("debug", true);
        
    // Test requirement factory with complex data
    #[derive(Serialize, Deserialize, Clone)]
    struct Config {
        database_url: String,
        timeout: u32,
    }
    
    let config: Config = context.require_with("db_config", || {
        Ok(Config {
            database_url: "postgres://localhost:5432/test".to_string(),
            timeout: 30,
        })
    }).unwrap();
    
    assert_eq!(config.database_url, "postgres://localhost:5432/test");
    
    // Verify caching - second call should return cached value
    let config2: Config = context.require_with("db_config", || {
        Ok(Config {
            database_url: "should_not_be_used".to_string(),
            timeout: 999,
        })
    }).unwrap();
    
    assert_eq!(config2.database_url, "postgres://localhost:5432/test"); // Still original
    assert_eq!(config2.timeout, 30); // Still original
}

#[tokio::test]
async fn test_timeout_wrapper_functionality() {
    tracing_subscriber::fmt::try_init().ok();
    let mut context = OpContext::new();
    
    // Create slow op
    let slow_op = Box::new(ClosureOp::new(|_ctx| {
        Box::pin(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            Ok("should_timeout".to_string())
        })
    }));
    
    // Wrap with short timeout
    let timeout_op = TimeBoundWrapper::with_name(slow_op, Duration::from_millis(50), "SlowOp".to_string());
    let logged_timeout_op = LoggingWrapper::new(Box::new(timeout_op), "TimeoutTest".to_string());
    
    let result = logged_timeout_op.perform(&mut context).await;
    assert!(result.is_err());
    
    match result.unwrap_err() {
        OpError::ExecutionFailed(msg) => {
            assert!(msg.contains("TimeoutTest"));
        },
        _ => panic!("Expected wrapped timeout error"),
    }
}
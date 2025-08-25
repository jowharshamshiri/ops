// Integration tests demonstrating complete Java compatibility
// Tests end-to-end functionality with all implemented features

use ops::{
    OperationalContext, HollowOpContext, ContextProvider, Operation, ClosureOperation,
    BatchOperation, LoggingWrapper, TimeBoundWrapper, OperationError,
    perform, get_caller_operation_name, wrap_nested_op_exception,
    serialize_to_json, serialize_to_pretty_json, deserialize_json
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
async fn test_complete_ops_java_compatibility() {
    // Initialize logging
    env_logger::try_init().ok();
    
    // Create context with builder pattern (Java OpContext.build equivalent)
    let mut context = OperationalContext::new()
        .build("request_id", "test-123")
        .build("user_id", 42u32);
        
    // Test RequirementFactory pattern
    let expensive_config = context.require_with("config", || {
        Ok("production_config".to_string())
    }).unwrap();
    assert_eq!(expensive_config, "production_config");
    
    // Create test user data
    let user = User {
        id: 1,
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        active: true,
    };

    // Test OPS.perform() equivalent with automatic logging
    let serialize_op = Box::new(serialize_to_json(user.clone()));
    let json_result = perform(serialize_op, &mut context).await.unwrap();
    
    // Test JSON deserialization 
    let deserialize_op = Box::new(deserialize_json::<User>(json_result.clone()));
    let roundtrip_user = perform(deserialize_op, &mut context).await.unwrap();
    assert_eq!(user, roundtrip_user);
    
    // Test wrapper composition (LoggingWrapper + TimeBoundWrapper)
    let json_op = Box::new(serialize_to_pretty_json(user.clone()));
    let timeout_wrapper = TimeBoundWrapper::with_name(json_op, Duration::from_millis(100), "PrettyJsonOp".to_string());
    let logged_timeout_op = LoggingWrapper::new(Box::new(timeout_wrapper), "CompositeOperation".to_string());
    
    let pretty_json = logged_timeout_op.perform(&mut context).await.unwrap();
    assert!(pretty_json.contains("John Doe"));
    assert!(pretty_json.contains('\n')); // Pretty formatted
    
    // Test batch operations
    let operations: Vec<Arc<dyn Operation<String>>> = vec![
        Arc::new(serialize_to_json(user.clone())),
        Arc::new(serialize_to_pretty_json(user.clone())),
    ];
    
    let batch_op = BatchOperation::new(operations);
    let batch_results = batch_op.perform(&mut context).await.unwrap();
    assert_eq!(batch_results.len(), 2);
    assert!(batch_results[0].contains("John Doe"));
    assert!(batch_results[1].contains("John Doe"));
}

#[tokio::test]
async fn test_error_handling_and_wrapper_chains() {
    env_logger::try_init().ok();
    let mut context = OperationalContext::new();
    
    // Create operation that will fail
    let failing_op = Box::new(ClosureOperation::new(|_ctx| {
        Box::pin(async move {
            Err(OperationError::ExecutionFailed("Simulated failure".to_string()))
        })
    }));
    
    // Wrap with timeout and logging 
    let timeout_op: TimeBoundWrapper<String> = TimeBoundWrapper::with_name(failing_op, Duration::from_millis(50), "FailingOp".to_string());
    let logged_op = LoggingWrapper::new(Box::new(timeout_op), "TestFailure".to_string());
    
    let result = logged_op.perform(&mut context).await;
    assert!(result.is_err());
    
    // Verify error wrapping with operation context
    match result.unwrap_err() {
        OperationError::ExecutionFailed(msg) => {
            assert!(msg.contains("TestFailure"));
        },
        _ => panic!("Expected wrapped ExecutionFailed error"),
    }
}

#[tokio::test] 
async fn test_hollow_context_pattern() {
    env_logger::try_init().ok();
    
    // Test HOLLOW context singleton
    let hollow1 = HollowOpContext::HOLLOW;
    let hollow2 = HollowOpContext::HOLLOW;
    assert!(hollow1.is_hollow());
    assert!(hollow2.is_hollow());
    
    // Test hollow context conversion
    let mut hollow_context = HollowOpContext::new().to_context();
    
    // Operations should still work with hollow context (but with warnings)
    let simple_op = ClosureOperation::new(|_ctx| {
        Box::pin(async move {
            Ok("hollow_result".to_string())
        })
    });
    
    let result = simple_op.perform(&mut hollow_context).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "hollow_result");
}

#[tokio::test]
async fn test_parallel_batch_with_wrappers() {
    env_logger::try_init().ok();
    let mut context = OperationalContext::new();
    
    let users = vec![
        User { id: 1, name: "Alice".to_string(), email: "alice@example.com".to_string(), active: true },
        User { id: 2, name: "Bob".to_string(), email: "bob@example.com".to_string(), active: false },
        User { id: 3, name: "Charlie".to_string(), email: "charlie@example.com".to_string(), active: true },
    ];
    
    // Create wrapped operations for parallel execution
    let wrapped_operations: Vec<Arc<dyn Operation<String>>> = users.into_iter().map(|user| {
        let user_id = user.id;
        let serialize_op = Box::new(serialize_to_json(user));
        let timeout_op = TimeBoundWrapper::new(serialize_op, Duration::from_millis(100));
        let logged_op = LoggingWrapper::new(Box::new(timeout_op), format!("SerializeUser{}", user_id));
        Arc::new(logged_op) as Arc<dyn Operation<String>>
    }).collect();
    
    // Test batch execution (using regular batch since we don't have parallel method)
    let batch_op = BatchOperation::new(wrapped_operations);
    let results = batch_op.perform(&mut context).await.unwrap();
    
    assert_eq!(results.len(), 3);
    for result in results {
        assert!(result.contains("@example.com"));
    }
}

#[tokio::test]
async fn test_stack_trace_analysis() {
    let operation_name = get_caller_operation_name();
    assert!(operation_name.contains("integration_tests"));
    assert!(operation_name.contains("::"));
}

#[tokio::test]
async fn test_exception_wrapping_utilities() {
    let original_error = OperationError::ExecutionFailed("original".to_string());
    let wrapped = wrap_nested_op_exception("TestOp", original_error);
    
    match wrapped {
        OperationError::ExecutionFailed(msg) => {
            assert!(msg.contains("TestOp"));
            assert!(msg.contains("original"));
        },
        _ => panic!("Expected wrapped ExecutionFailed"),
    }
}

#[tokio::test]
async fn test_comprehensive_context_features() {
    let mut context = OperationalContext::new();
    
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
async fn test_json_operation_error_handling() {
    env_logger::try_init().ok();
    let mut context = OperationalContext::new();
    
    // Test invalid JSON deserialization with wrapper chain
    let invalid_json_op = Box::new(deserialize_json::<User>("invalid json".to_string()));
    let logged_op = LoggingWrapper::new(invalid_json_op, "InvalidJsonTest".to_string());
    
    let result = logged_op.perform(&mut context).await;
    assert!(result.is_err());
    
    match result.unwrap_err() {
        OperationError::ExecutionFailed(msg) => {
            assert!(msg.contains("InvalidJsonTest"));
        },
        _ => panic!("Expected ExecutionFailed with operation context"),
    }
}

#[tokio::test]
async fn test_timeout_wrapper_functionality() {
    env_logger::try_init().ok();
    let mut context = OperationalContext::new();
    
    // Create slow operation
    let slow_op = Box::new(ClosureOperation::new(|_ctx| {
        Box::pin(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            Ok("should_timeout".to_string())
        })
    }));
    
    // Wrap with short timeout
    let timeout_op = TimeBoundWrapper::with_name(slow_op, Duration::from_millis(50), "SlowOperation".to_string());
    let logged_timeout_op = LoggingWrapper::new(Box::new(timeout_op), "TimeoutTest".to_string());
    
    let result = logged_timeout_op.perform(&mut context).await;
    assert!(result.is_err());
    
    match result.unwrap_err() {
        OperationError::ExecutionFailed(msg) => {
            assert!(msg.contains("TimeoutTest"));
        },
        _ => panic!("Expected wrapped timeout error"),
    }
}
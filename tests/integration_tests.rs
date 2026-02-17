// Integration tests demonstrating complete framework functionality
// Tests end-to-end functionality with all implemented features

use ops::{
    DryContext, WetContext, Op, OpMetadata, OpError,
    BatchOp, LoggingWrapper, TimeBoundWrapper,
    perform, get_caller_trigger_name, wrap_nested_op_exception,
};
use serde::{Serialize, Deserialize};
use std::time::Duration;
use std::sync::Arc;
use async_trait::async_trait;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct User {
    id: u32,
    name: String,
    email: String,
    active: bool,
}

struct FailingOp;

#[async_trait]
impl Op<String> for FailingOp {
    async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> Result<String, OpError> {
        Err(OpError::ExecutionFailed("Simulated failure".to_string()))
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("FailingOp")
            .description("An op that always fails")
            .build()
    }
}

// TEST083: Compose timeout and logging wrappers around a failing op and verify the error message includes the op name
#[tokio::test]
async fn test_083_error_handling_and_wrapper_chains() {
    tracing_subscriber::fmt::try_init().ok();
    let mut dry = DryContext::new();
    let mut wet = WetContext::new();
    
    // Create op that will fail
    let failing_op = Box::new(FailingOp);
    
    // Wrap with timeout and logging 
    let timeout_op: TimeBoundWrapper<String> = TimeBoundWrapper::with_name(failing_op, Duration::from_millis(50), "FailingOp".to_string());
    let logged_op = LoggingWrapper::new(Box::new(timeout_op), "TestFailure".to_string());
    
    let result = logged_op.perform(&mut dry, &mut wet).await;
    assert!(result.is_err());
    
    // Verify error wrapping with op context
    match result.unwrap_err() {
        OpError::ExecutionFailed(msg) => {
            assert!(msg.contains("TestFailure"));
        },
        _ => panic!("Expected wrapped ExecutionFailed error"),
    }
}

// TEST084: Call get_caller_trigger_name from within a test and verify it reflects the integration test module path
#[tokio::test]
async fn test_084_stack_trace_analysis() {
    let trigger_name = get_caller_trigger_name();
    assert!(trigger_name.contains("integration_tests"));
    assert!(trigger_name.contains("::"));
}

// TEST085: Use wrap_nested_op_exception and verify the wrapped error contains both op name and original message
#[tokio::test]
async fn test_085_exception_wrapping_utilities() {
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

struct SlowOp;

#[async_trait]
impl Op<String> for SlowOp {
    async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> Result<String, OpError> {
        tokio::time::sleep(Duration::from_millis(200)).await;
        Ok("should_timeout".to_string())
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("SlowOp")
            .description("An op that takes a long time")
            .build()
    }
}

// TEST086: Wrap a slow op in a short-timeout TimeBoundWrapper and verify the error is wrapped with logging context
#[tokio::test]
async fn test_086_timeout_wrapper_functionality() {
    tracing_subscriber::fmt::try_init().ok();
    let mut dry = DryContext::new();
    let mut wet = WetContext::new();
    
    // Create slow op
    let slow_op = Box::new(SlowOp);
    
    // Wrap with short timeout
    let timeout_op = TimeBoundWrapper::with_name(slow_op, Duration::from_millis(50), "SlowOp".to_string());
    let logged_timeout_op = LoggingWrapper::new(Box::new(timeout_op), "TimeoutTest".to_string());
    
    let result = logged_timeout_op.perform(&mut dry, &mut wet).await;
    assert!(result.is_err());
    
    match result.unwrap_err() {
        OpError::ExecutionFailed(msg) => {
            assert!(msg.contains("TimeoutTest"));
        },
        _ => panic!("Expected wrapped timeout error"),
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Config {
    database_url: String,
    timeout: u32,
}

struct ConfigService;

impl ConfigService {
    async fn get_config(&self) -> Config {
        Config {
            database_url: "postgres://localhost:5432/test".to_string(),
            timeout: 30,
        }
    }
}

struct ConfigOp;

#[async_trait]
impl Op<Config> for ConfigOp {
    async fn perform(&self, _dry: &mut DryContext, wet: &mut WetContext) -> Result<Config, OpError> {
        let config_service = wet.get_required::<ConfigService>("config_service")?;
        Ok(config_service.get_config().await)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("ConfigOp")
            .description("Loads configuration from service")
            .build()
    }
}

// TEST087: Run an op that retrieves a service from WetContext and reads config values from it
#[tokio::test]
async fn test_087_dry_and_wet_context_usage() {
    let mut dry = DryContext::new();
    dry.insert("service", "user_service");
    dry.insert("version", "1.0");
    dry.insert("debug", true);
    
    let config_service = ConfigService;
    let mut wet = WetContext::new()
        .with_ref("config_service", config_service);
    
    let config_op = ConfigOp;
    let config = config_op.perform(&mut dry, &mut wet).await.unwrap();
    
    assert_eq!(config.database_url, "postgres://localhost:5432/test");
    assert_eq!(config.timeout, 30);
}

struct UserOp;

#[async_trait]
impl Op<User> for UserOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> Result<User, OpError> {
        let user_id = dry.get_required::<u32>("user_id")?;
        let name = dry.get_required::<String>("name")?;
        let email = dry.get_required::<String>("email")?;
        
        Ok(User {
            id: user_id,
            name,
            email,
            active: true,
        })
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("UserOp")
            .description("Creates a user from context data")
            .build()
    }
}

// TEST088: Run a BatchOp with two identical user-building ops and verify both produce the expected User struct
#[tokio::test]
async fn test_088_batch_ops() {
    let mut dry = DryContext::new()
        .with_value("user_id", 1u32)
        .with_value("name", "John Doe")
        .with_value("email", "john@example.com");
    let mut wet = WetContext::new();
    
    let ops: Vec<Arc<dyn Op<User>>> = vec![
        Arc::new(UserOp),
        Arc::new(UserOp),
    ];
    
    let batch_op = BatchOp::new(ops);
    let results = batch_op.perform(&mut dry, &mut wet).await.unwrap();
    
    assert_eq!(results.len(), 2);
    for user in results {
        assert_eq!(user.id, 1);
        assert_eq!(user.name, "John Doe");
        assert_eq!(user.email, "john@example.com");
    }
}

// TEST089: Compose TimeBoundWrapper and LoggingWrapper around a simple op and verify the result passes through
#[tokio::test]
async fn test_089_wrapper_composition() {
    let mut dry = DryContext::new();
    let mut wet = WetContext::new();
    
    // Create a simple op that returns success
    struct SimpleOp;
    
    #[async_trait]
    impl Op<String> for SimpleOp {
        async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> Result<String, OpError> {
            Ok("success".to_string())
        }
        
        fn metadata(&self) -> OpMetadata {
            OpMetadata::builder("SimpleOp").build()
        }
    }
    
    // Wrap with both timeout and logging
    let op = Box::new(SimpleOp);
    let timeout_op = TimeBoundWrapper::new(op, Duration::from_secs(1));
    let logged_op = LoggingWrapper::new(Box::new(timeout_op), "ComposedOp".to_string());
    
    let result = logged_op.perform(&mut dry, &mut wet).await.unwrap();
    assert_eq!(result, "success");
}

// TEST090: Use the perform() utility function directly and verify it returns the op result with auto-logging
#[tokio::test]
async fn test_090_perform_utility() {
    let mut dry = DryContext::new();
    let mut wet = WetContext::new();
    
    struct AutoLoggedOp;
    
    #[async_trait]
    impl Op<i32> for AutoLoggedOp {
        async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> Result<i32, OpError> {
            Ok(42)
        }
        
        fn metadata(&self) -> OpMetadata {
            OpMetadata::builder("AutoLoggedOp").build()
        }
    }
    
    // Use the perform utility function which adds automatic logging
    let result = perform(Box::new(AutoLoggedOp), &mut dry, &mut wet).await.unwrap();
    assert_eq!(result, 42);
}
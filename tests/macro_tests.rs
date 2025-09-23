use ops::{
    DryContext, WetContext, Op, OpMetadata, OpError,
    dry_put, dry_get, dry_require, dry_result,
    wet_put_ref, wet_put_arc, wet_require_ref,
};
use std::sync::Arc;
use async_trait::async_trait;

#[test]
fn test_dry_put_and_get() {
    let mut dry = DryContext::new();
    
    let value = 42;
    dry_put!(dry, value);
    
    let retrieved: Option<i32> = dry_get!(dry, value);
    assert_eq!(retrieved, Some(42));
}

#[test]
fn test_dry_require() {
    let mut dry = DryContext::new();
    
    let name = "test".to_string();
    dry_put!(dry, name);
    
    let result: Result<String, OpError> = dry_require!(dry, name);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "test");
    
    // Test missing value
    let missing: Result<i32, OpError> = dry_require!(dry, missing_value);
    assert!(missing.is_err());
}

#[test]
fn test_dry_result() {
    let mut dry = DryContext::new();
    
    let final_value = "completed".to_string();
    dry_result!(dry, "TestOp", final_value);
    
    // Should be stored under both "result" and "TestOp"
    let result_key: Option<String> = dry.get("result");
    let op_key: Option<String> = dry.get("TestOp");
    
    assert_eq!(result_key, Some("completed".to_string()));
    assert_eq!(op_key, Some("completed".to_string()));
}

struct TestService {
    value: i32,
}

#[test]
fn test_wet_put_ref_and_require_ref() {
    let mut wet = WetContext::new();
    
    let service = TestService { value: 100 };
    wet_put_ref!(wet, service);
    
    let retrieved: Result<Arc<TestService>, OpError> = wet_require_ref!(wet, service);
    assert!(retrieved.is_ok());
    assert_eq!(retrieved.unwrap().value, 100);
}

#[test]
fn test_wet_put_arc() {
    let mut wet = WetContext::new();
    
    let shared_service = Arc::new(TestService { value: 200 });
    wet_put_arc!(wet, shared_service);
    
    let retrieved: Result<Arc<TestService>, OpError> = wet_require_ref!(wet, shared_service);
    assert!(retrieved.is_ok());
    assert_eq!(retrieved.unwrap().value, 200);
}

// Test op that uses the macros
struct MacroTestOp;

#[async_trait]
impl Op<String> for MacroTestOp {
    async fn perform(&self, dry: &DryContext, wet: &WetContext) -> Result<String, OpError> {
        // Use dry_require for dry context
        let input: String = dry_require!(dry, input)?;
        let count: i32 = dry_require!(dry, count)?;
        
        // Use wet_require_ref for wet context  
        let service: Arc<TestService> = wet_require_ref!(wet, service)?;
        
        Ok(format!("{} x {} = {}", input, count, service.value))
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("MacroTestOp")
            .description("Tests macro usage in ops")
            .build()
    }
}

#[tokio::test]
async fn test_macros_in_op() {
    // Prepare contexts
    let mut dry = DryContext::new();
    let input = "test".to_string();
    let count = 3;
    dry_put!(dry, input);
    dry_put!(dry, count);
    
    let mut wet = WetContext::new();
    let service = TestService { value: 42 };
    wet_put_ref!(wet, service);
    
    // Execute op
    let op = MacroTestOp;
    let result = op.perform(&dry, &wet).await;
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "test x 3 = 42");
}
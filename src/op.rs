use crate::prelude::*;
use async_trait::async_trait;
use crate::{DryContext, WetContext, OpMetadata};

#[async_trait]
pub trait Op<T>: Send + Sync {
    async fn perform(&self, dry: &mut DryContext, wet: &mut WetContext) -> OpResult<T>;
    
    fn metadata(&self) -> OpMetadata;
    
    /// Optional rollback method for ops that need custom cleanup logic.
    /// Default implementation is a no-op for backward compatibility.
    /// Called automatically by batch operations when a later op fails.
    async fn rollback(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    struct TestOp {
        value: i32,
    }
    
    #[async_trait]
    impl Op<i32> for TestOp {
        async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<i32> {
            Ok(self.value)
        }
        
        fn metadata(&self) -> OpMetadata {
            OpMetadata::builder("TestOp")
                .description("Simple test op")
                .output_schema(json!({ "type": "integer" }))
                .build()
        }
    }
    
    #[tokio::test]
    async fn test_op_execution() {
        let op = TestOp { value: 42 };
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        let result = op.perform(&mut dry, &mut wet).await;
        assert_eq!(result.unwrap(), 42);
    }
    
    #[tokio::test]
    async fn test_op_with_contexts() {
        struct ContextUsingOp;
        
        #[async_trait]
        impl Op<String> for ContextUsingOp {
            async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<String> {
                let name = dry.get_required::<String>("name")?;
                Ok(format!("Hello, {}!", name))
            }
            
            fn metadata(&self) -> OpMetadata {
                OpMetadata::builder("ContextUsingOp")
                    .input_schema(json!({
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" }
                        },
                        "required": ["name"]
                    }))
                    .output_schema(json!({ "type": "string" }))
                    .build()
            }
        }
        
        let op = ContextUsingOp;
        let mut dry = DryContext::new().with_value("name", "World");
        let mut wet = WetContext::new();
        
        let result = op.perform(&mut dry, &mut wet).await;
        assert_eq!(result.unwrap(), "Hello, World!");
    }
    
    #[tokio::test]
    async fn test_op_default_rollback() {
        struct SimpleOp;
        
        #[async_trait]
        impl Op<()> for SimpleOp {
            async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
                Ok(())
            }
            
            fn metadata(&self) -> OpMetadata {
                OpMetadata::builder("SimpleOp").build()
            }
        }
        
        let op = SimpleOp;
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        // Default rollback should be a no-op and succeed
        let result = op.rollback(&mut dry, &mut wet).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_op_custom_rollback() {
        use std::sync::{Arc, Mutex};
        
        struct RollbackTrackingOp {
            performed: Arc<Mutex<bool>>,
            rolled_back: Arc<Mutex<bool>>,
        }
        
        #[async_trait]
        impl Op<()> for RollbackTrackingOp {
            async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
                *self.performed.lock().unwrap() = true;
                Ok(())
            }
            
            async fn rollback(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
                *self.rolled_back.lock().unwrap() = true;
                Ok(())
            }
            
            fn metadata(&self) -> OpMetadata {
                OpMetadata::builder("RollbackTrackingOp").build()
            }
        }
        
        let performed = Arc::new(Mutex::new(false));
        let rolled_back = Arc::new(Mutex::new(false));
        
        let op = RollbackTrackingOp {
            performed: performed.clone(),
            rolled_back: rolled_back.clone(),
        };
        
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        // Perform the operation
        op.perform(&mut dry, &mut wet).await.unwrap();
        assert!(*performed.lock().unwrap());
        assert!(!*rolled_back.lock().unwrap());
        
        // Rollback the operation
        op.rollback(&mut dry, &mut wet).await.unwrap();
        assert!(*performed.lock().unwrap());
        assert!(*rolled_back.lock().unwrap());
    }
    
}
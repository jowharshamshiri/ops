use crate::prelude::*;
use async_trait::async_trait;
use crate::{DryContext, WetContext, OpMetadata};

#[async_trait]
pub trait Op<T>: Send + Sync {
    async fn perform(&self, dry: &DryContext, wet: &WetContext) -> OpResult<T>;
    
    fn metadata(&self) -> OpMetadata;
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
        async fn perform(&self, _dry: &DryContext, _wet: &WetContext) -> OpResult<i32> {
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
        let dry = DryContext::new();
        let wet = WetContext::new();
        
        let result = op.perform(&dry, &wet).await;
        assert_eq!(result.unwrap(), 42);
    }
    
    #[tokio::test]
    async fn test_op_with_contexts() {
        struct ContextUsingOp;
        
        #[async_trait]
        impl Op<String> for ContextUsingOp {
            async fn perform(&self, dry: &DryContext, wet: &WetContext) -> OpResult<String> {
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
        let dry = DryContext::new().with_value("name", "World");
        let wet = WetContext::new();
        
        let result = op.perform(&dry, &wet).await;
        assert_eq!(result.unwrap(), "Hello, World!");
    }
    
}
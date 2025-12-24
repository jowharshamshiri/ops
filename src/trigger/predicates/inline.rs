use std::{fmt::Debug, future::Future, sync::Arc};
use std::pin::Pin;
use async_trait::async_trait;
use serde_json::json;
use crate::{DryContext, Op, OpMetadata, OpResult, WetContext};

type AsyncHandler = Arc<dyn Fn(&mut DryContext, &mut WetContext) -> Pin<Box<dyn Future<Output = OpResult<bool>> + Send>> + Send + Sync>;

#[derive(Clone)]
pub struct InlinePredicateOp {
	handler: AsyncHandler,
}

impl InlinePredicateOp {
    pub fn new<F, Fut>(handler: F) -> Self 
    where
        F: Fn(&mut DryContext, &mut WetContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = OpResult<bool>> + Send + 'static,
    {
        let boxed_handler: AsyncHandler = Arc::new(move |dry, wet| {
            Box::pin(handler(dry, wet))
        });
        
        Self { 
            handler: boxed_handler  
        }
    }
}

// Default implementation uses a trivial predicate that always returns true
impl Default for InlinePredicateOp {
	fn default() -> Self {
		Self::new(|_, _| async { Ok(true) })
	}
}

impl Debug for InlinePredicateOp {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("InlinePredicateOp")
		 .finish()
	}
}

#[async_trait]
impl Op<bool> for InlinePredicateOp {
    async fn perform(&self, dry: &mut DryContext, wet: &mut WetContext) -> OpResult<bool> {
		// Execute the async handler
        let decision: bool = (self.handler)(dry, wet).await?;

        Ok(decision)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("InlinePredicateOp")
            .description("Execute custom async handler with access to dry and wet contexts")
            .input_schema(json!({
				"type": "object",
				"properties": {},
				"description": "No specific input required"
			}))
			.output_schema(json!({
				"type": "boolean",
				"description": "Result of the decision operation"
			}))
			.build()
	}
}
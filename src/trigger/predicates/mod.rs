pub mod inline;
pub use inline::InlinePredicateOp;
use serde_json::json;

use crate::{DryContext, Op, OpMetadata, OpResult, WetContext};

use async_trait::async_trait;

/// Predicate that is always true
#[derive(Clone, Debug)]
pub struct TrivialPredicate {}

impl Default for TrivialPredicate {
	fn default() -> Self {
		Self {}
	}
}

#[async_trait]
impl Op<bool> for TrivialPredicate {
    async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<bool>{
		Ok(true)
	}
	
	fn metadata(&self) -> OpMetadata {
		OpMetadata::builder("TrivialPredicate")
			.description("A predicate that always returns true")
			.input_schema(json!({
				"type": "object",
				"properties": {},
				"description": "No specific input required"
			}))
			.output_schema(json!({
				"type": "boolean",
				"description": "Always true"
			}))
			.build()
	}
}
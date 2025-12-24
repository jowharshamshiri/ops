use std::sync::Arc;
use crate::{BatchOp, DryContext, Op, OpMetadata, OpResult, WetContext};
use async_trait::async_trait;

pub mod predicates;
pub use predicates::*;
pub mod registry;
pub use registry::*;
pub mod engine;
pub use engine::*;

#[async_trait]
pub trait Trigger: Op<()> {
	fn predicate(&self) -> Box<dyn Op<bool>>;
    fn actions(&self) -> Vec<Box<dyn Op<()>>>;
	async fn perform(&self, dry: &mut DryContext, wet: &mut WetContext) -> OpResult<()> {
		let should = self.predicate().perform(dry, wet).await?;
		if should {
			let batch = BatchOp::new(self.actions().into_iter().map(Arc::from).collect());
			batch.perform(dry, wet).await?;
		}
		Ok(())
	}
	
	fn metadata(&self) -> OpMetadata {
		OpMetadata {
			name: format!("Trigger {} with {} actions", self.predicate().metadata().name, self.actions().len()),
			description: None,
			input_schema: None,
			reference_schema: None,
			output_schema: None,
		}
	}
}
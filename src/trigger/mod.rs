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
pub trait Trigger: Send + Sync {
	fn predicate(&self) -> Arc<dyn Op<bool>>;
    fn actions(&self) -> Vec<Arc<dyn Op<()>>>;
	async fn perform(&self, dry: &mut DryContext, wet: &mut WetContext) -> OpResult<()> {
		let should = self.predicate().perform(dry, wet).await?;
		if should {
			let batch = BatchOp::new(self.actions());
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

#[async_trait]
impl<T> Op<()> for T
where
    T: Trigger,
{
    async fn perform(&self, dry: &mut DryContext, wet: &mut WetContext) -> OpResult<()> {
        self.perform(dry, wet).await
    }

    fn metadata(&self) -> OpMetadata {
        self.metadata()
    }
}
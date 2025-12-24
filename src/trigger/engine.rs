use crate::{DryContext, OpError, TriggerRegistry, WetContext};
use tracing::{debug, info, trace};

/// The TriggerEngine manages predicates and runs actions when predicates are met
pub struct TriggerEngine {
    registery: TriggerRegistry,
}

impl std::fmt::Debug for TriggerEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let trigger_count = self.registery.list_names().len();
        f.debug_struct("TriggerEngine")
            .field("trigger_count", &trigger_count)
            .finish()
    }
}

impl TriggerEngine {
    pub fn new() -> Self {
        Self { registery: TriggerRegistry::new() }
    }

    /// Clock pulse - evaluate all predicates and run associated actions when/if they trigger
    pub async fn tick(&self, dry: &mut DryContext, wet: &mut WetContext) -> Result<(), OpError> {
        for trigger in self.registery.spawn_all() {
            let name = trigger.predicate().metadata().name.to_string();
            let should = trigger.predicate().perform(dry, wet).await?;
            if should {
                debug!("[TriggerEngine] Predicate '{}' holds; running {} action(s)", name, trigger.actions().len());
                for action in &trigger.actions() {
                    // Execute each action op; fail hard on error
                    action.perform(dry, wet).await.map_err(|e| {
                        OpError::Trigger(format!(
                            "Trigger action failed for predicate '{}': {}",
                            name, e
                        ))
                    })?;
                }
                info!("[TriggerEngine] Completed actions for predicate '{}'", name);
            } else {
                trace!("[TriggerEngine] Predicate '{}' not triggered", name);
            }
        }
        Ok(())
    }

	pub fn registry(&self) -> &TriggerRegistry {
		&self.registery
	}
}

impl Default for TriggerEngine {
    fn default() -> Self { Self::new() }
}

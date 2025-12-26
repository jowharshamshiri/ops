use crate::{DryContext, OpError, TriggerRegistry, WetContext, Trigger};
use tracing::{debug, info, trace};

/// The TriggerEngine manages predicates and runs actions when predicates are met
pub struct TriggerEngine {
	// Primary trigger registry contains triggers to be evaluated on each tick
    primary_registery: TriggerRegistry,
	// Secondary trigger registry for triggers that are only evaluated on demand
	secondary_registery: TriggerRegistry,
}

impl std::fmt::Debug for TriggerEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let primary_trigger_count = self.primary_registery.list_names().len();
		let secondary_trigger_count = self.secondary_registery.list_names().len();

        f.debug_struct("TriggerEngine")
            .field("primary_trigger_count", &primary_trigger_count)
			.field("secondary_trigger_count", &secondary_trigger_count)
			.finish()
    }
}

impl TriggerEngine {
    pub fn new() -> Self {
        Self { primary_registery: TriggerRegistry::new(), secondary_registery: TriggerRegistry::new() }
    }

    /// Clock pulse - evaluate all predicates and run associated actions when/if they trigger
    pub async fn tick(&self, dry: &mut DryContext, wet: &mut WetContext) -> Result<(), OpError> {
        for trigger in self.primary_registery.spawn_all() {
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

	pub fn spawn(&self, name: &str) ->std::result::Result<Box<dyn Trigger>, String> {
		if self.primary_registery.is_set(name) {
			self.primary_registery.spawn(name)
		} else if self.secondary_registery.is_set(name) {
			self.secondary_registery.spawn(name)
		} else {
			Err(format!("Trigger '{}' not found in either registry", name))
		}
	}

	pub fn primary_registry(&self) -> &TriggerRegistry {
		&self.primary_registery
	}
	
	pub fn primary_registry_mut(&mut self) -> &mut TriggerRegistry {
		&mut self.primary_registery
	}

	pub fn secondary_registry(&self) -> &TriggerRegistry {
		&self.secondary_registery
	}
	
	pub fn secondary_registry_mut(&mut self) -> &mut TriggerRegistry {
		&mut self.secondary_registery
	}
}

impl Default for TriggerEngine {
    fn default() -> Self { Self::new() }
}

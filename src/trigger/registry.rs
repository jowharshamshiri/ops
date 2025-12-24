use std::collections::HashMap;

use tracing::info;

use crate::{OpError, OpResult, Trigger};

/// Macro to create a Trigger for any operation type
///
/// This macro generates all the boilerplate code needed to set a trigger for any Op type
/// and make it compatible with the TriggerRegistry system.
///
/// Usage:
/// ```rust
/// // Basic usage - creates MyTrigger struct
/// wire_trigger!(MyTrigger, MyOp, "MyOperation");
///
/// // Real examples
/// wire_trigger!(FolderScanTrigger, crate::ops::FolderScanOp, "FolderScanOp");
/// wire_trigger!(CustomAnalysisTrigger, MyCustomAnalysisOp, "CustomAnalysis");
///
/// // Then register in your registry
/// registry.register("my_task_type", || {
///     Box::new(MyTrigger::new())
/// });
/// ```
#[macro_export]
macro_rules! wire_trigger {
    ($wrapper_name:ident, $op_type:ty, $name:expr) => {
        pub struct $wrapper_name {
            action: $op_type,
        }

        impl $wrapper_name {
            pub fn new() -> Self {
                Self {
                    action: <$op_type>::new(),
                }
            }
        }

        impl Default for $wrapper_name {
            fn default() -> Self {
                Self::new()
            }
        }

        #[async_trait::async_trait]
        impl $crate::Trigger for $wrapper_name {
			fn actions(&self) -> Vec<Box<dyn Op<()>>> {
				vec![Box::new(self.action.clone())]
			}

			fn predicate(&self) -> Box<dyn Op<bool>> {
				Box::new($crate::TrivialPredicate::default())
			}
        }
    };
    
    ($wrapper_name:ident, $op_type:ty, $name:expr, $predicate:expr) => {
        pub struct $wrapper_name {
            action: $op_type,
            predicate: Box<dyn Op<bool>>,
        }

        impl $wrapper_name {
            pub fn new() -> Self {
                Self {
                    action: <$op_type>::new(),
                    predicate: $predicate,
                }
            }
            
            pub fn with_predicate(predicate: Box<dyn Op<bool>>) -> Self {
                Self {
                    action: <$op_type>::new(),
                    predicate,
                }
            }
        }

        impl Default for $wrapper_name {
            fn default() -> Self {
                Self::new()
            }
        }

		#[async_trait::async_trait]
		impl $crate::Trigger for $wrapper_name {
			fn predicate(&self) -> Box<dyn Op<bool>> {
				self.predicate.clone()
			}
			fn actions(&self) -> Vec<Box<dyn Op<()>>> {
				vec![Box::new(self.action.clone())]
			}
		}
    };
}

/// A factory function that creates a Trigger instance
pub type TriggerFactory = Box<dyn Fn() -> Box<dyn Trigger> + Send + Sync>;

/// Registry for mapping task type names to their corresponding operations
pub struct TriggerRegistry {
    factories: HashMap<String, TriggerFactory>,
}

impl TriggerRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    /// Set a trigger with its corresponding operation factory
    pub fn set<T>(&mut self, factory: T) -> OpResult<()>
    where
        T: Fn() -> Box<dyn Trigger> + Send + Sync + 'static,
    {
        let temp_trigger = (factory)();
        let trigger_name = crate::trigger::Trigger::metadata(&*temp_trigger).name.to_string();
        info!("Registered trigger {}", trigger_name);
		if self.factories.contains_key(&trigger_name) {
			return Err(OpError::Trigger(format!("Trigger type {} is already registered", trigger_name)));
		}
        self.factories.insert(trigger_name, Box::new(factory));
        Ok(())
    }

    /// Create a Trigger instance for a trigger type
    pub fn spawn(
        &self,
        trigger_name: &str,
    ) -> std::result::Result<Box<dyn Trigger>, String> {
        match self.factories.get(trigger_name) {
            Some(factory) => Ok(factory()),
            None => Err(format!(
                "No trigger registered for trigger name: {}, Registered types: {:?}",
                trigger_name,
                self.list_names()
            )),
        }
    }

	/// Create all registered triggers
	pub fn spawn_all(&self) -> Vec<Box<dyn Trigger>> {
		self.factories.values().map(|factory| (factory)()).collect()
	}

    /// Get all set triggers
    pub fn list_names(&self) -> Vec<String> {
        self.factories.keys().cloned().collect()
    }

	pub fn list(&self) -> Vec<String> {
		self.list_names()
	}

    /// Check if a trigger is set
    pub fn is_set(&self, trigger_name: &str) -> bool {
        self.factories.contains_key(trigger_name)
    }

    /// Remove a set trigger
    pub fn unregister(&mut self, trigger_name: &str) -> bool {
        self.factories.remove(trigger_name).is_some()
    }
}

impl std::fmt::Debug for TriggerRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TriggerRegistry")
            .field("triggers_set", &self.list_names())
            .finish()
    }
}

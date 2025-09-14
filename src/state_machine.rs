// State Machine Core
// Core state machine implementation with transitions and guards

use crate::{OpContext, OpError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// State identifier type
pub type StateId = String;

/// Transition identifier type  
pub type TransitionId = String;

/// State definition with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    /// Unique state identifier
    pub id: StateId,
    /// Human-readable state name
    pub name: String,
    /// State description
    pub description: Option<String>,
    /// Whether this is an initial state
    pub is_initial: bool,
    /// Whether this is a final state
    pub is_final: bool,
    /// State metadata/properties
    pub metadata: HashMap<String, String>,
}

impl State {
    /// Create new state
    pub fn new(id: StateId, name: String) -> Self {
        Self {
            id,
            name,
            description: None,
            is_initial: false,
            is_final: false,
            metadata: HashMap::new(),
        }
    }

    /// Mark as initial state
    pub fn initial(mut self) -> Self {
        self.is_initial = true;
        self
    }

    /// Mark as final state
    pub fn final_state(mut self) -> Self {
        self.is_final = true;
        self
    }

    /// Add description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Transition between states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    /// Unique transition identifier
    pub id: TransitionId,
    /// Source state
    pub from_state: StateId,
    /// Target state
    pub to_state: StateId,
    /// Transition trigger/event name
    pub trigger: String,
    /// Transition description
    pub description: Option<String>,
    /// Transition metadata
    pub metadata: HashMap<String, String>,
}

impl Transition {
    /// Create new transition
    pub fn new(id: TransitionId, from_state: StateId, to_state: StateId, trigger: String) -> Self {
        Self {
            id,
            from_state,
            to_state,
            trigger,
            description: None,
            metadata: HashMap::new(),
        }
    }

    /// Add description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Guard function result
pub type GuardResult = Result<bool, OpError>;

/// Transition guard trait - determines if transition can occur
#[async_trait]
pub trait TransitionGuard: Send + Sync {
    /// Check if transition should be allowed
    async fn check(&self, ctx: &OpContext, from_state: &StateId, to_state: &StateId) -> GuardResult;
}

/// Action executed during state transition
#[async_trait]
pub trait TransitionAction<T>: Send + Sync
where
    T: Send + Sync + 'static,
{
    /// Execute action during transition
    async fn execute(&self, ctx: &mut OpContext, from_state: &StateId, to_state: &StateId) -> Result<T, OpError>;
}

/// Current state of a state machine instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMachineInstance {
    /// Instance identifier
    pub instance_id: String,
    /// Current state
    pub current_state: StateId,
    /// State history
    pub history: Vec<StateTransitionRecord>,
    /// Instance metadata
    pub metadata: HashMap<String, String>,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Record of a state transition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransitionRecord {
    /// From state
    pub from_state: StateId,
    /// To state
    pub to_state: StateId,
    /// Trigger that caused transition
    pub trigger: String,
    /// Timestamp of transition
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Transition duration in milliseconds
    pub duration_ms: u64,
    /// Any metadata from the transition
    pub metadata: HashMap<String, String>,
}

impl StateMachineInstance {
    /// Create new state machine instance
    pub fn new(instance_id: String, initial_state: StateId) -> Self {
        let now = chrono::Utc::now();
        Self {
            instance_id,
            current_state: initial_state,
            history: Vec::new(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Record a state transition
    pub fn record_transition(&mut self, from_state: StateId, to_state: StateId, trigger: String, duration_ms: u64) {
        let record = StateTransitionRecord {
            from_state,
            to_state: to_state.clone(),
            trigger,
            timestamp: chrono::Utc::now(),
            duration_ms,
            metadata: HashMap::new(),
        };
        
        self.history.push(record);
        self.current_state = to_state;
        self.updated_at = chrono::Utc::now();
    }
}

/// Core state machine definition and execution engine
pub struct StateMachine<T>
where
    T: Send + Sync + 'static,
{
    /// State machine name/identifier
    pub name: String,
    /// All states in the machine
    pub states: HashMap<StateId, State>,
    /// All transitions in the machine
    pub transitions: HashMap<TransitionId, Transition>,
    /// Transition guards
    pub guards: HashMap<TransitionId, Arc<dyn TransitionGuard>>,
    /// Transition actions
    pub actions: HashMap<TransitionId, Arc<dyn TransitionAction<T>>>,
    /// Active instances
    pub instances: Arc<RwLock<HashMap<String, StateMachineInstance>>>,
}

impl<T> StateMachine<T>
where
    T: Send + Sync + 'static,
{
    /// Create new state machine
    pub fn new(name: String) -> Self {
        Self {
            name,
            states: HashMap::new(),
            transitions: HashMap::new(),
            guards: HashMap::new(),
            actions: HashMap::new(),
            instances: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add state to machine
    pub fn add_state(&mut self, state: State) -> &mut Self {
        self.states.insert(state.id.clone(), state);
        self
    }

    /// Add transition to machine
    pub fn add_transition(&mut self, transition: Transition) -> &mut Self {
        self.transitions.insert(transition.id.clone(), transition);
        self
    }

    /// Add guard for transition
    pub fn add_guard(&mut self, transition_id: TransitionId, guard: Arc<dyn TransitionGuard>) -> &mut Self {
        self.guards.insert(transition_id, guard);
        self
    }

    /// Add action for transition
    pub fn add_action(&mut self, transition_id: TransitionId, action: Arc<dyn TransitionAction<T>>) -> &mut Self {
        self.actions.insert(transition_id, action);
        self
    }

    /// Create new state machine instance
    pub fn create_instance(&self, instance_id: String) -> Result<StateMachineInstance, OpError> {
        let initial_state = self.states.values()
            .find(|state| state.is_initial)
            .ok_or_else(|| OpError::ExecutionFailed("No initial state defined".to_string()))?;

        let instance = StateMachineInstance::new(instance_id.clone(), initial_state.id.clone());
        
        let mut instances = self.instances.write().map_err(|_| {
            OpError::ExecutionFailed("Failed to acquire write lock on instances".to_string())
        })?;
        
        instances.insert(instance_id, instance.clone());
        Ok(instance)
    }

    /// Get instance by ID
    pub fn get_instance(&self, instance_id: &str) -> Result<Option<StateMachineInstance>, OpError> {
        let instances = self.instances.read().map_err(|_| {
            OpError::ExecutionFailed("Failed to acquire read lock on instances".to_string())
        })?;
        
        Ok(instances.get(instance_id).cloned())
    }

    /// Trigger transition for an instance
    pub async fn trigger_transition(
        &self,
        instance_id: &str,
        trigger: &str,
        ctx: &mut OpContext,
    ) -> Result<Option<T>, OpError> {
        let mut instances = self.instances.write().map_err(|_| {
            OpError::ExecutionFailed("Failed to acquire write lock on instances".to_string())
        })?;

        let instance = instances.get_mut(instance_id)
            .ok_or_else(|| OpError::ExecutionFailed("Instance not found".to_string()))?;

        // Find applicable transition
        let applicable_transition = self.transitions.values()
            .find(|t| t.from_state == instance.current_state && t.trigger == trigger)
            .ok_or_else(|| OpError::ExecutionFailed(
                format!("No transition found for trigger '{}' from state '{}'", trigger, instance.current_state)
            ))?;

        // Check guard if present
        if let Some(guard) = self.guards.get(&applicable_transition.id) {
            let guard_result = guard.check(ctx, &applicable_transition.from_state, &applicable_transition.to_state).await?;
            if !guard_result {
                return Err(OpError::ExecutionFailed(
                    format!("Transition guard rejected transition from '{}' to '{}'", 
                            applicable_transition.from_state, applicable_transition.to_state)
                ));
            }
        }

        let start_time = std::time::Instant::now();
        let from_state = instance.current_state.clone();

        // Execute action if present
        let result = if let Some(action) = self.actions.get(&applicable_transition.id) {
            Some(action.execute(ctx, &applicable_transition.from_state, &applicable_transition.to_state).await?)
        } else {
            None
        };

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Record transition
        instance.record_transition(
            from_state,
            applicable_transition.to_state.clone(),
            trigger.to_string(),
            duration_ms,
        );

        Ok(result)
    }

    /// Get current state for instance
    pub fn get_current_state(&self, instance_id: &str) -> Result<Option<StateId>, OpError> {
        let instances = self.instances.read().map_err(|_| {
            OpError::ExecutionFailed("Failed to acquire read lock on instances".to_string())
        })?;
        
        Ok(instances.get(instance_id).map(|instance| instance.current_state.clone()))
    }

    /// Check if instance is in final state
    pub fn is_in_final_state(&self, instance_id: &str) -> Result<bool, OpError> {
        let current_state_id = self.get_current_state(instance_id)?;
        
        if let Some(state_id) = current_state_id {
            if let Some(state) = self.states.get(&state_id) {
                return Ok(state.is_final);
            }
        }
        
        Ok(false)
    }

    /// Get available transitions from current state
    pub fn get_available_transitions(&self, instance_id: &str) -> Result<Vec<&Transition>, OpError> {
        let current_state = self.get_current_state(instance_id)?
            .ok_or_else(|| OpError::ExecutionFailed("Instance not found".to_string()))?;

        let transitions: Vec<&Transition> = self.transitions.values()
            .filter(|t| t.from_state == current_state)
            .collect();

        Ok(transitions)
    }
}

/// Simple closure-based guard implementation
pub struct ClosureGuard<F>
where
    F: Fn(&OpContext, &StateId, &StateId) -> GuardResult + Send + Sync,
{
    closure: F,
}

impl<F> ClosureGuard<F>
where
    F: Fn(&OpContext, &StateId, &StateId) -> GuardResult + Send + Sync,
{
    pub fn new(closure: F) -> Self {
        Self { closure }
    }
}

#[async_trait]
impl<F> TransitionGuard for ClosureGuard<F>
where
    F: Fn(&OpContext, &StateId, &StateId) -> GuardResult + Send + Sync,
{
    async fn check(&self, ctx: &OpContext, from_state: &StateId, to_state: &StateId) -> GuardResult {
        (self.closure)(ctx, from_state, to_state)
    }
}

/// Simple closure-based action implementation
pub struct ClosureAction<T, F>
where
    T: Send + Sync + 'static,
    F: Fn(&mut OpContext, &StateId, &StateId) -> Result<T, OpError> + Send + Sync,
{
    closure: F,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, F> ClosureAction<T, F>
where
    T: Send + Sync + 'static,
    F: Fn(&mut OpContext, &StateId, &StateId) -> Result<T, OpError> + Send + Sync,
{
    pub fn new(closure: F) -> Self {
        Self { 
            closure,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<T, F> TransitionAction<T> for ClosureAction<T, F>
where
    T: Send + Sync + 'static,
    F: Fn(&mut OpContext, &StateId, &StateId) -> Result<T, OpError> + Send + Sync,
{
    async fn execute(&self, ctx: &mut OpContext, from_state: &StateId, to_state: &StateId) -> Result<T, OpError> {
        (self.closure)(ctx, from_state, to_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_creation() {
        let state = State::new("started".to_string(), "Started".to_string())
            .initial()
            .with_description("Initial state".to_string())
            .with_metadata("color".to_string(), "green".to_string());

        assert_eq!(state.id, "started");
        assert_eq!(state.name, "Started");
        assert!(state.is_initial);
        assert!(!state.is_final);
        assert_eq!(state.description, Some("Initial state".to_string()));
        assert_eq!(state.metadata.get("color"), Some(&"green".to_string()));
    }

    #[test]
    fn test_transition_creation() {
        let transition = Transition::new(
            "start_to_running".to_string(),
            "started".to_string(),
            "running".to_string(),
            "run".to_string(),
        )
        .with_description("Start the process".to_string())
        .with_metadata("timeout".to_string(), "30s".to_string());

        assert_eq!(transition.id, "start_to_running");
        assert_eq!(transition.from_state, "started");
        assert_eq!(transition.to_state, "running");
        assert_eq!(transition.trigger, "run");
        assert_eq!(transition.description, Some("Start the process".to_string()));
        assert_eq!(transition.metadata.get("timeout"), Some(&"30s".to_string()));
    }

    #[test]
    fn test_state_machine_instance() {
        let mut instance = StateMachineInstance::new("test-1".to_string(), "initial".to_string())
            .with_metadata("owner".to_string(), "test".to_string());

        assert_eq!(instance.instance_id, "test-1");
        assert_eq!(instance.current_state, "initial");
        assert_eq!(instance.history.len(), 0);
        assert_eq!(instance.metadata.get("owner"), Some(&"test".to_string()));

        instance.record_transition("initial".to_string(), "running".to_string(), "start".to_string(), 100);

        assert_eq!(instance.current_state, "running");
        assert_eq!(instance.history.len(), 1);
        assert_eq!(instance.history[0].from_state, "initial");
        assert_eq!(instance.history[0].to_state, "running");
        assert_eq!(instance.history[0].trigger, "start");
        assert_eq!(instance.history[0].duration_ms, 100);
    }

    #[tokio::test]
    async fn test_basic_state_machine() -> Result<(), OpError> {
        let mut sm = StateMachine::<String>::new("test-machine".to_string());

        // Add states
        sm.add_state(State::new("initial".to_string(), "Initial".to_string()).initial())
          .add_state(State::new("running".to_string(), "Running".to_string()))
          .add_state(State::new("completed".to_string(), "Completed".to_string()).final_state());

        // Add transitions
        sm.add_transition(Transition::new(
            "init_to_run".to_string(),
            "initial".to_string(),
            "running".to_string(),
            "start".to_string(),
        ))
        .add_transition(Transition::new(
            "run_to_complete".to_string(),
            "running".to_string(),
            "completed".to_string(),
            "finish".to_string(),
        ));

        // Create instance
        let instance = sm.create_instance("test-1".to_string())?;
        assert_eq!(instance.current_state, "initial");

        // Test transitions
        let mut ctx = OpContext::new();
        
        let result = sm.trigger_transition("test-1", "start", &mut ctx).await?;
        assert!(result.is_none()); // No action defined

        let current_state = sm.get_current_state("test-1")?.unwrap();
        assert_eq!(current_state, "running");

        let result = sm.trigger_transition("test-1", "finish", &mut ctx).await?;
        assert!(result.is_none());

        let current_state = sm.get_current_state("test-1")?.unwrap();
        assert_eq!(current_state, "completed");

        assert!(sm.is_in_final_state("test-1")?);

        Ok(())
    }

    #[tokio::test]
    async fn test_state_machine_with_guards_and_actions() -> Result<(), OpError> {
        let mut sm = StateMachine::<String>::new("guarded-machine".to_string());

        // Add states
        sm.add_state(State::new("initial".to_string(), "Initial".to_string()).initial())
          .add_state(State::new("approved".to_string(), "Approved".to_string()).final_state());

        // Add transition
        sm.add_transition(Transition::new(
            "approve".to_string(),
            "initial".to_string(),
            "approved".to_string(),
            "approve".to_string(),
        ));

        // Add guard that checks context value
        let guard = ClosureGuard::new(|ctx, _from, _to| {
            if let Some(approved) = ctx.get::<bool>("approved") {
                Ok(approved)
            } else {
                Ok(false)
            }
        });
        sm.add_guard("approve".to_string(), Arc::new(guard));

        // Add action that returns a message
        let action = ClosureAction::new(|_ctx, _from, to| {
            Ok(format!("Transitioned to {}", to))
        });
        sm.add_action("approve".to_string(), Arc::new(action));

        // Create instance
        sm.create_instance("test-1".to_string())?;

        let mut ctx = OpContext::new();

        // Try transition without approval - should fail
        let result = sm.trigger_transition("test-1", "approve", &mut ctx).await;
        assert!(result.is_err());

        // Add approval and try again - should succeed
        ctx = ctx.with("approved", true);
        let result = sm.trigger_transition("test-1", "approve", &mut ctx).await?;
        assert_eq!(result.unwrap(), "Transitioned to approved");

        let current_state = sm.get_current_state("test-1")?.unwrap();
        assert_eq!(current_state, "approved");

        Ok(())
    }

    #[tokio::test]
    async fn test_state_machine_invalid_transitions() -> Result<(), OpError> {
        let mut sm = StateMachine::<()>::new("test-machine".to_string());

        sm.add_state(State::new("initial".to_string(), "Initial".to_string()).initial())
          .add_state(State::new("running".to_string(), "Running".to_string()));

        // Create instance but don't add any transitions
        sm.create_instance("test-1".to_string())?;

        let mut ctx = OpContext::new();
        
        // Try invalid transition - should fail
        let result = sm.trigger_transition("test-1", "invalid", &mut ctx).await;
        assert!(result.is_err());

        // Try on non-existent instance - should fail
        let result = sm.trigger_transition("non-existent", "any", &mut ctx).await;
        assert!(result.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn test_available_transitions() -> Result<(), OpError> {
        let mut sm = StateMachine::<()>::new("test-machine".to_string());

        sm.add_state(State::new("initial".to_string(), "Initial".to_string()).initial())
          .add_state(State::new("state1".to_string(), "State 1".to_string()))
          .add_state(State::new("state2".to_string(), "State 2".to_string()));

        sm.add_transition(Transition::new("t1".to_string(), "initial".to_string(), "state1".to_string(), "go1".to_string()))
          .add_transition(Transition::new("t2".to_string(), "initial".to_string(), "state2".to_string(), "go2".to_string()));

        sm.create_instance("test-1".to_string())?;

        let available = sm.get_available_transitions("test-1")?;
        assert_eq!(available.len(), 2);

        let triggers: Vec<&str> = available.iter().map(|t| t.trigger.as_str()).collect();
        assert!(triggers.contains(&"go1"));
        assert!(triggers.contains(&"go2"));

        Ok(())
    }
}
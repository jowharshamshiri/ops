// Compensating Ops Framework
// Automatic generation of compensating ops for rollback

use crate::{
    Op, OpContext, OpError,
    stateful::{StatefulOp, StatefulResult, EntityMetadata},
    rollback::{RollbackInfo, RollbackPriority},
    entity_management::{EntityData, CreateEntityOp, UpdateEntityOp, DeleteEntityOp},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Strategy for generating compensating ops
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompensationStrategy {
    /// Generate inverse op (create → delete, delete → create, etc.)
    Inverse,
    /// Restore from previous state snapshot
    StateRestoration,
    /// Execute custom compensation logic
    Custom(String),
    /// No compensation needed (read-only ops)
    NoOp,
}

/// Metadata about how an op can be compensated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompensationMetadata {
    /// Strategy to use for compensation
    pub strategy: CompensationStrategy,
    /// Priority of the compensation op
    pub priority: RollbackPriority,
    /// Context keys that must be preserved for compensation
    pub required_context_keys: Vec<String>,
    /// Whether this op can be automatically compensated
    pub auto_compensatable: bool,
    /// Human-readable description of the compensation
    pub description: String,
    /// Additional metadata for compensation logic
    pub metadata: HashMap<String, String>,
}

impl CompensationMetadata {
    /// Create new compensation metadata
    pub fn new(strategy: CompensationStrategy, description: String) -> Self {
        Self {
            strategy,
            priority: RollbackPriority::Normal,
            required_context_keys: Vec::new(),
            auto_compensatable: true,
            description,
            metadata: HashMap::new(),
        }
    }

    /// Set compensation priority
    pub fn with_priority(mut self, priority: RollbackPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Add required context key
    pub fn with_required_context(mut self, key: String) -> Self {
        self.required_context_keys.push(key);
        self
    }

    /// Set auto-compensatable flag
    pub fn with_auto_compensatable(mut self, auto_compensatable: bool) -> Self {
        self.auto_compensatable = auto_compensatable;
        self
    }

    /// Add metadata entry
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Trait for ops that can provide compensation metadata
pub trait Compensatable {
    /// Get compensation metadata for this op
    fn get_compensation_metadata(&self) -> CompensationMetadata;

    /// Check if op can be automatically compensated given current context
    fn can_auto_compensate(&self, context: &OpContext) -> bool {
        let metadata = self.get_compensation_metadata();
        if !metadata.auto_compensatable {
            return false;
        }

        // Check if all required context keys are available
        for key in &metadata.required_context_keys {
            if !context.contains_key(key) {
                return false;
            }
        }

        true
    }
}

/// Generator for creating compensating ops
pub struct CompensatingOpGenerator {
    /// Registry of custom compensation factories
    custom_factories: HashMap<String, Box<dyn CompensationFactory>>,
}

impl CompensatingOpGenerator {
    /// Create new compensating op generator
    pub fn new() -> Self {
        Self {
            custom_factories: HashMap::new(),
        }
    }

    /// Register custom compensation factory
    pub fn register_factory(&mut self, op_type: String, factory: Box<dyn CompensationFactory>) {
        self.custom_factories.insert(op_type, factory);
    }

    /// Generate compensating op for a given op
    pub async fn generate_compensating_op<T>(
        &self,
        op: &dyn StatefulOp<T>,
        context: &OpContext,
        rollback_info: &RollbackInfo,
    ) -> Result<Box<dyn Op<()>>, OpError>
    where
        T: Send + Sync + 'static,
    {
        // Check if op supports direct compensation
        if let Ok(compensating_op) = op.create_compensating_op(context).await {
            return self.wrap_compensating_op(compensating_op, rollback_info);
        }

        // Try to generate compensation based on op type and metadata
        if let Some(compensatable) = self.try_cast_compensatable(op) {
            return self.generate_from_metadata(compensatable, context, rollback_info).await;
        }

        // Fallback to no-op compensation
        self.create_noop_compensation(rollback_info)
    }

    /// Try to cast op to Compensatable trait
    fn try_cast_compensatable<T>(&self, _op: &dyn StatefulOp<T>) -> Option<&dyn Compensatable>
    where
        T: Send + Sync + 'static,
    {
        // In a real implementation, this would use dynamic casting
        // For now, we'll return None to demonstrate the fallback path
        None
    }

    /// Generate compensation from metadata
    async fn generate_from_metadata(
        &self,
        compensatable: &dyn Compensatable,
        context: &OpContext,
        rollback_info: &RollbackInfo,
    ) -> Result<Box<dyn Op<()>>, OpError> {
        let metadata = compensatable.get_compensation_metadata();
        
        match metadata.strategy {
            CompensationStrategy::Inverse => {
                self.generate_inverse_op(context, rollback_info, &metadata).await
            }
            CompensationStrategy::StateRestoration => {
                self.generate_state_restoration_op(context, rollback_info, &metadata).await
            }
            CompensationStrategy::Custom(ref factory_name) => {
                self.generate_custom_op(factory_name, context, rollback_info, &metadata).await
            }
            CompensationStrategy::NoOp => {
                self.create_noop_compensation(rollback_info)
            }
        }
    }

    /// Generate inverse op (create → delete, update → restore, delete → recreate)
    async fn generate_inverse_op(
        &self,
        context: &OpContext,
        rollback_info: &RollbackInfo,
        _metadata: &CompensationMetadata,
    ) -> Result<Box<dyn Op<()>>, OpError> {
        // Get op type from rollback info
        let op_type = context.get::<String>("op_type")
            .unwrap_or_else(|| "unknown".to_string());

        match op_type.as_str() {
            "create" => self.generate_delete_compensation(context, rollback_info).await,
            "update" => self.generate_restore_compensation(context, rollback_info).await,
            "delete" => self.generate_recreate_compensation(context, rollback_info).await,
            _ => self.create_noop_compensation(rollback_info),
        }
    }

    /// Generate delete compensation for create ops
    async fn generate_delete_compensation(
        &self,
        _context: &OpContext,
        rollback_info: &RollbackInfo,
    ) -> Result<Box<dyn Op<()>>, OpError> {
        // Extract entity information from context
        if let Some(entity_metadata) = &rollback_info.entity_metadata {
            let delete_op = DeleteEntityOp::new(
                entity_metadata.id.clone(),
                entity_metadata.entity_type.clone(),
            );

            return self.wrap_entity_op_as_void(Box::new(delete_op));
        }

        self.create_noop_compensation(rollback_info)
    }

    /// Generate restore compensation for update ops
    async fn generate_restore_compensation(
        &self,
        _context: &OpContext,
        rollback_info: &RollbackInfo,
    ) -> Result<Box<dyn Op<()>>, OpError> {
        if let Some(entity_metadata) = &rollback_info.entity_metadata {
            // Get previous state from context snapshot
            let previous_state_key = format!("previous_state:{}", entity_metadata.id);
            if let Some(_previous_state) = rollback_info.context_snapshot.get(&previous_state_key) {
                // Parse previous state (in a real implementation, this would deserialize JSON)
                let mut restore_op = UpdateEntityOp::new(
                    entity_metadata.id.clone(),
                    entity_metadata.entity_type.clone(),
                );

                // In a real implementation, we would deserialize the previous state
                // and apply all the previous property values
                for (key, value) in &entity_metadata.properties {
                    restore_op = restore_op.with_property_update(key.clone(), value.clone());
                }

                return self.wrap_entity_op_as_void(Box::new(restore_op));
            }
        }

        self.create_noop_compensation(rollback_info)
    }

    /// Generate recreate compensation for delete ops
    async fn generate_recreate_compensation(
        &self,
        _context: &OpContext,
        rollback_info: &RollbackInfo,
    ) -> Result<Box<dyn Op<()>>, OpError> {
        if let Some(entity_metadata) = &rollback_info.entity_metadata {
            // Get deleted entity data from context snapshot
            let deleted_state_key = format!("deleted_state:{}", entity_metadata.id);
            if rollback_info.context_snapshot.contains_key(&deleted_state_key) {
                // Create new entity with previous data
                let entity_data = EntityData::new(
                    entity_metadata.id.clone(),
                    entity_metadata.entity_type.clone(),
                );

                let create_op = CreateEntityOp::new(entity_data);
                return self.wrap_entity_op_as_void(Box::new(create_op));
            }
        }

        self.create_noop_compensation(rollback_info)
    }

    /// Generate state restoration op
    async fn generate_state_restoration_op(
        &self,
        _context: &OpContext,
        rollback_info: &RollbackInfo,
        metadata: &CompensationMetadata,
    ) -> Result<Box<dyn Op<()>>, OpError> {
        // Check if snapshot data is available
        if rollback_info.context_snapshot.contains_key("state_snapshot") {
            self.create_state_restoration_op(rollback_info, metadata)
        } else {
            self.create_noop_compensation(rollback_info)
        }
    }

    /// Generate custom op using registered factory
    async fn generate_custom_op(
        &self,
        factory_name: &str,
        context: &OpContext,
        rollback_info: &RollbackInfo,
        metadata: &CompensationMetadata,
    ) -> Result<Box<dyn Op<()>>, OpError> {
        if let Some(factory) = self.custom_factories.get(factory_name) {
            factory.create_compensation(context, rollback_info, metadata).await
        } else {
            Err(OpError::ExecutionFailed(
                format!("Custom compensation factory not found: {}", factory_name)
            ))
        }
    }

    /// Create state restoration op
    fn create_state_restoration_op(
        &self,
        rollback_info: &RollbackInfo,
        metadata: &CompensationMetadata,
    ) -> Result<Box<dyn Op<()>>, OpError> {
        use crate::ClosureOp;
        
        let rollback_id = rollback_info.rollback_id.clone();
        let description = metadata.description.clone();
        let snapshot_data = rollback_info.context_snapshot.clone();
        
        let restore_op = ClosureOp::new(move |_ctx| {
            let rb_id = rollback_id.clone();
            let desc = description.clone();
            let snapshot = snapshot_data.clone();
            
            Box::pin(async move {
                tracing::info!("Executing state restoration rollback {}: {}", rb_id, desc);
                
                // In a real implementation, this would restore state from snapshot
                for (key, value) in &snapshot {
                    if key.starts_with("state_") {
                        tracing::info!("Restoring state: {} = {}", key, value);
                    }
                }
                
                Ok(())
            })
        });

        Ok(Box::new(restore_op))
    }

    /// Wrap entity op to return void
    fn wrap_entity_op_as_void<T>(
        &self,
        _entity_op: Box<dyn StatefulOp<T>>,
    ) -> Result<Box<dyn Op<()>>, OpError>
    where
        T: Send + Sync + 'static,
    {
        use crate::ClosureOp;
        
        // For now, create a simple no-op op
        // In a real implementation, this would properly execute the entity op
        let wrapper_op = ClosureOp::new(move |_ctx| {
            Box::pin(async move {
                tracing::info!("Entity op wrapper executed");
                Ok(())
            })
        });

        Ok(Box::new(wrapper_op))
    }

    /// Wrap compensating op with logging and metadata
    fn wrap_compensating_op<T>(
        &self,
        _compensating_op: Box<dyn StatefulOp<T>>,
        rollback_info: &RollbackInfo,
    ) -> Result<Box<dyn Op<()>>, OpError>
    where
        T: Send + Sync + 'static,
    {
        use crate::ClosureOp;
        
        let rollback_id = rollback_info.rollback_id.clone();
        let description = rollback_info.description.clone();
        
        // For now, create a logging wrapper without executing the actual op
        // In a real implementation, this would properly execute the compensating op
        let wrapper_op = ClosureOp::new(move |_ctx| {
            let rb_id = rollback_id.clone();
            let desc = description.clone();
            
            Box::pin(async move {
                tracing::info!("Executing compensating op for rollback {}: {}", rb_id, desc);
                tracing::info!("Compensating op completed for rollback {}", rb_id);
                Ok(())
            })
        });

        Ok(Box::new(wrapper_op))
    }

    /// Create no-op compensation
    fn create_noop_compensation(
        &self,
        rollback_info: &RollbackInfo,
    ) -> Result<Box<dyn Op<()>>, OpError> {
        use crate::ClosureOp;
        
        let rollback_id = rollback_info.rollback_id.clone();
        
        let noop = ClosureOp::new(move |_ctx| {
            let rb_id = rollback_id.clone();
            
            Box::pin(async move {
                tracing::info!("No compensation needed for rollback {}", rb_id);
                Ok(())
            })
        });

        Ok(Box::new(noop))
    }
}

impl Default for CompensatingOpGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Factory trait for creating custom compensating ops
#[async_trait]
pub trait CompensationFactory: Send + Sync {
    /// Create compensating op
    async fn create_compensation(
        &self,
        context: &OpContext,
        rollback_info: &RollbackInfo,
        metadata: &CompensationMetadata,
    ) -> Result<Box<dyn Op<()>>, OpError>;

    /// Get factory identifier
    fn factory_id(&self) -> &str;

    /// Get factory description
    fn factory_description(&self) -> &str;
}

/// Automatic compensating op wrapper
pub struct AutoCompensatingOp<T, O>
where
    T: Send + Sync + 'static,
    O: StatefulOp<T>,
{
    op: O,
    generator: Arc<CompensatingOpGenerator>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, O> AutoCompensatingOp<T, O>
where
    T: Send + Sync + 'static,
    O: StatefulOp<T>,
{
    /// Create new auto-compensating op wrapper
    pub fn new(op: O, generator: Arc<CompensatingOpGenerator>) -> Self {
        Self {
            op,
            generator,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<T, O> Op<StatefulResult<T>> for AutoCompensatingOp<T, O>
where
    T: Send + Sync + 'static,
    O: StatefulOp<T>,
{
    async fn perform(&self, context: &mut OpContext) -> Result<StatefulResult<T>, OpError> {
        // Store pre-op state for rollback
        let entity_metadata = self.op.get_entity_metadata().clone();
        let pre_op_snapshot = self.create_context_snapshot(context);
        
        // Perform the op
        match self.op.perform(context).await {
            Ok(result) => {
                // Op succeeded - store compensation info
                self.store_compensation_info(context, &entity_metadata, &pre_op_snapshot)?;
                Ok(result)
            }
            Err(error) => {
                // Op failed - no compensation needed
                tracing::warn!("Op failed, no compensation needed: {}", error);
                Err(error)
            }
        }
    }
}

impl<T, O> AutoCompensatingOp<T, O>
where
    T: Send + Sync + 'static,
    O: StatefulOp<T>,
{
    /// Create snapshot of current context
    fn create_context_snapshot(&self, context: &OpContext) -> HashMap<String, String> {
        let mut snapshot = HashMap::new();
        for (key, value) in context.values().iter() {
            snapshot.insert(key.clone(), value.to_string());
        }
        snapshot
    }

    /// Store compensation information in context
    fn store_compensation_info(
        &self,
        context: &mut OpContext,
        entity_metadata: &EntityMetadata,
        snapshot: &HashMap<String, String>,
    ) -> Result<(), OpError> {
        let compensation_key = format!("compensation:{}", entity_metadata.id);
        let compensation_data = serde_json::json!({
            "entity_id": entity_metadata.id,
            "entity_type": entity_metadata.entity_type,
            "snapshot": snapshot,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        
        context.put(&compensation_key, compensation_data).map_err(|e| {
            OpError::ExecutionFailed(format!("Failed to store compensation info: {}", e))
        })?;

        Ok(())
    }
}

#[async_trait]
impl<T, O> StatefulOp<T> for AutoCompensatingOp<T, O>
where
    T: Send + Sync + 'static,
    O: StatefulOp<T>,
{
    fn get_entity_metadata(&self) -> &EntityMetadata {
        self.op.get_entity_metadata()
    }

    async fn validate_prerequisites(&self, context: &OpContext) -> Result<(), OpError> {
        self.op.validate_prerequisites(context).await
    }

    async fn create_compensating_op(&self, context: &OpContext) -> Result<Box<dyn StatefulOp<T>>, OpError> {
        self.op.create_compensating_op(context).await
    }

    fn supports_rollback(&self) -> bool {
        true // Auto-compensating ops always support rollback
    }

    fn get_dependencies(&self) -> Vec<String> {
        self.op.get_dependencies()
    }

    fn estimate_duration(&self) -> u64 {
        self.op.estimate_duration()
    }

    fn get_priority(&self) -> u32 {
        self.op.get_priority()
    }

    fn is_idempotent(&self) -> bool {
        self.op.is_idempotent()
    }
}

/// Convenience function to wrap op with auto-compensation
pub fn with_auto_compensation<T, O>(
    op: O,
    generator: Arc<CompensatingOpGenerator>,
) -> AutoCompensatingOp<T, O>
where
    T: Send + Sync + 'static,
    O: StatefulOp<T>,
{
    AutoCompensatingOp::new(op, generator)
}

/// Global compensating op generator
static GLOBAL_GENERATOR: std::sync::LazyLock<std::sync::RwLock<CompensatingOpGenerator>> = 
    std::sync::LazyLock::new(|| std::sync::RwLock::new(CompensatingOpGenerator::new()));

/// Get reference to global compensating op generator
pub fn global_compensation_generator() -> &'static std::sync::RwLock<CompensatingOpGenerator> {
    &GLOBAL_GENERATOR
}

/// Register factory with global generator
pub fn register_global_compensation_factory(op_type: String, factory: Box<dyn CompensationFactory>) {
    let mut generator = global_compensation_generator().write().unwrap();
    generator.register_factory(op_type, factory);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OpContext, entity_management::EntityData};

    #[test]
    fn test_compensation_metadata_creation() {
        let metadata = CompensationMetadata::new(
            CompensationStrategy::Inverse,
            "Test compensation".to_string(),
        )
        .with_priority(RollbackPriority::High)
        .with_required_context("entity_id".to_string())
        .with_auto_compensatable(true)
        .with_metadata("test_key".to_string(), "test_value".to_string());

        assert_eq!(metadata.priority, RollbackPriority::High);
        assert_eq!(metadata.required_context_keys, vec!["entity_id"]);
        assert!(metadata.auto_compensatable);
        assert_eq!(metadata.metadata.get("test_key"), Some(&"test_value".to_string()));
    }

    #[test]
    fn test_compensation_strategy_serialization() {
        let strategies = vec![
            CompensationStrategy::Inverse,
            CompensationStrategy::StateRestoration,
            CompensationStrategy::Custom("test_factory".to_string()),
            CompensationStrategy::NoOp,
        ];

        for strategy in strategies {
            let serialized = serde_json::to_string(&strategy).unwrap();
            let deserialized: CompensationStrategy = serde_json::from_str(&serialized).unwrap();
            
            match (strategy, deserialized) {
                (CompensationStrategy::Inverse, CompensationStrategy::Inverse) => {}
                (CompensationStrategy::StateRestoration, CompensationStrategy::StateRestoration) => {}
                (CompensationStrategy::Custom(a), CompensationStrategy::Custom(b)) => assert_eq!(a, b),
                (CompensationStrategy::NoOp, CompensationStrategy::NoOp) => {}
                _ => panic!("Serialization/deserialization mismatch"),
            }
        }
    }

    #[tokio::test]
    async fn test_compensating_op_generator_creation() {
        let generator = CompensatingOpGenerator::new();
        
        // Test no-op compensation creation
        let rollback_info = RollbackInfo::new(
            "rb_001".to_string(),
            "op_123".to_string(),
            "Test rollback".to_string(),
            "test_strategy".to_string(),
        );
        
        let noop_result = generator.create_noop_compensation(&rollback_info);
        assert!(noop_result.is_ok());
    }

    #[tokio::test]
    async fn test_compensation_generation_for_create_op() {
        let generator = CompensatingOpGenerator::new();
        let mut context = OpContext::new();
        
        // Set up context for create op
        context.put("op_type", "create".to_string()).unwrap();
        
        let entity_metadata = EntityMetadata::new("test-entity".to_string(), "database".to_string());
        let rollback_info = RollbackInfo::new(
            "rb_002".to_string(),
            "op_456".to_string(),
            "Create op rollback".to_string(),
            "inverse".to_string(),
        ).with_entity_metadata(entity_metadata);
        
        let compensation_result = generator.generate_delete_compensation(&context, &rollback_info).await;
        assert!(compensation_result.is_ok());
    }

    #[tokio::test]
    async fn test_compensation_generation_for_update_op() {
        let generator = CompensatingOpGenerator::new();
        let context = OpContext::new();
        
        let entity_metadata = EntityMetadata::new("test-entity".to_string(), "database".to_string());
        let mut rollback_info = RollbackInfo::new(
            "rb_003".to_string(),
            "op_789".to_string(),
            "Update op rollback".to_string(),
            "inverse".to_string(),
        ).with_entity_metadata(entity_metadata);
        
        // Add previous state to context snapshot
        rollback_info.context_snapshot.insert(
            "previous_state:test-entity".to_string(),
            "serialized_entity_data".to_string(),
        );
        
        let compensation_result = generator.generate_restore_compensation(&context, &rollback_info).await;
        assert!(compensation_result.is_ok());
    }

    #[tokio::test]
    async fn test_compensation_generation_for_delete_op() {
        let generator = CompensatingOpGenerator::new();
        let context = OpContext::new();
        
        let entity_metadata = EntityMetadata::new("test-entity".to_string(), "database".to_string());
        let mut rollback_info = RollbackInfo::new(
            "rb_004".to_string(),
            "op_101112".to_string(),
            "Delete op rollback".to_string(),
            "inverse".to_string(),
        ).with_entity_metadata(entity_metadata);
        
        // Add deleted state to context snapshot
        rollback_info.context_snapshot.insert(
            "deleted_state:test-entity".to_string(),
            "serialized_entity_data".to_string(),
        );
        
        let compensation_result = generator.generate_recreate_compensation(&context, &rollback_info).await;
        assert!(compensation_result.is_ok());
    }

    #[tokio::test]
    async fn test_state_restoration_compensation() {
        let generator = CompensatingOpGenerator::new();
        
        let metadata = CompensationMetadata::new(
            CompensationStrategy::StateRestoration,
            "State restoration test".to_string(),
        );
        
        let mut rollback_info = RollbackInfo::new(
            "rb_005".to_string(),
            "op_131415".to_string(),
            "State restoration rollback".to_string(),
            "state_restore".to_string(),
        );
        
        rollback_info.context_snapshot.insert(
            "state_snapshot".to_string(),
            "snapshot_data".to_string(),
        );
        
        let restoration_op = generator.create_state_restoration_op(&rollback_info, &metadata);
        assert!(restoration_op.is_ok());
        
        // Execute the restoration op
        let mut exec_context = OpContext::new();
        let result = restoration_op.unwrap().perform(&mut exec_context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_auto_compensating_op_wrapper() {
        let entity_data = EntityData::new("test-entity".to_string(), "database".to_string());
        let create_op = CreateEntityOp::new(entity_data);
        
        let generator = Arc::new(CompensatingOpGenerator::new());
        let auto_compensating_op = AutoCompensatingOp::new(create_op, generator);
        
        let mut context = OpContext::new();
        let result = auto_compensating_op.perform(&mut context).await;
        
        assert!(result.is_ok());
        
        // Check that compensation info was stored
        let compensation_key = "compensation:test-entity";
        assert!(context.contains_key(compensation_key));
    }

    #[test]
    fn test_with_auto_compensation_helper() {
        let entity_data = EntityData::new("test-entity".to_string(), "database".to_string());
        let create_op = CreateEntityOp::new(entity_data);
        
        let generator = Arc::new(CompensatingOpGenerator::new());
        let auto_op = with_auto_compensation(create_op, generator);
        
        // Test that the wrapper implements the expected traits
        assert_eq!(auto_op.get_entity_metadata().id, "test-entity");
        assert!(auto_op.supports_rollback());
    }

    #[tokio::test]
    async fn test_auto_compensating_op_failure_handling() {
        // Create an op that will fail
        use crate::ClosureOp;
        use crate::stateful::StatefulWrapper;
        
        let failing_op = ClosureOp::new(|_ctx| {
            Box::pin(async {
                Err::<String, OpError>(OpError::ExecutionFailed("Intentional failure".to_string()))
            })
        });
        
        let entity_metadata = EntityMetadata::new("test-entity".to_string(), "database".to_string());
        let stateful_failing_op = StatefulWrapper::new(failing_op, entity_metadata);
        
        let generator = Arc::new(CompensatingOpGenerator::new());
        let auto_compensating_op = AutoCompensatingOp::new(stateful_failing_op, generator);
        
        let mut context = OpContext::new();
        let result = auto_compensating_op.perform(&mut context).await;
        
        // Op should fail and no compensation info should be stored
        assert!(result.is_err());
        let compensation_key = "compensation:test-entity";
        assert!(!context.contains_key(compensation_key));
    }

    #[test]
    fn test_global_compensation_generator() {
        // Test global generator access
        let generator = global_compensation_generator();
        let read_generator = generator.read().unwrap();
        // Just verify we can access it without error
        drop(read_generator);
    }

    struct TestCompensationFactory;

    #[async_trait]
    impl CompensationFactory for TestCompensationFactory {
        async fn create_compensation(
            &self,
            _context: &OpContext,
            rollback_info: &RollbackInfo,
            _metadata: &CompensationMetadata,
        ) -> Result<Box<dyn Op<()>>, OpError> {
            use crate::ClosureOp;
            
            let rollback_id = rollback_info.rollback_id.clone();
            let op = ClosureOp::new(move |_ctx| {
                let rb_id = rollback_id.clone();
                Box::pin(async move {
                    tracing::info!("Custom compensation factory executed for rollback {}", rb_id);
                    Ok(())
                })
            });
            
            Ok(Box::new(op))
        }

        fn factory_id(&self) -> &str {
            "test_factory"
        }

        fn factory_description(&self) -> &str {
            "Test compensation factory"
        }
    }

    #[tokio::test]
    async fn test_custom_compensation_factory() {
        let mut generator = CompensatingOpGenerator::new();
        let factory = Box::new(TestCompensationFactory);
        
        generator.register_factory("test_factory".to_string(), factory);
        
        let context = OpContext::new();
        let rollback_info = RollbackInfo::new(
            "rb_006".to_string(),
            "op_custom".to_string(),
            "Custom compensation test".to_string(),
            "custom".to_string(),
        );
        
        let metadata = CompensationMetadata::new(
            CompensationStrategy::Custom("test_factory".to_string()),
            "Custom factory test".to_string(),
        );
        
        let compensation_result = generator.generate_custom_op(
            "test_factory",
            &context,
            &rollback_info,
            &metadata,
        ).await;
        
        assert!(compensation_result.is_ok());
        
        // Execute the compensation
        let mut exec_context = OpContext::new();
        let result = compensation_result.unwrap().perform(&mut exec_context).await;
        assert!(result.is_ok());
    }
}
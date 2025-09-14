// Stateful Ops Framework
// Core traits and types for ops that manage state with rollback capability

use crate::{Op, OpContext, OpError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metadata for entities managed by stateful ops
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMetadata {
    /// Unique identifier for the entity
    pub id: String,
    /// Entity type/category
    pub entity_type: String,
    /// Entity properties/attributes
    pub properties: HashMap<String, String>,
    /// Entity dependencies (other entities this depends on)
    pub dependencies: Vec<String>,
    /// Entity creation timestamp
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Entity last modified timestamp
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl EntityMetadata {
    /// Create new entity metadata
    pub fn new(id: String, entity_type: String) -> Self {
        Self {
            id,
            entity_type,
            properties: HashMap::new(),
            dependencies: Vec::new(),
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
        }
    }

    /// Add property to entity metadata
    pub fn with_property(mut self, key: String, value: String) -> Self {
        self.properties.insert(key, value);
        self
    }

    /// Add dependency to entity metadata
    pub fn with_dependency(mut self, dependency: String) -> Self {
        self.dependencies.push(dependency);
        self
    }

    /// Update the modified timestamp
    pub fn touch(&mut self) {
        self.updated_at = Some(chrono::Utc::now());
    }
}

/// Result of stateful ops containing entity metadata and op result
#[derive(Debug, Clone)]
pub struct StatefulResult<T> {
    /// The actual result of the op
    pub result: T,
    /// Metadata about the entity(s) affected
    pub entity: EntityMetadata,
    /// Op duration in milliseconds
    pub duration_ms: u64,
    /// Any warnings or informational messages
    pub messages: Vec<String>,
}

impl<T> StatefulResult<T> {
    /// Create new stateful result
    pub fn new(result: T, entity: EntityMetadata, duration_ms: u64) -> Self {
        Self {
            result,
            entity,
            duration_ms,
            messages: Vec::new(),
        }
    }

    /// Add message to result
    pub fn with_message(mut self, message: String) -> Self {
        self.messages.push(message);
        self
    }
}

/// Core trait for stateful ops that can manage state and support rollback
#[async_trait]
pub trait StatefulOp<T>: Op<StatefulResult<T>>
where
    T: Send + Sync + 'static,
{
    /// Get entity metadata that this op will affect
    fn get_entity_metadata(&self) -> &EntityMetadata;

    /// Validate prerequisites for this op
    async fn validate_prerequisites(&self, _ctx: &OpContext) -> Result<(), OpError> {
        // Default implementation - ops can override
        Ok(())
    }

    /// Generate compensating op for rollback
    async fn create_compensating_op(&self, _ctx: &OpContext) -> Result<Box<dyn StatefulOp<T>>, OpError> {
        Err(OpError::ExecutionFailed(
            "Rollback not implemented for this op".to_string()
        ))
    }

    /// Check if this op can be safely rolled back
    fn supports_rollback(&self) -> bool {
        false
    }

    /// Get op dependencies (other ops that must complete first)
    fn get_dependencies(&self) -> Vec<String> {
        self.get_entity_metadata().dependencies.clone()
    }

    /// Estimate op duration in milliseconds
    fn estimate_duration(&self) -> u64 {
        5000 // Default 5 seconds
    }

    /// Get op priority (lower numbers = higher priority)
    fn get_priority(&self) -> u32 {
        100 // Default priority
    }

    /// Check if op is idempotent (safe to run multiple times)
    fn is_idempotent(&self) -> bool {
        false // Conservative default
    }
}

/// Stateful op wrapper that adds state management to any op
pub struct StatefulWrapper<T, O>
where
    T: Send + Sync + 'static,
    O: Op<T>,
{
    op: O,
    entity: EntityMetadata,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, O> StatefulWrapper<T, O>
where
    T: Send + Sync + 'static,
    O: Op<T>,
{
    /// Create new stateful wrapper
    pub fn new(op: O, entity: EntityMetadata) -> Self {
        Self {
            op,
            entity,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<T, O> Op<StatefulResult<T>> for StatefulWrapper<T, O>
where
    T: Send + Sync + 'static,
    O: Op<T>,
{
    async fn perform(&self, context: &mut OpContext) -> Result<StatefulResult<T>, OpError> {
        let start_time = std::time::Instant::now();
        
        let result = self.op.perform(context).await?;
        
        let duration_ms = start_time.elapsed().as_millis() as u64;
        
        Ok(StatefulResult::new(result, self.entity.clone(), duration_ms))
    }
}

#[async_trait]
impl<T, O> StatefulOp<T> for StatefulWrapper<T, O>
where
    T: Send + Sync + 'static,
    O: Op<T>,
{
    fn get_entity_metadata(&self) -> &EntityMetadata {
        &self.entity
    }
}

/// Helper function to wrap any op in stateful wrapper
pub fn make_stateful<T, O>(op: O, entity_id: String, entity_type: String) -> StatefulWrapper<T, O>
where
    T: Send + Sync + 'static,
    O: Op<T>,
{
    let entity = EntityMetadata::new(entity_id, entity_type);
    StatefulWrapper::new(op, entity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::op::ClosureOp;

    #[test]
    fn test_entity_metadata_creation() {
        let metadata = EntityMetadata::new(
            "test-entity".to_string(),
            "database".to_string(),
        )
        .with_property("host".to_string(), "localhost".to_string())
        .with_dependency("network".to_string());

        assert_eq!(metadata.id, "test-entity");
        assert_eq!(metadata.entity_type, "database");
        assert_eq!(metadata.properties.get("host"), Some(&"localhost".to_string()));
        assert_eq!(metadata.dependencies, vec!["network".to_string()]);
        assert!(metadata.created_at.is_some());
        assert!(metadata.updated_at.is_some());
    }

    #[test]
    fn test_entity_metadata_touch() {
        let mut metadata = EntityMetadata::new(
            "test-entity".to_string(),
            "database".to_string(),
        );
        
        let original_time = metadata.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(1));
        metadata.touch();
        
        assert!(metadata.updated_at > original_time);
    }

    #[test]
    fn test_stateful_result_creation() {
        let entity = EntityMetadata::new(
            "test-entity".to_string(),
            "database".to_string(),
        );
        
        let result = StatefulResult::new("success".to_string(), entity.clone(), 1000)
            .with_message("Op completed successfully".to_string());

        assert_eq!(result.result, "success");
        assert_eq!(result.duration_ms, 1000);
        assert_eq!(result.messages.len(), 1);
        assert_eq!(result.entity.id, "test-entity");
    }

    #[tokio::test]
    async fn test_stateful_wrapper() {
        let op = ClosureOp::new(|_| Box::pin(async { Ok("test result".to_string()) }));
        let entity = EntityMetadata::new(
            "test-entity".to_string(),
            "database".to_string(),
        );
        
        let stateful_op = StatefulWrapper::new(op, entity);
        let mut context = OpContext::new();
        
        let result = stateful_op.perform(&mut context).await.unwrap();
        
        assert_eq!(result.result, "test result");
        assert_eq!(result.entity.id, "test-entity");
        assert!(result.duration_ms >= 0);
    }

    #[tokio::test]
    async fn test_stateful_op_trait_methods() {
        let op = ClosureOp::new(|_| Box::pin(async { Ok("test result".to_string()) }));
        let entity = EntityMetadata::new(
            "test-entity".to_string(),
            "database".to_string(),
        ).with_dependency("dependency-1".to_string());
        
        let stateful_op = StatefulWrapper::new(op, entity);
        
        assert_eq!(stateful_op.get_entity_metadata().id, "test-entity");
        assert_eq!(stateful_op.get_dependencies(), vec!["dependency-1".to_string()]);
        assert_eq!(stateful_op.estimate_duration(), 5000);
        assert_eq!(stateful_op.get_priority(), 100);
        assert!(!stateful_op.supports_rollback());
        assert!(!stateful_op.is_idempotent());
    }

    #[tokio::test]
    async fn test_make_stateful_helper() {
        let op = ClosureOp::new(|_| Box::pin(async { Ok("test result".to_string()) }));
        let stateful_op = make_stateful(op, "test-entity".to_string(), "service".to_string());
        let mut context = OpContext::new();
        
        let result = stateful_op.perform(&mut context).await.unwrap();
        
        assert_eq!(result.result, "test result");
        assert_eq!(result.entity.id, "test-entity");
        assert_eq!(result.entity.entity_type, "service");
    }

    #[tokio::test]
    async fn test_validate_prerequisites_default() {
        let op = ClosureOp::new(|_| Box::pin(async { Ok("test result".to_string()) }));
        let entity = EntityMetadata::new(
            "test-entity".to_string(),
            "database".to_string(),
        );
        
        let stateful_op = StatefulWrapper::new(op, entity);
        let context = OpContext::new();
        
        let result = stateful_op.validate_prerequisites(&context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_compensating_op_default() {
        let op = ClosureOp::new(|_| Box::pin(async { Ok("test result".to_string()) }));
        let entity = EntityMetadata::new(
            "test-entity".to_string(),
            "database".to_string(),
        );
        
        let stateful_op = StatefulWrapper::new(op, entity);
        let context = OpContext::new();
        
        let result = stateful_op.create_compensating_op(&context).await;
        assert!(result.is_err());
        if let Err(OpError::ExecutionFailed(msg)) = result {
            assert_eq!(msg, "Rollback not implemented for this op");
        } else {
            panic!("Expected ExecutionFailed error");
        }
    }
}
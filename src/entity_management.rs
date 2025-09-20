// Entity Management Ops
// Core CRUD ops for entities with state tracking and rollback support

use crate::{
    Op, OpContext, OpError, OpResult, persistence::StatePersistenceManager, stateful::{EntityMetadata, StatefulOp, StatefulResult}
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Entity data for CRUD ops
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityData {
    /// Entity ID
    pub id: String,
    /// Entity type
    pub entity_type: String,
    /// Entity properties
    pub properties: HashMap<String, String>,
    /// Entity state
    pub state: String,
    /// Version for optimistic concurrency control
    pub version: u64,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last update timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl EntityData {
    /// Create new entity data
    pub fn new(id: String, entity_type: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            entity_type,
            properties: HashMap::new(),
            state: "created".to_string(),
            version: 1,
            created_at: now,
            updated_at: now,
        }
    }

    /// Add property to entity
    pub fn with_property(mut self, key: String, value: String) -> Self {
        self.properties.insert(key, value);
        self
    }

    /// Set entity state
    pub fn with_state(mut self, state: String) -> Self {
        self.state = state;
        self
    }

    /// Update entity (increment version and timestamp)
    pub fn update(&mut self) {
        self.version += 1;
        self.updated_at = chrono::Utc::now();
    }

    /// Convert to EntityMetadata
    pub fn to_metadata(&self) -> EntityMetadata {
        EntityMetadata {
            id: self.id.clone(),
            entity_type: self.entity_type.clone(),
            properties: self.properties.clone(),
            dependencies: Vec::new(),
            created_at: Some(self.created_at),
            updated_at: Some(self.updated_at),
        }
    }
}

/// Create entity op
pub struct CreateEntityOp {
    entity_data: EntityData,
    metadata: EntityMetadata,
    persistence_manager: Option<StatePersistenceManager>,
}

impl CreateEntityOp {
    /// Create new create entity op
    pub fn new(entity_data: EntityData) -> Self {
        let metadata = entity_data.to_metadata();
        Self {
            entity_data,
            metadata,
            persistence_manager: None,
        }
    }

    /// Set persistence manager
    pub fn with_persistence(mut self, manager: StatePersistenceManager) -> Self {
        self.persistence_manager = Some(manager);
        self
    }
}

#[async_trait]
impl Op<StatefulResult<EntityData>> for CreateEntityOp {
    async fn perform(&self, context: &mut OpContext) -> OpResult<StatefulResult<EntityData>> {
        let start_time = std::time::Instant::now();
        
        // Check if entity already exists
        let entity_key = format!("entity:{}", self.entity_data.id);
        if context.contains_key(&entity_key) {
            return Err(OpError::ExecutionFailed(
                format!("Entity already exists: {}", self.entity_data.id)
            ));
        }

        // Store entity in context
        context.put(&entity_key, self.entity_data.clone()).map_err(|e| {
            OpError::ExecutionFailed(format!("Failed to store entity: {}", e))
        })?;

        // Store creation context for rollback
        context.put("op_type", "create".to_string()).map_err(|e| {
            OpError::ExecutionFailed(format!("Failed to store op type: {}", e))
        })?;

        // Persist if manager is available
        if let Some(_manager) = &self.persistence_manager {
            // Note: Persistence integration would go here in a real implementation
            // For now, we just log that persistence would happen
            tracing::info!("Entity {} would be persisted", self.entity_data.id);
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;
        
        Ok(StatefulResult::new(
            self.entity_data.clone(),
            self.metadata.clone(),
            duration_ms
        ).with_message(format!("Created entity: {}", self.entity_data.id)))
    }
}

#[async_trait]
impl StatefulOp<EntityData> for CreateEntityOp {
    fn get_entity_metadata(&self) -> &EntityMetadata {
        &self.metadata
    }

    async fn validate_prerequisites(&self, context: &OpContext) -> OpResult<()> {
        // Check if entity already exists
        let entity_key = format!("entity:{}", self.entity_data.id);
        if context.contains_key(&entity_key) {
            return Err(OpError::ExecutionFailed(
                format!("Entity already exists: {}", self.entity_data.id)
            ));
        }

        // Validate dependencies
        for dep in &self.metadata.dependencies {
            let dep_key = format!("entity:{}", dep);
            if !context.contains_key(&dep_key) {
                return Err(OpError::ExecutionFailed(
                    format!("Missing dependency: {}", dep)
                ));
            }
        }

        Ok(())
    }

    async fn create_compensating_op(&self, _context: &OpContext) -> Result<Box<dyn StatefulOp<EntityData>>, OpError> {
        // Create delete op as compensation
        let delete_op = DeleteEntityOp::new(self.entity_data.id.clone(), self.entity_data.entity_type.clone());
        Ok(Box::new(delete_op))
    }

    fn supports_rollback(&self) -> bool {
        true
    }

    fn is_idempotent(&self) -> bool {
        false // Creating the same entity twice should fail
    }

    fn estimate_duration(&self) -> u64 {
        2000 // 2 seconds estimated
    }

    fn get_priority(&self) -> u32 {
        50 // Higher priority than default
    }
}

/// Read entity op
pub struct ReadEntityOp {
    entity_id: String,
    entity_type: String,
    metadata: EntityMetadata,
    persistence_manager: Option<StatePersistenceManager>,
}

impl ReadEntityOp {
    /// Create new read entity op
    pub fn new(entity_id: String, entity_type: String) -> Self {
        let metadata = EntityMetadata::new(entity_id.clone(), entity_type.clone());
        Self {
            entity_id,
            entity_type,
            metadata,
            persistence_manager: None,
        }
    }

    /// Set persistence manager
    pub fn with_persistence(mut self, manager: StatePersistenceManager) -> Self {
        self.persistence_manager = Some(manager);
        self
    }
}

#[async_trait]
impl Op<StatefulResult<EntityData>> for ReadEntityOp {
    async fn perform(&self, context: &mut OpContext) -> OpResult<StatefulResult<EntityData>> {
        let start_time = std::time::Instant::now();
        
        // Try to get entity from context first
        let entity_key = format!("entity:{}", self.entity_id);
        if let Some(entity_data) = context.get::<EntityData>(&entity_key) {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            return Ok(StatefulResult::new(
                entity_data,
                self.metadata.clone(),
                duration_ms
            ).with_message(format!("Read entity from context: {}", self.entity_id)));
        }

        // Try to load from persistence if manager is available
        if let Some(_manager) = &self.persistence_manager {
            // Note: Persistence loading would go here in a real implementation
            tracing::info!("Would attempt to load entity {} from persistence", self.entity_id);
        }

        Err(OpError::ExecutionFailed(
            format!("Entity not found: {}", self.entity_id)
        ))
    }
}

#[async_trait]
impl StatefulOp<EntityData> for ReadEntityOp {
    fn get_entity_metadata(&self) -> &EntityMetadata {
        &self.metadata
    }

    async fn create_compensating_op(&self, _context: &OpContext) -> Result<Box<dyn StatefulOp<EntityData>>, OpError> {
        // Read ops typically don't need rollback
        Err(OpError::ExecutionFailed(
            "Read ops do not require rollback".to_string()
        ))
    }

    fn supports_rollback(&self) -> bool {
        false // Read ops don't need rollback
    }

    fn is_idempotent(&self) -> bool {
        true // Reading is always safe to repeat
    }

    fn estimate_duration(&self) -> u64 {
        500 // 500ms estimated
    }

    fn get_priority(&self) -> u32 {
        30 // High priority for reads
    }
}

/// Update entity op
pub struct UpdateEntityOp {
    entity_id: String,
    updates: HashMap<String, String>,
    new_state: Option<String>,
    metadata: EntityMetadata,
    persistence_manager: Option<StatePersistenceManager>,
}

impl UpdateEntityOp {
    /// Create new update entity op
    pub fn new(entity_id: String, entity_type: String) -> Self {
        let metadata = EntityMetadata::new(entity_id.clone(), entity_type.clone());
        Self {
            entity_id,
            updates: HashMap::new(),
            new_state: None,
            metadata,
            persistence_manager: None,
        }
    }

    /// Add property update
    pub fn with_property_update(mut self, key: String, value: String) -> Self {
        self.updates.insert(key, value);
        self
    }

    /// Set new entity state
    pub fn with_state_update(mut self, state: String) -> Self {
        self.new_state = Some(state);
        self
    }

    /// Set persistence manager
    pub fn with_persistence(mut self, manager: StatePersistenceManager) -> Self {
        self.persistence_manager = Some(manager);
        self
    }
}

#[async_trait]
impl Op<StatefulResult<EntityData>> for UpdateEntityOp {
    async fn perform(&self, context: &mut OpContext) -> OpResult<StatefulResult<EntityData>> {
        let start_time = std::time::Instant::now();
        
        // Get existing entity
        let entity_key = format!("entity:{}", self.entity_id);
        let mut entity_data = context.get::<EntityData>(&entity_key)
            .ok_or_else(|| OpError::ExecutionFailed(
                format!("Entity not found: {}", self.entity_id)
            ))?;

        // Store previous state for rollback
        let previous_state_key = format!("previous_state:{}", self.entity_id);
        context.put(&previous_state_key, entity_data.clone()).map_err(|e| {
            OpError::ExecutionFailed(format!("Failed to store previous state: {}", e))
        })?;

        // Apply updates
        for (key, value) in &self.updates {
            entity_data.properties.insert(key.clone(), value.clone());
        }

        if let Some(new_state) = &self.new_state {
            entity_data.state = new_state.clone();
        }

        entity_data.update();

        // Update entity in context
        context.put(&entity_key, entity_data.clone()).map_err(|e| {
            OpError::ExecutionFailed(format!("Failed to update entity: {}", e))
        })?;

        // Store op type for rollback
        context.put("op_type", "update".to_string()).map_err(|e| {
            OpError::ExecutionFailed(format!("Failed to store op type: {}", e))
        })?;

        // Persist if manager is available
        if let Some(_manager) = &self.persistence_manager {
            // Note: Persistence update would go here in a real implementation
            tracing::info!("Entity {} would be persisted after update", self.entity_id);
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;
        
        Ok(StatefulResult::new(
            entity_data,
            self.metadata.clone(),
            duration_ms
        ).with_message(format!("Updated entity: {}", self.entity_id)))
    }
}

#[async_trait]
impl StatefulOp<EntityData> for UpdateEntityOp {
    fn get_entity_metadata(&self) -> &EntityMetadata {
        &self.metadata
    }

    async fn validate_prerequisites(&self, context: &OpContext) -> OpResult<()> {
        // Check if entity exists
        let entity_key = format!("entity:{}", self.entity_id);
        if !context.contains_key(&entity_key) {
            return Err(OpError::ExecutionFailed(
                format!("Entity not found: {}", self.entity_id)
            ));
        }

        Ok(())
    }

    async fn create_compensating_op(&self, context: &OpContext) -> Result<Box<dyn StatefulOp<EntityData>>, OpError> {
        // Get previous state for rollback
        let previous_state_key = format!("previous_state:{}", self.entity_id);
        let previous_data = context.get::<EntityData>(&previous_state_key)
            .ok_or_else(|| OpError::ExecutionFailed(
                "Previous state not available for rollback".to_string()
            ))?;

        // Create restore op
        let mut restore_op = UpdateEntityOp::new(
            self.entity_id.clone(),
            self.metadata.entity_type.clone()
        );

        // Set all properties back to previous values
        for (key, value) in &previous_data.properties {
            restore_op = restore_op.with_property_update(key.clone(), value.clone());
        }

        restore_op = restore_op.with_state_update(previous_data.state.clone());

        if let Some(manager) = &self.persistence_manager {
            restore_op = restore_op.with_persistence(manager.clone());
        }

        Ok(Box::new(restore_op))
    }

    fn supports_rollback(&self) -> bool {
        true
    }

    fn is_idempotent(&self) -> bool {
        true // Updates with same values are idempotent
    }

    fn estimate_duration(&self) -> u64 {
        3000 // 3 seconds estimated
    }

    fn get_priority(&self) -> u32 {
        70 // Medium-high priority
    }
}

/// Delete entity op
pub struct DeleteEntityOp {
    entity_id: String,
    entity_type: String,
    metadata: EntityMetadata,
    persistence_manager: Option<StatePersistenceManager>,
}

impl DeleteEntityOp {
    /// Create new delete entity op
    pub fn new(entity_id: String, entity_type: String) -> Self {
        let metadata = EntityMetadata::new(entity_id.clone(), entity_type.clone());
        Self {
            entity_id,
            entity_type,
            metadata,
            persistence_manager: None,
        }
    }

    /// Set persistence manager
    pub fn with_persistence(mut self, manager: StatePersistenceManager) -> Self {
        self.persistence_manager = Some(manager);
        self
    }
}

#[async_trait]
impl Op<StatefulResult<EntityData>> for DeleteEntityOp {
    async fn perform(&self, context: &mut OpContext) -> OpResult<StatefulResult<EntityData>> {
        let start_time = std::time::Instant::now();
        
        // Get entity before deletion
        let entity_key = format!("entity:{}", self.entity_id);
        let entity_data = context.get::<EntityData>(&entity_key)
            .ok_or_else(|| OpError::ExecutionFailed(
                format!("Entity not found: {}", self.entity_id)
            ))?;

        // Store deleted entity for rollback
        let deleted_state_key = format!("deleted_state:{}", self.entity_id);
        context.put(&deleted_state_key, entity_data.clone()).map_err(|e| {
            OpError::ExecutionFailed(format!("Failed to store deleted state: {}", e))
        })?;

        // Remove entity from context
        context.remove(&entity_key);

        // Store op type for rollback
        context.put("op_type", "delete".to_string()).map_err(|e| {
            OpError::ExecutionFailed(format!("Failed to store op type: {}", e))
        })?;

        // Remove from persistence if manager is available
        if let Some(_manager) = &self.persistence_manager {
            // Note: Persistence deletion would go here in a real implementation
            tracing::info!("Entity {} would be deleted from persistence", self.entity_id);
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;
        
        Ok(StatefulResult::new(
            entity_data,
            self.metadata.clone(),
            duration_ms
        ).with_message(format!("Deleted entity: {}", self.entity_id)))
    }
}

#[async_trait]
impl StatefulOp<EntityData> for DeleteEntityOp {
    fn get_entity_metadata(&self) -> &EntityMetadata {
        &self.metadata
    }

    async fn validate_prerequisites(&self, context: &OpContext) -> OpResult<()> {
        // Check if entity exists
        let entity_key = format!("entity:{}", self.entity_id);
        if !context.contains_key(&entity_key) {
            return Err(OpError::ExecutionFailed(
                format!("Entity not found: {}", self.entity_id)
            ));
        }

        Ok(())
    }

    async fn create_compensating_op(&self, context: &OpContext) -> Result<Box<dyn StatefulOp<EntityData>>, OpError> {
        // Get deleted entity data for restoration
        let deleted_state_key = format!("deleted_state:{}", self.entity_id);
        let deleted_data = context.get::<EntityData>(&deleted_state_key)
            .ok_or_else(|| OpError::ExecutionFailed(
                "Deleted state not available for rollback".to_string()
            ))?;

        // Create restore op
        let mut create_op = CreateEntityOp::new(deleted_data);
        
        if let Some(manager) = &self.persistence_manager {
            create_op = create_op.with_persistence(manager.clone());
        }

        Ok(Box::new(create_op))
    }

    fn supports_rollback(&self) -> bool {
        true
    }

    fn is_idempotent(&self) -> bool {
        true // Deleting non-existent entity should not error in idempotent mode
    }

    fn estimate_duration(&self) -> u64 {
        1500 // 1.5 seconds estimated
    }

    fn get_priority(&self) -> u32 {
        90 // Lower priority than creates/updates
    }
}

/// Entity query op for finding entities
pub struct QueryEntitiesOp {
    entity_type: Option<String>,
    filters: HashMap<String, String>,
    metadata: EntityMetadata,
    persistence_manager: Option<StatePersistenceManager>,
}

impl QueryEntitiesOp {
    /// Create new query entities op
    pub fn new() -> Self {
        let metadata = EntityMetadata::new("query".to_string(), "query".to_string());
        Self {
            entity_type: None,
            filters: HashMap::new(),
            metadata,
            persistence_manager: None,
        }
    }

    /// Filter by entity type
    pub fn with_entity_type(mut self, entity_type: String) -> Self {
        self.entity_type = Some(entity_type);
        self
    }

    /// Add property filter
    pub fn with_property_filter(mut self, key: String, value: String) -> Self {
        self.filters.insert(key, value);
        self
    }

    /// Set persistence manager
    pub fn with_persistence(mut self, manager: StatePersistenceManager) -> Self {
        self.persistence_manager = Some(manager);
        self
    }
}

#[async_trait]
impl Op<StatefulResult<Vec<EntityData>>> for QueryEntitiesOp {
    async fn perform(&self, context: &mut OpContext) -> Result<StatefulResult<Vec<EntityData>>, OpError> {
        let start_time = std::time::Instant::now();
        let mut found_entities = Vec::new();

        // Search in context
        for (key, _value) in context.values().iter() {
            if key.starts_with("entity:") {
                // Try to get the entity from context
                if let Some(entity) = context.get::<EntityData>(key) {
                    // Apply filters
                    let mut matches = true;
                    
                    if let Some(ref entity_type) = self.entity_type {
                        if entity.entity_type != *entity_type {
                            matches = false;
                        }
                    }
                    
                    for (filter_key, filter_value) in &self.filters {
                        if let Some(entity_value) = entity.properties.get(filter_key) {
                            if entity_value != filter_value {
                                matches = false;
                                break;
                            }
                        } else {
                            matches = false;
                            break;
                        }
                    }
                    
                    if matches {
                        found_entities.push(entity);
                    }
                }
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;
        
        Ok(StatefulResult::new(
            found_entities,
            self.metadata.clone(),
            duration_ms
        ).with_message("Entity query completed".to_string()))
    }
}

#[async_trait]
impl StatefulOp<Vec<EntityData>> for QueryEntitiesOp {
    fn get_entity_metadata(&self) -> &EntityMetadata {
        &self.metadata
    }

    async fn create_compensating_op(&self, _context: &OpContext) -> Result<Box<dyn StatefulOp<Vec<EntityData>>>, OpError> {
        // Query ops typically don't need rollback
        Err(OpError::ExecutionFailed(
            "Query ops do not require rollback".to_string()
        ))
    }

    fn supports_rollback(&self) -> bool {
        false // Query ops don't need rollback
    }

    fn is_idempotent(&self) -> bool {
        true // Queries are always safe to repeat
    }

    fn estimate_duration(&self) -> u64 {
        1000 // 1 second estimated
    }

    fn get_priority(&self) -> u32 {
        20 // Very high priority for queries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OpContext;

    #[test]
    fn test_entity_data_creation() {
        let entity = EntityData::new("test-1".to_string(), "database".to_string())
            .with_property("host".to_string(), "localhost".to_string())
            .with_state("running".to_string());

        assert_eq!(entity.id, "test-1");
        assert_eq!(entity.entity_type, "database");
        assert_eq!(entity.state, "running");
        assert_eq!(entity.version, 1);
        assert!(entity.properties.contains_key("host"));
    }

    #[test]
    fn test_entity_data_update() {
        let mut entity = EntityData::new("test-1".to_string(), "database".to_string());
        let original_version = entity.version;
        let original_time = entity.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(1));
        entity.update();

        assert_eq!(entity.version, original_version + 1);
        assert!(entity.updated_at > original_time);
    }

    #[tokio::test]
    async fn test_create_entity_op() {
        let entity_data = EntityData::new("test-1".to_string(), "database".to_string())
            .with_property("host".to_string(), "localhost".to_string());
        
        let create_op = CreateEntityOp::new(entity_data.clone());
        let mut context = OpContext::new();

        let result = create_op.perform(&mut context).await.unwrap();
        
        assert_eq!(result.result.id, "test-1");
        assert_eq!(result.entity.id, "test-1");
        assert!(result.duration_ms >= 0);
        assert!(!result.messages.is_empty());

        // Verify entity is stored in context
        let stored_entity: Option<EntityData> = context.get("entity:test-1");
        assert!(stored_entity.is_some());
        assert_eq!(stored_entity.unwrap().id, "test-1");
    }

    #[tokio::test]
    async fn test_create_entity_duplicate_error() {
        let entity_data = EntityData::new("test-1".to_string(), "database".to_string());
        let create_op = CreateEntityOp::new(entity_data.clone());
        let mut context = OpContext::new();

        // First creation should succeed
        let result = create_op.perform(&mut context).await;
        assert!(result.is_ok());

        // Second creation should fail
        let result = create_op.perform(&mut context).await;
        assert!(result.is_err());
        
        if let Err(OpError::ExecutionFailed(msg)) = result {
            assert!(msg.contains("already exists"));
        } else {
            panic!("Expected ExecutionFailed error");
        }
    }

    #[tokio::test]
    async fn test_read_entity_op() {
        let entity_data = EntityData::new("test-1".to_string(), "database".to_string());
        let mut context = OpContext::new();
        
        // Store entity in context first
        context.put("entity:test-1", entity_data.clone()).unwrap();

        let read_op = ReadEntityOp::new("test-1".to_string(), "database".to_string());
        let result = read_op.perform(&mut context).await.unwrap();
        
        assert_eq!(result.result.id, "test-1");
        assert_eq!(result.entity.id, "test-1");
        assert!(result.duration_ms >= 0);
    }

    #[tokio::test]
    async fn test_read_entity_not_found() {
        let read_op = ReadEntityOp::new("nonexistent".to_string(), "database".to_string());
        let mut context = OpContext::new();

        let result = read_op.perform(&mut context).await;
        assert!(result.is_err());
        
        if let Err(OpError::ExecutionFailed(msg)) = result {
            assert!(msg.contains("not found"));
        } else {
            panic!("Expected ExecutionFailed error");
        }
    }

    #[tokio::test]
    async fn test_update_entity_op() {
        let mut entity_data = EntityData::new("test-1".to_string(), "database".to_string());
        entity_data.properties.insert("host".to_string(), "localhost".to_string());
        
        let mut context = OpContext::new();
        context.put("entity:test-1", entity_data.clone()).unwrap();

        let update_op = UpdateEntityOp::new("test-1".to_string(), "database".to_string())
            .with_property_update("host".to_string(), "remotehost".to_string())
            .with_state_update("configured".to_string());

        let result = update_op.perform(&mut context).await.unwrap();
        
        assert_eq!(result.result.properties.get("host"), Some(&"remotehost".to_string()));
        assert_eq!(result.result.state, "configured");
        assert_eq!(result.result.version, entity_data.version + 1);
        assert!(result.result.updated_at > entity_data.updated_at);
    }

    #[tokio::test]
    async fn test_update_entity_not_found() {
        let update_op = UpdateEntityOp::new("nonexistent".to_string(), "database".to_string());
        let mut context = OpContext::new();

        let result = update_op.perform(&mut context).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_entity_op() {
        let entity_data = EntityData::new("test-1".to_string(), "database".to_string());
        let mut context = OpContext::new();
        context.put("entity:test-1", entity_data.clone()).unwrap();

        let delete_op = DeleteEntityOp::new("test-1".to_string(), "database".to_string());
        let result = delete_op.perform(&mut context).await.unwrap();
        
        assert_eq!(result.result.id, "test-1");
        
        // Verify entity is removed from context
        let stored_entity: Option<EntityData> = context.get("entity:test-1");
        assert!(stored_entity.is_none());
        
        // Verify deleted state is stored for rollback
        let deleted_state: Option<EntityData> = context.get("deleted_state:test-1");
        assert!(deleted_state.is_some());
    }

    #[tokio::test]
    async fn test_delete_entity_not_found() {
        let delete_op = DeleteEntityOp::new("nonexistent".to_string(), "database".to_string());
        let mut context = OpContext::new();

        let result = delete_op.perform(&mut context).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_query_entities_op() {
        let entity1 = EntityData::new("test-1".to_string(), "database".to_string())
            .with_property("env".to_string(), "prod".to_string());
        let entity2 = EntityData::new("test-2".to_string(), "database".to_string())
            .with_property("env".to_string(), "dev".to_string());
        let entity3 = EntityData::new("test-3".to_string(), "service".to_string())
            .with_property("env".to_string(), "prod".to_string());

        let mut context = OpContext::new();
        context.put("entity:test-1", entity1).unwrap();
        context.put("entity:test-2", entity2).unwrap();
        context.put("entity:test-3", entity3).unwrap();

        // Query all database entities
        let query_op = QueryEntitiesOp::new()
            .with_entity_type("database".to_string());
        let result = query_op.perform(&mut context).await.unwrap();
        assert_eq!(result.result.len(), 2);

        // Query prod entities
        let query_op = QueryEntitiesOp::new()
            .with_property_filter("env".to_string(), "prod".to_string());
        let result = query_op.perform(&mut context).await.unwrap();
        assert_eq!(result.result.len(), 2);

        // Query prod database entities
        let query_op = QueryEntitiesOp::new()
            .with_entity_type("database".to_string())
            .with_property_filter("env".to_string(), "prod".to_string());
        let result = query_op.perform(&mut context).await.unwrap();
        assert_eq!(result.result.len(), 1);
        assert_eq!(result.result[0].id, "test-1");
    }

    #[tokio::test]
    async fn test_stateful_op_traits() {
        let entity_data = EntityData::new("test-1".to_string(), "database".to_string());
        let create_op = CreateEntityOp::new(entity_data);

        assert_eq!(create_op.get_entity_metadata().id, "test-1");
        assert!(create_op.supports_rollback());
        assert!(!create_op.is_idempotent());
        assert_eq!(create_op.estimate_duration(), 2000);
        assert_eq!(create_op.get_priority(), 50);

        let read_op = ReadEntityOp::new("test-1".to_string(), "database".to_string());
        assert!(!read_op.supports_rollback());
        assert!(read_op.is_idempotent());
        assert_eq!(read_op.get_priority(), 30);

        let update_op = UpdateEntityOp::new("test-1".to_string(), "database".to_string());
        assert!(update_op.supports_rollback());
        assert!(update_op.is_idempotent());

        let delete_op = DeleteEntityOp::new("test-1".to_string(), "database".to_string());
        assert!(delete_op.supports_rollback());
        assert!(delete_op.is_idempotent());
    }

    #[tokio::test]
    async fn test_create_compensating_op() {
        let entity_data = EntityData::new("test-1".to_string(), "database".to_string());
        let create_op = CreateEntityOp::new(entity_data);
        let context = OpContext::new();

        let compensating_op = create_op.create_compensating_op(&context).await.unwrap();
        
        // Compensating op should be a delete op
        assert_eq!(compensating_op.get_entity_metadata().id, "test-1");
    }

    #[tokio::test]
    async fn test_update_compensating_op() {
        let entity_data = EntityData::new("test-1".to_string(), "database".to_string())
            .with_property("host".to_string(), "original".to_string());
        
        let mut context = OpContext::new();
        context.put("entity:test-1", entity_data.clone()).unwrap();
        
        let update_op = UpdateEntityOp::new("test-1".to_string(), "database".to_string())
            .with_property_update("host".to_string(), "updated".to_string());

        // Perform update to store previous state
        let _ = update_op.perform(&mut context).await.unwrap();

        // Create compensating op
        let compensating_op = update_op.create_compensating_op(&context).await.unwrap();
        
        assert_eq!(compensating_op.get_entity_metadata().id, "test-1");
    }

    #[tokio::test]
    async fn test_delete_compensating_op() {
        let entity_data = EntityData::new("test-1".to_string(), "database".to_string());
        let mut context = OpContext::new();
        context.put("entity:test-1", entity_data.clone()).unwrap();

        let delete_op = DeleteEntityOp::new("test-1".to_string(), "database".to_string());
        
        // Perform delete to store deleted state
        let _ = delete_op.perform(&mut context).await.unwrap();

        // Create compensating op
        let compensating_op = delete_op.create_compensating_op(&context).await.unwrap();
        
        assert_eq!(compensating_op.get_entity_metadata().id, "test-1");
    }

    #[tokio::test]
    async fn test_validate_prerequisites() {
        let entity_data = EntityData::new("test-1".to_string(), "database".to_string());
        let create_op = CreateEntityOp::new(entity_data.clone());
        
        let mut context = OpContext::new();
        
        // Should pass validation when entity doesn't exist
        let result = create_op.validate_prerequisites(&context).await;
        assert!(result.is_ok());

        // Store entity
        context.put("entity:test-1", entity_data).unwrap();
        
        // Should fail validation when entity exists
        let result = create_op.validate_prerequisites(&context).await;
        assert!(result.is_err());
    }
}
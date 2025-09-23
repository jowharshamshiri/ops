use crate::prelude::*;
// Safe Point Management
// Create and manage safe points for rollback boundaries

use crate::{Op, OpContext, OpError};
use crate::op_state::{OpStateInfo, OpInstanceId};
use crate::persistence::StatePersistenceManager;
use crate::rollback::{RollbackStrategy, RollbackPriority};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// Unique identifier for safe points
pub type SafePointId = String;

/// Safe point status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SafePointStatus {
    /// Safe point created and ready
    Active,
    /// Safe point being used for rollback
    InUse,
    /// Safe point successfully used for rollback
    Restored,
    /// Safe point expired or invalidated
    Expired,
    /// Safe point cleanup in progress
    Cleanup,
}

/// Safe point metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafePointMetadata {
    /// Unique safe point identifier
    pub id: SafePointId,
    /// Human-readable description
    pub description: String,
    /// When the safe point was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the safe point expires (if applicable)
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Current safe point status
    pub status: SafePointStatus,
    /// Associated op instance ID
    pub op_instance_id: OpInstanceId,
    /// Safe point priority for restoration ordering
    pub priority: RollbackPriority,
    /// Tags for categorization and search
    pub tags: Vec<String>,
    /// Custom attributes
    pub attributes: HashMap<String, String>,
}

impl SafePointMetadata {
    /// Create new safe point metadata
    pub fn new(id: SafePointId, description: String, op_instance_id: OpInstanceId) -> Self {
        Self {
            id,
            description,
            created_at: chrono::Utc::now(),
            expires_at: None,
            status: SafePointStatus::Active,
            op_instance_id,
            priority: RollbackPriority::Normal,
            tags: Vec::new(),
            attributes: HashMap::new(),
        }
    }

    /// Set expiration time
    pub fn with_expiration(mut self, expires_at: chrono::DateTime<chrono::Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: RollbackPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Add tag
    pub fn with_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }

    /// Add attribute
    pub fn with_attribute(mut self, key: String, value: String) -> Self {
        self.attributes.insert(key, value);
        self
    }

    /// Check if safe point is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            chrono::Utc::now() > expires_at
        } else {
            false
        }
    }

    /// Check if safe point is active and usable
    pub fn is_active(&self) -> bool {
        self.status == SafePointStatus::Active && !self.is_expired()
    }
}

/// Safe point snapshot data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafePointSnapshot {
    /// Safe point metadata
    pub metadata: SafePointMetadata,
    /// Op context at safe point
    pub context_snapshot: OpContext,
    /// Op state at safe point
    pub state_snapshot: OpStateInfo,
    /// Additional snapshot data
    pub snapshot_data: HashMap<String, serde_json::Value>,
    /// Checksum for integrity verification
    pub checksum: Option<String>,
}

impl SafePointSnapshot {
    /// Create new safe point snapshot
    pub fn new(
        metadata: SafePointMetadata,
        context: &OpContext,
        state: &OpStateInfo,
    ) -> Self {
        let mut snapshot = Self {
            metadata,
            context_snapshot: context.clone(),
            state_snapshot: state.clone(),
            snapshot_data: HashMap::new(),
            checksum: None,
        };
        
        // Calculate checksum for integrity
        snapshot.checksum = Some(snapshot.calculate_checksum());
        snapshot
    }

    /// Add custom snapshot data
    pub fn with_data(mut self, key: String, value: serde_json::Value) -> Self {
        self.snapshot_data.insert(key, value);
        // Recalculate checksum after adding data
        self.checksum = Some(self.calculate_checksum());
        self
    }

    /// Calculate integrity checksum
    fn calculate_checksum(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        
        // Hash metadata
        self.metadata.id.hash(&mut hasher);
        self.metadata.op_instance_id.hash(&mut hasher);
        self.metadata.created_at.timestamp().hash(&mut hasher);
        
        // Hash context data
        let context_json = serde_json::to_string(&self.context_snapshot).unwrap_or_default();
        context_json.hash(&mut hasher);
        
        // Hash state data
        self.state_snapshot.instance_id.hash(&mut hasher);
        self.state_snapshot.op_name.hash(&mut hasher);
        
        // Hash snapshot data
        let snapshot_json = serde_json::to_string(&self.snapshot_data).unwrap_or_default();
        snapshot_json.hash(&mut hasher);
        
        format!("{:x}", hasher.finish())
    }

    /// Verify snapshot integrity
    pub fn verify_integrity(&self) -> bool {
        if let Some(stored_checksum) = &self.checksum {
            let calculated_checksum = self.calculate_checksum();
            *stored_checksum == calculated_checksum
        } else {
            false
        }
    }

    /// Check if snapshot is ready for restoration
    pub fn is_restorable(&self) -> bool {
        self.metadata.is_active() && self.verify_integrity()
    }
}

/// Safe point creation options
#[derive(Debug, Clone)]
pub struct SafePointOptions {
    /// Safe point description
    pub description: String,
    /// Expiration duration from creation
    pub expires_after: Option<Duration>,
    /// Safe point priority
    pub priority: RollbackPriority,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Custom attributes
    pub attributes: HashMap<String, String>,
    /// Whether to include custom snapshot data
    pub include_custom_data: bool,
}

impl Default for SafePointOptions {
    fn default() -> Self {
        Self {
            description: "Automatic safe point".to_string(),
            expires_after: Some(Duration::from_secs(3600)), // 1 hour default
            priority: RollbackPriority::Normal,
            tags: Vec::new(),
            attributes: HashMap::new(),
            include_custom_data: false,
        }
    }
}

impl SafePointOptions {
    /// Create new safe point options
    pub fn new(description: String) -> Self {
        Self {
            description,
            ..Default::default()
        }
    }

    /// Set expiration duration
    pub fn expires_after(mut self, duration: Duration) -> Self {
        self.expires_after = Some(duration);
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: RollbackPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Add tag
    pub fn with_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }

    /// Add attribute
    pub fn with_attribute(mut self, key: String, value: String) -> Self {
        self.attributes.insert(key, value);
        self
    }

    /// Include custom snapshot data
    pub fn include_custom_data(mut self) -> Self {
        self.include_custom_data = true;
        self
    }
}

/// Safe point manager for creating and managing safe points
pub struct SafePointManager {
    /// Active safe points
    safe_points: Arc<RwLock<HashMap<SafePointId, SafePointSnapshot>>>,
    /// Persistence manager for safe point storage
    persistence_manager: Option<Arc<StatePersistenceManager>>,
    /// Next safe point ID counter
    next_id_counter: Arc<RwLock<u64>>,
}

impl SafePointManager {
    /// Create new safe point manager
    pub fn new() -> Self {
        Self {
            safe_points: Arc::new(RwLock::new(HashMap::new())),
            persistence_manager: None,
            next_id_counter: Arc::new(RwLock::new(1)),
        }
    }

    /// Create safe point manager with persistence
    pub fn with_persistence(persistence_manager: Arc<StatePersistenceManager>) -> Self {
        Self {
            safe_points: Arc::new(RwLock::new(HashMap::new())),
            persistence_manager: Some(persistence_manager),
            next_id_counter: Arc::new(RwLock::new(1)),
        }
    }

    /// Generate unique safe point ID
    pub fn generate_safe_point_id(&self) -> SafePointId {
        let mut counter = self.next_id_counter.write().unwrap();
        let id = *counter;
        *counter += 1;
        format!("sp_{:08}_{}", id, chrono::Utc::now().timestamp())
    }

    /// Create safe point
    pub async fn create_safe_point(
        &self,
        op_instance_id: OpInstanceId,
        context: &OpContext,
        state: &OpStateInfo,
        options: SafePointOptions,
    ) -> Result<SafePointId, OpError> {
        let safe_point_id = self.generate_safe_point_id();
        
        // Create safe point metadata
        let mut metadata = SafePointMetadata::new(
            safe_point_id.clone(),
            options.description,
            op_instance_id,
        ).with_priority(options.priority);

        // Set expiration if specified
        if let Some(expires_after) = options.expires_after {
            let expires_at = chrono::Utc::now() + chrono::Duration::from_std(expires_after)
                .map_err(|e| OpError::ExecutionFailed(format!("Invalid expiration duration: {}", e)))?;
            metadata = metadata.with_expiration(expires_at);
        }

        // Add tags and attributes
        for tag in options.tags {
            metadata = metadata.with_tag(tag);
        }
        for (key, value) in options.attributes {
            metadata = metadata.with_attribute(key, value);
        }

        // Create snapshot
        let mut snapshot = SafePointSnapshot::new(metadata, context, state);

        // Add custom data if requested
        if options.include_custom_data {
            snapshot = snapshot.with_data(
                "creation_timestamp".to_string(),
                serde_json::json!(chrono::Utc::now().timestamp()),
            ).with_data(
                "system_info".to_string(),
                serde_json::json!({
                    "hostname": "localhost", // Could be actual hostname
                    "process_id": std::process::id(),
                }),
            );
        }

        // Store safe point
        {
            let mut safe_points = self.safe_points.write().unwrap();
            safe_points.insert(safe_point_id.clone(), snapshot.clone());
        }

        // Persist if manager available
        if let Some(persistence_manager) = &self.persistence_manager {
            // Convert SafePointSnapshot to StateSnapshot for persistence
            let state_snapshot = crate::persistence::StateSnapshot {
                id: safe_point_id.clone(),
                created_at: snapshot.metadata.created_at,
                entity_metadata: crate::stateful::EntityMetadata::new(
                    snapshot.metadata.op_instance_id.clone(),
                    "safe_point".to_string(),
                ),
                current_state: "active".to_string(),
                state_machine_states: vec![],
                context_data: snapshot.context_snapshot.values().iter()
                    .map(|(k, v)| (k.clone(), v.to_string()))
                    .collect(),
                transition_history: vec![],
            };
            
            persistence_manager.persist_snapshot(&state_snapshot)
                .await.map_err(|e| OpError::ExecutionFailed(format!("Failed to persist safe point: {}", e)))?;
        }

        Ok(safe_point_id)
    }

    /// Get safe point
    pub fn get_safe_point(&self, safe_point_id: &SafePointId) -> Option<SafePointSnapshot> {
        let safe_points = self.safe_points.read().unwrap();
        safe_points.get(safe_point_id).cloned()
    }

    /// List safe points with filters
    pub fn list_safe_points(&self, filter_expired: bool) -> Vec<SafePointSnapshot> {
        let safe_points = self.safe_points.read().unwrap();
        safe_points.values()
            .filter(|snapshot| !filter_expired || !snapshot.metadata.is_expired())
            .cloned()
            .collect()
    }

    /// Find safe points by op
    pub fn find_safe_points_by_op(&self, op_instance_id: &OpInstanceId) -> Vec<SafePointSnapshot> {
        let safe_points = self.safe_points.read().unwrap();
        safe_points.values()
            .filter(|snapshot| snapshot.metadata.op_instance_id == *op_instance_id)
            .cloned()
            .collect()
    }

    /// Find safe points by tag
    pub fn find_safe_points_by_tag(&self, tag: &str) -> Vec<SafePointSnapshot> {
        let safe_points = self.safe_points.read().unwrap();
        safe_points.values()
            .filter(|snapshot| snapshot.metadata.tags.contains(&tag.to_string()))
            .cloned()
            .collect()
    }

    /// Delete safe point
    pub async fn delete_safe_point(&self, safe_point_id: &SafePointId) -> Result<bool, OpError> {
        // Remove from memory
        let removed = {
            let mut safe_points = self.safe_points.write().unwrap();
            safe_points.remove(safe_point_id).is_some()
        };

        // Remove from persistence if available
        if removed && self.persistence_manager.is_some() {
            if let Some(_persistence_manager) = &self.persistence_manager {
                // Note: Persistence manager doesn't have delete_state method
                // In a full implementation, we would need to add this method
                // For now, we'll just log that deletion would happen
                tracing::info!("Would delete persisted safe point: {}", safe_point_id);
            }
        }

        Ok(removed)
    }

    /// Cleanup expired safe points
    pub async fn cleanup_expired_safe_points(&self) -> Result<usize, OpError> {
        let expired_ids: Vec<SafePointId> = {
            let safe_points = self.safe_points.read().unwrap();
            safe_points.values()
                .filter(|snapshot| snapshot.metadata.is_expired())
                .map(|snapshot| snapshot.metadata.id.clone())
                .collect()
        };

        let mut cleaned_count = 0;
        for safe_point_id in expired_ids {
            if self.delete_safe_point(&safe_point_id).await? {
                cleaned_count += 1;
            }
        }

        Ok(cleaned_count)
    }

    /// Get safe point count
    pub fn safe_point_count(&self) -> usize {
        let safe_points = self.safe_points.read().unwrap();
        safe_points.len()
    }

    /// Get active safe point count
    pub fn active_safe_point_count(&self) -> usize {
        let safe_points = self.safe_points.read().unwrap();
        safe_points.values()
            .filter(|snapshot| snapshot.metadata.is_active())
            .count()
    }
}

impl Default for SafePointManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Safe point restoration op
pub struct SafePointRestoreOp {
    safe_point_id: SafePointId,
    safe_point_manager: Arc<SafePointManager>,
    rollback_strategy: Arc<dyn RollbackStrategy>,
    verify_integrity: bool,
}

impl SafePointRestoreOp {
    /// Create new safe point restore op
    pub fn new(
        safe_point_id: SafePointId,
        safe_point_manager: Arc<SafePointManager>,
        rollback_strategy: Arc<dyn RollbackStrategy>,
    ) -> Self {
        Self {
            safe_point_id,
            safe_point_manager,
            rollback_strategy,
            verify_integrity: true,
        }
    }

    /// Skip integrity verification (use with caution)
    pub fn skip_integrity_verification(mut self) -> Self {
        self.verify_integrity = false;
        self
    }
}

#[async_trait]
impl Op<SafePointSnapshot> for SafePointRestoreOp {
    async fn perform(&self, context: &mut OpContext) -> Result<SafePointSnapshot, OpError> {
        // Get safe point snapshot
        let snapshot = self.safe_point_manager.get_safe_point(&self.safe_point_id)
            .ok_or_else(|| OpError::ExecutionFailed(
                format!("Safe point not found: {}", self.safe_point_id)
            ))?;

        // Verify integrity if required
        if self.verify_integrity && !snapshot.verify_integrity() {
            return Err(OpError::ExecutionFailed(
                format!("Safe point integrity verification failed: {}", self.safe_point_id)
            ));
        }

        // Check if safe point is restorable
        if !snapshot.is_restorable() {
            return Err(OpError::ExecutionFailed(
                format!("Safe point is not restorable: {}", self.safe_point_id)
            ));
        }

        // Update safe point status to in-use
        {
            let safe_points = self.safe_point_manager.safe_points.write().unwrap();
            if let Some(stored_snapshot) = safe_points.get(&self.safe_point_id).cloned() {
                let mut updated_snapshot = stored_snapshot;
                updated_snapshot.metadata.status = SafePointStatus::InUse;
                // Note: In a full implementation, we'd update the stored snapshot
            }
        }

        // Perform rollback using the strategy
        // Execute rollback using the strategy (simplified for now)
        if !self.rollback_strategy.can_rollback("safe_point_restore", context) {
            return Err(OpError::ExecutionFailed(
                "Rollback strategy indicates rollback not possible".to_string()
            ));
        }

        // Restore context from snapshot
        *context = snapshot.context_snapshot.clone();

        // Add restoration metadata to context
        context.insert("restored_from_safe_point".to_string(), 
                      serde_json::Value::String(self.safe_point_id.clone()))?;
        context.insert("restoration_timestamp".to_string(), 
                      serde_json::Value::String(chrono::Utc::now().to_rfc3339()))?;

        // Update safe point status to restored
        {
            let safe_points = self.safe_point_manager.safe_points.write().unwrap();
            if let Some(stored_snapshot) = safe_points.get(&self.safe_point_id).cloned() {
                let mut updated_snapshot = stored_snapshot;
                updated_snapshot.metadata.status = SafePointStatus::Restored;
                // Note: In a full implementation, we'd update the stored snapshot
            }
        }

        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::OpContext;
    use crate::op_state::OpStateInfo;
    use crate::rollback::NoOpRollbackStrategy;

    #[test]
    fn test_safe_point_metadata_creation() {
        let metadata = SafePointMetadata::new(
            "sp_001".to_string(),
            "Test safe point".to_string(),
            "op_123".to_string(),
        );

        assert_eq!(metadata.id, "sp_001");
        assert_eq!(metadata.description, "Test safe point");
        assert_eq!(metadata.op_instance_id, "op_123");
        assert_eq!(metadata.status, SafePointStatus::Active);
        assert!(!metadata.is_expired());
        assert!(metadata.is_active());
    }

    #[test]
    fn test_safe_point_metadata_expiration() {
        let past_time = chrono::Utc::now() - chrono::Duration::hours(1);
        let metadata = SafePointMetadata::new(
            "sp_002".to_string(),
            "Expired safe point".to_string(),
            "op_456".to_string(),
        ).with_expiration(past_time);

        assert!(metadata.is_expired());
        assert!(!metadata.is_active());
    }

    #[test]
    fn test_safe_point_snapshot_creation() {
        let metadata = SafePointMetadata::new(
            "sp_003".to_string(),
            "Snapshot test".to_string(),
            "op_789".to_string(),
        );
        
        let context = OpContext::new();
        let state = OpStateInfo::new("op_789".to_string(), "TestOp".to_string());
        
        let snapshot = SafePointSnapshot::new(metadata, &context, &state);
        
        assert!(snapshot.verify_integrity());
        assert!(snapshot.is_restorable());
        assert!(snapshot.checksum.is_some());
    }

    #[test]
    fn test_safe_point_snapshot_with_custom_data() {
        let metadata = SafePointMetadata::new(
            "sp_004".to_string(),
            "Custom data test".to_string(),
            "op_101112".to_string(),
        );
        
        let context = OpContext::new();
        let state = OpStateInfo::new("op_101112".to_string(), "CustomOp".to_string());
        
        let snapshot = SafePointSnapshot::new(metadata, &context, &state)
            .with_data("test_key".to_string(), serde_json::json!("test_value"));
        
        assert!(snapshot.verify_integrity());
        assert_eq!(snapshot.snapshot_data.get("test_key").unwrap(), &serde_json::json!("test_value"));
    }

    #[tokio::test]
    async fn test_safe_point_manager_creation() {
        let manager = SafePointManager::new();
        
        let context = OpContext::new();
        let state = OpStateInfo::new("op_131415".to_string(), "ManagerTestOp".to_string());
        let options = SafePointOptions::new("Manager test safe point".to_string());
        
        let safe_point_id = manager.create_safe_point("op_131415".to_string(), &context, &state, options).await.unwrap();
        
        assert!(safe_point_id.starts_with("sp_"));
        assert_eq!(manager.safe_point_count(), 1);
        assert_eq!(manager.active_safe_point_count(), 1);
        
        let retrieved = manager.get_safe_point(&safe_point_id).unwrap();
        assert_eq!(retrieved.metadata.id, safe_point_id);
        assert!(retrieved.is_restorable());
    }

    #[tokio::test]
    async fn test_safe_point_manager_find_ops() {
        let manager = SafePointManager::new();
        
        let context = OpContext::new();
        let state1 = OpStateInfo::new("op_161718".to_string(), "FindTest1".to_string());
        let state2 = OpStateInfo::new("op_192021".to_string(), "FindTest2".to_string());
        
        let options1 = SafePointOptions::new("Find test 1".to_string()).with_tag("test".to_string());
        let options2 = SafePointOptions::new("Find test 2".to_string()).with_tag("test".to_string());
        
        let sp1 = manager.create_safe_point("op_161718".to_string(), &context, &state1, options1).await.unwrap();
        let sp2 = manager.create_safe_point("op_192021".to_string(), &context, &state2, options2).await.unwrap();
        
        // Test find by op
        let sp1_found = manager.find_safe_points_by_op(&"op_161718".to_string());
        assert_eq!(sp1_found.len(), 1);
        assert_eq!(sp1_found[0].metadata.id, sp1);
        
        // Test find by tag
        let tagged = manager.find_safe_points_by_tag("test");
        assert_eq!(tagged.len(), 2);
    }

    #[tokio::test]
    async fn test_safe_point_manager_cleanup() {
        let manager = SafePointManager::new();
        
        let context = OpContext::new();
        let state = OpStateInfo::new("op_222324".to_string(), "CleanupTest".to_string());
        
        // Create safe point with very short expiration
        let options = SafePointOptions::new("Cleanup test".to_string())
            .expires_after(Duration::from_millis(1));
        
        let _sp_id = manager.create_safe_point("op_222324".to_string(), &context, &state, options).await.unwrap();
        
        assert_eq!(manager.safe_point_count(), 1);
        
        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        let cleaned = manager.cleanup_expired_safe_points().await.unwrap();
        assert_eq!(cleaned, 1);
        assert_eq!(manager.safe_point_count(), 0);
    }

    #[tokio::test]
    async fn test_safe_point_restore_op() {
        let manager = Arc::new(SafePointManager::new());
        let rollback_strategy = Arc::new(NoOpRollbackStrategy {
            strategy_id: "test_rollback".to_string(),
        });
        
        let mut context = OpContext::new();
        context.insert("original_value".to_string(), serde_json::json!("test")).unwrap();
        
        let state = OpStateInfo::new("op_252627".to_string(), "RestoreTest".to_string());
        let options = SafePointOptions::new("Restore test".to_string());
        
        let sp_id = manager.create_safe_point("op_252627".to_string(), &context, &state, options).await.unwrap();
        
        // Modify context
        context.insert("modified_value".to_string(), serde_json::json!("changed")).unwrap();
        
        // Create restore op
        let restore_op = SafePointRestoreOp::new(sp_id.clone(), manager, rollback_strategy);
        
        // Execute restore
        let restored_snapshot = restore_op.perform(&mut context).await.unwrap();
        
        assert_eq!(restored_snapshot.metadata.id, sp_id);
        assert!(context.contains_key("restored_from_safe_point"));
        assert_eq!(context.get::<serde_json::Value>("original_value").unwrap(), serde_json::json!("test"));
    }
}
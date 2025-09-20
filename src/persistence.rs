// State Persistence Layer
// Durable storage for op and infrastructure states

use crate::{OpContext, OpError, OpResult, state_machine::{State, StateId, TransitionId}, stateful::EntityMetadata};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tokio::fs;

/// Unique identifier for persisted state
pub type PersistenceId = String;

/// State snapshot with metadata for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// Unique identifier for this snapshot
    pub id: PersistenceId,
    /// Timestamp when snapshot was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Entity metadata being persisted
    pub entity_metadata: EntityMetadata,
    /// Current state in state machine
    pub current_state: StateId,
    /// Complete state machine definition
    pub state_machine_states: Vec<State>,
    /// Op context data
    pub context_data: HashMap<String, String>,
    /// Transition history leading to current state
    pub transition_history: Vec<TransitionRecord>,
}

/// Record of a state transition for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionRecord {
    /// Transition identifier
    pub transition_id: TransitionId,
    /// Source state
    pub from_state: StateId,
    /// Target state
    pub to_state: StateId,
    /// Transition timestamp
    pub occurred_at: chrono::DateTime<chrono::Utc>,
    /// Context at time of transition
    pub context_snapshot: HashMap<String, String>,
}

impl TransitionRecord {
    /// Create new transition record
    pub fn new(transition_id: TransitionId, from_state: StateId, to_state: StateId) -> Self {
        Self {
            transition_id,
            from_state,
            to_state,
            occurred_at: chrono::Utc::now(),
            context_snapshot: HashMap::new(),
        }
    }

    /// Add context data to transition record
    pub fn with_context(mut self, context: &OpContext) -> Self {
        // Extract serializable data from context
        for (key, value) in context.values().iter() {
            self.context_snapshot.insert(key.clone(), value.to_string());
        }
        self
    }
}

/// Trait for persistent state storage backends
#[async_trait]
pub trait StateStorage: Send + Sync {
    /// Save state snapshot to persistent storage
    async fn save_snapshot(&self, snapshot: &StateSnapshot) -> OpResult<()>;
    
    /// Load state snapshot by ID
    async fn load_snapshot(&self, id: &PersistenceId) -> Result<StateSnapshot, OpError>;
    
    /// List all available snapshots for an entity
    async fn list_snapshots(&self, entity_id: &str) -> Result<Vec<PersistenceId>, OpError>;
    
    /// Delete a state snapshot
    async fn delete_snapshot(&self, id: &PersistenceId) -> OpResult<()>;
    
    /// Find latest snapshot for an entity
    async fn find_latest_snapshot(&self, entity_id: &str) -> Result<Option<StateSnapshot>, OpError>;
}

/// File system based state storage implementation
pub struct FileSystemStateStorage {
    /// Base directory for storing state files
    base_path: PathBuf,
    /// In-memory index of snapshots for fast lookup
    index: Arc<RwLock<HashMap<PersistenceId, PathBuf>>>,
}

impl FileSystemStateStorage {
    /// Create new file system state storage
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize storage directory and index
    pub async fn initialize(&self) -> OpResult<()> {
        // Create base directory if it doesn't exist
        fs::create_dir_all(&self.base_path)
            .await
            .map_err(|e| OpError::ExecutionFailed(
                format!("Failed to create storage directory: {}", e)
            ))?;

        // Build index by scanning existing files
        self.rebuild_index().await?;
        
        Ok(())
    }

    /// Rebuild the in-memory index from existing files
    async fn rebuild_index(&self) -> OpResult<()> {
        let mut index = self.index.write().unwrap();
        index.clear();

        let mut entries = fs::read_dir(&self.base_path)
            .await
            .map_err(|e| OpError::ExecutionFailed(
                format!("Failed to read storage directory: {}", e)
            ))?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            OpError::ExecutionFailed(format!("Failed to read directory entry: {}", e))
        })? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    index.insert(stem.to_string(), path);
                }
            }
        }

        Ok(())
    }

    /// Get file path for a snapshot ID
    fn snapshot_path(&self, id: &PersistenceId) -> PathBuf {
        self.base_path.join(format!("{}.json", id))
    }
}

#[async_trait]
impl StateStorage for FileSystemStateStorage {
    async fn save_snapshot(&self, snapshot: &StateSnapshot) -> OpResult<()> {
        let path = self.snapshot_path(&snapshot.id);
        
        // Serialize snapshot to JSON
        let json_data = serde_json::to_string_pretty(snapshot)
            .map_err(|e| OpError::ExecutionFailed(
                format!("Failed to serialize snapshot: {}", e)
            ))?;

        // Write to file atomically
        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, json_data)
            .await
            .map_err(|e| OpError::ExecutionFailed(
                format!("Failed to write snapshot file: {}", e)
            ))?;

        // Atomic rename to final location
        fs::rename(&temp_path, &path)
            .await
            .map_err(|e| OpError::ExecutionFailed(
                format!("Failed to rename snapshot file: {}", e)
            ))?;

        // Update index
        let mut index = self.index.write().unwrap();
        index.insert(snapshot.id.clone(), path);

        Ok(())
    }

    async fn load_snapshot(&self, id: &PersistenceId) -> Result<StateSnapshot, OpError> {
        let path = self.snapshot_path(id);
        
        // Read file contents
        let json_data = fs::read_to_string(&path)
            .await
            .map_err(|e| OpError::ExecutionFailed(
                format!("Failed to read snapshot file {}: {}", path.display(), e)
            ))?;

        // Deserialize from JSON
        let snapshot = serde_json::from_str(&json_data)
            .map_err(|e| OpError::ExecutionFailed(
                format!("Failed to deserialize snapshot: {}", e)
            ))?;

        Ok(snapshot)
    }

    async fn list_snapshots(&self, entity_id: &str) -> Result<Vec<PersistenceId>, OpError> {
        let snapshot_ids: Vec<String> = {
            let index = self.index.read().unwrap();
            index.keys().cloned().collect()
        };
        
        let mut matching_snapshots = Vec::new();

        // Filter snapshots by entity ID (requires loading each to check entity_id)
        for snapshot_id in snapshot_ids {
            // Try to load snapshot to check entity ID
            if let Ok(snapshot) = self.load_snapshot(&snapshot_id).await {
                if snapshot.entity_metadata.id == entity_id {
                    matching_snapshots.push(snapshot_id);
                }
            }
        }

        // Sort by creation time (newest first)
        matching_snapshots.sort_by(|a, b| b.cmp(a));
        
        Ok(matching_snapshots)
    }

    async fn delete_snapshot(&self, id: &PersistenceId) -> OpResult<()> {
        let path = self.snapshot_path(id);
        
        // Remove file
        fs::remove_file(&path)
            .await
            .map_err(|e| OpError::ExecutionFailed(
                format!("Failed to delete snapshot file: {}", e)
            ))?;

        // Update index
        let mut index = self.index.write().unwrap();
        index.remove(id);

        Ok(())
    }

    async fn find_latest_snapshot(&self, entity_id: &str) -> Result<Option<StateSnapshot>, OpError> {
        let snapshots = self.list_snapshots(entity_id).await?;
        
        if let Some(latest_id) = snapshots.first() {
            Ok(Some(self.load_snapshot(latest_id).await?))
        } else {
            Ok(None)
        }
    }
}

/// State persistence manager - high-level interface for state ops
#[derive(Clone)]
pub struct StatePersistenceManager {
    /// Underlying storage backend
    storage: Arc<dyn StateStorage>,
    /// Default storage path for new snapshots
    default_entity_type: String,
}

impl StatePersistenceManager {
    /// Create new persistence manager with storage backend
    pub fn new(storage: Arc<dyn StateStorage>) -> Self {
        Self {
            storage,
            default_entity_type: "entity".to_string(),
        }
    }

    /// Set default entity type for new snapshots
    pub fn with_default_entity_type(mut self, entity_type: String) -> Self {
        self.default_entity_type = entity_type;
        self
    }

    /// Create snapshot from current op state
    pub async fn create_snapshot(
        &self,
        entity_metadata: EntityMetadata,
        current_state: StateId,
        state_machine_states: Vec<State>,
        context: &OpContext,
        transition_history: Vec<TransitionRecord>,
    ) -> Result<StateSnapshot, OpError> {
        // Generate unique ID for snapshot
        let snapshot_id = format!("{}_{}", 
            entity_metadata.id, 
            chrono::Utc::now().timestamp_millis()
        );

        // Extract context data for persistence
        let mut context_data = HashMap::new();
        for (key, value) in context.values().iter() {
            context_data.insert(key.clone(), value.to_string());
        }

        let snapshot = StateSnapshot {
            id: snapshot_id,
            created_at: chrono::Utc::now(),
            entity_metadata,
            current_state,
            state_machine_states,
            context_data,
            transition_history,
        };

        Ok(snapshot)
    }

    /// Persist a state snapshot
    pub async fn persist_snapshot(&self, snapshot: &StateSnapshot) -> OpResult<()> {
        self.storage.save_snapshot(snapshot).await
    }

    /// Restore state from latest snapshot
    pub async fn restore_latest_state(&self, entity_id: &str) -> Result<Option<StateSnapshot>, OpError> {
        self.storage.find_latest_snapshot(entity_id).await
    }

    /// Create and persist snapshot in one op
    pub async fn checkpoint_state(
        &self,
        entity_metadata: EntityMetadata,
        current_state: StateId,
        state_machine_states: Vec<State>,
        context: &OpContext,
        transition_history: Vec<TransitionRecord>,
    ) -> Result<PersistenceId, OpError> {
        let snapshot = self.create_snapshot(
            entity_metadata, 
            current_state, 
            state_machine_states, 
            context, 
            transition_history
        ).await?;

        let snapshot_id = snapshot.id.clone();
        self.persist_snapshot(&snapshot).await?;
        
        Ok(snapshot_id)
    }

    /// List all snapshots for an entity (newest first)
    pub async fn list_entity_snapshots(&self, entity_id: &str) -> Result<Vec<PersistenceId>, OpError> {
        self.storage.list_snapshots(entity_id).await
    }

    /// Clean up old snapshots, keeping only the latest N
    pub async fn cleanup_old_snapshots(&self, entity_id: &str, keep_count: usize) -> Result<usize, OpError> {
        let snapshots = self.list_entity_snapshots(entity_id).await?;
        
        if snapshots.len() <= keep_count {
            return Ok(0);
        }

        let to_delete = &snapshots[keep_count..];
        let mut deleted_count = 0;

        for snapshot_id in to_delete {
            if let Err(e) = self.storage.delete_snapshot(snapshot_id).await {
                tracing::warn!("Failed to delete snapshot {}: {}", snapshot_id, e);
            } else {
                deleted_count += 1;
            }
        }

        Ok(deleted_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::OpContext;
    use tempfile::TempDir;
    use tokio;

    #[tokio::test]
    async fn test_filesystem_storage_basic_ops() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileSystemStateStorage::new(temp_dir.path().to_path_buf());
        storage.initialize().await.unwrap();

        // Create test snapshot
        let entity_metadata = EntityMetadata::new("test-entity".to_string(), "test-type".to_string());
        let states = vec![
            State::new("initial".to_string(), "Initial State".to_string()).initial(),
            State::new("processing".to_string(), "Processing State".to_string()),
            State::new("complete".to_string(), "Complete State".to_string()).final_state(),
        ];

        let snapshot = StateSnapshot {
            id: "test-snapshot-1".to_string(),
            created_at: chrono::Utc::now(),
            entity_metadata: entity_metadata.clone(),
            current_state: "processing".to_string(),
            state_machine_states: states,
            context_data: HashMap::new(),
            transition_history: vec![],
        };

        // Save snapshot
        storage.save_snapshot(&snapshot).await.unwrap();

        // Load snapshot
        let loaded_snapshot = storage.load_snapshot(&snapshot.id).await.unwrap();
        assert_eq!(loaded_snapshot.id, snapshot.id);
        assert_eq!(loaded_snapshot.entity_metadata.id, entity_metadata.id);
        assert_eq!(loaded_snapshot.current_state, "processing");

        // List snapshots
        let snapshots = storage.list_snapshots(&entity_metadata.id).await.unwrap();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0], "test-snapshot-1");

        // Find latest snapshot
        let latest = storage.find_latest_snapshot(&entity_metadata.id).await.unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().id, "test-snapshot-1");

        // Delete snapshot
        storage.delete_snapshot(&snapshot.id).await.unwrap();
        let snapshots_after_delete = storage.list_snapshots(&entity_metadata.id).await.unwrap();
        assert_eq!(snapshots_after_delete.len(), 0);
    }

    #[tokio::test]
    async fn test_persistence_manager_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileSystemStateStorage::new(temp_dir.path().to_path_buf());
        storage.initialize().await.unwrap();
        let storage_arc: Arc<dyn StateStorage> = Arc::new(storage);
        
        let manager = StatePersistenceManager::new(storage_arc)
            .with_default_entity_type("test-entity".to_string());

        // Create entity and states
        let entity_metadata = EntityMetadata::new("workflow-1".to_string(), "deployment".to_string())
            .with_property("version".to_string(), "1.0.0".to_string());

        let states = vec![
            State::new("init".to_string(), "Initialize".to_string()).initial(),
            State::new("deploy".to_string(), "Deploy".to_string()),
            State::new("verify".to_string(), "Verify".to_string()),
            State::new("complete".to_string(), "Complete".to_string()).final_state(),
        ];

        let context = OpContext::new();
        let transitions = vec![
            TransitionRecord::new("start".to_string(), "init".to_string(), "deploy".to_string()),
        ];

        // Create and persist checkpoint
        let snapshot_id = manager.checkpoint_state(
            entity_metadata.clone(),
            "deploy".to_string(),
            states.clone(),
            &context,
            transitions
        ).await.unwrap();

        assert!(snapshot_id.starts_with("workflow-1_"));

        // Restore latest state
        let restored = manager.restore_latest_state(&entity_metadata.id).await.unwrap();
        assert!(restored.is_some());
        
        let restored_snapshot = restored.unwrap();
        assert_eq!(restored_snapshot.entity_metadata.id, "workflow-1");
        assert_eq!(restored_snapshot.current_state, "deploy");
        assert_eq!(restored_snapshot.state_machine_states.len(), 4);
        assert_eq!(restored_snapshot.transition_history.len(), 1);
    }

    #[tokio::test]
    async fn test_snapshot_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileSystemStateStorage::new(temp_dir.path().to_path_buf());
        storage.initialize().await.unwrap();
        let storage_arc: Arc<dyn StateStorage> = Arc::new(storage);
        
        let manager = StatePersistenceManager::new(storage_arc);

        let entity_metadata = EntityMetadata::new("cleanup-test".to_string(), "test".to_string());
        let states = vec![State::new("state".to_string(), "State".to_string())];
        let context = OpContext::new();

        // Create multiple snapshots
        for _i in 0..5 {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await; // Ensure different timestamps
            manager.checkpoint_state(
                entity_metadata.clone(),
                "state".to_string(),
                states.clone(),
                &context,
                vec![]
            ).await.unwrap();
        }

        // Verify 5 snapshots created
        let snapshots = manager.list_entity_snapshots(&entity_metadata.id).await.unwrap();
        assert_eq!(snapshots.len(), 5);

        // Cleanup keeping only 2 most recent
        let deleted_count = manager.cleanup_old_snapshots(&entity_metadata.id, 2).await.unwrap();
        assert_eq!(deleted_count, 3);

        // Verify only 2 snapshots remain
        let remaining_snapshots = manager.list_entity_snapshots(&entity_metadata.id).await.unwrap();
        assert_eq!(remaining_snapshots.len(), 2);
    }

    #[tokio::test] 
    async fn test_transition_record_with_context() {
        let mut context = OpContext::new();
        context.insert("deployment_id".to_string(), serde_json::json!("deploy-123"));
        context.insert("version".to_string(), serde_json::json!("2.1.0"));

        let record = TransitionRecord::new(
            "deploy_transition".to_string(),
            "ready".to_string(), 
            "deploying".to_string()
        ).with_context(&context);

        assert_eq!(record.transition_id, "deploy_transition");
        assert_eq!(record.from_state, "ready");
        assert_eq!(record.to_state, "deploying");
        assert_eq!(record.context_snapshot.len(), 2);
        assert!(record.context_snapshot.contains_key("deployment_id"));
        assert!(record.context_snapshot.contains_key("version"));
    }
}
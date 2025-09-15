// Rollback & Recovery System
// Pluggable rollback strategies for different op types

use crate::{Op, OpContext, OpError, stateful::EntityMetadata};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Unique identifier for rollback ops
pub type RollbackId = String;

/// Rollback execution priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RollbackPriority {
    /// Low priority rollback (cleanup, optimization)
    Low = 1,
    /// Normal priority rollback (standard ops)
    Normal = 2,
    /// High priority rollback (critical dependencies)
    High = 3,
    /// Emergency rollback (system integrity at risk)
    Emergency = 4,
}

impl Default for RollbackPriority {
    fn default() -> Self {
        RollbackPriority::Normal
    }
}

/// Rollback op result status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RollbackStatus {
    /// Rollback not yet attempted
    Pending,
    /// Rollback is currently in progress
    InProgress,
    /// Rollback completed successfully
    Completed,
    /// Rollback failed (original op state may be inconsistent)
    Failed,
    /// Rollback was skipped (not needed or not possible)
    Skipped,
    /// Rollback partially completed (some parts succeeded, some failed)
    PartiallyCompleted,
}

impl RollbackStatus {
    /// Check if rollback is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            RollbackStatus::Completed
                | RollbackStatus::Failed
                | RollbackStatus::Skipped
                | RollbackStatus::PartiallyCompleted
        )
    }

    /// Check if rollback completed successfully
    pub fn is_success(&self) -> bool {
        matches!(self, RollbackStatus::Completed)
    }

    /// Check if rollback needs attention (failed or partial)
    pub fn needs_attention(&self) -> bool {
        matches!(
            self,
            RollbackStatus::Failed | RollbackStatus::PartiallyCompleted
        )
    }
}

/// Information about a rollback op
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackInfo {
    /// Unique rollback identifier
    pub rollback_id: RollbackId,
    /// Original op that is being rolled back
    pub original_op_id: String,
    /// Human-readable description of rollback
    pub description: String,
    /// Rollback execution priority
    pub priority: RollbackPriority,
    /// Current rollback status
    pub status: RollbackStatus,
    /// Rollback strategy identifier
    pub strategy_id: String,
    /// Context snapshot from when rollback was initiated
    pub context_snapshot: HashMap<String, String>,
    /// Entity metadata if this is a stateful rollback
    pub entity_metadata: Option<EntityMetadata>,
    /// Rollback creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Rollback start timestamp
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Rollback completion timestamp
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Error message if rollback failed
    pub error_message: Option<String>,
    /// Additional rollback metadata
    pub metadata: HashMap<String, String>,
}

impl RollbackInfo {
    /// Create new rollback info
    pub fn new(
        rollback_id: RollbackId,
        original_op_id: String,
        description: String,
        strategy_id: String,
    ) -> Self {
        Self {
            rollback_id,
            original_op_id,
            description,
            priority: RollbackPriority::default(),
            status: RollbackStatus::Pending,
            strategy_id,
            context_snapshot: HashMap::new(),
            entity_metadata: None,
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            error_message: None,
            metadata: HashMap::new(),
        }
    }

    /// Set rollback priority
    pub fn with_priority(mut self, priority: RollbackPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set entity metadata for stateful rollbacks
    pub fn with_entity_metadata(mut self, entity_metadata: EntityMetadata) -> Self {
        self.entity_metadata = Some(entity_metadata);
        self
    }

    /// Add context snapshot
    pub fn with_context_snapshot(mut self, context: &OpContext) -> Self {
        for (key, value) in context.values().iter() {
            self.context_snapshot.insert(key.clone(), value.to_string());
        }
        self
    }

    /// Add metadata entry
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Mark rollback as started
    pub fn mark_started(&mut self) {
        self.status = RollbackStatus::InProgress;
        self.started_at = Some(chrono::Utc::now());
    }

    /// Mark rollback as completed
    pub fn mark_completed(&mut self) {
        self.status = RollbackStatus::Completed;
        self.completed_at = Some(chrono::Utc::now());
    }

    /// Mark rollback as failed
    pub fn mark_failed(&mut self, error_message: String) {
        self.status = RollbackStatus::Failed;
        self.completed_at = Some(chrono::Utc::now());
        self.error_message = Some(error_message);
    }

    /// Mark rollback as skipped
    pub fn mark_skipped(&mut self, reason: String) {
        self.status = RollbackStatus::Skipped;
        self.completed_at = Some(chrono::Utc::now());
        self.add_metadata("skip_reason".to_string(), reason);
    }

    /// Mark rollback as partially completed
    pub fn mark_partially_completed(&mut self, details: String) {
        self.status = RollbackStatus::PartiallyCompleted;
        self.completed_at = Some(chrono::Utc::now());
        self.add_metadata("partial_details".to_string(), details);
    }

    /// Calculate rollback execution duration
    pub fn execution_duration(&self) -> Option<std::time::Duration> {
        if let (Some(started), Some(completed)) = (self.started_at, self.completed_at) {
            let duration = completed - started;
            Some(std::time::Duration::from_millis(
                duration.num_milliseconds().max(0) as u64
            ))
        } else {
            None
        }
    }
}

/// Trait for rollback strategies
#[async_trait]
pub trait RollbackStrategy: Send + Sync {
    /// Unique identifier for this strategy
    fn strategy_id(&self) -> &str;
    
    /// Human-readable strategy name
    fn strategy_name(&self) -> &str;
    
    /// Check if this strategy can handle rollback for the given op type
    fn can_rollback(&self, op_type: &str, context: &OpContext) -> bool;
    
    /// Create a compensating op for rollback
    async fn create_compensating_op(
        &self,
        original_context: &OpContext,
        rollback_info: &RollbackInfo,
    ) -> Result<Box<dyn Op<()>>, OpError>;
    
    /// Estimate rollback complexity/time (0.0 to 1.0 scale)
    fn estimate_complexity(&self, context: &OpContext) -> f64;
    
    /// Get strategy-specific metadata
    fn get_metadata(&self) -> HashMap<String, String>;
}

/// No-op rollback strategy (for ops that don't need rollback)
pub struct NoOpRollbackStrategy {
    pub strategy_id: String,
}

impl NoOpRollbackStrategy {
    /// Create new no-op rollback strategy
    pub fn new() -> Self {
        Self {
            strategy_id: "no-op".to_string(),
        }
    }
}

impl Default for NoOpRollbackStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RollbackStrategy for NoOpRollbackStrategy {
    fn strategy_id(&self) -> &str {
        &self.strategy_id
    }
    
    fn strategy_name(&self) -> &str {
        "No-Op Rollback"
    }
    
    fn can_rollback(&self, _op_type: &str, _context: &OpContext) -> bool {
        true // Can handle any op by doing nothing
    }
    
    async fn create_compensating_op(
        &self,
        _original_context: &OpContext,
        _rollback_info: &RollbackInfo,
    ) -> Result<Box<dyn Op<()>>, OpError> {
        // Create a no-op op
        use crate::ClosureOp;
        let noop = ClosureOp::new(|_ctx| Box::pin(async { Ok(()) }));
        Ok(Box::new(noop))
    }
    
    fn estimate_complexity(&self, _context: &OpContext) -> f64 {
        0.0 // No complexity for no-op
    }
    
    fn get_metadata(&self) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        metadata.insert("type".to_string(), "no-op".to_string());
        metadata.insert("description".to_string(), "Does not perform any rollback action".to_string());
        metadata
    }
}

/// Command-based rollback strategy (stores inverse commands)
pub struct CommandRollbackStrategy {
    strategy_id: String,
    command_mappings: HashMap<String, String>,
}

impl CommandRollbackStrategy {
    /// Create new command rollback strategy
    pub fn new() -> Self {
        let mut command_mappings = HashMap::new();
        
        // Common command rollback mappings
        command_mappings.insert("create".to_string(), "delete".to_string());
        command_mappings.insert("delete".to_string(), "create".to_string());
        command_mappings.insert("start".to_string(), "stop".to_string());
        command_mappings.insert("stop".to_string(), "start".to_string());
        command_mappings.insert("enable".to_string(), "disable".to_string());
        command_mappings.insert("disable".to_string(), "enable".to_string());
        command_mappings.insert("deploy".to_string(), "undeploy".to_string());
        command_mappings.insert("install".to_string(), "uninstall".to_string());
        
        Self {
            strategy_id: "command-based".to_string(),
            command_mappings,
        }
    }
    
    /// Add custom command mapping
    pub fn add_command_mapping(mut self, forward: String, reverse: String) -> Self {
        self.command_mappings.insert(forward, reverse);
        self
    }
    
    /// Get reverse command for a given command
    pub fn get_reverse_command(&self, command: &str) -> Option<&String> {
        self.command_mappings.get(command)
    }
}

impl Default for CommandRollbackStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RollbackStrategy for CommandRollbackStrategy {
    fn strategy_id(&self) -> &str {
        &self.strategy_id
    }
    
    fn strategy_name(&self) -> &str {
        "Command-Based Rollback"
    }
    
    fn can_rollback(&self, op_type: &str, context: &OpContext) -> bool {
        // Check if we have a reverse command mapping
        if let Some(command) = context.get::<String>("command") {
            self.command_mappings.contains_key(&command)
        } else {
            // Check if op type has a known reverse
            self.command_mappings.contains_key(op_type)
        }
    }
    
    async fn create_compensating_op(
        &self,
        original_context: &OpContext,
        rollback_info: &RollbackInfo,
    ) -> Result<Box<dyn Op<()>>, OpError> {
        // Extract command from original context
        let command = original_context
            .get::<String>("command")
            .ok_or_else(|| OpError::ExecutionFailed(
                "No command found in original context".to_string()
            ))?;
            
        // Get reverse command
        let reverse_command = self.get_reverse_command(&command)
            .ok_or_else(|| OpError::ExecutionFailed(
                format!("No reverse command found for: {}", command)
            ))?;
        
        // Create compensating op
        use crate::ClosureOp;
        let reverse_cmd = reverse_command.clone();
        let rollback_id = rollback_info.rollback_id.clone();
        
        let compensating_op = ClosureOp::new(move |ctx| {
            let cmd = reverse_cmd.clone();
            let rb_id = rollback_id.clone();
            
            // Log rollback execution
            tracing::info!("Executing rollback {} with reverse command: {}", rb_id, cmd);
            
            // In a real implementation, this would execute the reverse command
            // For now, we'll just simulate success
            let _ = ctx.put("executed_command", cmd.clone());
            let _ = ctx.put("rollback_id", rb_id.clone());
            
            Box::pin(async move { Ok(()) })
        });
        
        Ok(Box::new(compensating_op))
    }
    
    fn estimate_complexity(&self, context: &OpContext) -> f64 {
        // Complexity depends on the command type
        if let Some(command) = context.get::<String>("command") {
            match command.as_str() {
                "create" | "delete" => 0.7, // High complexity
                "start" | "stop" => 0.3,    // Low complexity
                "enable" | "disable" => 0.2, // Very low complexity
                _ => 0.5, // Medium complexity
            }
        } else {
            0.5
        }
    }
    
    fn get_metadata(&self) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        metadata.insert("type".to_string(), "command-based".to_string());
        metadata.insert("description".to_string(), "Uses inverse commands for rollback".to_string());
        metadata.insert("supported_commands".to_string(), 
                        self.command_mappings.keys().cloned().collect::<Vec<_>>().join(", "));
        metadata
    }
}

/// Snapshot-based rollback strategy (restores previous state)
pub struct SnapshotRollbackStrategy {
    strategy_id: String,
}

impl SnapshotRollbackStrategy {
    /// Create new snapshot rollback strategy
    pub fn new() -> Self {
        Self {
            strategy_id: "snapshot-based".to_string(),
        }
    }
}

impl Default for SnapshotRollbackStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RollbackStrategy for SnapshotRollbackStrategy {
    fn strategy_id(&self) -> &str {
        &self.strategy_id
    }
    
    fn strategy_name(&self) -> &str {
        "Snapshot-Based Rollback"
    }
    
    fn can_rollback(&self, _op_type: &str, context: &OpContext) -> bool {
        // Check if snapshot data is available
        context.contains_key("previous_state_snapshot") || 
        context.contains_key("backup_location")
    }
    
    async fn create_compensating_op(
        &self,
        original_context: &OpContext,
        rollback_info: &RollbackInfo,
    ) -> Result<Box<dyn Op<()>>, OpError> {
        // Extract snapshot information
        let snapshot = original_context
            .get::<String>("previous_state_snapshot")
            .or_else(|| original_context.get::<String>("backup_location"))
            .ok_or_else(|| OpError::ExecutionFailed(
                "No snapshot or backup location found".to_string()
            ))?;
        
        // Create snapshot restoration op
        use crate::ClosureOp;
        let snapshot_data = snapshot.clone();
        let rollback_id = rollback_info.rollback_id.clone();
        
        let restore_op = ClosureOp::new(move |ctx| {
            let snapshot = snapshot_data.clone();
            let rb_id = rollback_id.clone();
            
            // Log snapshot restoration
            tracing::info!("Executing rollback {} by restoring snapshot: {}", rb_id, snapshot);
            
            // In a real implementation, this would restore from the snapshot
            // For now, we'll simulate the restoration
            let _ = ctx.put("restored_snapshot", snapshot.clone());
            let _ = ctx.put("rollback_id", rb_id.clone());
            
            Box::pin(async move { Ok(()) })
        });
        
        Ok(Box::new(restore_op))
    }
    
    fn estimate_complexity(&self, context: &OpContext) -> f64 {
        // Complexity depends on snapshot size and type
        if let Some(size_str) = context.get::<String>("snapshot_size") {
            // Parse size and estimate complexity
            let size: u64 = size_str.parse().unwrap_or(1000);
            match size {
                0..=100 => 0.2,      // Small snapshots
                101..=10000 => 0.5,   // Medium snapshots
                _ => 0.8,             // Large snapshots
            }
        } else {
            0.4 // Default medium complexity
        }
    }
    
    fn get_metadata(&self) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        metadata.insert("type".to_string(), "snapshot-based".to_string());
        metadata.insert("description".to_string(), "Restores previous state from snapshots".to_string());
        metadata.insert("requirements".to_string(), "previous_state_snapshot or backup_location in context".to_string());
        metadata
    }
}

/// Rollback strategy registry for managing multiple strategies
pub struct RollbackStrategyRegistry {
    strategies: HashMap<String, Arc<dyn RollbackStrategy>>,
    default_strategy_id: String,
}

impl RollbackStrategyRegistry {
    /// Create new rollback strategy registry
    pub fn new() -> Self {
        let mut registry = Self {
            strategies: HashMap::new(),
            default_strategy_id: "no-op".to_string(),
        };
        
        // Register default strategies
        registry.register_strategy(Arc::new(NoOpRollbackStrategy::new()));
        registry.register_strategy(Arc::new(CommandRollbackStrategy::new()));
        registry.register_strategy(Arc::new(SnapshotRollbackStrategy::new()));
        
        registry
    }
    
    /// Register a rollback strategy
    pub fn register_strategy(&mut self, strategy: Arc<dyn RollbackStrategy>) {
        self.strategies.insert(strategy.strategy_id().to_string(), strategy);
    }
    
    /// Unregister a rollback strategy
    pub fn unregister_strategy(&mut self, strategy_id: &str) {
        self.strategies.remove(strategy_id);
    }
    
    /// Set default strategy
    pub fn set_default_strategy(&mut self, strategy_id: String) -> Result<(), OpError> {
        if self.strategies.contains_key(&strategy_id) {
            self.default_strategy_id = strategy_id;
            Ok(())
        } else {
            Err(OpError::ExecutionFailed(
                format!("Strategy not found: {}", strategy_id)
            ))
        }
    }
    
    /// Get strategy by ID
    pub fn get_strategy(&self, strategy_id: &str) -> Option<Arc<dyn RollbackStrategy>> {
        self.strategies.get(strategy_id).cloned()
    }
    
    /// Get default strategy
    pub fn get_default_strategy(&self) -> Arc<dyn RollbackStrategy> {
        self.strategies.get(&self.default_strategy_id).unwrap().clone()
    }
    
    /// Find best strategy for op
    pub fn find_best_strategy(
        &self,
        op_type: &str,
        context: &OpContext,
    ) -> Arc<dyn RollbackStrategy> {
        // Find all strategies that can handle this op
        let candidates: Vec<_> = self.strategies
            .values()
            .filter(|strategy| strategy.can_rollback(op_type, context))
            .collect();
        
        if candidates.is_empty() {
            return self.get_default_strategy();
        }
        
        // Separate no-op from other strategies
        let specific_strategies: Vec<_> = candidates.iter()
            .filter(|strategy| strategy.strategy_id() != "no-op")
            .collect();
        
        // If we have specific strategies, prefer them over no-op
        if !specific_strategies.is_empty() {
            // Sort specific strategies by complexity (prefer simpler ones)
            let mut specific_candidates = specific_strategies;
            specific_candidates.sort_by(|a, b| {
                let complexity_a = a.estimate_complexity(context);
                let complexity_b = b.estimate_complexity(context);
                complexity_a.partial_cmp(&complexity_b).unwrap_or(std::cmp::Ordering::Equal)
            });
            
            Arc::clone(specific_candidates.first().unwrap())
        } else {
            // Fall back to no-op if no specific strategies available
            self.get_default_strategy()
        }
    }
    
    /// List all registered strategies
    pub fn list_strategies(&self) -> Vec<(String, String)> {
        self.strategies
            .values()
            .map(|strategy| (strategy.strategy_id().to_string(), strategy.strategy_name().to_string()))
            .collect()
    }
    
    /// Get strategy count
    pub fn strategy_count(&self) -> usize {
        self.strategies.len()
    }
}

impl Default for RollbackStrategyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global rollback strategy registry
static GLOBAL_REGISTRY: std::sync::LazyLock<std::sync::RwLock<RollbackStrategyRegistry>> = 
    std::sync::LazyLock::new(|| std::sync::RwLock::new(RollbackStrategyRegistry::new()));

/// Get reference to global rollback strategy registry
pub fn global_rollback_registry() -> &'static std::sync::RwLock<RollbackStrategyRegistry> {
    &GLOBAL_REGISTRY
}

/// Convenience function to register a strategy globally
pub fn register_global_strategy(strategy: Arc<dyn RollbackStrategy>) {
    let mut registry = global_rollback_registry().write().unwrap();
    registry.register_strategy(strategy);
}

/// Convenience function to find the best strategy globally
pub fn find_global_strategy(
    op_type: &str,
    context: &OpContext,
) -> Arc<dyn RollbackStrategy> {
    let registry = global_rollback_registry().read().unwrap();
    registry.find_best_strategy(op_type, context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OpContext;

    #[test]
    fn test_rollback_priority_ordering() {
        assert!(RollbackPriority::Emergency > RollbackPriority::High);
        assert!(RollbackPriority::High > RollbackPriority::Normal);
        assert!(RollbackPriority::Normal > RollbackPriority::Low);
    }

    #[test]
    fn test_rollback_status_checks() {
        assert!(!RollbackStatus::Pending.is_terminal());
        assert!(!RollbackStatus::InProgress.is_terminal());
        assert!(RollbackStatus::Completed.is_terminal());
        assert!(RollbackStatus::Failed.is_terminal());

        assert!(RollbackStatus::Completed.is_success());
        assert!(!RollbackStatus::Failed.is_success());

        assert!(RollbackStatus::Failed.needs_attention());
        assert!(RollbackStatus::PartiallyCompleted.needs_attention());
        assert!(!RollbackStatus::Completed.needs_attention());
    }

    #[test]
    fn test_rollback_info_lifecycle() {
        let mut rollback_info = RollbackInfo::new(
            "rb_001".to_string(),
            "op_123".to_string(),
            "Rollback test op".to_string(),
            "command-based".to_string(),
        ).with_priority(RollbackPriority::High);

        assert_eq!(rollback_info.rollback_id, "rb_001");
        assert_eq!(rollback_info.priority, RollbackPriority::High);
        assert_eq!(rollback_info.status, RollbackStatus::Pending);

        rollback_info.mark_started();
        assert_eq!(rollback_info.status, RollbackStatus::InProgress);
        assert!(rollback_info.started_at.is_some());

        rollback_info.mark_completed();
        assert_eq!(rollback_info.status, RollbackStatus::Completed);
        assert!(rollback_info.completed_at.is_some());

        let duration = rollback_info.execution_duration();
        assert!(duration.is_some());
        assert!(duration.is_some());
    }

    #[test]
    fn test_noop_rollback_strategy() {
        let strategy = NoOpRollbackStrategy::new();
        let context = OpContext::new();

        assert_eq!(strategy.strategy_id(), "no-op");
        assert!(strategy.can_rollback("any_op", &context));
        assert_eq!(strategy.estimate_complexity(&context), 0.0);

        let metadata = strategy.get_metadata();
        assert_eq!(metadata.get("type"), Some(&"no-op".to_string()));
    }

    #[tokio::test]
    async fn test_noop_compensating_op() {
        let strategy = NoOpRollbackStrategy::new();
        let context = OpContext::new();
        let rollback_info = RollbackInfo::new(
            "rb_001".to_string(),
            "op_123".to_string(),
            "Test rollback".to_string(),
            "no-op".to_string(),
        );

        let compensating_op = strategy
            .create_compensating_op(&context, &rollback_info)
            .await
            .unwrap();

        // Execute the compensating op
        let mut exec_context = OpContext::new();
        let result = compensating_op.perform(&mut exec_context).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_command_rollback_strategy() {
        let strategy = CommandRollbackStrategy::new()
            .add_command_mapping("upload".to_string(), "download".to_string());
        
        let mut context = OpContext::new();
        context.put("command", "create").unwrap();

        assert!(strategy.can_rollback("create_op", &context));
        assert_eq!(strategy.get_reverse_command("create"), Some(&"delete".to_string()));
        assert_eq!(strategy.get_reverse_command("upload"), Some(&"download".to_string()));
        
        let complexity = strategy.estimate_complexity(&context);
        assert_eq!(complexity, 0.7); // Create ops are high complexity
    }

    #[tokio::test]
    async fn test_command_compensating_op() {
        let strategy = CommandRollbackStrategy::new();
        let mut context = OpContext::new();
        context.put("command", "create").unwrap();
        
        let rollback_info = RollbackInfo::new(
            "rb_002".to_string(),
            "op_456".to_string(),
            "Command rollback test".to_string(),
            "command-based".to_string(),
        );

        let compensating_op = strategy
            .create_compensating_op(&context, &rollback_info)
            .await
            .unwrap();

        // Execute the compensating op
        let mut exec_context = OpContext::new();
        let result = compensating_op.perform(&mut exec_context).await;
        assert!(result.is_ok());

        // Check that reverse command was executed
        let executed_command: Option<String> = exec_context.get("executed_command");
        assert_eq!(executed_command, Some("delete".to_string()));
    }

    #[test]
    fn test_snapshot_rollback_strategy() {
        let strategy = SnapshotRollbackStrategy::new();
        
        let mut context_with_snapshot = OpContext::new();
        context_with_snapshot.put("previous_state_snapshot", "snapshot_123").unwrap();
        
        let context_without_snapshot = OpContext::new();

        assert!(strategy.can_rollback("any_op", &context_with_snapshot));
        assert!(!strategy.can_rollback("any_op", &context_without_snapshot));
        
        // Test complexity estimation
        let mut context_large = OpContext::new();
        context_large.put("snapshot_size", "50000").unwrap();
        assert_eq!(strategy.estimate_complexity(&context_large), 0.8);
        
        let mut context_small = OpContext::new();
        context_small.put("snapshot_size", "50").unwrap();
        assert_eq!(strategy.estimate_complexity(&context_small), 0.2);
    }

    #[tokio::test]
    async fn test_snapshot_compensating_op() {
        let strategy = SnapshotRollbackStrategy::new();
        let mut context = OpContext::new();
        context.put("previous_state_snapshot", "backup_xyz_2023").unwrap();
        
        let rollback_info = RollbackInfo::new(
            "rb_003".to_string(),
            "op_789".to_string(),
            "Snapshot rollback test".to_string(),
            "snapshot-based".to_string(),
        );

        let compensating_op = strategy
            .create_compensating_op(&context, &rollback_info)
            .await
            .unwrap();

        // Execute the compensating op
        let mut exec_context = OpContext::new();
        let result = compensating_op.perform(&mut exec_context).await;
        assert!(result.is_ok());

        // Check that snapshot was restored
        let restored_snapshot: Option<String> = exec_context.get("restored_snapshot");
        assert_eq!(restored_snapshot, Some("backup_xyz_2023".to_string()));
    }

    #[test]
    fn test_rollback_strategy_registry() {
        let registry = RollbackStrategyRegistry::new();
        
        // Check default strategies are registered
        assert_eq!(registry.strategy_count(), 3);
        assert!(registry.get_strategy("no-op").is_some());
        assert!(registry.get_strategy("command-based").is_some());
        assert!(registry.get_strategy("snapshot-based").is_some());
        
        // Test default strategy
        let default = registry.get_default_strategy();
        assert_eq!(default.strategy_id(), "no-op");
        
        // Test strategy listing
        let strategies = registry.list_strategies();
        assert_eq!(strategies.len(), 3);
        assert!(strategies.iter().any(|(id, _)| id == "no-op"));
    }

    #[test]
    fn test_find_best_strategy() {
        let registry = RollbackStrategyRegistry::new();
        
        // Test with command context - should prefer command strategy
        let mut command_context = OpContext::new();
        command_context.put("command", "create").unwrap();
        
        let best_strategy = registry.find_best_strategy("create_op", &command_context);
        assert_eq!(best_strategy.strategy_id(), "command-based");
        
        // Test with snapshot context - should prefer snapshot strategy
        let mut snapshot_context = OpContext::new();
        snapshot_context.put("previous_state_snapshot", "snap_123").unwrap();
        
        let best_strategy = registry.find_best_strategy("restore_op", &snapshot_context);
        assert_eq!(best_strategy.strategy_id(), "snapshot-based");
        
        // Test with empty context - should fallback to no-op
        let empty_context = OpContext::new();
        let best_strategy = registry.find_best_strategy("unknown_op", &empty_context);
        assert_eq!(best_strategy.strategy_id(), "no-op");
    }

    #[test]
    fn test_registry_management() {
        let mut registry = RollbackStrategyRegistry::new();
        let initial_count = registry.strategy_count();
        
        // Register custom strategy with different ID
        let mut custom_strategy = NoOpRollbackStrategy::new();
        custom_strategy.strategy_id = "custom-no-op".to_string();
        registry.register_strategy(Arc::new(custom_strategy));
        assert_eq!(registry.strategy_count(), initial_count + 1);
        
        // Unregister strategy
        registry.unregister_strategy("no-op");
        assert_eq!(registry.strategy_count(), initial_count);
        assert!(registry.get_strategy("no-op").is_none());
        
        // Set default strategy
        let result = registry.set_default_strategy("command-based".to_string());
        assert!(result.is_ok());
        
        let default = registry.get_default_strategy();
        assert_eq!(default.strategy_id(), "command-based");
    }

    #[test]
    fn test_global_registry() {
        // Test global registry access
        let registry = global_rollback_registry();
        let read_registry = registry.read().unwrap();
        assert!(read_registry.strategy_count() > 0);
        drop(read_registry);
        
        // Test global strategy registration
        let custom_strategy = Arc::new(NoOpRollbackStrategy::new());
        register_global_strategy(custom_strategy);
        
        // Test global strategy finding
        let context = OpContext::new();
        let strategy = find_global_strategy("test_op", &context);
        assert_eq!(strategy.strategy_id(), "no-op");
    }
}
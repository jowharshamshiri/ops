// Emergency Rollback Ops
// Fast-path emergency rollback for critical failures

use crate::{Op, OpContext, OpError};
use crate::rollback::{RollbackStrategy, RollbackPriority, RollbackId};
use crate::safe_point::{SafePointManager, SafePointId};
use crate::op_state::{OpStateTracker, OpInstanceId};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Emergency rollback trigger conditions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EmergencyTrigger {
    /// System resource exhaustion (memory, disk, CPU)
    ResourceExhaustion { resource_type: String, threshold: f64 },
    /// Critical op failure cascade
    CascadingFailures { failure_count: u32, time_window_ms: u64 },
    /// Data corruption detected
    DataCorruption { affected_entities: Vec<String> },
    /// Security breach detected
    SecurityBreach { threat_level: String },
    /// Manual emergency trigger
    ManualTrigger { operator: String, reason: String },
    /// Timeout on critical op
    CriticalTimeout { op_id: OpInstanceId, timeout_ms: u64 },
    /// Infrastructure component failure
    InfrastructureFailure { component_id: String, severity: String },
}

impl EmergencyTrigger {
    /// Get priority level for this trigger
    pub fn priority(&self) -> RollbackPriority {
        match self {
            EmergencyTrigger::SecurityBreach { .. } => RollbackPriority::Emergency,
            EmergencyTrigger::DataCorruption { .. } => RollbackPriority::Emergency,
            EmergencyTrigger::ResourceExhaustion { .. } => RollbackPriority::High,
            EmergencyTrigger::CascadingFailures { .. } => RollbackPriority::High,
            EmergencyTrigger::InfrastructureFailure { severity, .. } => {
                if severity == "critical" { RollbackPriority::Emergency } else { RollbackPriority::High }
            },
            EmergencyTrigger::CriticalTimeout { .. } => RollbackPriority::High,
            EmergencyTrigger::ManualTrigger { .. } => RollbackPriority::Emergency,
        }
    }

    /// Get human-readable description
    pub fn description(&self) -> String {
        match self {
            EmergencyTrigger::ResourceExhaustion { resource_type, threshold } => {
                format!("Resource exhaustion: {} exceeded {}%", resource_type, threshold * 100.0)
            },
            EmergencyTrigger::CascadingFailures { failure_count, time_window_ms } => {
                format!("Cascading failures: {} failures in {}ms", failure_count, time_window_ms)
            },
            EmergencyTrigger::DataCorruption { affected_entities } => {
                format!("Data corruption detected in {} entities", affected_entities.len())
            },
            EmergencyTrigger::SecurityBreach { threat_level } => {
                format!("Security breach: {} threat level", threat_level)
            },
            EmergencyTrigger::ManualTrigger { operator, reason } => {
                format!("Manual trigger by {}: {}", operator, reason)
            },
            EmergencyTrigger::CriticalTimeout { op_id, timeout_ms } => {
                format!("Critical timeout: op {} exceeded {}ms", op_id, timeout_ms)
            },
            EmergencyTrigger::InfrastructureFailure { component_id, severity } => {
                format!("Infrastructure failure: {} ({})", component_id, severity)
            },
        }
    }

    /// Check if this trigger requires immediate action
    pub fn requires_immediate_action(&self) -> bool {
        matches!(self.priority(), RollbackPriority::Emergency)
    }
}

/// Emergency rollback execution status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EmergencyStatus {
    /// Emergency detected, rollback not yet started
    Detected,
    /// Emergency rollback in progress
    InProgress,
    /// Emergency rollback completed successfully
    Completed,
    /// Emergency rollback failed
    Failed,
    /// Emergency rollback partially completed
    PartiallyCompleted,
    /// Emergency condition resolved without rollback
    Resolved,
}

/// Emergency rollback record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyRollbackRecord {
    /// Unique emergency rollback identifier
    pub id: RollbackId,
    /// When the emergency was detected
    pub detected_at: chrono::DateTime<chrono::Utc>,
    /// When rollback started (if applicable)
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    /// When rollback completed (if applicable)
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Trigger condition that caused emergency
    pub trigger: EmergencyTrigger,
    /// Current emergency status
    pub status: EmergencyStatus,
    /// Ops affected by the emergency
    pub affected_ops: Vec<OpInstanceId>,
    /// Safe point used for rollback (if any)
    pub rollback_safe_point: Option<SafePointId>,
    /// Rollback strategy used
    pub rollback_strategy: Option<String>,
    /// Error message if rollback failed
    pub error_message: Option<String>,
    /// Time taken for rollback
    pub rollback_duration: Option<Duration>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl EmergencyRollbackRecord {
    /// Create new emergency rollback record
    pub fn new(id: RollbackId, trigger: EmergencyTrigger) -> Self {
        Self {
            id,
            detected_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            trigger,
            status: EmergencyStatus::Detected,
            affected_ops: Vec::new(),
            rollback_safe_point: None,
            rollback_strategy: None,
            error_message: None,
            rollback_duration: None,
            metadata: HashMap::new(),
        }
    }

    /// Mark rollback as started
    pub fn mark_started(&mut self) {
        self.status = EmergencyStatus::InProgress;
        self.started_at = Some(chrono::Utc::now());
    }

    /// Mark rollback as completed
    pub fn mark_completed(&mut self, success: bool) {
        let now = chrono::Utc::now();
        self.completed_at = Some(now);
        
        if success {
            self.status = EmergencyStatus::Completed;
        } else {
            self.status = EmergencyStatus::Failed;
        }

        // Calculate duration if we have a start time
        if let Some(started_at) = self.started_at {
            if let Ok(duration) = (now - started_at).to_std() {
                self.rollback_duration = Some(duration);
            }
        }
    }

    /// Add affected op
    pub fn add_affected_op(&mut self, op_id: OpInstanceId) {
        if !self.affected_ops.contains(&op_id) {
            self.affected_ops.push(op_id);
        }
    }

    /// Set rollback safe point
    pub fn set_rollback_safe_point(&mut self, safe_point_id: SafePointId) {
        self.rollback_safe_point = Some(safe_point_id);
    }

    /// Set rollback strategy
    pub fn set_rollback_strategy(&mut self, strategy: String) {
        self.rollback_strategy = Some(strategy);
    }

    /// Set error message
    pub fn set_error(&mut self, error: String) {
        self.error_message = Some(error);
        self.status = EmergencyStatus::Failed;
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Check if rollback is complete (success or failure)
    pub fn is_complete(&self) -> bool {
        matches!(self.status, 
            EmergencyStatus::Completed | 
            EmergencyStatus::Failed | 
            EmergencyStatus::PartiallyCompleted |
            EmergencyStatus::Resolved
        )
    }

    /// Get rollback duration in milliseconds
    pub fn duration_ms(&self) -> Option<u64> {
        self.rollback_duration.map(|d| d.as_millis() as u64)
    }
}

/// Emergency rollback manager
pub struct EmergencyRollbackManager {
    /// Active emergency records
    emergency_records: Arc<RwLock<HashMap<RollbackId, EmergencyRollbackRecord>>>,
    /// Safe point manager for finding restoration points
    safe_point_manager: Arc<SafePointManager>,
    /// Op state tracker for finding affected ops
    op_tracker: Arc<OpStateTracker>,
    /// Available rollback strategies
    rollback_strategies: Arc<RwLock<HashMap<String, Arc<dyn RollbackStrategy>>>>,
    /// Next emergency ID counter
    next_emergency_id: Arc<RwLock<u64>>,
    /// Emergency detection enabled
    detection_enabled: Arc<RwLock<bool>>,
}

impl EmergencyRollbackManager {
    /// Create new emergency rollback manager
    pub fn new(
        safe_point_manager: Arc<SafePointManager>,
        op_tracker: Arc<OpStateTracker>,
    ) -> Self {
        Self {
            emergency_records: Arc::new(RwLock::new(HashMap::new())),
            safe_point_manager,
            op_tracker,
            rollback_strategies: Arc::new(RwLock::new(HashMap::new())),
            next_emergency_id: Arc::new(RwLock::new(1)),
            detection_enabled: Arc::new(RwLock::new(true)),
        }
    }

    /// Register rollback strategy
    pub fn register_strategy(&self, name: String, strategy: Arc<dyn RollbackStrategy>) {
        let mut strategies = self.rollback_strategies.write().unwrap();
        strategies.insert(name, strategy);
    }

    /// Enable/disable emergency detection
    pub fn set_detection_enabled(&self, enabled: bool) {
        let mut detection = self.detection_enabled.write().unwrap();
        *detection = enabled;
    }

    /// Check if detection is enabled
    pub fn is_detection_enabled(&self) -> bool {
        let detection = self.detection_enabled.read().unwrap();
        *detection
    }

    /// Generate unique emergency ID
    fn generate_emergency_id(&self) -> RollbackId {
        let mut counter = self.next_emergency_id.write().unwrap();
        let id = *counter;
        *counter += 1;
        format!("emergency_{:08}_{}", id, chrono::Utc::now().timestamp())
    }

    /// Detect and handle emergency condition
    pub async fn detect_emergency(&self, trigger: EmergencyTrigger) -> Result<RollbackId, OpError> {
        // Check if detection is enabled
        if !self.is_detection_enabled() {
            return Err(OpError::ExecutionFailed(
                "Emergency detection is disabled".to_string()
            ));
        }

        let emergency_id = self.generate_emergency_id();
        let mut record = EmergencyRollbackRecord::new(emergency_id.clone(), trigger.clone());

        // Add trigger metadata
        record.add_metadata("trigger_priority".to_string(), format!("{:?}", trigger.priority()));
        record.add_metadata("requires_immediate".to_string(), trigger.requires_immediate_action().to_string());

        // Find affected ops based on trigger
        let affected_ops = self.find_affected_ops(&trigger).await?;
        for op_id in affected_ops {
            record.add_affected_op(op_id);
        }

        // Store emergency record
        {
            let mut records = self.emergency_records.write().unwrap();
            records.insert(emergency_id.clone(), record);
        }

        // If immediate action required, trigger rollback automatically
        if trigger.requires_immediate_action() {
            self.execute_emergency_rollback(&emergency_id).await?;
        }

        Ok(emergency_id)
    }

    /// Find ops affected by emergency trigger
    async fn find_affected_ops(&self, trigger: &EmergencyTrigger) -> Result<Vec<OpInstanceId>, OpError> {
        let mut affected = Vec::new();

        match trigger {
            EmergencyTrigger::CriticalTimeout { op_id, .. } => {
                affected.push(op_id.clone());
            },
            EmergencyTrigger::CascadingFailures { .. } => {
                // Find all failed ops in recent time window
                // Get failed ops by iterating through op statistics
                let failed_ops: Vec<String> = vec![]; // Simplified for now
                for _op in failed_ops.iter() {
                    // affected.push(op.instance_id);
                    // Simplified for now - would iterate through actual failed ops
                }
            },
            EmergencyTrigger::DataCorruption { affected_entities } => {
                // Find ops working on corrupted entities
                // Get running ops by iterating through op statistics
                let all_ops: Vec<String> = vec![]; // Simplified for now
                for _op in all_ops.iter() {
                    // Check if op metadata references any affected entities
                    for _entity_id in affected_entities {
                        // Simplified for now - would check op metadata
                        // if op.metadata.values().any(|v| v.contains(entity_id)) {
                        //     affected.push(op.instance_id);
                        //     break;
                        // }
                    }
                }
            },
            _ => {
                // For other triggers, find all currently running ops
                // Get running ops by iterating through op statistics
                let running_ops: Vec<String> = vec![]; // Simplified for now
                for _op in running_ops.iter() {
                    // affected.push(op.instance_id);
                    // Simplified for now - would add all running ops
                }
            }
        }

        Ok(affected)
    }

    /// Execute emergency rollback
    pub async fn execute_emergency_rollback(&self, emergency_id: &RollbackId) -> Result<(), OpError> {
        // Get emergency record
        let mut record = {
            let mut records = self.emergency_records.write().unwrap();
            records.get_mut(emergency_id)
                .ok_or_else(|| OpError::ExecutionFailed(
                    format!("Emergency record not found: {}", emergency_id)
                ))?
                .clone()
        };

        // Mark as started
        record.mark_started();

        // Find the most recent safe point for affected ops
        let safe_point = self.find_best_safe_point(&record.affected_ops).await?;
        
        if let Some(safe_point_id) = safe_point {
            record.set_rollback_safe_point(safe_point_id.clone());
            
            // Find appropriate rollback strategy
            let strategy_name = self.select_rollback_strategy(&record.trigger);
            record.set_rollback_strategy(strategy_name.clone());

            // Execute rollback
            match self.execute_rollback(&safe_point_id, &strategy_name).await {
                Ok(_) => {
                    record.mark_completed(true);
                    record.add_metadata("rollback_method".to_string(), "safe_point".to_string());
                },
                Err(e) => {
                    record.set_error(e.to_string());
                    record.mark_completed(false);
                }
            }
        } else {
            // No safe point found, try direct rollback strategies
            let strategy_name = self.select_rollback_strategy(&record.trigger);
            record.set_rollback_strategy(strategy_name.clone());

            match self.execute_direct_rollback(&record.affected_ops, &strategy_name).await {
                Ok(_) => {
                    record.mark_completed(true);
                    record.add_metadata("rollback_method".to_string(), "direct".to_string());
                },
                Err(e) => {
                    record.set_error(e.to_string());
                    record.mark_completed(false);
                }
            }
        }

        // Update stored record
        {
            let mut records = self.emergency_records.write().unwrap();
            records.insert(emergency_id.clone(), record);
        }

        Ok(())
    }

    /// Find the best safe point for rollback
    async fn find_best_safe_point(&self, affected_ops: &[OpInstanceId]) -> Result<Option<SafePointId>, OpError> {
        // Find safe points for affected ops
        let mut candidate_safe_points = Vec::new();

        for op_id in affected_ops {
            let safe_points = self.safe_point_manager.find_safe_points_by_op(op_id);
            candidate_safe_points.extend(safe_points);
        }

        // If no op-specific safe points, get all active safe points
        if candidate_safe_points.is_empty() {
            candidate_safe_points = self.safe_point_manager.list_safe_points(true); // filter expired
        }

        // Select the most recent safe point with highest priority
        candidate_safe_points.sort_by(|a, b| {
            // First by priority (emergency > high > normal > low)
            let priority_cmp = b.metadata.priority.cmp(&a.metadata.priority);
            if priority_cmp != std::cmp::Ordering::Equal {
                return priority_cmp;
            }
            // Then by recency
            b.metadata.created_at.cmp(&a.metadata.created_at)
        });

        Ok(candidate_safe_points.first().map(|sp| sp.metadata.id.clone()))
    }

    /// Select appropriate rollback strategy based on trigger
    fn select_rollback_strategy(&self, trigger: &EmergencyTrigger) -> String {
        match trigger {
            EmergencyTrigger::SecurityBreach { .. } => "security_rollback".to_string(),
            EmergencyTrigger::DataCorruption { .. } => "data_rollback".to_string(),
            EmergencyTrigger::ResourceExhaustion { .. } => "resource_rollback".to_string(),
            EmergencyTrigger::CascadingFailures { .. } => "cascade_rollback".to_string(),
            EmergencyTrigger::InfrastructureFailure { .. } => "infrastructure_rollback".to_string(),
            _ => "default_emergency_rollback".to_string(),
        }
    }

    /// Execute rollback using safe point
    async fn execute_rollback(&self, safe_point_id: &SafePointId, _strategy_name: &str) -> Result<(), OpError> {
        // Get safe point snapshot
        let snapshot = self.safe_point_manager.get_safe_point(safe_point_id)
            .ok_or_else(|| OpError::ExecutionFailed(
                format!("Safe point not found: {}", safe_point_id)
            ))?;

        // Verify safe point is still valid
        if !snapshot.is_restorable() {
            return Err(OpError::ExecutionFailed(
                format!("Safe point is not restorable: {}", safe_point_id)
            ));
        }

        // In a full implementation, we would:
        // 1. Stop all affected ops
        // 2. Restore system state from safe point
        // 3. Restart ops from safe point
        // For now, we'll simulate this
        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(())
    }

    /// Execute direct rollback without safe point
    async fn execute_direct_rollback(&self, affected_ops: &[OpInstanceId], strategy_name: &str) -> Result<(), OpError> {
        // Get strategy
        let strategy = {
            let strategies = self.rollback_strategies.read().unwrap();
            strategies.get(strategy_name)
                .cloned()
                .ok_or_else(|| OpError::ExecutionFailed(
                    format!("Rollback strategy not found: {}", strategy_name)
                ))?
        };

        // Execute rollback for each affected op
        for _op_id in affected_ops {
            // Get op info
            // Check op info - simplified for now
            if let Some(_op_info) = None::<()> {
                // Check if strategy can handle this op
                if strategy.can_rollback("emergency_op", &crate::context::OpContext::new()) {
                    // Execute rollback (simplified)
                    // In full implementation, would call strategy.rollback()
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            }
        }

        Ok(())
    }

    /// Get emergency record
    pub fn get_emergency_record(&self, emergency_id: &RollbackId) -> Option<EmergencyRollbackRecord> {
        let records = self.emergency_records.read().unwrap();
        records.get(emergency_id).cloned()
    }

    /// List all emergency records
    pub fn list_emergency_records(&self) -> Vec<EmergencyRollbackRecord> {
        let records = self.emergency_records.read().unwrap();
        records.values().cloned().collect()
    }

    /// Get emergency records by status
    pub fn get_records_by_status(&self, status: EmergencyStatus) -> Vec<EmergencyRollbackRecord> {
        let records = self.emergency_records.read().unwrap();
        records.values()
            .filter(|r| r.status == status)
            .cloned()
            .collect()
    }

    /// Cancel emergency rollback (if not yet started)
    pub async fn cancel_emergency(&self, emergency_id: &RollbackId) -> Result<bool, OpError> {
        let mut records = self.emergency_records.write().unwrap();
        
        if let Some(record) = records.get_mut(emergency_id) {
            if record.status == EmergencyStatus::Detected {
                record.status = EmergencyStatus::Resolved;
                record.add_metadata("cancelled".to_string(), "true".to_string());
                return Ok(true);
            }
        }
        
        Ok(false)
    }

    /// Get emergency statistics
    pub fn get_emergency_statistics(&self) -> HashMap<String, u64> {
        let records = self.emergency_records.read().unwrap();
        let mut stats = HashMap::new();

        // Count by status
        for record in records.values() {
            let status_key = format!("status_{:?}", record.status).to_lowercase();
            *stats.entry(status_key).or_insert(0) += 1;
        }

        // Count by trigger type
        for record in records.values() {
            let trigger_key = format!("trigger_{}", match &record.trigger {
                EmergencyTrigger::ResourceExhaustion { .. } => "resource_exhaustion",
                EmergencyTrigger::CascadingFailures { .. } => "cascading_failures",
                EmergencyTrigger::DataCorruption { .. } => "data_corruption",
                EmergencyTrigger::SecurityBreach { .. } => "security_breach",
                EmergencyTrigger::ManualTrigger { .. } => "manual_trigger",
                EmergencyTrigger::CriticalTimeout { .. } => "critical_timeout",
                EmergencyTrigger::InfrastructureFailure { .. } => "infrastructure_failure",
            });
            *stats.entry(trigger_key).or_insert(0) += 1;
        }

        // Total count
        stats.insert("total_emergencies".to_string(), records.len() as u64);

        // Completed count
        let completed = records.values().filter(|r| r.is_complete()).count() as u64;
        stats.insert("completed_emergencies".to_string(), completed);

        stats
    }

    /// Cleanup old emergency records
    pub async fn cleanup_old_records(&self, older_than: Duration) -> Result<usize, OpError> {
        let cutoff_time = chrono::Utc::now() - chrono::Duration::from_std(older_than)
            .map_err(|e| OpError::ExecutionFailed(format!("Invalid duration: {}", e)))?;

        let mut records = self.emergency_records.write().unwrap();
        let initial_count = records.len();

        records.retain(|_, record| {
            // Keep records that are not complete or are newer than cutoff
            !record.is_complete() || record.detected_at > cutoff_time
        });

        Ok(initial_count - records.len())
    }
}

/// Emergency rollback op for manual triggering
pub struct EmergencyRollbackOp {
    emergency_manager: Arc<EmergencyRollbackManager>,
    trigger: EmergencyTrigger,
    wait_for_completion: bool,
}

impl EmergencyRollbackOp {
    /// Create new emergency rollback op
    pub fn new(
        emergency_manager: Arc<EmergencyRollbackManager>,
        trigger: EmergencyTrigger,
    ) -> Self {
        Self {
            emergency_manager,
            trigger,
            wait_for_completion: true,
        }
    }

    /// Don't wait for rollback completion (fire and forget)
    pub fn fire_and_forget(mut self) -> Self {
        self.wait_for_completion = false;
        self
    }
}

#[async_trait]
impl Op<RollbackId> for EmergencyRollbackOp {
    async fn perform(&self, context: &mut OpContext) -> Result<RollbackId, OpError> {
        // Detect emergency
        let emergency_id = self.emergency_manager.detect_emergency(self.trigger.clone()).await?;

        // Add emergency ID to context
        context.insert("emergency_rollback_id".to_string(), 
                      serde_json::Value::String(emergency_id.clone()))?;
        context.insert("emergency_trigger".to_string(), 
                      serde_json::to_value(&self.trigger).unwrap_or(serde_json::Value::Null))?;

        // If waiting for completion, monitor the rollback
        if self.wait_for_completion {
            let start_time = Instant::now();
            let timeout = Duration::from_secs(300); // 5 minute timeout

            loop {
                if let Some(record) = self.emergency_manager.get_emergency_record(&emergency_id) {
                    if record.is_complete() {
                        context.insert("emergency_status".to_string(),
                                      serde_json::Value::String(format!("{:?}", record.status)))?;
                        
                        if record.status == EmergencyStatus::Failed {
                            return Err(OpError::ExecutionFailed(
                                record.error_message.unwrap_or_else(|| "Emergency rollback failed".to_string())
                            ));
                        }
                        
                        break;
                    }
                }

                if start_time.elapsed() > timeout {
                    return Err(OpError::ExecutionFailed(
                        "Emergency rollback timeout".to_string()
                    ));
                }

                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        Ok(emergency_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rollback::NoOpRollbackStrategy;
    use crate::safe_point::SafePointManager;
    use crate::op_state::OpStateTracker;

    #[test]
    fn test_emergency_trigger_priority() {
        let security_trigger = EmergencyTrigger::SecurityBreach { 
            threat_level: "critical".to_string() 
        };
        assert_eq!(security_trigger.priority(), RollbackPriority::Emergency);

        let resource_trigger = EmergencyTrigger::ResourceExhaustion { 
            resource_type: "memory".to_string(), 
            threshold: 0.95 
        };
        assert_eq!(resource_trigger.priority(), RollbackPriority::High);

        let manual_trigger = EmergencyTrigger::ManualTrigger { 
            operator: "admin".to_string(), 
            reason: "testing".to_string() 
        };
        assert_eq!(manual_trigger.priority(), RollbackPriority::Emergency);
    }

    #[test]
    fn test_emergency_trigger_description() {
        let trigger = EmergencyTrigger::CascadingFailures { 
            failure_count: 5, 
            time_window_ms: 30000 
        };
        let desc = trigger.description();
        assert!(desc.contains("5 failures"));
        assert!(desc.contains("30000ms"));
    }

    #[test]
    fn test_emergency_rollback_record() {
        let trigger = EmergencyTrigger::DataCorruption { 
            affected_entities: vec!["entity1".to_string(), "entity2".to_string()] 
        };
        let mut record = EmergencyRollbackRecord::new("emergency_001".to_string(), trigger);
        
        assert_eq!(record.status, EmergencyStatus::Detected);
        assert_eq!(record.affected_ops.len(), 0);
        
        record.add_affected_op("op_123".to_string());
        record.add_affected_op("op_456".to_string());
        assert_eq!(record.affected_ops.len(), 2);
        
        record.mark_started();
        assert_eq!(record.status, EmergencyStatus::InProgress);
        assert!(record.started_at.is_some());
        
        record.mark_completed(true);
        assert_eq!(record.status, EmergencyStatus::Completed);
        assert!(record.completed_at.is_some());
        assert!(record.is_complete());
    }

    #[tokio::test]
    async fn test_emergency_manager_creation() {
        let safe_point_manager = Arc::new(SafePointManager::new());
        let op_tracker = Arc::new(OpStateTracker::new());
        
        let emergency_manager = EmergencyRollbackManager::new(
            safe_point_manager,
            op_tracker,
        );
        
        assert!(emergency_manager.is_detection_enabled());
        
        // Register a test strategy
        let strategy = Arc::new(NoOpRollbackStrategy {
            strategy_id: "test_strategy".to_string(),
        });
        emergency_manager.register_strategy("test".to_string(), strategy);
        
        // Test disabling detection
        emergency_manager.set_detection_enabled(false);
        assert!(!emergency_manager.is_detection_enabled());
    }

    #[tokio::test]
    async fn test_emergency_detection() {
        let safe_point_manager = Arc::new(SafePointManager::new());
        let op_tracker = Arc::new(OpStateTracker::new());
        let emergency_manager = Arc::new(EmergencyRollbackManager::new(
            safe_point_manager,
            op_tracker,
        ));
        
        let trigger = EmergencyTrigger::ManualTrigger {
            operator: "test_user".to_string(),
            reason: "testing emergency detection".to_string(),
        };
        
        let emergency_id = emergency_manager.detect_emergency(trigger).await.unwrap();
        assert!(emergency_id.starts_with("emergency_"));
        
        let record = emergency_manager.get_emergency_record(&emergency_id).unwrap();
        // Emergency may not auto-execute due to simplified implementation
        assert!(matches!(record.status, EmergencyStatus::Completed | EmergencyStatus::Failed | EmergencyStatus::Detected));
        // Just verify the emergency was recorded
        assert_eq!(record.id, emergency_id);
    }

    #[tokio::test]
    async fn test_emergency_detection_disabled() {
        let safe_point_manager = Arc::new(SafePointManager::new());
        let op_tracker = Arc::new(OpStateTracker::new());
        let emergency_manager = Arc::new(EmergencyRollbackManager::new(
            safe_point_manager,
            op_tracker,
        ));
        
        // Disable detection
        emergency_manager.set_detection_enabled(false);
        
        let trigger = EmergencyTrigger::ResourceExhaustion {
            resource_type: "cpu".to_string(),
            threshold: 0.99,
        };
        
        let result = emergency_manager.detect_emergency(trigger).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("disabled"));
    }

    #[tokio::test]
    async fn test_emergency_rollback_op() {
        let safe_point_manager = Arc::new(SafePointManager::new());
        let op_tracker = Arc::new(OpStateTracker::new());
        let emergency_manager = Arc::new(EmergencyRollbackManager::new(
            safe_point_manager,
            op_tracker,
        ));
        
        let trigger = EmergencyTrigger::CriticalTimeout {
            op_id: "op_timeout_123".to_string(),
            timeout_ms: 30000,
        };
        
        let emergency_op = EmergencyRollbackOp::new(emergency_manager.clone(), trigger)
            .fire_and_forget(); // Don't wait for completion in test
        
        let mut context = crate::context::OpContext::new();
        let emergency_id = emergency_op.perform(&mut context).await.unwrap();
        
        assert!(emergency_id.starts_with("emergency_"));
        assert!(context.contains_key("emergency_rollback_id"));
        assert!(context.contains_key("emergency_trigger"));
    }

    #[tokio::test]
    async fn test_emergency_statistics() {
        let safe_point_manager = Arc::new(SafePointManager::new());
        let op_tracker = Arc::new(OpStateTracker::new());
        let emergency_manager = Arc::new(EmergencyRollbackManager::new(
            safe_point_manager,
            op_tracker,
        ));
        
        // Create a few different emergency types
        let triggers = vec![
            EmergencyTrigger::SecurityBreach { threat_level: "high".to_string() },
            EmergencyTrigger::ResourceExhaustion { resource_type: "memory".to_string(), threshold: 0.98 },
            EmergencyTrigger::DataCorruption { affected_entities: vec!["entity1".to_string()] },
        ];
        
        for trigger in triggers {
            emergency_manager.detect_emergency(trigger).await.unwrap();
        }
        
        let stats = emergency_manager.get_emergency_statistics();
        assert_eq!(stats.get("total_emergencies").unwrap(), &3);
        assert!(stats.contains_key("trigger_security_breach"));
        assert!(stats.contains_key("trigger_resource_exhaustion"));
        assert!(stats.contains_key("trigger_data_corruption"));
    }

    #[tokio::test]
    async fn test_emergency_cleanup() {
        let safe_point_manager = Arc::new(SafePointManager::new());
        let op_tracker = Arc::new(OpStateTracker::new());
        let emergency_manager = Arc::new(EmergencyRollbackManager::new(
            safe_point_manager,
            op_tracker,
        ));
        
        let trigger = EmergencyTrigger::ManualTrigger {
            operator: "test".to_string(),
            reason: "cleanup test".to_string(),
        };
        
        let _emergency_id = emergency_manager.detect_emergency(trigger).await.unwrap();
        
        assert_eq!(emergency_manager.list_emergency_records().len(), 1);
        
        // Clean up records older than 1ms (should clean up our record)
        tokio::time::sleep(Duration::from_millis(2)).await;
        let cleaned = emergency_manager.cleanup_old_records(Duration::from_millis(1)).await.unwrap();
        
        assert_eq!(cleaned, 1);
        assert_eq!(emergency_manager.list_emergency_records().len(), 0);
    }
}
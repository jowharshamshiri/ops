// Op State Tracking
// Track state of individual ops (pending, running, completed, failed)

use crate::{OpContext, OpError, OpResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// Unique identifier for op instances
pub type OpInstanceId = String;

/// Op execution states
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpState {
    /// Op is queued but not yet started
    Pending,
    /// Op is currently running
    Running,
    /// Op completed successfully
    Completed,
    /// Op failed with an error
    Failed,
    /// Op was cancelled before completion
    Cancelled,
    /// Op timed out during execution
    TimedOut,
}

impl OpState {
    /// Check if op is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, 
            OpState::Completed | 
            OpState::Failed | 
            OpState::Cancelled | 
            OpState::TimedOut
        )
    }

    /// Check if op is currently active
    pub fn is_active(&self) -> bool {
        matches!(self, OpState::Pending | OpState::Running)
    }

    /// Check if op completed successfully
    pub fn is_success(&self) -> bool {
        matches!(self, OpState::Completed)
    }

    /// Check if op ended in failure
    pub fn is_failure(&self) -> bool {
        matches!(self, 
            OpState::Failed | 
            OpState::Cancelled | 
            OpState::TimedOut
        )
    }
}

/// Execution priority levels for ops
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

/// Complete op state tracking information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpStateInfo {
    /// Unique identifier for this op instance
    pub instance_id: OpInstanceId,
    /// Human-readable op name
    pub op_name: String,
    /// Op type for categorization
    pub op_type: String,
    /// Current execution state
    pub state: OpState,
    /// Op execution priority
    pub priority: Priority,
    /// When the op was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the op started execution (if applicable)
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    /// When the op finished execution (if applicable)
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Error message if op failed
    pub error_message: Option<String>,
    /// Op result (serialized if successful)
    pub result: Option<String>,
    /// Op context snapshot at start
    pub context_snapshot: HashMap<String, String>,
    /// Parent op instance ID (for hierarchical ops)
    pub parent_instance_id: Option<OpInstanceId>,
    /// Child op instance IDs
    pub child_instance_ids: Vec<OpInstanceId>,
    /// Op execution metadata
    pub metadata: HashMap<String, String>,
    /// Op execution statistics
    pub statistics: OpStatistics,
}

impl OpStateInfo {
    /// Create new op state info
    pub fn new(instance_id: OpInstanceId, op_name: String) -> Self {
        Self {
            instance_id,
            op_type: op_name.clone(),
            op_name,
            state: OpState::Pending,
            priority: Priority::default(),
            created_at: chrono::Utc::now(),
            started_at: None,
            finished_at: None,
            error_message: None,
            result: None,
            context_snapshot: HashMap::new(),
            parent_instance_id: None,
            child_instance_ids: Vec::new(),
            metadata: HashMap::new(),
            statistics: OpStatistics::new(),
        }
    }

    /// Set op priority
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Set parent op instance
    pub fn with_parent(mut self, parent_id: OpInstanceId) -> Self {
        self.parent_instance_id = Some(parent_id);
        self
    }

    /// Add child op instance
    pub fn add_child(&mut self, child_id: OpInstanceId) {
        self.child_instance_ids.push(child_id);
    }

    /// Add metadata entry
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Capture context snapshot
    pub fn capture_context(&mut self, context: &OpContext) {
        self.context_snapshot.clear();
        for (key, value) in context.values().iter() {
            self.context_snapshot.insert(key.clone(), value.to_string());
        }
    }

    /// Mark op as started
    pub fn mark_started(&mut self) {
        self.state = OpState::Running;
        self.started_at = Some(chrono::Utc::now());
    }

    /// Mark op as completed successfully
    pub fn mark_completed(&mut self, result: Option<String>) {
        self.state = OpState::Completed;
        self.finished_at = Some(chrono::Utc::now());
        self.result = result;
    }

    /// Mark op as failed
    pub fn mark_failed(&mut self, error_message: String) {
        self.state = OpState::Failed;
        self.finished_at = Some(chrono::Utc::now());
        self.error_message = Some(error_message);
    }

    /// Mark op as cancelled
    pub fn mark_cancelled(&mut self) {
        self.state = OpState::Cancelled;
        self.finished_at = Some(chrono::Utc::now());
    }

    /// Mark op as timed out
    pub fn mark_timed_out(&mut self) {
        self.state = OpState::TimedOut;
        self.finished_at = Some(chrono::Utc::now());
        self.error_message = Some("Op timed out".to_string());
    }

    /// Calculate op execution duration
    pub fn execution_duration(&self) -> Option<Duration> {
        if let (Some(started), Some(finished)) = (self.started_at, self.finished_at) {
            let duration = finished - started;
            Some(Duration::from_millis(duration.num_milliseconds().max(0) as u64))
        } else {
            None
        }
    }

    /// Calculate total op lifetime
    pub fn total_duration(&self) -> Duration {
        let end_time = self.finished_at.unwrap_or_else(chrono::Utc::now);
        let duration = end_time - self.created_at;
        Duration::from_millis(duration.num_milliseconds().max(0) as u64)
    }

    /// Check if op is overdue (created long ago but not finished)
    pub fn is_overdue(&self, threshold: Duration) -> bool {
        !self.state.is_terminal() && self.total_duration() > threshold
    }
}

/// Op state tracker - manages state for multiple ops
pub struct OpStateTracker {
    /// Active op states
    states: Arc<RwLock<HashMap<OpInstanceId, OpStateInfo>>>,
    /// Next instance ID counter
    next_instance_id: Arc<RwLock<u64>>,
}

impl OpStateTracker {
    /// Create new op state tracker
    pub fn new() -> Self {
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
            next_instance_id: Arc::new(RwLock::new(1)),
        }
    }

    /// Generate unique op instance ID
    pub fn generate_instance_id(&self) -> OpInstanceId {
        let mut counter = self.next_instance_id.write().unwrap();
        let id = *counter;
        *counter += 1;
        format!("op_{:08}", id)
    }

    /// Create new op state
    pub fn create_op(&self, op_name: String, priority: Priority) -> OpInstanceId {
        let instance_id = self.generate_instance_id();
        let state_info = OpStateInfo::new(instance_id.clone(), op_name)
            .with_priority(priority);

        let mut states = self.states.write().unwrap();
        states.insert(instance_id.clone(), state_info);

        instance_id
    }

    /// Get op state info
    pub fn get_op_state(&self, instance_id: &OpInstanceId) -> Option<OpStateInfo> {
        let states = self.states.read().unwrap();
        states.get(instance_id).cloned()
    }

    /// Update op state
    pub fn update_op_state<F>(&self, instance_id: &OpInstanceId, update_fn: F) -> OpResult<()>
    where
        F: FnOnce(&mut OpStateInfo),
    {
        let mut states = self.states.write().unwrap();
        if let Some(state_info) = states.get_mut(instance_id) {
            update_fn(state_info);
            Ok(())
        } else {
            Err(OpError::ExecutionFailed(
                format!("Op instance not found: {}", instance_id)
            ))
        }
    }

    /// Mark op as started
    pub fn mark_started(&self, instance_id: &OpInstanceId, context: &OpContext) -> OpResult<()> {
        self.update_op_state(instance_id, |state| {
            state.capture_context(context);
            state.mark_started();
        })
    }

    /// Mark op as completed
    pub fn mark_completed(&self, instance_id: &OpInstanceId, result: Option<String>) -> OpResult<()> {
        self.update_op_state(instance_id, |state| {
            state.mark_completed(result);
        })
    }

    /// Mark op as failed
    pub fn mark_failed(&self, instance_id: &OpInstanceId, error: &OpError) -> OpResult<()> {
        let error_message = error.to_string();
        self.update_op_state(instance_id, |state| {
            state.mark_failed(error_message);
        })
    }

    /// Mark op as cancelled
    pub fn mark_cancelled(&self, instance_id: &OpInstanceId) -> OpResult<()> {
        self.update_op_state(instance_id, |state| {
            state.mark_cancelled();
        })
    }

    /// Mark op as timed out
    pub fn mark_timed_out(&self, instance_id: &OpInstanceId) -> OpResult<()> {
        self.update_op_state(instance_id, |state| {
            state.mark_timed_out();
        })
    }

    /// List all ops in a given state
    pub fn list_ops_by_state(&self, target_state: OpState) -> Vec<OpStateInfo> {
        let states = self.states.read().unwrap();
        states.values()
            .filter(|state| state.state == target_state)
            .cloned()
            .collect()
    }

    /// List all active ops (pending or running)
    pub fn list_active_ops(&self) -> Vec<OpStateInfo> {
        let states = self.states.read().unwrap();
        states.values()
            .filter(|state| state.state.is_active())
            .cloned()
            .collect()
    }

    /// List ops by priority
    pub fn list_ops_by_priority(&self, priority: Priority) -> Vec<OpStateInfo> {
        let states = self.states.read().unwrap();
        states.values()
            .filter(|state| state.priority == priority)
            .cloned()
            .collect()
    }

    /// Find overdue ops
    pub fn find_overdue_ops(&self, threshold: Duration) -> Vec<OpStateInfo> {
        let states = self.states.read().unwrap();
        states.values()
            .filter(|state| state.is_overdue(threshold))
            .cloned()
            .collect()
    }

    /// Get op statistics
    pub fn get_statistics(&self) -> OpStatistics {
        let states = self.states.read().unwrap();
        
        let mut stats = OpStatistics::default();
        
        for state in states.values() {
            stats.total_ops += 1;
            
            match state.state {
                OpState::Pending => stats.pending_ops += 1,
                OpState::Running => stats.running_ops += 1,
                OpState::Completed => stats.completed_ops += 1,
                OpState::Failed => stats.failed_ops += 1,
                OpState::Cancelled => stats.cancelled_ops += 1,
                OpState::TimedOut => stats.timed_out_ops += 1,
            }

            match state.priority {
                Priority::Low => stats.low_priority_ops += 1,
                Priority::Normal => stats.normal_priority_ops += 1,
                Priority::High => stats.high_priority_ops += 1,
                Priority::Critical => stats.critical_priority_ops += 1,
            }

            if let Some(duration) = state.execution_duration() {
                if stats.average_execution_duration.is_none() {
                    stats.average_execution_duration = Some(duration);
                } else {
                    let current_avg = stats.average_execution_duration.unwrap();
                    stats.average_execution_duration = Some(
                        Duration::from_millis(
                            (current_avg.as_millis() + duration.as_millis()) as u64 / 2
                        )
                    );
                }
            }
        }

        stats
    }

    /// Remove completed ops older than threshold
    pub fn cleanup_old_ops(&self, age_threshold: Duration) -> usize {
        let cutoff_time = chrono::Utc::now() - chrono::Duration::from_std(age_threshold).unwrap();
        let mut states = self.states.write().unwrap();
        
        let initial_count = states.len();
        
        states.retain(|_id, state| {
            // Keep ops that are not terminal or are newer than threshold
            !state.state.is_terminal() || 
            state.finished_at.map_or(true, |finished| finished > cutoff_time)
        });

        initial_count - states.len()
    }

    /// Get total number of tracked ops
    pub fn op_count(&self) -> usize {
        let states = self.states.read().unwrap();
        states.len()
    }

    /// Create parent-child relationship between ops
    pub fn link_ops(&self, parent_id: &OpInstanceId, child_id: &OpInstanceId) -> OpResult<()> {
        let mut states = self.states.write().unwrap();
        
        // Set parent in child
        if let Some(child_state) = states.get_mut(child_id) {
            child_state.parent_instance_id = Some(parent_id.clone());
        } else {
            return Err(OpError::ExecutionFailed(
                format!("Child op not found: {}", child_id)
            ));
        }

        // Add child to parent
        if let Some(parent_state) = states.get_mut(parent_id) {
            parent_state.add_child(child_id.clone());
        } else {
            return Err(OpError::ExecutionFailed(
                format!("Parent op not found: {}", parent_id)
            ));
        }

        Ok(())
    }
}

impl Default for OpStateTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Op execution statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpStatistics {
    pub total_ops: usize,
    pub pending_ops: usize,
    pub running_ops: usize,
    pub completed_ops: usize,
    pub failed_ops: usize,
    pub cancelled_ops: usize,
    pub timed_out_ops: usize,
    pub low_priority_ops: usize,
    pub normal_priority_ops: usize,
    pub high_priority_ops: usize,
    pub critical_priority_ops: usize,
    pub average_execution_duration: Option<Duration>,
    /// Success count for this specific op
    pub success_count: u64,
    /// Failure count for this specific op  
    pub failure_count: u64,
    /// Total duration in milliseconds for this op
    pub total_duration_ms: u64,
}

impl OpStatistics {
    /// Create new op statistics
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_ops == 0 {
            0.0
        } else {
            (self.completed_ops as f64 / self.total_ops as f64) * 100.0
        }
    }

    /// Calculate failure rate as percentage
    pub fn failure_rate(&self) -> f64 {
        if self.total_ops == 0 {
            0.0
        } else {
            let failed = self.failed_ops + self.cancelled_ops + self.timed_out_ops;
            (failed as f64 / self.total_ops as f64) * 100.0
        }
    }

    /// Get terminal ops count
    pub fn terminal_ops(&self) -> usize {
        self.completed_ops + self.failed_ops + self.cancelled_ops + self.timed_out_ops
    }

    /// Get active ops count
    pub fn active_ops(&self) -> usize {
        self.pending_ops + self.running_ops
    }
}

// Note: StateTrackedOp removed due to Op trait interface mismatch
// This would require updates to the core Op trait to support state tracking

/// Global op state tracker instance
static GLOBAL_TRACKER: std::sync::LazyLock<Arc<OpStateTracker>> = 
    std::sync::LazyLock::new(|| Arc::new(OpStateTracker::new()));

/// Get reference to global op state tracker
pub fn global_op_tracker() -> Arc<OpStateTracker> {
    Arc::clone(&GLOBAL_TRACKER)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OpContext;

    #[test]
    fn test_op_state_transitions() {
        let state = OpState::Pending;
        assert!(state.is_active());
        assert!(!state.is_terminal());

        let state = OpState::Running;
        assert!(state.is_active());
        assert!(!state.is_terminal());

        let state = OpState::Completed;
        assert!(!state.is_active());
        assert!(state.is_terminal());
        assert!(state.is_success());
        assert!(!state.is_failure());

        let state = OpState::Failed;
        assert!(!state.is_active());
        assert!(state.is_terminal());
        assert!(!state.is_success());
        assert!(state.is_failure());
    }

    #[test]
    fn test_op_state_info_creation() {
        let info = OpStateInfo::new("test_op_1".to_string(), "TestOp".to_string())
            .with_priority(Priority::High);

        assert_eq!(info.instance_id, "test_op_1");
        assert_eq!(info.op_name, "TestOp");
        assert_eq!(info.state, OpState::Pending);
        assert_eq!(info.priority, Priority::High);
        assert!(info.started_at.is_none());
        assert!(info.finished_at.is_none());
    }

    #[test]
    fn test_op_state_info_lifecycle() {
        let mut info = OpStateInfo::new("test_op_2".to_string(), "TestOp".to_string());
        
        // Start op
        info.mark_started();
        assert_eq!(info.state, OpState::Running);
        assert!(info.started_at.is_some());

        // Complete op
        info.mark_completed(Some("success".to_string()));
        assert_eq!(info.state, OpState::Completed);
        assert!(info.finished_at.is_some());
        assert_eq!(info.result, Some("success".to_string()));

        // Check duration calculation
        let duration = info.execution_duration();
        assert!(duration.is_some());
        assert!(duration.unwrap() >= Duration::from_millis(0));
    }

    #[test]
    fn test_op_state_tracker_basic() {
        let tracker = OpStateTracker::new();

        // Create op
        let instance_id = tracker.create_op("TestOp".to_string(), Priority::Normal);
        assert!(instance_id.starts_with("op_"));

        // Get state
        let state = tracker.get_op_state(&instance_id);
        assert!(state.is_some());
        
        let state = state.unwrap();
        assert_eq!(state.op_name, "TestOp");
        assert_eq!(state.state, OpState::Pending);
        assert_eq!(state.priority, Priority::Normal);
    }

    #[test]
    fn test_op_state_tracker_updates() {
        let tracker = OpStateTracker::new();
        let context = OpContext::new();

        let instance_id = tracker.create_op("UpdateTest".to_string(), Priority::High);

        // Mark started
        tracker.mark_started(&instance_id, &context).unwrap();
        let state = tracker.get_op_state(&instance_id).unwrap();
        assert_eq!(state.state, OpState::Running);

        // Mark completed
        tracker.mark_completed(&instance_id, Some("result".to_string())).unwrap();
        let state = tracker.get_op_state(&instance_id).unwrap();
        assert_eq!(state.state, OpState::Completed);
        assert_eq!(state.result, Some("result".to_string()));

        // Test failure path
        let failed_id = tracker.create_op("FailTest".to_string(), Priority::Normal);
        let error = OpError::ExecutionFailed("test error".to_string());
        tracker.mark_failed(&failed_id, &error).unwrap();
        let state = tracker.get_op_state(&failed_id).unwrap();
        assert_eq!(state.state, OpState::Failed);
        assert!(state.error_message.is_some());
    }

    #[test]
    fn test_op_state_tracker_queries() {
        let tracker = OpStateTracker::new();
        let context = OpContext::new();

        // Create ops in different states
        let _pending_id = tracker.create_op("Pending".to_string(), Priority::Low);
        
        let running_id = tracker.create_op("Running".to_string(), Priority::High);
        tracker.mark_started(&running_id, &context).unwrap();

        let completed_id = tracker.create_op("Completed".to_string(), Priority::Normal);
        tracker.mark_started(&completed_id, &context).unwrap();
        tracker.mark_completed(&completed_id, None).unwrap();

        // Test queries
        let pending_ops = tracker.list_ops_by_state(OpState::Pending);
        assert_eq!(pending_ops.len(), 1);

        let running_ops = tracker.list_ops_by_state(OpState::Running);
        assert_eq!(running_ops.len(), 1);

        let active_ops = tracker.list_active_ops();
        assert_eq!(active_ops.len(), 2); // pending + running

        let high_priority_ops = tracker.list_ops_by_priority(Priority::High);
        assert_eq!(high_priority_ops.len(), 1);

        assert_eq!(tracker.op_count(), 3);
    }

    #[test]
    fn test_op_statistics() {
        let tracker = OpStateTracker::new();
        let context = OpContext::new();

        // Create ops with different outcomes
        let completed_id = tracker.create_op("Completed".to_string(), Priority::High);
        tracker.mark_started(&completed_id, &context).unwrap();
        tracker.mark_completed(&completed_id, None).unwrap();

        let failed_id = tracker.create_op("Failed".to_string(), Priority::Normal);
        tracker.mark_started(&failed_id, &context).unwrap();
        let error = OpError::ExecutionFailed("test".to_string());
        tracker.mark_failed(&failed_id, &error).unwrap();

        let _pending_id = tracker.create_op("Pending".to_string(), Priority::Critical);

        let stats = tracker.get_statistics();
        
        assert_eq!(stats.total_ops, 3);
        assert_eq!(stats.completed_ops, 1);
        assert_eq!(stats.failed_ops, 1);
        assert_eq!(stats.pending_ops, 1);
        assert_eq!(stats.critical_priority_ops, 1);
        assert_eq!(stats.high_priority_ops, 1);
        assert_eq!(stats.normal_priority_ops, 1);

        assert!((stats.success_rate() - (100.0 / 3.0)).abs() < 0.01);
        assert!((stats.failure_rate() - (100.0 / 3.0)).abs() < 0.01);
        assert_eq!(stats.active_ops(), 1);
        assert_eq!(stats.terminal_ops(), 2);
    }

    #[test]
    fn test_op_hierarchy() {
        let tracker = OpStateTracker::new();

        let parent_id = tracker.create_op("Parent".to_string(), Priority::Normal);
        let child_id = tracker.create_op("Child".to_string(), Priority::Normal);

        tracker.link_ops(&parent_id, &child_id).unwrap();

        let parent_state = tracker.get_op_state(&parent_id).unwrap();
        let child_state = tracker.get_op_state(&child_id).unwrap();

        assert_eq!(parent_state.child_instance_ids.len(), 1);
        assert_eq!(parent_state.child_instance_ids[0], child_id);
        assert_eq!(child_state.parent_instance_id, Some(parent_id));
    }

    // Note: StateTrackedOp tests removed due to Op trait interface mismatch

    #[test]
    fn test_cleanup_old_ops() {
        let tracker = OpStateTracker::new();
        let context = OpContext::new();

        // Create and complete an op
        let old_id = tracker.create_op("Old".to_string(), Priority::Normal);
        tracker.mark_started(&old_id, &context).unwrap();
        tracker.mark_completed(&old_id, None).unwrap();

        // Create pending op
        let _new_id = tracker.create_op("New".to_string(), Priority::Normal);

        assert_eq!(tracker.op_count(), 2);

        // Cleanup with very small threshold (should remove completed op)
        let removed = tracker.cleanup_old_ops(Duration::from_millis(1));
        
        // Sleep briefly to ensure time passes
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        // Try cleanup again with ops that are definitely old
        let removed_after_wait = tracker.cleanup_old_ops(Duration::from_millis(1));
        
        // At least the completed op should be eligible for cleanup
        assert!(removed + removed_after_wait >= 1);
    }

    #[test]
    fn test_overdue_ops() {
        let tracker = OpStateTracker::new();

        let overdue_id = tracker.create_op("Overdue".to_string(), Priority::Normal);
        
        // Sleep to make op overdue
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        let overdue_ops = tracker.find_overdue_ops(Duration::from_millis(5));
        assert_eq!(overdue_ops.len(), 1);
        assert_eq!(overdue_ops[0].instance_id, overdue_id);
    }

    #[test]
    fn test_global_tracker() {
        let tracker1 = global_op_tracker();
        let tracker2 = global_op_tracker();
        
        // Should be the same instance
        assert!(Arc::ptr_eq(&tracker1, &tracker2));
    }

    /// Mock implementation for testing
    pub struct MockOpStateTracker {
        ops: std::sync::RwLock<HashMap<OpInstanceId, OpStateInfo>>,
    }

    impl MockOpStateTracker {
        pub fn new() -> Self {
            Self {
                ops: std::sync::RwLock::new(HashMap::new()),
            }
        }

        pub fn add_op_info(&self, instance_id: OpInstanceId, info: OpStateInfo) {
            let mut ops = self.ops.write().unwrap();
            ops.insert(instance_id, info);
        }
    }

    // Mock implementation methods
    impl MockOpStateTracker {
        pub fn create_op(&self, op_type: String, priority: Priority) -> OpInstanceId {
            let instance_id = format!("mock_{}_{}", op_type, chrono::Utc::now().timestamp_millis());
            let info = OpStateInfo::new(instance_id.clone(), op_type);
            self.add_op_info(instance_id.clone(), info);
            instance_id
        }

        pub fn start_op(&self, instance_id: &OpInstanceId, context: &OpContext) {
            let mut ops = self.ops.write().unwrap();
            if let Some(info) = ops.get_mut(instance_id) {
                info.state = OpState::Running;
                info.started_at = Some(chrono::Utc::now());
            }
        }

        pub fn complete_op(&self, instance_id: &OpInstanceId, success: bool) {
            let mut ops = self.ops.write().unwrap();
            if let Some(info) = ops.get_mut(instance_id) {
                info.state = if success { OpState::Completed } else { OpState::Failed };
                info.finished_at = Some(chrono::Utc::now());
                if success {
                    info.statistics.success_count += 1;
                } else {
                    info.statistics.failure_count += 1;
                }
            }
        }

        pub fn get_op_info(&self, instance_id: &OpInstanceId) -> Option<OpStateInfo> {
            let ops = self.ops.read().unwrap();
            ops.get(instance_id).cloned()
        }

        pub fn list_ops(&self) -> Vec<OpStateInfo> {
            let ops = self.ops.read().unwrap();
            ops.values().cloned().collect()
        }

        pub fn op_count(&self) -> usize {
            let ops = self.ops.read().unwrap();
            ops.len()
        }

        pub fn get_statistics(&self) -> OpStatistics {
            let ops = self.ops.read().unwrap();
            let mut stats = OpStatistics::new();
            
            for info in ops.values() {
                stats.success_count += info.statistics.success_count;
                stats.failure_count += info.statistics.failure_count;
                stats.total_duration_ms += info.statistics.total_duration_ms;
            }
            
            stats
        }

        fn cleanup_old_ops(&self, _older_than: std::time::Duration) -> usize {
            // Mock implementation - doesn't actually clean up
            0
        }

        fn find_overdue_ops(&self, _timeout: std::time::Duration) -> Vec<OpStateInfo> {
            // Mock implementation
            Vec::new()
        }
    }
}
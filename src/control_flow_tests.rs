//! Tests for control flow macros and integration with batch and loop ops

use crate::prelude::*;
use crate::{abort, continue_loop, check_abort};
use async_trait::async_trait;

/// Test operation that can be configured to abort
struct AbortTestOp {
    should_abort: bool,
    abort_reason: Option<String>,
}

impl AbortTestOp {
    fn new(should_abort: bool, abort_reason: Option<String>) -> Self {
        Self { should_abort, abort_reason }
    }
}

#[async_trait]
impl Op<i32> for AbortTestOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<i32> {
        if self.should_abort {
            if let Some(ref reason) = self.abort_reason {
                abort!(dry, reason.clone());
            } else {
                abort!(dry);
            }
        }
        Ok(42)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("AbortTestOp").build()
    }
}

/// Test operation that can be configured to continue in loop
struct ContinueTestOp {
    should_continue: bool,
    value: i32,
}

impl ContinueTestOp {
    fn new(should_continue: bool, value: i32) -> Self {
        Self { should_continue, value }
    }
}

#[async_trait]
impl Op<i32> for ContinueTestOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<i32> {
        if self.should_continue {
            continue_loop!(dry);
        }
        Ok(self.value)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("ContinueTestOp").build()
    }
}

/// Test operation that checks abort flag
struct CheckAbortOp;

#[async_trait]
impl Op<i32> for CheckAbortOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<i32> {
        check_abort!(dry);
        Ok(100)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("CheckAbortOp").build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BatchOp, loop_op::LoopOp};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_abort_macro_without_reason() {
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        let op = AbortTestOp::new(true, None);
        let result = op.perform(&mut dry, &mut wet).await;
        
        assert!(result.is_err());
        assert!(dry.is_aborted());
        assert_eq!(dry.abort_reason(), None);
        
        if let Err(OpError::Aborted(msg)) = result {
            assert_eq!(msg, "Operation aborted");
        } else {
            panic!("Expected Aborted error");
        }
    }

    #[tokio::test]
    async fn test_abort_macro_with_reason() {
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        let op = AbortTestOp::new(true, Some("Test reason".to_string()));
        let result = op.perform(&mut dry, &mut wet).await;
        
        assert!(result.is_err());
        assert!(dry.is_aborted());
        assert_eq!(dry.abort_reason(), Some(&"Test reason".to_string()));
        
        if let Err(OpError::Aborted(msg)) = result {
            assert_eq!(msg, "Test reason");
        } else {
            panic!("Expected Aborted error");
        }
    }

    #[tokio::test]
    async fn test_continue_loop_macro() {
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        let op = ContinueTestOp::new(true, 99);
        let result = op.perform(&mut dry, &mut wet).await;
        
        assert!(result.is_ok());
        assert!(dry.is_continue_loop());
        assert_eq!(result.unwrap(), 0); // Default value for i32
    }

    #[tokio::test]
    async fn test_check_abort_macro() {
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        // First test without abort flag
        let op = CheckAbortOp;
        let result = op.perform(&mut dry, &mut wet).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 100);
        
        // Now set abort flag and test again
        dry.set_abort(Some("Pre-existing abort".to_string()));
        let result = op.perform(&mut dry, &mut wet).await;
        
        assert!(result.is_err());
        if let Err(OpError::Aborted(msg)) = result {
            assert_eq!(msg, "Pre-existing abort");
        } else {
            panic!("Expected Aborted error");
        }
    }

    #[tokio::test]
    async fn test_batch_op_with_abort() {
        let ops = vec![
            Arc::new(AbortTestOp::new(false, None)) as Arc<dyn Op<i32>>,
            Arc::new(AbortTestOp::new(true, Some("Batch abort".to_string()))) as Arc<dyn Op<i32>>,
            Arc::new(AbortTestOp::new(false, None)) as Arc<dyn Op<i32>>, // Should not execute
        ];
        
        let batch = BatchOp::new(ops);
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        let result = batch.perform(&mut dry, &mut wet).await;
        
        assert!(result.is_err());
        if let Err(OpError::Aborted(msg)) = result {
            assert_eq!(msg, "Batch abort");
        } else {
            panic!("Expected Aborted error, got: {:?}", result);
        }
    }

    #[tokio::test]
    async fn test_batch_op_with_pre_existing_abort() {
        let ops = vec![
            Arc::new(AbortTestOp::new(false, None)) as Arc<dyn Op<i32>>,
            Arc::new(AbortTestOp::new(false, None)) as Arc<dyn Op<i32>>,
        ];
        
        let batch = BatchOp::new(ops);
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        // Set abort flag before running batch
        dry.set_abort(Some("Pre-existing abort".to_string()));
        
        let result = batch.perform(&mut dry, &mut wet).await;
        
        assert!(result.is_err());
        if let Err(OpError::Aborted(msg)) = result {
            assert_eq!(msg, "Pre-existing abort");
        } else {
            panic!("Expected Aborted error");
        }
    }

    #[tokio::test]
    async fn test_loop_op_with_continue() {
        let ops: Vec<Box<dyn Op<i32>>> = vec![
            Box::new(ContinueTestOp::new(false, 10)), // Should execute
            Box::new(ContinueTestOp::new(true, 20)),  // Should continue, skip rest
            Box::new(AbortTestOp::new(false, None)),  // Should not execute due to continue
        ];
        
        let loop_op = LoopOp::new("test_counter".to_string(), 2, ops);
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        let result = loop_op.perform(&mut dry, &mut wet).await;
        
        assert!(result.is_ok());
        let results = result.unwrap();
        
        // Should have results from both iterations, but only first two ops in each
        // Iteration 1: [10, 0] (continue after second op, but it returns 0)
        // Iteration 2: [10, 0] (continue after second op, but it returns 0)  
        assert_eq!(results.len(), 4);
        assert_eq!(results, vec![10, 0, 10, 0]);
        
        // Continue flag should be cleared
        assert!(!dry.is_continue_loop());
    }

    #[tokio::test]
    async fn test_loop_op_with_abort() {
        let ops: Vec<Box<dyn Op<i32>>> = vec![
            Box::new(AbortTestOp::new(false, None)),
            Box::new(AbortTestOp::new(true, Some("Loop abort".to_string()))),
            Box::new(AbortTestOp::new(false, None)), // Should not execute
        ];
        
        let loop_op = LoopOp::new("test_counter".to_string(), 3, ops);
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        let result = loop_op.perform(&mut dry, &mut wet).await;
        
        assert!(result.is_err());
        if let Err(OpError::Aborted(msg)) = result {
            assert_eq!(msg, "Loop abort");
        } else {
            panic!("Expected Aborted error");
        }
    }

    #[tokio::test]
    async fn test_loop_op_with_pre_existing_abort() {
        let ops: Vec<Box<dyn Op<i32>>> = vec![
            Box::new(AbortTestOp::new(false, None)),
        ];
        
        let loop_op = LoopOp::new("test_counter".to_string(), 2, ops);
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        // Set abort flag before running loop
        dry.set_abort(Some("Pre-existing loop abort".to_string()));
        
        let result = loop_op.perform(&mut dry, &mut wet).await;
        
        assert!(result.is_err());
        if let Err(OpError::Aborted(msg)) = result {
            assert_eq!(msg, "Pre-existing loop abort");
        } else {
            panic!("Expected Aborted error");
        }
    }

    #[tokio::test]
    async fn test_complex_control_flow_scenario() {
        // Test a complex scenario with nested batch and loop operations
        
        // Create a batch with one normal op and one that sets continue
        let batch_ops = vec![
            Arc::new(ContinueTestOp::new(false, 100)) as Arc<dyn Op<i32>>,
            Arc::new(ContinueTestOp::new(true, 200)) as Arc<dyn Op<i32>>, // Will continue
        ];
        
        // Use the batch in a loop
        let loop_ops: Vec<Box<dyn Op<Vec<i32>>>> = vec![
            Box::new(BatchOp::new(batch_ops)),
        ];
        
        let loop_op = LoopOp::new("complex_counter".to_string(), 2, loop_ops);
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        let result = loop_op.perform(&mut dry, &mut wet).await;
        
        assert!(result.is_ok());
        let results = result.unwrap();
        
        // Each batch iteration should return [100, 0] (0 is default from continue)
        // And we have 2 loop iterations, so 2 results total
        assert_eq!(results.len(), 2);
        
        // Each result should be a Vec<i32> from the batch
        for batch_result in results {
            assert_eq!(batch_result.len(), 2);
            assert_eq!(batch_result[0], 100);
            assert_eq!(batch_result[1], 0); // Default from continue
        }
    }
}
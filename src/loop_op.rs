use crate::prelude::*;
use async_trait::async_trait;
use crate::{Op, DryContext, WetContext, OpMetadata};

/// Loop operation that executes a batch of operations repeatedly until a limit is reached
pub struct LoopOp<T> {
    counter_var: String,
    limit: usize,
    ops: Vec<Arc<dyn Op<T>>>,
}

impl<T> LoopOp<T> 
where
    T: Send + 'static,
{
    /// Create a new loop operation
    /// 
    /// # Arguments
    /// * `counter_var` - Name of the counter variable to store in context
    /// * `limit` - Maximum number of iterations to perform
    /// * `ops` - Vector of operations to execute in each iteration
    pub fn new(counter_var: String, limit: usize, ops: Vec<Arc<dyn Op<T>>>) -> Self {
        Self {
            counter_var,
            limit,
            ops,
        }
    }

    /// Add an operation to the loop
    pub fn add_op(mut self, op: Arc<dyn Op<T>>) -> Self {
        self.ops.push(op);
        self
    }

    /// Rollback succeeded ops from the current iteration in reverse order (LIFO)
    async fn rollback_iteration_ops(&self, succeeded_ops: &[Arc<dyn Op<T>>], dry: &mut DryContext, wet: &mut WetContext) {
        for op in succeeded_ops.iter().rev() {
            if let Err(rollback_error) = op.rollback(dry, wet).await {
                error!("Failed to rollback op {} in loop iteration: {}", op.metadata().name, rollback_error);
            } else {
                debug!("Successfully rolled back op {} in loop iteration", op.metadata().name);
            }
        }
    }

    /// Get the current counter value from dry context
    fn get_counter(&self, dry: &DryContext) -> usize {
        dry.get::<usize>(&self.counter_var).unwrap_or(0)
    }

    /// Set the counter value in dry context
    fn set_counter(&self, dry: &mut DryContext, value: usize) {
        dry.insert(&self.counter_var, value);
    }
}

#[async_trait]
impl<T> Op<Vec<T>> for LoopOp<T>
where
    T: Send + 'static,
{
    async fn perform(&self, dry: &mut DryContext, wet: &mut WetContext) -> OpResult<Vec<T>> {
        let mut results = Vec::new();
        let mut counter = self.get_counter(dry);

        // Initialize counter in context if it doesn't exist
        if !dry.contains(&self.counter_var) {
            self.set_counter(dry, counter);
        }

        while counter < self.limit {
            // Check if we should abort before each iteration
            if dry.is_aborted() {
                let reason = dry.abort_reason()
                    .cloned()
                    .unwrap_or_else(|| "Loop operation aborted".to_string());
                return Err(OpError::Aborted(reason));
            }

            // Track succeeded ops in this iteration for potential rollback
            let mut iteration_succeeded_ops = Vec::new();

            // Execute all operations in the batch for this iteration
            for op in &self.ops {
                // Check abort before each op
                if dry.is_aborted() {
                    // Rollback succeeded ops from current iteration before aborting
                    self.rollback_iteration_ops(&iteration_succeeded_ops, dry, wet).await;
                    let reason = dry.abort_reason()
                        .cloned()
                        .unwrap_or_else(|| "Loop operation aborted".to_string());
                    return Err(OpError::Aborted(reason));
                }
                
                match op.perform(dry, wet).await {
                    Ok(result) => {
                        results.push(result);
                        iteration_succeeded_ops.push(op.clone());
                        
                        // Check if continue flag was set, skip rest of this iteration
                        if dry.is_continue_loop() {
                            dry.clear_continue_loop();
                            break; // Break out of ops loop, continue to next iteration
                        }
                    }
                    Err(OpError::Aborted(reason)) => {
                        // Rollback succeeded ops from current iteration before aborting
                        self.rollback_iteration_ops(&iteration_succeeded_ops, dry, wet).await;
                        return Err(OpError::Aborted(reason));
                    }
                    Err(error) => {
                        // Rollback succeeded ops from current iteration before failing
                        self.rollback_iteration_ops(&iteration_succeeded_ops, dry, wet).await;
                        return Err(error);
                    }
                }
            }

            // Increment counter and update context
            counter += 1;
            self.set_counter(dry, counter);
        }

        Ok(results)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("LoopOp")
            .description(format!("Loop {} times over {} ops", self.limit, self.ops.len()))
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestOp {
        value: i32,
    }

    #[async_trait]
    impl Op<i32> for TestOp {
        async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<i32> {
            Ok(self.value)
        }
        
        fn metadata(&self) -> OpMetadata {
            OpMetadata::builder("TestOp").build()
        }
    }

    struct CounterOp;

    #[async_trait]
    impl Op<usize> for CounterOp {
        async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<usize> {
            let counter: usize = dry.get("loop_counter").unwrap_or(0);
            Ok(counter)
        }
        
        fn metadata(&self) -> OpMetadata {
            OpMetadata::builder("CounterOp").build()
        }
    }

    #[tokio::test]
    async fn test_loop_op_basic() {
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        let ops: Vec<Arc<dyn Op<i32>>> = vec![
            Arc::new(TestOp { value: 10 }),
            Arc::new(TestOp { value: 20 }),
        ];

        let loop_op = LoopOp::new("loop_counter".to_string(), 3, ops);
        let results = loop_op.perform(&mut dry, &mut wet).await.unwrap();

        // Should have 6 results (2 ops * 3 iterations)
        assert_eq!(results.len(), 6);
        assert_eq!(results, vec![10, 20, 10, 20, 10, 20]);
    }

    #[tokio::test]
    async fn test_loop_op_with_counter_access() {
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        let ops: Vec<Arc<dyn Op<usize>>> = vec![
            Arc::new(CounterOp),
        ];

        let loop_op = LoopOp::new("loop_counter".to_string(), 3, ops);
        let results = loop_op.perform(&mut dry, &mut wet).await.unwrap();

        // Should have counter values: [0, 1, 2]
        assert_eq!(results, vec![0, 1, 2]);
    }

    #[tokio::test]
    async fn test_loop_op_existing_counter() {
        let mut dry = DryContext::new().with_value("my_counter", 2_usize);
        let mut wet = WetContext::new();
        
        let ops: Vec<Arc<dyn Op<i32>>> = vec![
            Arc::new(TestOp { value: 42 }),
        ];

        let loop_op = LoopOp::new("my_counter".to_string(), 4, ops);
        let results = loop_op.perform(&mut dry, &mut wet).await.unwrap();

        // Should execute 2 times (from 2 to 4)
        assert_eq!(results.len(), 2);
        assert_eq!(results, vec![42, 42]);
    }

    #[tokio::test]
    async fn test_loop_op_zero_limit() {
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        let ops: Vec<Arc<dyn Op<i32>>> = vec![
            Arc::new(TestOp { value: 99 }),
        ];

        let loop_op = LoopOp::new("counter".to_string(), 0, ops);
        let results = loop_op.perform(&mut dry, &mut wet).await.unwrap();

        // Should not execute any operations
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_loop_op_builder_pattern() {
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        let loop_op = LoopOp::new("builder_counter".to_string(), 2, vec![])
            .add_op(Arc::new(TestOp { value: 1 }))
            .add_op(Arc::new(TestOp { value: 2 }));

        let results = loop_op.perform(&mut dry, &mut wet).await.unwrap();

        assert_eq!(results.len(), 4);
        assert_eq!(results, vec![1, 2, 1, 2]);
    }

    #[tokio::test]
    async fn test_loop_op_rollback_on_iteration_failure() {
        use std::sync::{Arc, Mutex};
        
        struct RollbackTrackingOp {
            id: u32,
            should_fail: bool,
            performed: Arc<Mutex<Vec<u32>>>, // Track all perform calls
            rolled_back: Arc<Mutex<Vec<u32>>>, // Track all rollback calls
        }
        
        #[async_trait]
        impl Op<u32> for RollbackTrackingOp {
            async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<u32> {
                self.performed.lock().unwrap().push(self.id);
                if self.should_fail {
                    Err(OpError::ExecutionFailed(format!("Op {} failed", self.id)))
                } else {
                    Ok(self.id)
                }
            }
            
            async fn rollback(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
                self.rolled_back.lock().unwrap().push(self.id);
                Ok(())
            }
            
            fn metadata(&self) -> OpMetadata {
                OpMetadata::builder(&format!("RollbackTrackingOp{}", self.id)).build()
            }
        }
        
        // Create tracking state
        let performed = Arc::new(Mutex::new(Vec::new()));
        let rolled_back = Arc::new(Mutex::new(Vec::new()));
        
        let ops = vec![
            Arc::new(RollbackTrackingOp {
                id: 1,
                should_fail: false,
                performed: performed.clone(),
                rolled_back: rolled_back.clone(),
            }) as Arc<dyn Op<u32>>,
            Arc::new(RollbackTrackingOp {
                id: 2,
                should_fail: false,
                performed: performed.clone(),
                rolled_back: rolled_back.clone(),
            }) as Arc<dyn Op<u32>>,
            Arc::new(RollbackTrackingOp {
                id: 3,
                should_fail: true, // This will fail in the first iteration
                performed: performed.clone(),
                rolled_back: rolled_back.clone(),
            }) as Arc<dyn Op<u32>>,
        ];
        
        let loop_op = LoopOp::new("test_counter".to_string(), 2, ops);
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        // Execute loop - should fail on op3 in first iteration
        let result = loop_op.perform(&mut dry, &mut wet).await;
        assert!(result.is_err());
        
        // Verify execution state - only first iteration should have run
        let performed_calls = performed.lock().unwrap();
        assert_eq!(*performed_calls, vec![1, 2, 3], "Should have performed ops 1, 2, 3 in first iteration");
        
        // Verify rollback state - only succeeded ops from failed iteration should be rolled back
        let rolled_back_calls = rolled_back.lock().unwrap();
        assert_eq!(*rolled_back_calls, vec![2, 1], "Should have rolled back ops 2, 1 in reverse order (op 3 failed so no rollback)");
    }

    #[tokio::test]
    async fn test_loop_op_rollback_order_within_iteration() {
        use std::sync::{Arc, Mutex};
        
        struct OrderTrackingOp {
            id: u32,
            rollback_order: Arc<Mutex<Vec<u32>>>,
        }
        
        #[async_trait]
        impl Op<u32> for OrderTrackingOp {
            async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<u32> {
                Ok(self.id)
            }
            
            async fn rollback(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
                self.rollback_order.lock().unwrap().push(self.id);
                Ok(())
            }
            
            fn metadata(&self) -> OpMetadata {
                OpMetadata::builder(&format!("OrderTrackingOp{}", self.id)).build()
            }
        }
        
        struct FailingOp;
        
        #[async_trait]
        impl Op<u32> for FailingOp {
            async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<u32> {
                Err(OpError::ExecutionFailed("Intentional failure".to_string()))
            }
            
            fn metadata(&self) -> OpMetadata {
                OpMetadata::builder("FailingOp").build()
            }
        }
        
        let rollback_order = Arc::new(Mutex::new(Vec::new()));
        
        let ops = vec![
            Arc::new(OrderTrackingOp {
                id: 1,
                rollback_order: rollback_order.clone(),
            }) as Arc<dyn Op<u32>>,
            Arc::new(OrderTrackingOp {
                id: 2,
                rollback_order: rollback_order.clone(),
            }) as Arc<dyn Op<u32>>,
            Arc::new(OrderTrackingOp {
                id: 3,
                rollback_order: rollback_order.clone(),
            }) as Arc<dyn Op<u32>>,
            Arc::new(FailingOp) as Arc<dyn Op<u32>>, // Fails, triggering rollback
        ];
        
        let loop_op = LoopOp::new("test_counter".to_string(), 1, ops);
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        // Execute loop - should fail on FailingOp
        let result = loop_op.perform(&mut dry, &mut wet).await;
        assert!(result.is_err());
        
        // Verify rollback order is LIFO (reverse of execution order)
        let order = rollback_order.lock().unwrap();
        assert_eq!(*order, vec![3, 2, 1], "Rollback should happen in reverse order within iteration");
    }

    #[tokio::test]
    async fn test_loop_op_successful_iterations_not_rolled_back() {
        use std::sync::{Arc, Mutex};
        
        struct IterationTrackingOp {
            id: u32,
            fail_on_iteration: Option<usize>, // Fail on specific iteration (0-based)
            performed_iterations: Arc<Mutex<Vec<usize>>>,
            rolled_back_iterations: Arc<Mutex<Vec<usize>>>,
        }
        
        #[async_trait]
        impl Op<u32> for IterationTrackingOp {
            async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<u32> {
                let counter: usize = dry.get("test_counter").unwrap_or(0);
                self.performed_iterations.lock().unwrap().push(counter);
                
                if let Some(fail_iteration) = self.fail_on_iteration {
                    if counter == fail_iteration {
                        return Err(OpError::ExecutionFailed(format!("Op {} failed on iteration {}", self.id, counter)));
                    }
                }
                
                Ok(self.id)
            }
            
            async fn rollback(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
                let counter: usize = dry.get("test_counter").unwrap_or(0);
                self.rolled_back_iterations.lock().unwrap().push(counter);
                Ok(())
            }
            
            fn metadata(&self) -> OpMetadata {
                OpMetadata::builder(&format!("IterationTrackingOp{}", self.id)).build()
            }
        }
        
        let performed_iterations = Arc::new(Mutex::new(Vec::new()));
        let rolled_back_iterations = Arc::new(Mutex::new(Vec::new()));
        
        let ops = vec![
            Arc::new(IterationTrackingOp {
                id: 1,
                fail_on_iteration: Some(2), // Fail on third iteration (index 2)
                performed_iterations: performed_iterations.clone(),
                rolled_back_iterations: rolled_back_iterations.clone(),
            }) as Arc<dyn Op<u32>>,
        ];
        
        let loop_op = LoopOp::new("test_counter".to_string(), 5, ops);
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        // Execute loop - should fail on third iteration
        let result = loop_op.perform(&mut dry, &mut wet).await;
        assert!(result.is_err());
        
        // Verify execution: should have run iterations 0, 1, 2
        let performed = performed_iterations.lock().unwrap();
        assert_eq!(*performed, vec![0, 1, 2], "Should have performed iterations 0, 1, 2");
        
        // Verify rollback: no rollback should happen because the op failed during perform()
        // (ops that fail during perform() are not added to succeeded_ops, so they don't get rolled back)
        let rolled_back = rolled_back_iterations.lock().unwrap();
        assert_eq!(*rolled_back, Vec::<usize>::new(), "No rollback should happen - the failing op wasn't successfully performed");
    }

    #[tokio::test]
    async fn test_loop_op_mixed_iteration_with_rollback() {
        use std::sync::{Arc, Mutex};
        
        struct MixedIterationOp {
            id: u32,
            fail_on_iteration: Option<usize>,
            performed_iterations: Arc<Mutex<Vec<(u32, usize)>>>, // (op_id, iteration)
            rolled_back_iterations: Arc<Mutex<Vec<(u32, usize)>>>, // (op_id, iteration)
        }
        
        #[async_trait]
        impl Op<u32> for MixedIterationOp {
            async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<u32> {
                let counter: usize = dry.get("test_counter").unwrap_or(0);
                self.performed_iterations.lock().unwrap().push((self.id, counter));
                
                if let Some(fail_iteration) = self.fail_on_iteration {
                    if counter == fail_iteration {
                        return Err(OpError::ExecutionFailed(format!("Op {} failed on iteration {}", self.id, counter)));
                    }
                }
                
                Ok(self.id)
            }
            
            async fn rollback(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
                let counter: usize = dry.get("test_counter").unwrap_or(0);
                self.rolled_back_iterations.lock().unwrap().push((self.id, counter));
                Ok(())
            }
            
            fn metadata(&self) -> OpMetadata {
                OpMetadata::builder(&format!("MixedIterationOp{}", self.id)).build()
            }
        }
        
        let performed_iterations = Arc::new(Mutex::new(Vec::new()));
        let rolled_back_iterations = Arc::new(Mutex::new(Vec::new()));
        
        // Create ops: first succeeds, second fails on iteration 1
        let ops = vec![
            Arc::new(MixedIterationOp {
                id: 1,
                fail_on_iteration: None, // Never fails
                performed_iterations: performed_iterations.clone(),
                rolled_back_iterations: rolled_back_iterations.clone(),
            }) as Arc<dyn Op<u32>>,
            Arc::new(MixedIterationOp {
                id: 2,
                fail_on_iteration: Some(1), // Fail on second iteration (index 1)
                performed_iterations: performed_iterations.clone(),
                rolled_back_iterations: rolled_back_iterations.clone(),
            }) as Arc<dyn Op<u32>>,
        ];
        
        let loop_op = LoopOp::new("test_counter".to_string(), 3, ops);
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        // Execute loop - should fail on op2 in second iteration
        let result = loop_op.perform(&mut dry, &mut wet).await;
        assert!(result.is_err());
        
        // Verify execution: should have run:
        // Iteration 0: op1, op2 (both succeed)
        // Iteration 1: op1 (succeeds), op2 (fails)
        let performed = performed_iterations.lock().unwrap();
        assert_eq!(*performed, vec![(1, 0), (2, 0), (1, 1), (2, 1)], "Should have performed all ops");
        
        // Verify rollback: only op1 from iteration 1 should be rolled back
        // (op2 failed so it doesn't get rolled back, and iteration 0 was successful so no rollback)
        let rolled_back = rolled_back_iterations.lock().unwrap();
        assert_eq!(*rolled_back, vec![(1, 1)], "Should only rollback op1 from failed iteration 1");
    }
}
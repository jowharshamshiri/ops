use crate::prelude::*;
use async_trait::async_trait;
use crate::{Op, DryContext, WetContext, OpMetadata};

/// Loop operation that executes a batch of operations repeatedly until a limit is reached
pub struct LoopOp<T> {
    counter_var: String,
    limit: usize,
    ops: Vec<Arc<dyn Op<T>>>,
    loop_id: String,
    continue_var: String,
    break_var: String,
    continue_on_error: bool,
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
		// Generate a unique loop ID for scoped control flow variables
		// Keep the preceding double underscores to minimize risk of collisions
        let loop_id = ::uuid::Uuid::new_v4().to_string();
        Self {
            counter_var,
            limit,
            ops,
            continue_var: format!("__continue_loop_{}", loop_id),
            break_var: format!("__break_loop_{}", loop_id),
            loop_id,
            continue_on_error: false,
        }
    }

    /// Add an operation to the loop
    pub fn add_op(mut self, op: Arc<dyn Op<T>>) -> Self {
        self.ops.push(op);
        self
    }
    
    /// Set whether to continue on error
    pub fn with_continue_on_error(mut self, continue_on_error: bool) -> Self {
        self.continue_on_error = continue_on_error;
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

        // Set loop context for scoped control flow
        dry.insert("__current_loop_id", &self.loop_id);

        while counter < self.limit {
            // Check if we should abort before each iteration
            if dry.is_aborted() {
                let reason = dry.abort_reason()
                    .cloned()
                    .unwrap_or_else(|| "Loop operation aborted".to_string());
                return Err(OpError::Aborted(reason));
            }

            // Clear scoped control flags for this iteration
            dry.insert(&self.continue_var, false);
            dry.insert(&self.break_var, false);

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
                        
                        // Check scoped continue flag
                        if dry.get::<bool>(&self.continue_var).unwrap_or(false) {
                            dry.insert(&self.continue_var, false); // Clear flag
                            break; // Break out of ops loop, continue to next iteration
                        }
                        
                        // Check scoped break flag
                        if dry.get::<bool>(&self.break_var).unwrap_or(false) {
                            dry.insert(&self.break_var, false); // Clear flag
                            return Ok(results); // Break out of entire loop
                        }
                    }
                    Err(OpError::Aborted(reason)) => {
                        // Rollback succeeded ops from current iteration before aborting
                        self.rollback_iteration_ops(&iteration_succeeded_ops, dry, wet).await;
                        return Err(OpError::Aborted(reason));
                    }
                    Err(error) => {
                        if self.continue_on_error {
                            // Log the error and continue with next iteration
                            warn!("Operation {} failed in loop iteration {}: {}. Continuing with next iteration.", 
                                  op.metadata().name, counter, error);
                            // Rollback succeeded ops from current iteration
                            self.rollback_iteration_ops(&iteration_succeeded_ops, dry, wet).await;
                            break; // Break out of ops loop, continue to next iteration
                        } else {
                            // Rollback succeeded ops from current iteration before failing
                            self.rollback_iteration_ops(&iteration_succeeded_ops, dry, wet).await;
                            return Err(error);
                        }
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
        let description = if self.continue_on_error {
            format!("Loop {} times over {} ops (continue on error)", self.limit, self.ops.len())
        } else {
            format!("Loop {} times over {} ops", self.limit, self.ops.len())
        };
        OpMetadata::builder("LoopOp")
            .description(description)
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

    // TEST067: Run a LoopOp for 3 iterations with 2 ops each and verify all 6 results in order
    #[tokio::test]
    async fn test_067_loop_op_basic() {
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

    // TEST068: Run a LoopOp where each op reads the loop counter and verify values are 0, 1, 2
    #[tokio::test]
    async fn test_068_loop_op_with_counter_access() {
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

    // TEST069: Start a LoopOp with a pre-initialized counter and verify it only executes the remaining iterations
    #[tokio::test]
    async fn test_069_loop_op_existing_counter() {
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

    // TEST070: Run a LoopOp with a zero iteration limit and verify no ops are executed
    #[tokio::test]
    async fn test_070_loop_op_zero_limit() {
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

    // TEST071: Build a LoopOp with add_op chaining and verify all added ops run across all iterations
    #[tokio::test]
    async fn test_071_loop_op_builder_pattern() {
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        let loop_op = LoopOp::new("builder_counter".to_string(), 2, vec![])
            .add_op(Arc::new(TestOp { value: 1 }))
            .add_op(Arc::new(TestOp { value: 2 }));

        let results = loop_op.perform(&mut dry, &mut wet).await.unwrap();

        assert_eq!(results.len(), 4);
        assert_eq!(results, vec![1, 2, 1, 2]);
    }

    // TEST072: Run a LoopOp where the third op fails and verify succeeded ops are rolled back in reverse order
    #[tokio::test]
    async fn test_072_loop_op_rollback_on_iteration_failure() {
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

    // TEST073: Run a LoopOp where the last op fails and verify rollback occurs in LIFO order within the iteration
    #[tokio::test]
    async fn test_073_loop_op_rollback_order_within_iteration() {
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

    // TEST074: Run a LoopOp that fails on iteration 2 and verify previously completed iterations are not rolled back
    #[tokio::test]
    async fn test_074_loop_op_successful_iterations_not_rolled_back() {
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

    // TEST075: Run a LoopOp where op2 fails on iteration 1 and verify only op1 from that iteration is rolled back
    #[tokio::test]
    async fn test_075_loop_op_mixed_iteration_with_rollback() {
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

    struct BreakOp {
        should_break: bool,
        value: i32,
    }

    #[async_trait]
    impl Op<i32> for BreakOp {
        async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<i32> {
            if self.should_break {
                // Set the scoped break flag — LoopOp reads __break_loop_{loop_id}
                // We set it via __current_loop_id which LoopOp stores before the loop
                let loop_id: String = dry.get("__current_loop_id").unwrap_or_default();
                dry.insert(format!("__break_loop_{}", loop_id), true);
            }
            Ok(self.value)
        }
        fn metadata(&self) -> OpMetadata { OpMetadata::builder("BreakOp").build() }
    }

    // TEST113: Run a LoopOp where an op sets the break flag and verify the loop terminates early
    #[tokio::test]
    async fn test_113_loop_op_break_terminates_loop() {
        let ops: Vec<Arc<dyn Op<i32>>> = vec![
            Arc::new(TestOp { value: 10 }),
            Arc::new(BreakOp { should_break: true, value: 99 }),
            Arc::new(TestOp { value: 20 }), // should NOT execute after break
        ];

        let loop_op = LoopOp::new("counter".to_string(), 5, ops);
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();

        let results = loop_op.perform(&mut dry, &mut wet).await.unwrap();
        // Only two results: TestOp(10) and BreakOp(99) from iteration 0, then loop stops
        assert_eq!(results, vec![10, 99]);
    }

    // TEST114: Run LoopOp::with_continue_on_error where an op fails and verify the loop continues
    #[tokio::test]
    async fn test_114_loop_op_continue_on_error_skips_failed_iterations() {
        use std::sync::{Arc as StdArc, Mutex};

        let iterations_seen: StdArc<Mutex<Vec<usize>>> = StdArc::new(Mutex::new(vec![]));
        let log = iterations_seen.clone();

        struct IterationLogOp {
            fail_on: Option<usize>,
            log: StdArc<Mutex<Vec<usize>>>,
        }

        #[async_trait]
        impl Op<i32> for IterationLogOp {
            async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<i32> {
                let counter: usize = dry.get("it_counter").unwrap_or(0);
                self.log.lock().unwrap().push(counter);
                if Some(counter) == self.fail_on {
                    return Err(OpError::ExecutionFailed(format!("fail on {}", counter)));
                }
                Ok(counter as i32)
            }
            fn metadata(&self) -> OpMetadata { OpMetadata::builder("IterationLogOp").build() }
        }

        let loop_op = LoopOp::new(
            "it_counter".to_string(),
            4,
            vec![Arc::new(IterationLogOp { fail_on: Some(1), log: log }) as Arc<dyn Op<i32>>],
        ).with_continue_on_error(true);

        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        let results = loop_op.perform(&mut dry, &mut wet).await.unwrap();

        let seen = iterations_seen.lock().unwrap().clone();
        // All 4 iterations were attempted despite failure on iteration 1
        assert_eq!(seen, vec![0, 1, 2, 3]);
        // Iteration 1 produced no result (failed), others did
        assert_eq!(results, vec![0, 2, 3]);
    }

    // TEST115: Run an empty LoopOp with a non-zero limit and verify it produces no results
    #[tokio::test]
    async fn test_115_loop_op_with_no_ops_produces_no_results() {
        let loop_op: LoopOp<i32> = LoopOp::new("counter".to_string(), 5, vec![]);
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        let results = loop_op.perform(&mut dry, &mut wet).await.unwrap();
        assert!(results.is_empty());
        // Counter advances to limit
        assert_eq!(dry.get::<usize>("counter").unwrap(), 5);
    }

    // TEST076: Run a LoopOp configured to continue on error and verify subsequent iterations still execute
    #[tokio::test]
    async fn test_076_loop_op_continue_on_error() {
        use std::sync::{Arc, Mutex};
        
        struct ContinueOnErrorOp {
            id: u32,
            fail_on_iteration: Option<usize>,
            performed_iterations: Arc<Mutex<Vec<(u32, usize)>>>, // (op_id, iteration)
            rolled_back_iterations: Arc<Mutex<Vec<(u32, usize)>>>, // (op_id, iteration)
        }
        
        #[async_trait]
        impl Op<u32> for ContinueOnErrorOp {
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
                OpMetadata::builder(&format!("ContinueOnErrorOp{}", self.id)).build()
            }
        }
        
        let performed_iterations = Arc::new(Mutex::new(Vec::new()));
        let rolled_back_iterations = Arc::new(Mutex::new(Vec::new()));
        
        // Create ops: first succeeds, second fails on iteration 1
        let ops = vec![
            Arc::new(ContinueOnErrorOp {
                id: 1,
                fail_on_iteration: None, // Never fails
                performed_iterations: performed_iterations.clone(),
                rolled_back_iterations: rolled_back_iterations.clone(),
            }) as Arc<dyn Op<u32>>,
            Arc::new(ContinueOnErrorOp {
                id: 2,
                fail_on_iteration: Some(1), // Fail on second iteration (index 1)
                performed_iterations: performed_iterations.clone(),
                rolled_back_iterations: rolled_back_iterations.clone(),
            }) as Arc<dyn Op<u32>>,
        ];
        
        let loop_op = LoopOp::new("test_counter".to_string(), 3, ops)
            .with_continue_on_error(true);
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        // Execute loop - should continue despite op2 failure in iteration 1
        let result = loop_op.perform(&mut dry, &mut wet).await;
        assert!(result.is_ok());
        let results = result.unwrap();
        
        // Verify execution: should have run:
        // Iteration 0: op1, op2 (both succeed) -> results: [1, 2]
        // Iteration 1: op1 (succeeds), op2 (fails, iteration continues) -> results: [1, 2, 1]
        // Iteration 2: op1, op2 (both succeed) -> results: [1, 2, 1, 1, 2]
        let performed = performed_iterations.lock().unwrap();
        assert_eq!(*performed, vec![(1, 0), (2, 0), (1, 1), (2, 1), (1, 2), (2, 2)], 
                   "Should have performed all ops across all iterations");
        
        // Should have 5 successful results (op1 and op2 from iteration 0, op1 from iteration 1, op1 and op2 from iteration 2)
        assert_eq!(results, vec![1, 2, 1, 1, 2], "Should have results from successful operations only");
        
        // Verify rollback: only op1 from iteration 1 should be rolled back (when op2 failed)
        let rolled_back = rolled_back_iterations.lock().unwrap();
        assert_eq!(*rolled_back, vec![(1, 1)], "Should only rollback op1 from failed iteration 1");
    }
}
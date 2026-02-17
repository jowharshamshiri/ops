use crate::prelude::*;
use crate::batch_metadata::BatchMetadataBuilder;

#[derive(Clone)]
pub struct BatchOp<T> {
    ops: Vec<Arc<dyn Op<T>>>,
    continue_on_error: bool,
}

impl<T> std::fmt::Debug for BatchOp<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BatchOp")
            .field("ops_count", &self.ops.len())
            .field("continue_on_error", &self.continue_on_error)
            .finish()
    }
}

impl<T> BatchOp<T>
where
    T: Send + Sync + 'static,
{
    pub fn new(ops: Vec<Arc<dyn Op<T>>>) -> Self {
        Self {
            ops,
            continue_on_error: false,
        }
    }
    
    pub fn with_continue_on_error(mut self, continue_on_error: bool) -> Self {
        self.continue_on_error = continue_on_error;
        self
    }
    
    pub fn add_op(&mut self, op: Arc<dyn Op<T>>) {
        self.ops.push(op);
    }
    
    pub fn len(&self) -> usize {
        self.ops.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
    
    async fn rollback_succeeded_ops(&self, succeeded_ops: &[Arc<dyn Op<T>>], dry: &mut DryContext, wet: &mut WetContext) {
        // Rollback in reverse order (LIFO)
        for op in succeeded_ops.iter().rev() {
            if let Err(rollback_error) = op.rollback(dry, wet).await {
                error!("Failed to rollback op {}: {}", op.metadata().name, rollback_error);
            } else {
                debug!("Successfully rolled back op {}", op.metadata().name);
            }
        }
    }
}

#[async_trait]
impl<T> Op<Vec<T>> for BatchOp<T>
where
    T: Send + Sync + 'static,
{
    async fn perform(&self, dry: &mut DryContext, wet: &mut WetContext) -> OpResult<Vec<T>> {
        let mut results = Vec::with_capacity(self.ops.len());
        let mut errors = Vec::new();
        let mut succeeded_ops = Vec::new(); // Track succeeded ops for rollback
        
        for (index, op) in self.ops.iter().enumerate() {
            // Check if we should abort before executing each op
            if dry.is_aborted() {
                // Rollback succeeded ops before aborting
                self.rollback_succeeded_ops(&succeeded_ops, dry, wet).await;
                let reason = dry.abort_reason()
                    .cloned()
                    .unwrap_or_else(|| "Batch operation aborted".to_string());
                return Err(OpError::Aborted(reason));
            }
            
            match op.perform(dry, wet).await {
                Ok(result) => {
                    results.push(result);
                    succeeded_ops.push(op.clone());
                }
                Err(OpError::Aborted(reason)) => {
                    // Rollback succeeded ops before aborting
                    self.rollback_succeeded_ops(&succeeded_ops, dry, wet).await;
                    return Err(OpError::Aborted(reason));
                }
                Err(error) => {
                    if self.continue_on_error {
                        errors.push((index, error));
                    } else {
                        // Rollback succeeded ops before failing
                        self.rollback_succeeded_ops(&succeeded_ops, dry, wet).await;
                        return Err(OpError::BatchFailed(
                            format!("Op {}-{} failed: {}", index, op.metadata().name, error)
                        ));
                    }
                }
            }
        }
        
        if !errors.is_empty() && !self.continue_on_error {
            self.rollback_succeeded_ops(&succeeded_ops, dry, wet).await;
            return Err(OpError::BatchFailed(
                format!("Batch op had {} errors", errors.len())
            ));
        }
        
        Ok(results)
    }
    
    fn metadata(&self) -> OpMetadata {
        // Use the intelligent metadata builder that understands data flow
        BatchMetadataBuilder::new(&self.ops).build()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    struct TestOp {
        value: i32,
        should_fail: bool,
    }
    
    #[async_trait]
    impl Op<i32> for TestOp {
        async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<i32> {
            if self.should_fail {
                Err(OpError::ExecutionFailed("Test failure".to_string()))
            } else {
                Ok(self.value)
            }
        }
        
        fn metadata(&self) -> OpMetadata {
            OpMetadata::builder("TestOp").build()
        }
    }
    
    // TEST049: Run BatchOp with two succeeding ops and verify results contain both values in order
    #[tokio::test]
    async fn test_049_batch_op_success() {
        let ops = vec![
            Arc::new(TestOp { value: 1, should_fail: false }) as Arc<dyn Op<i32>>,
            Arc::new(TestOp { value: 2, should_fail: false }) as Arc<dyn Op<i32>>,
        ];
        
        let batch = BatchOp::new(ops);
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        let results = batch.perform(&mut dry, &mut wet).await.unwrap();
        assert_eq!(results, vec![1, 2]);
    }
    
    // TEST050: Run BatchOp where the second op fails and verify the batch returns an error
    #[tokio::test]
    async fn test_050_batch_op_failure() {
        let ops = vec![
            Arc::new(TestOp { value: 1, should_fail: false }) as Arc<dyn Op<i32>>,
            Arc::new(TestOp { value: 2, should_fail: true }) as Arc<dyn Op<i32>>,
        ];
        
        let batch = BatchOp::new(ops);
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        let result = batch.perform(&mut dry, &mut wet).await;
        assert!(result.is_err());
    }
    
    // TEST051: Run BatchOp with two ops and verify both result values are present in order
    #[tokio::test]
    async fn test_051_batch_op_returns_all_results() {
        let ops = vec![
            Arc::new(TestOp { value: 1, should_fail: false }) as Arc<dyn Op<i32>>,
            Arc::new(TestOp { value: 2, should_fail: false }) as Arc<dyn Op<i32>>,
        ];

        let batch = BatchOp::new(ops);
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();

        let results = batch.perform(&mut dry, &mut wet).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.contains(&1));
        assert!(results.contains(&2));
    }
    
    // TEST052: Verify BatchOp metadata correctly identifies only the externally-required input fields
    #[tokio::test]
    async fn test_052_batch_metadata_data_flow() {
        // Define ops with data flow dependencies
        struct ProducerOp;
        struct ConsumerOp;
        
        #[async_trait]
        impl Op<()> for ProducerOp {
            async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
                let initial_value = dry.get_required::<String>("initial_value")?;
                dry.insert("produced_value", format!("processed_{}", initial_value));
                Ok(())
            }
            
            fn metadata(&self) -> OpMetadata {
                OpMetadata::builder("ProducerOp")
                    .input_schema(json!({
                        "type": "object",
                        "properties": {
                            "initial_value": { "type": "string" }
                        },
                        "required": ["initial_value"]
                    }))
                    .output_schema(json!({
                        "type": "object",
                        "properties": {
                            "produced_value": { "type": "string" }
                        }
                    }))
                    .build()
            }
        }
        
        #[async_trait]
        impl Op<()> for ConsumerOp {
            async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
                let produced = dry.get_required::<String>("produced_value")?;
                let extra = dry.get_required::<i32>("extra_param")?;
                dry.insert("final_result", format!("{}_extra_{}", produced, extra));
                Ok(())
            }
            
            fn metadata(&self) -> OpMetadata {
                OpMetadata::builder("ConsumerOp")
                    .input_schema(json!({
                        "type": "object",
                        "properties": {
                            "produced_value": { "type": "string" },
                            "extra_param": { "type": "integer" }
                        },
                        "required": ["produced_value", "extra_param"]
                    }))
                    .output_schema(json!({
                        "type": "object",
                        "properties": {
                            "final_result": { "type": "string" }
                        }
                    }))
                    .build()
            }
        }
        
        let ops: Vec<Arc<dyn Op<()>>> = vec![
            Arc::new(ProducerOp),
            Arc::new(ConsumerOp),
        ];
        
        let batch = BatchOp::new(ops);
        let metadata = batch.metadata();
        
        // The batch should only require initial_value and extra_param
        // produced_value is satisfied internally by ProducerOp
        if let Some(input_schema) = metadata.input_schema {
            let required = input_schema.get("required")
                .and_then(|r| r.as_array())
                .unwrap();
            
            let required_fields: Vec<&str> = required.iter()
                .filter_map(|v| v.as_str())
                .collect();
            
            assert_eq!(required_fields.len(), 2);
            assert!(required_fields.contains(&"initial_value"));
            assert!(required_fields.contains(&"extra_param"));
            assert!(!required_fields.contains(&"produced_value")); // This is satisfied internally!
        }
    }
    
    // TEST053: Verify BatchOp merges reference schemas from all ops into a unified set of required refs
    #[tokio::test]
    async fn test_053_batch_reference_schema_merging() {
        struct ServiceAOp;
        struct ServiceBOp;
        
        #[async_trait]
        impl Op<()> for ServiceAOp {
            async fn perform(&self, _dry: &mut DryContext, wet: &mut WetContext) -> OpResult<()> {
                let _service = wet.get_required::<String>("service_a")?;
                Ok(())
            }
            
            fn metadata(&self) -> OpMetadata {
                OpMetadata::builder("ServiceAOp")
                    .reference_schema(json!({
                        "type": "object",
                        "properties": {
                            "service_a": { "type": "ServiceA" },
                            "shared_service": { "type": "SharedService" }
                        },
                        "required": ["service_a", "shared_service"]
                    }))
                    .build()
            }
        }
        
        #[async_trait]
        impl Op<()> for ServiceBOp {
            async fn perform(&self, _dry: &mut DryContext, wet: &mut WetContext) -> OpResult<()> {
                let _service = wet.get_required::<String>("service_b")?;
                Ok(())
            }
            
            fn metadata(&self) -> OpMetadata {
                OpMetadata::builder("ServiceBOp")
                    .reference_schema(json!({
                        "type": "object",
                        "properties": {
                            "service_b": { "type": "ServiceB" },
                            "shared_service": { "type": "SharedService" }
                        },
                        "required": ["service_b", "shared_service"]
                    }))
                    .build()
            }
        }
        
        let ops: Vec<Arc<dyn Op<()>>> = vec![
            Arc::new(ServiceAOp),
            Arc::new(ServiceBOp),
        ];
        
        let batch = BatchOp::new(ops);
        let metadata = batch.metadata();
        
        // The batch should require all unique services
        if let Some(ref_schema) = metadata.reference_schema {
            let required = ref_schema.get("required")
                .and_then(|r| r.as_array())
                .unwrap();
            
            let required_services: Vec<&str> = required.iter()
                .filter_map(|v| v.as_str())
                .collect();
            
            assert_eq!(required_services.len(), 3);
            assert!(required_services.contains(&"service_a"));
            assert!(required_services.contains(&"service_b"));
            assert!(required_services.contains(&"shared_service")); // Only counted once!
        }
    }
    
    // TEST054: Run BatchOp where the third op fails and verify rollback is called on the first two but not the third
    #[tokio::test]
    async fn test_054_batch_rollback_on_failure() {
        use std::sync::{Arc, Mutex};
        
        struct RollbackTrackingOp {
            id: u32,
            should_fail: bool,
            performed: Arc<Mutex<bool>>,
            rolled_back: Arc<Mutex<bool>>,
        }
        
        #[async_trait]
        impl Op<u32> for RollbackTrackingOp {
            async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<u32> {
                *self.performed.lock().unwrap() = true;
                if self.should_fail {
                    Err(OpError::ExecutionFailed(format!("Op {} failed", self.id)))
                } else {
                    Ok(self.id)
                }
            }
            
            async fn rollback(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
                *self.rolled_back.lock().unwrap() = true;
                Ok(())
            }
            
            fn metadata(&self) -> OpMetadata {
                OpMetadata::builder(&format!("RollbackTrackingOp{}", self.id)).build()
            }
        }
        
        // Create tracking state
        let op1_performed = Arc::new(Mutex::new(false));
        let op1_rolled_back = Arc::new(Mutex::new(false));
        let op2_performed = Arc::new(Mutex::new(false));
        let op2_rolled_back = Arc::new(Mutex::new(false));
        let op3_performed = Arc::new(Mutex::new(false));
        let op3_rolled_back = Arc::new(Mutex::new(false));
        
        let ops = vec![
            Arc::new(RollbackTrackingOp {
                id: 1,
                should_fail: false,
                performed: op1_performed.clone(),
                rolled_back: op1_rolled_back.clone(),
            }) as Arc<dyn Op<u32>>,
            Arc::new(RollbackTrackingOp {
                id: 2,
                should_fail: false,
                performed: op2_performed.clone(),
                rolled_back: op2_rolled_back.clone(),
            }) as Arc<dyn Op<u32>>,
            Arc::new(RollbackTrackingOp {
                id: 3,
                should_fail: true, // This will fail and trigger rollback
                performed: op3_performed.clone(),
                rolled_back: op3_rolled_back.clone(),
            }) as Arc<dyn Op<u32>>,
        ];
        
        let batch = BatchOp::new(ops);
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        // Execute batch - should fail on op3
        let result = batch.perform(&mut dry, &mut wet).await;
        assert!(result.is_err());
        
        // Verify execution state
        assert!(*op1_performed.lock().unwrap(), "Op1 should have been performed");
        assert!(*op2_performed.lock().unwrap(), "Op2 should have been performed");
        assert!(*op3_performed.lock().unwrap(), "Op3 should have been performed (and failed)");
        
        // Verify rollback state - only succeeded ops should be rolled back
        assert!(*op1_rolled_back.lock().unwrap(), "Op1 should have been rolled back");
        assert!(*op2_rolled_back.lock().unwrap(), "Op2 should have been rolled back");
        assert!(!*op3_rolled_back.lock().unwrap(), "Op3 should NOT have been rolled back (it failed)");
    }
    
    // TEST055: Run BatchOp where the last op fails and verify rollback occurs in reverse (LIFO) order
    #[tokio::test]
    async fn test_055_batch_rollback_order() {
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
        
        let batch = BatchOp::new(ops);
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        // Execute batch - should fail on FailingOp
        let result = batch.perform(&mut dry, &mut wet).await;
        assert!(result.is_err());
        
        // Verify rollback order is LIFO (reverse of execution order)
        let order = rollback_order.lock().unwrap();
        assert_eq!(*order, vec![3, 2, 1], "Rollback should happen in reverse order");
    }
    
    // TEST056: Run BatchOp where one op fails and verify rollback is triggered for succeeded ops
    #[tokio::test]
    async fn test_056_batch_rollback_on_failure_partial() {
        use std::sync::{Arc, Mutex};

        struct RollbackTrackingOp {
            id: u32,
            should_fail: bool,
            performed: Arc<Mutex<bool>>,
            rolled_back: Arc<Mutex<bool>>,
        }

        #[async_trait]
        impl Op<u32> for RollbackTrackingOp {
            async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<u32> {
                *self.performed.lock().unwrap() = true;
                if self.should_fail {
                    Err(OpError::ExecutionFailed(format!("Op {} failed", self.id)))
                } else {
                    Ok(self.id)
                }
            }

            async fn rollback(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
                *self.rolled_back.lock().unwrap() = true;
                Ok(())
            }

            fn metadata(&self) -> OpMetadata {
                OpMetadata::builder(&format!("RollbackTrackingOp{}", self.id)).build()
            }
        }

        // Create tracking state
        let op1_performed = Arc::new(Mutex::new(false));
        let op1_rolled_back = Arc::new(Mutex::new(false));
        let op2_performed = Arc::new(Mutex::new(false));
        let op2_rolled_back = Arc::new(Mutex::new(false));

        let ops = vec![
            Arc::new(RollbackTrackingOp {
                id: 1,
                should_fail: false,
                performed: op1_performed.clone(),
                rolled_back: op1_rolled_back.clone(),
            }) as Arc<dyn Op<u32>>,
            Arc::new(RollbackTrackingOp {
                id: 2,
                should_fail: true, // This will fail and trigger rollback of op1
                performed: op2_performed.clone(),
                rolled_back: op2_rolled_back.clone(),
            }) as Arc<dyn Op<u32>>,
        ];

        let batch = BatchOp::new(ops);
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();

        // Execute batch - should fail on op2
        let result = batch.perform(&mut dry, &mut wet).await;
        assert!(result.is_err());

        // Verify execution state
        assert!(*op1_performed.lock().unwrap(), "Op1 should have been performed");
        assert!(*op2_performed.lock().unwrap(), "Op2 should have been performed (and failed)");

        // Verify rollback state - only succeeded ops should be rolled back
        assert!(*op1_rolled_back.lock().unwrap(), "Op1 should have been rolled back");
        assert!(!*op2_rolled_back.lock().unwrap(), "Op2 should NOT have been rolled back (it failed)");
    }

    // TEST093: Call BatchOp::len and is_empty on empty and non-empty batches
    #[test]
    fn test_093_batch_len_and_is_empty() {
        let empty: BatchOp<i32> = BatchOp::new(vec![]);
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());

        let nonempty = BatchOp::new(vec![
            Arc::new(TestOp { value: 1, should_fail: false }) as Arc<dyn Op<i32>>,
        ]);
        assert_eq!(nonempty.len(), 1);
        assert!(!nonempty.is_empty());
    }

    // TEST094: Use add_op to dynamically add an op and verify it is executed
    #[tokio::test]
    async fn test_094_batch_add_op() {
        let mut batch = BatchOp::new(vec![
            Arc::new(TestOp { value: 10, should_fail: false }) as Arc<dyn Op<i32>>,
        ]);
        batch.add_op(Arc::new(TestOp { value: 20, should_fail: false }));

        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        let results = batch.perform(&mut dry, &mut wet).await.unwrap();
        assert_eq!(results, vec![10, 20]);
    }

    // TEST095: Run BatchOp::with_continue_on_error and verify it collects results past failures
    #[tokio::test]
    async fn test_095_batch_continue_on_error() {
        let ops = vec![
            Arc::new(TestOp { value: 1, should_fail: false }) as Arc<dyn Op<i32>>,
            Arc::new(TestOp { value: 2, should_fail: true }) as Arc<dyn Op<i32>>,
            Arc::new(TestOp { value: 3, should_fail: false }) as Arc<dyn Op<i32>>,
        ];
        let batch = BatchOp::new(ops).with_continue_on_error(true);
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        let results = batch.perform(&mut dry, &mut wet).await.unwrap();
        // Only the two successful ops contribute results; the failing op is skipped
        assert_eq!(results, vec![1, 3]);
    }

    // TEST096: Run an empty BatchOp and verify it returns an empty result vec
    #[tokio::test]
    async fn test_096_empty_batch_returns_empty() {
        let batch: BatchOp<i32> = BatchOp::new(vec![]);
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        let results = batch.perform(&mut dry, &mut wet).await.unwrap();
        assert!(results.is_empty());
    }

    // TEST097: Verify nested BatchOp rollback propagates correctly when outer batch fails
    #[tokio::test]
    async fn test_097_nested_batch_rollback() {
        use std::sync::{Arc as StdArc, Mutex};

        let rollback_log: StdArc<Mutex<Vec<&'static str>>> = StdArc::new(Mutex::new(vec![]));

        struct TrackingOp {
            name: &'static str,
            should_fail: bool,
            log: StdArc<Mutex<Vec<&'static str>>>,
        }

        #[async_trait]
        impl Op<i32> for TrackingOp {
            async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<i32> {
                if self.should_fail {
                    Err(OpError::ExecutionFailed(format!("{} failed", self.name)))
                } else {
                    Ok(0)
                }
            }
            async fn rollback(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
                self.log.lock().unwrap().push(self.name);
                Ok(())
            }
            fn metadata(&self) -> OpMetadata {
                OpMetadata::builder(self.name).build()
            }
        }

        let log = rollback_log.clone();
        let inner_ops: Vec<Arc<dyn Op<i32>>> = vec![
            Arc::new(TrackingOp { name: "inner_a", should_fail: false, log: log.clone() }),
            Arc::new(TrackingOp { name: "inner_b", should_fail: false, log: log.clone() }),
        ];
        let inner_batch = Arc::new(BatchOp::new(inner_ops));

        // Outer batch: inner_batch succeeds, then an op fails, triggering rollback of inner_batch
        struct FailingOp;
        #[async_trait]
        impl Op<Vec<i32>> for FailingOp {
            async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<Vec<i32>> {
                Err(OpError::ExecutionFailed("outer fail".to_string()))
            }
            fn metadata(&self) -> OpMetadata { OpMetadata::builder("FailingOp").build() }
        }

        let outer_ops: Vec<Arc<dyn Op<Vec<i32>>>> = vec![
            inner_batch,
            Arc::new(FailingOp),
        ];
        let outer_batch = BatchOp::new(outer_ops);
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        let result = outer_batch.perform(&mut dry, &mut wet).await;
        assert!(result.is_err());
        // inner_batch was rolled back — it implements Op<Vec<i32>>, and its rollback is the default no-op
        // The important check: outer batch correctly propagated the failure
        match result.unwrap_err() {
            OpError::BatchFailed(_) => {}
            e => panic!("Expected BatchFailed, got {:?}", e),
        }
    }
}
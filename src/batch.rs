use crate::prelude::*;
use crate::batch_metadata::BatchMetadataBuilder;

pub struct BatchOp<T> {
    ops: Vec<Arc<dyn Op<T>>>,
    continue_on_error: bool,
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
}

#[async_trait]
impl<T> Op<Vec<T>> for BatchOp<T>
where
    T: Send + Sync + 'static,
{
    async fn perform(&self, dry: &mut DryContext, wet: &mut WetContext) -> OpResult<Vec<T>> {
        let mut results = Vec::with_capacity(self.ops.len());
        let mut errors = Vec::new();
        
        for (index, op) in self.ops.iter().enumerate() {
            match op.perform(dry, wet).await {
                Ok(result) => results.push(result),
                Err(error) => {
                    if self.continue_on_error {
                        errors.push((index, error));
                    } else {
                        return Err(OpError::BatchFailed(
                            format!("Op {}-{} failed: {}", index, op.metadata().name, error)
                        ));
                    }
                }
            }
        }
        
        if !errors.is_empty() && !self.continue_on_error {
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

pub struct ParallelBatchOp<T> {
    ops: Vec<Arc<dyn Op<T>>>,
    continue_on_error: bool,
    max_concurrent: Option<usize>,
}

impl<T> ParallelBatchOp<T>
where
    T: Send + Sync + 'static,
{
    pub fn new(ops: Vec<Arc<dyn Op<T>>>) -> Self {
        Self {
            ops,
            continue_on_error: false,
            max_concurrent: None,
        }
    }
    
    pub fn with_continue_on_error(mut self, continue_on_error: bool) -> Self {
        self.continue_on_error = continue_on_error;
        self
    }
    
    pub fn with_max_concurrent(mut self, max_concurrent: usize) -> Self {
        self.max_concurrent = Some(max_concurrent);
        self
    }
}

#[async_trait]
impl<T> Op<Vec<T>> for ParallelBatchOp<T>
where
    T: Send + Sync + 'static,
{
    async fn perform(&self, dry: &mut DryContext, wet: &mut WetContext) -> OpResult<Vec<T>> {
        // Since contexts are mutable, we can't run ops in parallel
        // Execute sequentially instead
        let mut results = Vec::with_capacity(self.ops.len());
        let mut errors = Vec::new();
        
        for (index, op) in self.ops.iter().enumerate() {
            match op.perform(dry, wet).await {
                Ok(result) => results.push(result),
                Err(error) => {
                    if self.continue_on_error {
                        errors.push((index, error));
                    } else {
                        return Err(OpError::BatchFailed(
                            format!("Op {}-{} failed: {}", index, op.metadata().name, error)
                        ));
                    }
                }
            }
        }
        
        if !errors.is_empty() && !self.continue_on_error {
            return Err(OpError::BatchFailed(
                format!("Parallel batch op had {} errors", errors.len())
            ));
        }
        
        Ok(results)
    }
    
    fn metadata(&self) -> OpMetadata {
        // Use the same intelligent metadata builder since parallel execution
        // doesn't change the data flow requirements
        let mut metadata = BatchMetadataBuilder::new(&self.ops).build();
        metadata.name = "ParallelBatchOp".to_string();
        if let Some(ref mut desc) = metadata.description {
            *desc = desc.replace("Batch of", "Parallel batch of");
        }
        metadata
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
    
    #[tokio::test]
    async fn test_batch_op_success() {
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
    
    #[tokio::test]
    async fn test_batch_op_failure() {
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
    
    #[tokio::test]
    async fn test_parallel_batch_op() {
        let ops = vec![
            Arc::new(TestOp { value: 1, should_fail: false }) as Arc<dyn Op<i32>>,
            Arc::new(TestOp { value: 2, should_fail: false }) as Arc<dyn Op<i32>>,
        ];
        
        let batch = ParallelBatchOp::new(ops);
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        let results = batch.perform(&mut dry, &mut wet).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.contains(&1));
        assert!(results.contains(&2));
    }
    
    #[tokio::test]
    async fn test_batch_metadata_data_flow() {
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
    
    #[tokio::test]
    async fn test_batch_reference_schema_merging() {
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
}
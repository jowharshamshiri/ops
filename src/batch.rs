use crate::prelude::*;

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
    async fn perform(&self, context: &mut OpContext) -> OpResult<Vec<T>> {
        let mut results = Vec::with_capacity(self.ops.len());
        let mut errors = Vec::new();
        
        for (index, op) in self.ops.iter().enumerate() {
            match op.perform(context).await {
                Ok(result) => results.push(result),
                Err(error) => {
                    if self.continue_on_error {
                        errors.push((index, error));
                    } else {
                        return Err(OpError::BatchFailed(
                            format!("Op {} failed: {}", index, error)
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
    async fn perform(&self, context: &mut OpContext) -> OpResult<Vec<T>> {
        use tokio::task::JoinSet;
        
        let mut join_set = JoinSet::new();
        let context_clone = context.clone();
        
        for (index, op) in self.ops.iter().enumerate() {
            let op = Arc::clone(op);
            let mut ctx = context_clone.clone();
            
            join_set.spawn(async move {
                let result = op.perform(&mut ctx).await;
                (index, result)
            });
            
            if let Some(max_concurrent) = self.max_concurrent {
                if join_set.len() >= max_concurrent {
                    if let Some(result) = join_set.join_next().await {
                        match result {
                            Ok((_, Ok(_))) => {},
                            Ok((idx, Err(e))) if !self.continue_on_error => {
                                return Err(OpError::BatchFailed(
                                    format!("Op {} failed: {}", idx, e)
                                ));
                            },
                            Err(join_error) => {
                                return Err(OpError::BatchFailed(
                                    format!("Task join error: {}", join_error)
                                ));
                            },
                            _ => {},
                        }
                    }
                }
            }
        }
        
        let mut results: Vec<Option<T>> = (0..self.ops.len()).map(|_| None).collect();
        let mut errors = Vec::new();
        
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok((index, Ok(value))) => {
                    results[index] = Some(value);
                },
                Ok((index, Err(error))) => {
                    if self.continue_on_error {
                        errors.push((index, error));
                    } else {
                        return Err(OpError::BatchFailed(
                            format!("Op {} failed: {}", index, error)
                        ));
                    }
                },
                Err(join_error) => {
                    return Err(OpError::BatchFailed(
                        format!("Task join error: {}", join_error)
                    ));
                }
            }
        }
        
        let final_results: Result<Vec<T>, _> = results
            .into_iter()
            .collect::<Option<Vec<T>>>()
            .ok_or_else(|| OpError::BatchFailed("Missing results".to_string()));
            
        final_results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    struct TestOp {
        value: i32,
        should_fail: bool,
    }
    
    #[async_trait]
    impl Op<i32> for TestOp {
        async fn perform(&self, _context: &mut OpContext) -> Result<i32, OpError> {
            if self.should_fail {
                Err(OpError::ExecutionFailed("Test failure".to_string()))
            } else {
                Ok(self.value)
            }
        }
    }
    
    #[tokio::test]
    async fn test_batch_op_success() {
        let ops = vec![
            Arc::new(TestOp { value: 1, should_fail: false }) as Arc<dyn Op<i32>>,
            Arc::new(TestOp { value: 2, should_fail: false }) as Arc<dyn Op<i32>>,
        ];
        
        let batch = BatchOp::new(ops);
        let mut ctx = OpContext::new();
        
        let results = batch.perform(&mut ctx).await.unwrap();
        assert_eq!(results, vec![1, 2]);
    }
    
    #[tokio::test]
    async fn test_batch_op_failure() {
        let ops = vec![
            Arc::new(TestOp { value: 1, should_fail: false }) as Arc<dyn Op<i32>>,
            Arc::new(TestOp { value: 2, should_fail: true }) as Arc<dyn Op<i32>>,
        ];
        
        let batch = BatchOp::new(ops);
        let mut ctx = OpContext::new();
        
        let result = batch.perform(&mut ctx).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_parallel_batch_op() {
        let ops = vec![
            Arc::new(TestOp { value: 1, should_fail: false }) as Arc<dyn Op<i32>>,
            Arc::new(TestOp { value: 2, should_fail: false }) as Arc<dyn Op<i32>>,
        ];
        
        let batch = ParallelBatchOp::new(ops);
        let mut ctx = OpContext::new();
        
        let results = batch.perform(&mut ctx).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.contains(&1));
        assert!(results.contains(&2));
    }
}
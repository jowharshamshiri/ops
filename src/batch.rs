use async_trait::async_trait;
use std::sync::Arc;
use crate::{Operation, OperationalContext, OperationError};

pub struct BatchOperation<T> {
    operations: Vec<Arc<dyn Operation<T>>>,
    continue_on_error: bool,
}

impl<T> BatchOperation<T>
where
    T: Send + Sync + 'static,
{
    pub fn new(operations: Vec<Arc<dyn Operation<T>>>) -> Self {
        Self {
            operations,
            continue_on_error: false,
        }
    }
    
    pub fn with_continue_on_error(mut self, continue_on_error: bool) -> Self {
        self.continue_on_error = continue_on_error;
        self
    }
    
    pub fn add_operation(&mut self, operation: Arc<dyn Operation<T>>) {
        self.operations.push(operation);
    }
    
    pub fn len(&self) -> usize {
        self.operations.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }
}

#[async_trait]
impl<T> Operation<Vec<T>> for BatchOperation<T>
where
    T: Send + Sync + 'static,
{
    async fn perform(&self, context: &mut OperationalContext) -> Result<Vec<T>, OperationError> {
        let mut results = Vec::with_capacity(self.operations.len());
        let mut errors = Vec::new();
        
        for (index, operation) in self.operations.iter().enumerate() {
            match operation.perform(context).await {
                Ok(result) => results.push(result),
                Err(error) => {
                    if self.continue_on_error {
                        errors.push((index, error));
                    } else {
                        return Err(OperationError::BatchFailed(
                            format!("Operation {} failed: {}", index, error)
                        ));
                    }
                }
            }
        }
        
        if !errors.is_empty() && !self.continue_on_error {
            return Err(OperationError::BatchFailed(
                format!("Batch operation had {} errors", errors.len())
            ));
        }
        
        Ok(results)
    }
}

pub struct ParallelBatchOperation<T> {
    operations: Vec<Arc<dyn Operation<T>>>,
    continue_on_error: bool,
    max_concurrent: Option<usize>,
}

impl<T> ParallelBatchOperation<T>
where
    T: Send + Sync + 'static,
{
    pub fn new(operations: Vec<Arc<dyn Operation<T>>>) -> Self {
        Self {
            operations,
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
impl<T> Operation<Vec<T>> for ParallelBatchOperation<T>
where
    T: Send + Sync + 'static,
{
    async fn perform(&self, context: &mut OperationalContext) -> Result<Vec<T>, OperationError> {
        use tokio::task::JoinSet;
        
        let mut join_set = JoinSet::new();
        let context_clone = context.clone();
        
        for (index, operation) in self.operations.iter().enumerate() {
            let op = Arc::clone(operation);
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
                                return Err(OperationError::BatchFailed(
                                    format!("Operation {} failed: {}", idx, e)
                                ));
                            },
                            Err(join_error) => {
                                return Err(OperationError::BatchFailed(
                                    format!("Task join error: {}", join_error)
                                ));
                            },
                            _ => {},
                        }
                    }
                }
            }
        }
        
        let mut results: Vec<Option<T>> = (0..self.operations.len()).map(|_| None).collect();
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
                        return Err(OperationError::BatchFailed(
                            format!("Operation {} failed: {}", index, error)
                        ));
                    }
                },
                Err(join_error) => {
                    return Err(OperationError::BatchFailed(
                        format!("Task join error: {}", join_error)
                    ));
                }
            }
        }
        
        let final_results: Result<Vec<T>, _> = results
            .into_iter()
            .collect::<Option<Vec<T>>>()
            .ok_or_else(|| OperationError::BatchFailed("Missing results".to_string()));
            
        final_results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    struct TestOperation {
        value: i32,
        should_fail: bool,
    }
    
    #[async_trait]
    impl Operation<i32> for TestOperation {
        async fn perform(&self, _context: &mut OperationalContext) -> Result<i32, OperationError> {
            if self.should_fail {
                Err(OperationError::ExecutionFailed("Test failure".to_string()))
            } else {
                Ok(self.value)
            }
        }
    }
    
    #[tokio::test]
    async fn test_batch_operation_success() {
        let ops = vec![
            Arc::new(TestOperation { value: 1, should_fail: false }) as Arc<dyn Operation<i32>>,
            Arc::new(TestOperation { value: 2, should_fail: false }) as Arc<dyn Operation<i32>>,
        ];
        
        let batch = BatchOperation::new(ops);
        let mut ctx = OperationalContext::new();
        
        let results = batch.perform(&mut ctx).await.unwrap();
        assert_eq!(results, vec![1, 2]);
    }
    
    #[tokio::test]
    async fn test_batch_operation_failure() {
        let ops = vec![
            Arc::new(TestOperation { value: 1, should_fail: false }) as Arc<dyn Operation<i32>>,
            Arc::new(TestOperation { value: 2, should_fail: true }) as Arc<dyn Operation<i32>>,
        ];
        
        let batch = BatchOperation::new(ops);
        let mut ctx = OperationalContext::new();
        
        let result = batch.perform(&mut ctx).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_parallel_batch_operation() {
        let ops = vec![
            Arc::new(TestOperation { value: 1, should_fail: false }) as Arc<dyn Operation<i32>>,
            Arc::new(TestOperation { value: 2, should_fail: false }) as Arc<dyn Operation<i32>>,
        ];
        
        let batch = ParallelBatchOperation::new(ops);
        let mut ctx = OperationalContext::new();
        
        let results = batch.perform(&mut ctx).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.contains(&1));
        assert!(results.contains(&2));
    }
}
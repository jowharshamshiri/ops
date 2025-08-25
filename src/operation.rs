use async_trait::async_trait;
use crate::{OperationalContext, OperationError};
use std::pin::Pin;
use std::future::Future;

#[async_trait]
pub trait Operation<T>: Send + Sync {
    async fn perform(&self, context: &mut OperationalContext) -> Result<T, OperationError>;
}

#[async_trait]
impl<T, F, Fut> Operation<T> for F
where
    F: Fn(&mut OperationalContext) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<T, OperationError>> + Send,
    T: Send,
{
    async fn perform(&self, context: &mut OperationalContext) -> Result<T, OperationError> {
        self(context).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    struct TestOperation {
        value: i32,
    }
    
    #[async_trait]
    impl Operation<i32> for TestOperation {
        async fn perform(&self, _context: &mut OperationalContext) -> Result<i32, OperationError> {
            Ok(self.value)
        }
    }
    
    #[tokio::test]
    async fn test_operation_execution() {
        let op = TestOperation { value: 42 };
        let mut ctx = OperationalContext::new();
        
        let result = op.perform(&mut ctx).await;
        assert_eq!(result.unwrap(), 42);
    }
    
    #[tokio::test]
    async fn test_closure_operation() {
        let mut ctx = OperationalContext::new();
        
        let op = |_ctx: &mut OperationalContext| async move {
            Ok::<i32, OperationError>(123)
        };
        
        let result = op.perform(&mut ctx).await;
        assert_eq!(result.unwrap(), 123);
    }
}

/// Closure-based operation implementation for testing and simple use cases
pub struct ClosureOperation<T, F> 
where 
    F: Fn(&mut OperationalContext) -> Pin<Box<dyn Future<Output = Result<T, OperationError>> + Send>> + Send + Sync,
{
    closure: F,
}

impl<T, F> ClosureOperation<T, F>
where 
    F: Fn(&mut OperationalContext) -> Pin<Box<dyn Future<Output = Result<T, OperationError>> + Send>> + Send + Sync,
{
    pub fn new(closure: F) -> Self {
        Self { closure }
    }
}

#[async_trait]
impl<T, F> Operation<T> for ClosureOperation<T, F>
where
    T: Send + 'static,
    F: Fn(&mut OperationalContext) -> Pin<Box<dyn Future<Output = Result<T, OperationError>> + Send>> + Send + Sync,
{
    async fn perform(&self, context: &mut OperationalContext) -> Result<T, OperationError> {
        (self.closure)(context).await
    }
}
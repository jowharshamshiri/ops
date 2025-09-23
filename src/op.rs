use crate::prelude::*;
use async_trait::async_trait;
use crate::{OpContext, OpError};
use std::pin::Pin;
use std::future::Future;

#[async_trait]
pub trait Op<T>: Send + Sync {
    async fn perform(&self, context: &mut OpContext) -> Result<T, OpError>;
}

#[async_trait]
impl<T, F, Fut> Op<T> for F
where
    F: Fn(&mut OpContext) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<T, OpError>> + Send,
    T: Send,
{
    async fn perform(&self, context: &mut OpContext) -> Result<T, OpError> {
        self(context).await
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
        async fn perform(&self, _context: &mut OpContext) -> Result<i32, OpError> {
            Ok(self.value)
        }
    }
    
    #[tokio::test]
    async fn test_op_execution() {
        let op = TestOp { value: 42 };
        let mut ctx = OpContext::new();
        
        let result = op.perform(&mut ctx).await;
        assert_eq!(result.unwrap(), 42);
    }
    
    #[tokio::test]
    async fn test_closure_op() {
        let mut ctx = OpContext::new();
        
        let op = |_ctx: &mut OpContext| async move {
            Ok::<i32, OpError>(123)
        };
        
        let result = op.perform(&mut ctx).await;
        assert_eq!(result.unwrap(), 123);
    }
}

/// Closure-based op implementation for testing and simple use cases
pub struct ClosureOp<T, F> 
where 
    F: Fn(&mut OpContext) -> Pin<Box<dyn Future<Output = Result<T, OpError>> + Send>> + Send + Sync,
{
    closure: F,
}

impl<T, F> ClosureOp<T, F>
where 
    F: Fn(&mut OpContext) -> Pin<Box<dyn Future<Output = Result<T, OpError>> + Send>> + Send + Sync,
{
    pub fn new(closure: F) -> Self {
        Self { closure }
    }
}

#[async_trait]
impl<T, F> Op<T> for ClosureOp<T, F>
where
    T: Send + 'static,
    F: Fn(&mut OpContext) -> Pin<Box<dyn Future<Output = Result<T, OpError>> + Send>> + Send + Sync,
{
    async fn perform(&self, context: &mut OpContext) -> Result<T, OpError> {
        (self.closure)(context).await
    }
}
use crate::prelude::*;
use async_trait::async_trait;
use crate::{Op, OpContext, OpError};

/// Loop operation that executes a batch of operations repeatedly until a limit is reached
pub struct LoopOp<T> {
    counter_var: String,
    limit: usize,
    ops: Vec<Box<dyn Op<T>>>,
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
    pub fn new(counter_var: String, limit: usize, ops: Vec<Box<dyn Op<T>>>) -> Self {
        Self {
            counter_var,
            limit,
            ops,
        }
    }

    /// Add an operation to the loop
    pub fn add_op(mut self, op: Box<dyn Op<T>>) -> Self {
        self.ops.push(op);
        self
    }

    /// Get the current counter value from context
    fn get_counter(&self, context: &OpContext) -> usize {
        context.get::<usize>(&self.counter_var).unwrap_or(0)
    }

    /// Set the counter value in context
    fn set_counter(&self, context: &mut OpContext, value: usize) -> Result<(), OpError> {
        context.put(&self.counter_var, value)
    }
}

#[async_trait]
impl<T> Op<Vec<T>> for LoopOp<T>
where
    T: Send + 'static,
{
    async fn perform(&self, context: &mut OpContext) -> Result<Vec<T>, OpError> {
        let mut results = Vec::new();
        let mut counter = self.get_counter(context);

        // Initialize counter in context if it doesn't exist
        if !context.contains_key(&self.counter_var) {
            self.set_counter(context, counter)?;
        }

        while counter < self.limit {
            // Execute all operations in the batch for this iteration
            for op in &self.ops {
                let result = op.perform(context).await?;
                results.push(result);
            }

            // Increment counter and update context
            counter += 1;
            self.set_counter(context, counter)?;
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OpContext;

    struct TestOp {
        value: i32,
    }

    #[async_trait]
    impl Op<i32> for TestOp {
        async fn perform(&self, _context: &mut OpContext) -> Result<i32, OpError> {
            Ok(self.value)
        }
    }

    struct CounterOp;

    #[async_trait]
    impl Op<usize> for CounterOp {
        async fn perform(&self, context: &mut OpContext) -> Result<usize, OpError> {
            let counter: usize = context.get("loop_counter").unwrap_or(0);
            Ok(counter)
        }
    }

    #[tokio::test]
    async fn test_loop_op_basic() {
        let mut ctx = OpContext::new();
        
        let ops: Vec<Box<dyn Op<i32>>> = vec![
            Box::new(TestOp { value: 10 }),
            Box::new(TestOp { value: 20 }),
        ];

        let loop_op = LoopOp::new("loop_counter".to_string(), 3, ops);
        let results = loop_op.perform(&mut ctx).await.unwrap();

        // Should have 6 results (2 ops * 3 iterations)
        assert_eq!(results.len(), 6);
        assert_eq!(results, vec![10, 20, 10, 20, 10, 20]);

        // Counter should be 3
        assert_eq!(ctx.get::<usize>("loop_counter"), Some(3));
    }

    #[tokio::test]
    async fn test_loop_op_with_counter_access() {
        let mut ctx = OpContext::new();
        
        let ops: Vec<Box<dyn Op<usize>>> = vec![
            Box::new(CounterOp),
        ];

        let loop_op = LoopOp::new("loop_counter".to_string(), 3, ops);
        let results = loop_op.perform(&mut ctx).await.unwrap();

        // Should have counter values: [0, 1, 2]
        assert_eq!(results, vec![0, 1, 2]);
        assert_eq!(ctx.get::<usize>("loop_counter"), Some(3));
    }

    #[tokio::test]
    async fn test_loop_op_existing_counter() {
        let mut ctx = OpContext::new();
        ctx.put("my_counter", 2_usize).unwrap();
        
        let ops: Vec<Box<dyn Op<i32>>> = vec![
            Box::new(TestOp { value: 42 }),
        ];

        let loop_op = LoopOp::new("my_counter".to_string(), 4, ops);
        let results = loop_op.perform(&mut ctx).await.unwrap();

        // Should execute 2 times (from 2 to 4)
        assert_eq!(results.len(), 2);
        assert_eq!(results, vec![42, 42]);
        assert_eq!(ctx.get::<usize>("my_counter"), Some(4));
    }

    #[tokio::test]
    async fn test_loop_op_zero_limit() {
        let mut ctx = OpContext::new();
        
        let ops: Vec<Box<dyn Op<i32>>> = vec![
            Box::new(TestOp { value: 99 }),
        ];

        let loop_op = LoopOp::new("counter".to_string(), 0, ops);
        let results = loop_op.perform(&mut ctx).await.unwrap();

        // Should not execute any operations
        assert_eq!(results.len(), 0);
        assert_eq!(ctx.get::<usize>("counter"), Some(0));
    }

    #[tokio::test]
    async fn test_loop_op_builder_pattern() {
        let mut ctx = OpContext::new();
        
        let loop_op = LoopOp::new("builder_counter".to_string(), 2, vec![])
            .add_op(Box::new(TestOp { value: 1 }))
            .add_op(Box::new(TestOp { value: 2 }));

        let results = loop_op.perform(&mut ctx).await.unwrap();

        assert_eq!(results.len(), 4);
        assert_eq!(results, vec![1, 2, 1, 2]);
        assert_eq!(ctx.get::<usize>("builder_counter"), Some(2));
    }
}
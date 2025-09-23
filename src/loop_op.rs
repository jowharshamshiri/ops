use crate::prelude::*;
use async_trait::async_trait;
use crate::{Op, DryContext, WetContext, OpMetadata};

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
    async fn perform(&self, dry: &DryContext, wet: &WetContext) -> OpResult<Vec<T>> {
        let mut results = Vec::new();
        let mut working_dry = dry.clone();
        let mut counter = self.get_counter(&working_dry);

        // Initialize counter in context if it doesn't exist
        if !working_dry.contains(&self.counter_var) {
            self.set_counter(&mut working_dry, counter);
        }

        while counter < self.limit {
            // Execute all operations in the batch for this iteration
            for op in &self.ops {
                let result = op.perform(&working_dry, wet).await?;
                results.push(result);
            }

            // Increment counter and update context
            counter += 1;
            self.set_counter(&mut working_dry, counter);
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
        async fn perform(&self, _dry: &DryContext, _wet: &WetContext) -> OpResult<i32> {
            Ok(self.value)
        }
        
        fn metadata(&self) -> OpMetadata {
            OpMetadata::builder("TestOp").build()
        }
    }

    struct CounterOp;

    #[async_trait]
    impl Op<usize> for CounterOp {
        async fn perform(&self, dry: &DryContext, _wet: &WetContext) -> OpResult<usize> {
            let counter: usize = dry.get("loop_counter").unwrap_or(0);
            Ok(counter)
        }
        
        fn metadata(&self) -> OpMetadata {
            OpMetadata::builder("CounterOp").build()
        }
    }

    #[tokio::test]
    async fn test_loop_op_basic() {
        let dry = DryContext::new();
        let wet = WetContext::new();
        
        let ops: Vec<Box<dyn Op<i32>>> = vec![
            Box::new(TestOp { value: 10 }),
            Box::new(TestOp { value: 20 }),
        ];

        let loop_op = LoopOp::new("loop_counter".to_string(), 3, ops);
        let results = loop_op.perform(&dry, &wet).await.unwrap();

        // Should have 6 results (2 ops * 3 iterations)
        assert_eq!(results.len(), 6);
        assert_eq!(results, vec![10, 20, 10, 20, 10, 20]);
    }

    #[tokio::test]
    async fn test_loop_op_with_counter_access() {
        let dry = DryContext::new();
        let wet = WetContext::new();
        
        let ops: Vec<Box<dyn Op<usize>>> = vec![
            Box::new(CounterOp),
        ];

        let loop_op = LoopOp::new("loop_counter".to_string(), 3, ops);
        let results = loop_op.perform(&dry, &wet).await.unwrap();

        // Should have counter values: [0, 1, 2]
        assert_eq!(results, vec![0, 1, 2]);
    }

    #[tokio::test]
    async fn test_loop_op_existing_counter() {
        let dry = DryContext::new().with_value("my_counter", 2_usize);
        let wet = WetContext::new();
        
        let ops: Vec<Box<dyn Op<i32>>> = vec![
            Box::new(TestOp { value: 42 }),
        ];

        let loop_op = LoopOp::new("my_counter".to_string(), 4, ops);
        let results = loop_op.perform(&dry, &wet).await.unwrap();

        // Should execute 2 times (from 2 to 4)
        assert_eq!(results.len(), 2);
        assert_eq!(results, vec![42, 42]);
    }

    #[tokio::test]
    async fn test_loop_op_zero_limit() {
        let dry = DryContext::new();
        let wet = WetContext::new();
        
        let ops: Vec<Box<dyn Op<i32>>> = vec![
            Box::new(TestOp { value: 99 }),
        ];

        let loop_op = LoopOp::new("counter".to_string(), 0, ops);
        let results = loop_op.perform(&dry, &wet).await.unwrap();

        // Should not execute any operations
        assert_eq!(results.len(), 0);
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
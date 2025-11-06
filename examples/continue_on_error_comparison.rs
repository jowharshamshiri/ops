use ops::prelude::*;
use ops::{repeat, DryContext, WetContext, Op, OpResult, OpMetadata, OpError};
use async_trait::async_trait;

#[derive(Debug)]
struct TestOp {
    name: String,
    counter_key: String,
}

impl TestOp {
    fn new(name: String, counter_key: String) -> Self {
        Self { name, counter_key }
    }
}

#[async_trait]
impl Op<()> for TestOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        let counter: usize = dry.get(&self.counter_key).unwrap_or(0);
        println!("{}: processing iteration {}", self.name, counter);
        
        // Simulate failure on iteration 2 for the first op
        if self.name == "FailingOp" && counter == 2 {
            return Err(OpError::ExecutionFailed("Intentional failure".to_string()));
        }
        
        Ok(())
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder(&self.name).build()
    }
}

// Without continue_on_error (default behavior)
repeat! {
    StrictLoopOp<()> = {
        counter: "counter",
        limit: "limit",
        ops: [
            TestOp::new("FailingOp".to_string(), "counter".to_string()),
            TestOp::new("SecondOp".to_string(), "counter".to_string())
        ]
    }
}

// With continue_on_error enabled
repeat! {
    ResilientLoopOp<()> = {
        counter: "counter",
        limit: "limit",
        continue_on_error: true,
        ops: [
            TestOp::new("FailingOp".to_string(), "counter".to_string()),
            TestOp::new("SecondOp".to_string(), "counter".to_string())
        ]
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Testing WITHOUT continue_on_error ===");
    
    let mut dry = DryContext::new().with_value("limit", 5_usize);
    let mut wet = WetContext::new();
    
    let strict_loop = StrictLoopOp::new();
    match strict_loop.perform(&mut dry, &mut wet).await {
        Ok(_) => println!("Strict loop completed successfully"),
        Err(e) => println!("Strict loop failed: {}", e),
    }
    
    println!("\n=== Testing WITH continue_on_error=true ===");
    
    let mut dry = DryContext::new().with_value("limit", 5_usize);
    let mut wet = WetContext::new();
    
    let resilient_loop = ResilientLoopOp::new();
    match resilient_loop.perform(&mut dry, &mut wet).await {
        Ok(_) => println!("Resilient loop completed successfully"),
        Err(e) => println!("Resilient loop failed: {}", e),
    }
    
    Ok(())
}
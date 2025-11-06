use ops::prelude::*;
use ops::{abort, continue_loop, check_abort, BatchOp, LoopOp};
use std::sync::Arc;

/// Example op that aborts based on input
struct ConditionalAbortOp {
    should_abort: bool,
    value: i32,
}

#[async_trait]
impl Op<i32> for ConditionalAbortOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<i32> {
        // Always check for existing abort before proceeding
        check_abort!(dry);
        
        if self.should_abort {
            abort!(dry, format!("Conditional abort with value {}", self.value));
        }
        
        Ok(self.value)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("ConditionalAbortOp")
            .description("Op that may abort based on condition")
            .build()
    }
}

/// Example op that continues in loops based on input
struct ConditionalContinueOp {
    should_continue: bool,
    value: i32,
}

#[async_trait]
impl Op<i32> for ConditionalContinueOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<i32> {
        // Check abort before processing
        check_abort!(dry);
        
        if self.should_continue {
            info!("Continuing loop iteration early");
            continue_loop!(dry);
        }
        
        Ok(self.value)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("ConditionalContinueOp")
            .description("Op that may continue loop based on condition")
            .build()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    println!("=== Control Flow Demo ===");
    
    // Example 1: Batch operation with abort
    println!("\n1. Batch operation with abort:");
    let mut dry = DryContext::new();
    let mut wet = WetContext::new();
    
    let batch_ops = vec![
        Arc::new(ConditionalAbortOp { should_abort: false, value: 10 }) as Arc<dyn Op<i32>>,
        Arc::new(ConditionalAbortOp { should_abort: false, value: 20 }) as Arc<dyn Op<i32>>,
        Arc::new(ConditionalAbortOp { should_abort: true, value: 30 }) as Arc<dyn Op<i32>>,
        Arc::new(ConditionalAbortOp { should_abort: false, value: 40 }) as Arc<dyn Op<i32>>, // Won't execute
    ];
    
    let batch = BatchOp::new(batch_ops);
    
    match batch.perform(&mut dry, &mut wet).await {
        Ok(results) => println!("Batch results: {:?}", results),
        Err(OpError::Aborted(reason)) => println!("Batch aborted: {}", reason),
        Err(e) => println!("Batch error: {}", e),
    }
    
    // Example 2: Loop operation with continue
    println!("\n2. Loop operation with continue:");
    dry.clear_control_flags(); // Reset control flags
    
    let loop_ops: Vec<std::sync::Arc<dyn Op<i32>>> = vec![
        std::sync::Arc::new(ConditionalContinueOp { should_continue: false, value: 100 }),
        std::sync::Arc::new(ConditionalContinueOp { should_continue: true, value: 200 }), // Will continue
        std::sync::Arc::new(ConditionalAbortOp { should_abort: false, value: 300 }), // Won't execute due to continue
    ];
    
    let loop_op = LoopOp::new("demo_counter".to_string(), 3, loop_ops);
    
    match loop_op.perform(&mut dry, &mut wet).await {
        Ok(results) => {
            println!("Loop results: {:?}", results);
            println!("Result count: {}", results.len());
        },
        Err(e) => println!("Loop error: {}", e),
    }
    
    // Example 3: Demonstrating check_abort
    println!("\n3. Check abort functionality:");
    dry.clear_control_flags();
    dry.set_abort(Some("Pre-existing abort condition".to_string()));
    
    let check_op = ConditionalAbortOp { should_abort: false, value: 999 };
    
    match check_op.perform(&mut dry, &mut wet).await {
        Ok(result) => println!("Operation succeeded: {}", result),
        Err(OpError::Aborted(reason)) => println!("Operation aborted due to pre-existing condition: {}", reason),
        Err(e) => println!("Operation error: {}", e),
    }
    
    // Example 4: Error handling that doesn't trigger retries
    println!("\n4. Abort vs regular errors:");
    println!("   - Aborted errors should NOT trigger retries");
    println!("   - Regular errors MAY trigger retries");
    println!("   - Use abort!() when you know retrying won't help");
    
    Ok(())
}
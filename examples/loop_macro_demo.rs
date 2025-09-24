use ops::prelude::*;
use ops::repeat;
use serde_json::json;

// Op that increments a value
struct IncrementOp;

#[async_trait]
impl Op<i32> for IncrementOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<i32> {
        // Get current value or start at 0
        let current = dry.get::<i32>("value").unwrap_or(0);
        let iteration = dry.get::<usize>("loop_iteration").unwrap_or(0);
        
        let new_value = current + 1;
        dry.insert("value", new_value);
        
        println!("Iteration {}: incremented value to {}", iteration, new_value);
        
        Ok(new_value)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("IncrementOp")
            .description("Increments a value")
            .input_schema(json!({
                "type": "object",
                "properties": {
                    "value": { "type": "integer" }
                }
            }))
            .output_schema(json!({ "type": "integer" }))
            .build()
    }
}

// Op that logs the current state
struct LogStateOp;

#[async_trait]
impl Op<String> for LogStateOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<String> {
        let value = dry.get::<i32>("value").unwrap_or(0);
        let iteration = dry.get::<usize>("loop_iteration").unwrap_or(0);
        
        let message = format!("State at iteration {}: value = {}", iteration, value);
        println!("  {}", message);
        
        Ok(message)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("LogStateOp")
            .description("Logs the current state")
            .build()
    }
}

// Define our loop using the macro - increments and logs 5 times
repeat! {
    IncrementLoop<i32> = {
        counter: "loop_iteration",
        limit: 5,
        ops: [
            IncrementOp,
            IncrementOp  // Double increment per iteration
        ]
    }
}

// Another loop that does logging
repeat! {
    LoggingLoop<String> = {
        counter: "log_counter",
        limit: 3,
        ops: [
            LogStateOp
        ]
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Loop Macro Demo ===\n");
    
    // Test the increment loop
    let mut dry = DryContext::new()
        .with_value("value", 0i32);
    let mut wet = WetContext::new();
    
    println!("Running increment loop (5 iterations, 2 increments each):");
    let increment_loop = IncrementLoop::new();
    let results = increment_loop.perform(&mut dry, &mut wet).await?;
    
    println!("\nIncrement loop results: {} values", results.len());
    println!("Final value in context: {:?}", dry.get::<i32>("value"));
    
    // Test the logging loop
    println!("\n--- Testing logging loop ---\n");
    
    let logging_loop = LoggingLoop::new();
    let log_results = logging_loop.perform(&mut dry, &mut wet).await?;
    
    println!("\nLogging loop produced {} messages", log_results.len());
    
    // Demonstrate loop metadata
    println!("\n--- Loop Metadata ---\n");
    
    let loop_metadata = increment_loop.metadata();
    println!("Loop metadata:");
    println!("  Name: {}", loop_metadata.name);
    println!("  Description: {}", loop_metadata.description.unwrap_or_default());
    
    Ok(())
}
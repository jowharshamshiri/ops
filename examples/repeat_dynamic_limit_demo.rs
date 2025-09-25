use ops::prelude::*;
use ops::repeat;

// Simple op that prints the iteration
struct PrintOp;

#[async_trait]
impl Op<String> for PrintOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<String> {
        // Try both possible counter names
        let iteration = dry.get::<usize>("iteration_count")
            .or_else(|| dry.get::<usize>("counter"))
            .unwrap_or(0);
        let msg = format!("Iteration {}", iteration);
        println!("{}", msg);
        Ok(msg)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("PrintOp").build()
    }
}

// Loop with dynamic limit from context
repeat! {
    DynamicLoop<String> = {
        counter: "iteration_count",
        limit: "max_iterations",  // This will be loaded from dry context
        ops: [PrintOp]
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Dynamic Limit Demo ===\n");
    
    // Test dynamic loop
    println!("1. Dynamic loop (limit from context):");
    let mut dry2 = DryContext::new()
        .with_value("max_iterations", 5usize);  // Set limit in context
    let mut wet2 = WetContext::new();
    
    
    let dynamic_loop = DynamicLoop::new();
    
    // Show metadata - it should require max_iterations
    let metadata = dynamic_loop.metadata();
    println!("  Required inputs: {:?}", 
        metadata.input_schema.as_ref()
            .and_then(|s| s.get("required"))
            .and_then(|r| r.as_array()));
    
    let _results = dynamic_loop.perform(&mut dry2, &mut wet2).await?;
    
    // Test changing the limit
    println!("\n2. Same loop with different limit:");
    dry2.insert("max_iterations", 2usize);  // Change limit
    let _results = dynamic_loop.perform(&mut dry2, &mut wet2).await?;
    
    // Test missing limit
    println!("\n3. Testing error when limit is missing:");
    let mut dry3 = DryContext::new();  // No max_iterations set
    let mut wet3 = WetContext::new();
    
    match dynamic_loop.perform(&mut dry3, &mut wet3).await {
        Ok(_) => println!("  Unexpected success"),
        Err(e) => println!("  Expected error: {}", e),
    }
    
    Ok(())
}
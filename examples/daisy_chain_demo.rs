use ops::prelude::*;
use ops::{dry_put, dry_get, dry_require, dry_result, perform};

// Define ops that can work in a daisy chain
struct DoubleNumberOp;

#[async_trait]
impl Op<i32> for DoubleNumberOp {
    async fn perform(&self, dry: &DryContext, _wet: &WetContext) -> OpResult<i32> {
        let x: i32 = dry_require!(dry, x)?;
        Ok(x * 2)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("DoubleNumberOp")
            .description("Doubles a number")
            .build()
    }
}

struct AddTenOp;

#[async_trait]
impl Op<i32> for AddTenOp {
    async fn perform(&self, dry: &DryContext, _wet: &WetContext) -> OpResult<i32> {
        // Try to get from "result" first (for chaining), then from "x"
        let x: i32 = if let Some(result) = dry_get!(dry, result) {
            result
        } else {
            dry_require!(dry, x)?
        };
        Ok(x + 10)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("AddTenOp")
            .description("Adds 10 to a number")
            .build()
    }
}

struct FormatResultOp;

#[async_trait]
impl Op<String> for FormatResultOp {
    async fn perform(&self, dry: &DryContext, _wet: &WetContext) -> OpResult<String> {
        let x: i32 = if let Some(result) = dry_get!(dry, result) {
            result
        } else {
            dry_require!(dry, x)?
        };
        Ok(format!("Final result: {}", x))
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("FormatResultOp")
            .description("Formats a number as a string")
            .build()
    }
}

struct GreetPersonOp;

#[async_trait]
impl Op<String> for GreetPersonOp {
    async fn perform(&self, dry: &DryContext, _wet: &WetContext) -> OpResult<String> {
        let name: String = dry_require!(dry, name)?;
        Ok(format!("Hello, {}!", name))
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("GreetPersonOp")
            .description("Greets a person by name")
            .build()
    }
}

struct AppendSuffixOp;

#[async_trait]
impl Op<String> for AppendSuffixOp {
    async fn perform(&self, dry: &DryContext, _wet: &WetContext) -> OpResult<String> {
        let text: String = if let Some(result) = dry_get!(dry, result) {
            result
        } else {
            dry_require!(dry, text)?
        };
        Ok(format!("{} Have a great day!", text))
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("AppendSuffixOp")
            .description("Appends a suffix to text")
            .build()
    }
}

// Helper function to chain operations by storing results
async fn chain_ops<T: Send + Serialize + Clone>(
    mut dry: DryContext,
    wet: &WetContext,
    ops: Vec<Box<dyn Op<T>>>,
) -> OpResult<T> {
    let mut last_result = None;
    
    for op in ops {
        let result = op.perform(&dry, wet).await?;
        
        // Store result for next op to potentially use
        dry_result!(dry, "op_result", result.clone());
        last_result = Some(result);
    }
    
    last_result.ok_or_else(|| OpError::ExecutionFailed("No ops provided".to_string()))
}

#[tokio::main]
async fn main() -> OpResult<()> {
    tracing_subscriber::fmt::init();
    println!("=== Testing Daisy Chain Ops with Dry/Wet Contexts ===\n");

    let wet = WetContext::new(); // No services needed for this demo

    // Example 1: Simple number processing chain
    println!("1. Number processing chain: 5 -> double -> add 10 -> format");
    
    let mut dry = DryContext::new();
    let x = 5;
    dry_put!(dry, x);
    
    // Execute first op
    let double_op = DoubleNumberOp;
    let doubled = double_op.perform(&dry, &wet).await?;
    println!("After doubling: {}", doubled);
    
    // Store result and continue chain
    let result = doubled;
    dry_put!(dry, result);
    
    let add_op = AddTenOp;
    let added = add_op.perform(&dry, &wet).await?;
    println!("After adding 10: {}", added);
    
    // Store and format
    let result = added;
    dry_put!(dry, result);
    
    let format_op = FormatResultOp;
    let formatted = format_op.perform(&dry, &wet).await?;
    println!("Final result: {}\n", formatted);

    // Example 2: Text processing chain
    println!("2. Text processing chain: World -> greet -> append suffix");
    
    let mut dry = DryContext::new();
    let name = "World".to_string();
    dry_put!(dry, name);
    
    let greet_op = GreetPersonOp;
    let greeting = greet_op.perform(&dry, &wet).await?;
    println!("After greeting: {}", greeting);
    
    // Store result for next op
    let result = greeting;
    dry_put!(dry, result);
    
    let suffix_op = AppendSuffixOp;
    let final_text = suffix_op.perform(&dry, &wet).await?;
    println!("Final result: {}\n", final_text);

    // Example 3: Using the perform utility for automatic logging
    println!("3. Using perform utility with logging:");
    
    let mut dry = DryContext::new();
    let x = 7;
    dry_put!(dry, x);
    
    let result = perform(Box::new(DoubleNumberOp), &dry, &wet).await?;
    println!("Doubled result: {}", result);
    
    // Example 4: Error handling - missing input
    println!("\n4. Error handling - missing input:");
    let empty_dry = DryContext::new();
    
    match DoubleNumberOp.perform(&empty_dry, &wet).await {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("Expected error: {}", e),
    }
    
    // Example 5: Demonstrate dry_get for optional chaining
    println!("\n5. Optional chaining with dry_get:");
    
    let mut dry = DryContext::new();
    // Don't set x, but set result to see fallback behavior
    let result = 15;
    dry_put!(dry, result);
    
    let add_result = AddTenOp.perform(&dry, &wet).await?;
    println!("Using 'result' value (15) + 10 = {}", add_result);
    
    // Example 6: Show all available metadata
    println!("\n6. Op Metadata:");
    let ops: Vec<Box<dyn Op<i32>>> = vec![
        Box::new(DoubleNumberOp),
        Box::new(AddTenOp),
    ];
    
    for op in ops {
        let metadata = op.metadata();
        println!("- {}: {}", metadata.name, metadata.description.unwrap_or("No description".to_string()));
    }

    Ok(())
}
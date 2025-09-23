use ops::prelude::*;
use ops::{dry_put, dry_get, dry_require, wet_put_ref, wet_require_ref};
use std::sync::Arc;

// Example service for demonstration
struct GreetingService {
    prefix: String,
}

impl GreetingService {
    fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
        }
    }
    
    fn greet(&self, name: &str) -> String {
        format!("{} {}!", self.prefix, name)
    }
}

// Modern op implementation using dry/wet contexts
struct HelloOp;

#[async_trait]
impl Op<String> for HelloOp {
    async fn perform(&self, dry: &mut DryContext, wet: &mut WetContext) -> OpResult<String> {
        let name: String = dry_require!(dry, name)?;
        let greeting_service: Arc<GreetingService> = wet_require_ref!(wet, greeting_service)?;
        
        Ok(greeting_service.greet(&name))
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("HelloOp")
            .description("Greets a person using a greeting service")
            .build()
    }
}

struct MathOp;

#[async_trait]
impl Op<i32> for MathOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<i32> {
        let x: i32 = dry_require!(dry, x)?;
        Ok(x * 2 + 1)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("MathOp")
            .description("Performs math operation: x * 2 + 1")
            .build()
    }
}

struct ComplexOp;

#[async_trait]
impl Op<String> for ComplexOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<String> {
        let data: Vec<String> = dry_require!(dry, data)?;
        
        if data.is_empty() {
            Err(OpError::ExecutionFailed("Empty input".to_string()))
        } else {
            Ok(data.join(", "))
        }
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("ComplexOp")
            .description("Joins a vector of strings")
            .build()
    }
}

struct AddOp;

#[async_trait]
impl Op<i32> for AddOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<i32> {
        let a: i32 = dry_require!(dry, a)?;
        let b: i32 = dry_require!(dry, b)?;
        Ok(a + b)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("AddOp")
            .description("Adds two integers")
            .build()
    }
}

#[tokio::main]
async fn main() -> OpResult<()> {
    tracing_subscriber::fmt::init();
    println!("=== Testing modern Op implementations with dry/wet contexts ===");
    
    // Create wet context with services
    let mut wet = WetContext::new();
    let greeting_service = GreetingService::new("Hello");
    wet_put_ref!(wet, greeting_service);
    
    // Test HelloOp
    let mut dry = DryContext::new();
    let name = "World".to_string();
    dry_put!(dry, name);
    
    let hello_op = HelloOp;
    let result = hello_op.perform(&mut dry, &mut wet).await?;
    println!("HelloOp result: {}", result);
    
    // Test MathOp
    let mut dry = DryContext::new();
    let x = 5;
    dry_put!(dry, x);
    
    let math_op = MathOp;
    let result = math_op.perform(&mut dry, &mut wet).await?;
    println!("MathOp result: {}", result);
    
    // Test ComplexOp with data
    let mut dry = DryContext::new();
    let data = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    dry_put!(dry, data);
    
    let complex_op = ComplexOp;
    let result = complex_op.perform(&mut dry, &mut wet).await?;
    println!("ComplexOp result: {}", result);
    
    // Test ComplexOp with empty data
    let mut dry = DryContext::new();
    let data: Vec<String> = vec![];
    dry_put!(dry, data);
    
    match complex_op.perform(&mut dry, &mut wet).await {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("Expected error: {}", e),
    }
    
    // Test AddOp
    let mut dry = DryContext::new();
    let a = 10;
    let b = 20;
    dry_put!(dry, a);
    dry_put!(dry, b);
    
    let add_op = AddOp;
    let result = add_op.perform(&mut dry, &mut wet).await?;
    println!("AddOp result: {}", result);
    
    // Test error case - missing input
    let mut empty_dry = DryContext::new();
    match hello_op.perform(&mut empty_dry, &mut wet).await {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("Expected missing input error: {}", e),
    }
    
    // Demonstrate using dry_get for optional values
    let optional_value: Option<String> = dry_get!(empty_dry, name);
    println!("Optional value from empty context: {:?}", optional_value);
    
    // Show metadata
    println!("\n=== Op Metadata ===");
    println!("HelloOp: {}", hello_op.metadata().name);
    println!("MathOp: {}", math_op.metadata().name);
    println!("ComplexOp: {}", complex_op.metadata().name);
    println!("AddOp: {}", add_op.metadata().name);

    Ok(())
}
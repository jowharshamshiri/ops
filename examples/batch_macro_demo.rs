use ops::prelude::*;
use ops::batch;
use serde_json::json;

// First op: validates input
struct ValidationOp;

#[async_trait]
impl Op<String> for ValidationOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<String> {
        let input = dry.get_required::<String>("input")?;
        
        if input.is_empty() {
            return Err(OpError::ExecutionFailed("Input cannot be empty".to_string()));
        }
        
        // Store validated input for next op
        dry.insert("validated_input", input.clone());
        
        Ok(format!("Validated: {}", input))
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("ValidationOp")
            .description("Validates input is not empty")
            .input_schema(json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string" }
                },
                "required": ["input"]
            }))
            .output_schema(json!({
                "type": "object",
                "properties": {
                    "validated_input": { "type": "string" }
                }
            }))
            .build()
    }
}

// Second op: transforms the validated input
struct TransformOp;

#[async_trait]
impl Op<String> for TransformOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<String> {
        let input = dry.get_required::<String>("validated_input")?;
        
        let transformed = input.to_uppercase();
        
        // Store for next op
        dry.insert("transformed_input", transformed.clone());
        
        Ok(format!("Transformed: {}", transformed))
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("TransformOp")
            .description("Transforms input to uppercase")
            .input_schema(json!({
                "type": "object",
                "properties": {
                    "validated_input": { "type": "string" }
                },
                "required": ["validated_input"]
            }))
            .output_schema(json!({
                "type": "object", 
                "properties": {
                    "transformed_input": { "type": "string" }
                }
            }))
            .build()
    }
}

// Third op: persists the result
struct PersistOp;

#[async_trait]
impl Op<String> for PersistOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<String> {
        let input = dry.get_required::<String>("transformed_input")?;
        
        // In a real app, this would use a database service from wet context
        println!("Persisting: {}", input);
        
        // Store final result
        dry.insert("final_result", input.clone());
        
        Ok(format!("Persisted: {}", input))
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("PersistOp")
            .description("Persists the transformed data")
            .input_schema(json!({
                "type": "object",
                "properties": {
                    "transformed_input": { "type": "string" }
                },
                "required": ["transformed_input"]
            }))
            .output_schema(json!({
                "type": "object",
                "properties": {
                    "final_result": { "type": "string" }
                }
            }))
            .build()
    }
}

// Define our batch op using the macro
batch! {
    ProcessingPipeline<String> = [
        ValidationOp,
        TransformOp,
        PersistOp
    ]
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Batch Macro Demo ===\n");
    
    // Create a dry context with initial input
    let mut dry = DryContext::new()
        .with_value("input", "hello world");
    let mut wet = WetContext::new();
    
    // Create our pipeline using the generated type
    let pipeline = ProcessingPipeline::new();
    
    // Check the metadata - it should only require "input" since the other
    // requirements are satisfied by previous ops in the batch
    let metadata = pipeline.metadata();
    println!("Pipeline metadata:");
    println!("  Name: {}", metadata.name);
    println!("  Description: {}", metadata.description.unwrap_or_default());
    
    if let Some(input_schema) = &metadata.input_schema {
        if let Some(required) = input_schema.get("required").and_then(|r| r.as_array()) {
            println!("  Required inputs: {:?}", required);
        }
    }
    
    println!("\nExecuting pipeline...\n");
    
    // Execute the pipeline
    let results = pipeline.perform(&mut dry, &mut wet).await?;
    
    // Display results
    println!("Pipeline results:");
    for (i, result) in results.iter().enumerate() {
        println!("  Op {}: {}", i + 1, result);
    }
    
    println!("\nFinal context state:");
    println!("  final_result: {:?}", dry.get::<String>("final_result"));
    
    // Test with continue_on_error
    println!("\n--- Testing with empty input (will fail validation) ---\n");
    
    let mut dry_fail = DryContext::new()
        .with_value("input", ""); // Empty input will fail validation
    let mut wet_fail = WetContext::new();
    
    let pipeline_continue = ProcessingPipeline::new()
        .with_continue_on_error(true);
    
    match pipeline_continue.perform(&mut dry_fail, &mut wet_fail).await {
        Ok(results) => {
            println!("Results with continue_on_error: {:?}", results);
        }
        Err(e) => {
            println!("Pipeline failed: {}", e);
        }
    }
    
    Ok(())
}
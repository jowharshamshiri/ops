use ops::prelude::*;
use ops::{
    dry_put, dry_get, dry_require, dry_result,
    wet_put_ref, wet_put_arc, wet_get_ref, wet_require_ref,
};
use std::sync::Arc;

// Example service for wet context
struct DataService {
    name: String,
}

impl DataService {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    fn process(&self, data: &str) -> String {
        format!("{} processed by {}", data, self.name)
    }
}

// Example op that uses the macros
struct ProcessDataOp;

#[async_trait]
impl Op<String> for ProcessDataOp {
    async fn perform(&self, dry: &DryContext, wet: &WetContext) -> OpResult<String> {
        // Using dry_require for dry context
        let input_data: String = dry_require!(dry, input_data)?;
        let process_count: i32 = dry_require!(dry, process_count)?;
        
        // Using wet_require_ref for wet context
        let data_service: Arc<DataService> = wet_require_ref!(wet, data_service)?;
        
        // Process the data
        let mut result = input_data;
        for _ in 0..process_count {
            result = data_service.process(&result);
        }
        
        Ok(result)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("ProcessDataOp")
            .description("Processes data using a service with iteration count")
            .input_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "input_data": {
                        "type": "string",
                        "minLength": 1,
                        "description": "The data to process"
                    },
                    "process_count": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 10,
                        "description": "Number of times to process the data"
                    }
                },
                "required": ["input_data", "process_count"],
                "additionalProperties": false
            }))
            .reference_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "data_service": {
                        "type": "DataService",
                        "description": "Service for processing data"
                    }
                },
                "required": ["data_service"],
                "additionalProperties": false
            }))
            .output_schema(serde_json::json!({
                "type": "string",
                "description": "The processed result after all iterations"
            }))
            .build()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Context Macros Demo ===\n");
    
    // Create dry context and use ctx_put
    let mut dry = DryContext::new();
    let input_data = "Hello World".to_string();
    let process_count = 3;
    
    // Using dry_put macro for dry context
    dry_put!(dry, input_data);
    dry_put!(dry, process_count);
    
    // Verify with dry_get
    let retrieved_data: Option<String> = dry_get!(dry, input_data);
    println!("Retrieved data: {:?}", retrieved_data);
    
    // Create wet context
    let mut wet = WetContext::new();
    let data_service = DataService::new("MainProcessor");
    
    // Using wet_put_ref macro for wet context
    wet_put_ref!(wet, data_service);
    
    // Example with Arc
    let shared_service = Arc::new(DataService::new("SharedProcessor"));
    wet_put_arc!(wet, shared_service);
    
    // Execute the op
    let op = ProcessDataOp;
    let result = op.perform(&dry, &wet).await?;
    println!("\nProcessed result: {}", result);
    
    // Demonstrate dry_result macro
    let mut result_dry = DryContext::new();
    let final_result = "Final processed data".to_string();
    dry_result!(result_dry, "ProcessDataOp", final_result);
    
    // Verify result storage
    let stored_result: Option<String> = dry_get!(result_dry, result);
    let op_result: Option<String> = result_dry.get("ProcessDataOp");
    
    println!("\nResult stored under 'result': {:?}", stored_result);
    println!("Result stored under op name: {:?}", op_result);
    
    // Also demonstrate the explicitly named macros
    println!("\n=== Using explicitly named macros ===");
    
    let mut dry2 = DryContext::new();
    let test_value = 42;
    dry_put!(dry2, test_value);
    
    let retrieved: Option<i32> = dry_get!(dry2, test_value);
    println!("Retrieved using dry_get: {:?}", retrieved);
    
    let mut wet2 = WetContext::new();
    let service2 = DataService::new("SecondaryProcessor");
    wet_put_ref!(wet2, service2);
    
    let service_ref: Option<Arc<DataService>> = wet_get_ref!(wet2, service2);
    println!("Retrieved service ref: {}", service_ref.is_some());
    
    Ok(())
}
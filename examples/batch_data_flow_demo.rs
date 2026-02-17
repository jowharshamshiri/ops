use ops::prelude::*;
use ops::{BatchOp, Op, DryContext, WetContext, OpMetadata};
use std::sync::Arc;
use serde_json::json;

// Op that produces user data
struct FetchUserOp;

#[async_trait]
impl Op<()> for FetchUserOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        let user_id = dry.get_required::<i32>("user_id")?;

        // Simulate fetching user data
        dry.insert("username", format!("user_{}", user_id));
        dry.insert("email", format!("user_{}@example.com", user_id));
        dry.insert("age", 25);
        
        Ok(())
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("FetchUserOp")
            .description("Fetches user data from database")
            .input_schema(json!({
                "type": "object",
                "properties": {
                    "user_id": { "type": "integer" }
                },
                "required": ["user_id"]
            }))
            .output_schema(json!({
                "type": "object",
                "properties": {
                    "username": { "type": "string" },
                    "email": { "type": "string" },
                    "age": { "type": "integer" }
                }
            }))
            .build()
    }
}

// Op that validates user data (consumes username and email)
struct ValidateUserOp;

#[async_trait]
impl Op<()> for ValidateUserOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        let username = dry.get_required::<String>("username")?;
        let email = dry.get_required::<String>("email")?;
        
        // Validation logic
        if username.is_empty() || !email.contains('@') {
            return Err(OpError::ExecutionFailed("Invalid user data".to_string()));
        }
        
        dry.insert("is_valid", true);
        dry.insert("validation_timestamp", chrono::Utc::now().to_rfc3339());
        
        Ok(())
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("ValidateUserOp")
            .description("Validates user data")
            .input_schema(json!({
                "type": "object",
                "properties": {
                    "username": { "type": "string" },
                    "email": { "type": "string", "format": "email" }
                },
                "required": ["username", "email"]
            }))
            .output_schema(json!({
                "type": "object",
                "properties": {
                    "is_valid": { "type": "boolean" },
                    "validation_timestamp": { "type": "string", "format": "date-time" }
                }
            }))
            .build()
    }
}

// Op that creates a report (needs user_id and validation results)
struct CreateReportOp;

#[async_trait]
impl Op<String> for CreateReportOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<String> {
        let user_id = dry.get_required::<i32>("user_id")?;
        let username = dry.get_required::<String>("username")?;
        let is_valid = dry.get_required::<bool>("is_valid")?;
        let timestamp = dry.get_required::<String>("validation_timestamp")?;
        let report_format = dry.get_required::<String>("report_format")?;
        
        let report = match report_format.as_str() {
            "json" => json!({
                "user_id": user_id,
                "username": username,
                "validation": {
                    "is_valid": is_valid,
                    "timestamp": timestamp
                }
            }).to_string(),
            "text" => format!(
                "User Report\n===========\nID: {}\nUsername: {}\nValid: {}\nChecked: {}",
                user_id, username, is_valid, timestamp
            ),
            _ => return Err(OpError::ExecutionFailed("Unknown report format".to_string()))
        };
        
        Ok(report)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("CreateReportOp")
            .description("Creates a user validation report")
            .input_schema(json!({
                "type": "object",
                "properties": {
                    "user_id": { "type": "integer" },
                    "username": { "type": "string" },
                    "is_valid": { "type": "boolean" },
                    "validation_timestamp": { "type": "string" },
                    "report_format": { 
                        "type": "string",
                        "enum": ["json", "text"]
                    }
                },
                "required": ["user_id", "username", "is_valid", "validation_timestamp", "report_format"]
            }))
            .output_schema(json!({
                "type": "string",
                "description": "The formatted report"
            }))
            .build()
    }
}

// Service that requires a connection
struct NotificationService;

// Op that sends notifications (requires service from wet context)
struct SendNotificationOp;

#[async_trait]
impl Op<()> for SendNotificationOp {
    async fn perform(&self, dry: &mut DryContext, wet: &mut WetContext) -> OpResult<()> {
        let email = dry.get_required::<String>("email")?;
        let is_valid = dry.get_required::<bool>("is_valid")?;
        
        let _notification_service = wet.get_required::<NotificationService>("notification_service")?;
        
        if is_valid {
            println!("Sending validation success notification to {}", email);
        } else {
            println!("Sending validation failure notification to {}", email);
        }
        
        dry.insert("notification_sent", true);
        
        Ok(())
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("SendNotificationOp")
            .description("Sends email notification about validation status")
            .input_schema(json!({
                "type": "object",
                "properties": {
                    "email": { "type": "string" },
                    "is_valid": { "type": "boolean" }
                },
                "required": ["email", "is_valid"]
            }))
            .reference_schema(json!({
                "type": "object",
                "properties": {
                    "notification_service": { "type": "NotificationService" }
                },
                "required": ["notification_service"]
            }))
            .output_schema(json!({
                "type": "object",
                "properties": {
                    "notification_sent": { "type": "boolean" }
                }
            }))
            .build()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    println!("=== Batch Data Flow Demo ===\n");
    println!("This demo shows how BatchOp intelligently analyzes data flow between ops.\n");
    
    // Create a batch of ops where later ops consume outputs from earlier ones
    let ops: Vec<Arc<dyn Op<()>>> = vec![
        Arc::new(FetchUserOp),         // Requires: Produces: username, email, age
        Arc::new(ValidateUserOp),      // Requires: username, email (from FetchUserOp), Produces: is_valid, validation_timestamp
        Arc::new(SendNotificationOp),  // Requires: email (from FetchUserOp), is_valid (from ValidateUserOp)
    ];
    
    let batch = BatchOp::new(ops);
    let batch_metadata = batch.metadata();
    
    println!("Batch Metadata Analysis:");
    println!("========================");
    
    // Show the intelligent input requirements
    if let Some(input_schema) = &batch_metadata.input_schema {
        println!("\nInput Schema (only unsatisfied requirements):");
        println!("{}", serde_json::to_string_pretty(input_schema)?);
        
        // The batch should only require: report_format
        // because username, email, is_valid, and validation_timestamp are produced internally
    }
    
    // Show the reference requirements (union of all ops)
    if let Some(ref_schema) = &batch_metadata.reference_schema {
        println!("\nReference Schema (all service requirements):");
        println!("{}", serde_json::to_string_pretty(ref_schema)?);
    }
    
    // Execute the batch
    println!("\nExecuting batch...\n");
    
    let mut dry = DryContext::new()
        .with_value("user_id", 42)
        .with_value("report_format", "json");  // Only these two are needed!
    
    let mut wet = WetContext::new();
    wet.insert_ref("notification_service", NotificationService);
    
    // Validate contexts
    let validation = batch_metadata.validate_contexts(&dry, &wet)?;
    if validation.is_valid {
        println!("✓ Context validation passed");
    } else {
        println!("✗ Context validation failed: {:?}", validation.errors);
        return Ok(());
    }
    
    // Execute the batch
    batch.perform(&mut dry, &mut wet).await?;
    
    println!("\n✓ Batch completed successfully!");
    
    // Create another batch with the report creation op
    println!("\n\nCreating a batch with report generation...");
    
    let report_ops: Vec<Arc<dyn Op<String>>> = vec![
        Arc::new(CreateReportOp),  // This will use all the data produced by previous ops
    ];
    
    let report_batch = BatchOp::new(report_ops);
    let report_metadata = report_batch.metadata();
    
    if let Some(input_schema) = &report_metadata.input_schema {
        println!("\nReport Batch Input Requirements:");
        println!("{}", serde_json::to_string_pretty(input_schema)?);
    }
    
    // Execute report generation
    let results = report_batch.perform(&mut dry, &mut wet).await?;
    
    println!("\nGenerated Report:");
    println!("{}", results[0]);
    
    // Demonstrate a complete pipeline
    println!("\n\nComplete Pipeline Example:");
    println!("==========================");
    
    // Create a batch that does everything including report generation
    let complete_ops: Vec<Arc<dyn Op<()>>> = vec![
        Arc::new(FetchUserOp),
        Arc::new(ValidateUserOp),
        Arc::new(SendNotificationOp),
    ];
    
    let complete_batch = BatchOp::new(complete_ops);
    let complete_metadata = complete_batch.metadata();
    
    println!("\nComplete Pipeline Requirements:");
    if let Some(input_schema) = &complete_metadata.input_schema {
        let required = input_schema.get("required")
            .and_then(|r| r.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>())
            .unwrap_or_default();
        
        println!("  Required inputs: {:?}", required);
        println!("  Note: username, email, is_valid, etc. are NOT required as inputs");
        println!("  because they are produced by ops within the batch!");
    }
    
    Ok(())
}
use ops::prelude::*;
use ops::OpRequest;
use serde_json::json;

// Example service that would be in the wet context
struct DatabaseService {
    connection_string: String,
}

impl DatabaseService {
    fn new(connection_string: &str) -> Self {
        Self {
            connection_string: connection_string.to_string(),
        }
    }
    
    async fn query(&self, query: &str) -> String {
        format!("Result of '{}' from {}", query, self.connection_string)
    }
}

// Example op that uses both dry and wet contexts
struct QueryOp;

#[async_trait]
impl Op<String> for QueryOp {
    async fn perform(&self, dry: &mut DryContext, wet: &mut WetContext) -> OpResult<String> {
        // Get query from dry context (serializable data)
        let query = dry.get_required::<String>("query")?;
        
        // Get database service from wet context (runtime reference)
        let db_service = wet.get_required::<DatabaseService>("db_service")?;
        
        // Execute the query
        let result = db_service.query(&query).await;
        
        Ok(result)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("QueryOp")
            .description("Execute a database query with validation")
            .input_schema(json!({
                "type": "object",
                "properties": {
                    "query": { 
                        "type": "string",
                        "minLength": 1,
                        "maxLength": 1000,
                        "description": "SQL query to execute"
                    }
                },
                "required": ["query"],
                "additionalProperties": false
            }))
            .reference_schema(json!({
                "type": "object",
                "properties": {
                    "db_service": { 
                        "type": "DatabaseService",
                        "description": "Database service for query execution"
                    }
                },
                "required": ["db_service"],
                "additionalProperties": false
            }))
            .output_schema(json!({
                "type": "string",
                "description": "Query result as formatted string"
            }))
            .build()
    }
}

// Example of saving and loading op requests
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    println!("=== Dry/Wet Context Demo ===\n");
    
    // Create an op
    let op = QueryOp;
    
    // Create dry context with serializable data
    let mut dry = DryContext::new()
        .with_value("query", "SELECT * FROM users WHERE active = true");
    
    // Create wet context with runtime references
    let db_service = DatabaseService::new("postgres://localhost/mydb");
    let mut wet = WetContext::new()
        .with_ref("db_service", db_service);
    
    // Validate contexts against op metadata
    println!("Validating contexts...");
    let validation_report = op.metadata().validate_contexts(&dry, &wet)?;
    if validation_report.is_fully_valid() {
        println!("✓ Contexts are valid!\n");
    } else {
        println!("✗ Validation errors: {:?}\n", validation_report.errors);
        return Ok(());
    }
    
    // Execute the op
    println!("Executing op...");
    let result = op.perform(&mut dry, &mut wet).await?;
    println!("Result: {}\n", result);
    
    // Demonstrate op request persistence (simulated)
    println!("Creating op request for later execution...");
    let op_request = OpRequest::new("QueryOp")
        .with_data("query", "SELECT COUNT(*) FROM orders")
        .with_metadata(op.metadata());
    
    println!("Op request created:");
    println!("  ID: {}", op_request.id);
    println!("  Op: {}", op_request.trigger_name);
    println!("  Created: {}", op_request.created_at);
    
    // Simulate saving (not actually implemented)
    op_request.save()?;
    println!("✓ Op request saved (simulated)\n");
    
    // Later: validate and get dry context
    let loaded_dry = op_request.validate_and_get_dry_context()?;
    println!("Loaded dry context:");
    println!("  query: {:?}", loaded_dry.get::<String>("query"));
    
    // Execute with new wet context
    let new_db_service = DatabaseService::new("postgres://backup-server/mydb");
    let mut new_wet = WetContext::new()
        .with_ref("db_service", new_db_service);
    
    println!("\nExecuting loaded op with different database...");
    let mut loaded_dry = loaded_dry;
    let new_result = op.perform(&mut loaded_dry, &mut new_wet).await?;
    println!("Result: {}", new_result);
    
    // Demonstrate output validation
    println!("\nValidating output...");
    let output_validation = op.metadata().validate_output(&new_result)?;
    if output_validation.is_fully_valid() {
        println!("✓ Output is valid!");
    }
    
    Ok(())
}
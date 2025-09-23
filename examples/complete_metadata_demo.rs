use ops::prelude::*;
use ops::{
    dry_put, dry_require,
    wet_put_ref, wet_require_ref,
    OpRequest,
};
use serde_json::json;
use std::sync::Arc;

// Example services for wet context
struct DatabaseService {
    connection_string: String,
}

impl DatabaseService {
    fn new(connection_string: &str) -> Self {
        Self {
            connection_string: connection_string.to_string(),
        }
    }
    
    async fn query(&self, sql: &str) -> String {
        format!("Result of '{}' from {}", sql, self.connection_string)
    }
}

struct EmailService {
    smtp_host: String,
}

impl EmailService {
    fn new(smtp_host: &str) -> Self {
        Self {
            smtp_host: smtp_host.to_string(),
        }
    }
    
    async fn send_email(&self, to: &str, subject: &str, body: &str) -> bool {
        println!("Sending email via {}: To={}, Subject={}, Body={}", 
                 self.smtp_host, to, subject, body);
        true
    }
}

// Example op with comprehensive metadata
struct UserRegistrationOp;

#[async_trait]
impl Op<String> for UserRegistrationOp {
    async fn perform(&self, dry: &DryContext, wet: &WetContext) -> OpResult<String> {
        // Extract required data from dry context
        let username: String = dry_require!(dry, username)?;
        let email: String = dry_require!(dry, email)?;
        let age: i32 = dry_require!(dry, age)?;
        let is_premium: bool = dry_require!(dry, is_premium)?;
        
        // Extract required services from wet context
        let db_service: Arc<DatabaseService> = wet_require_ref!(wet, db_service)?;
        let email_service: Arc<EmailService> = wet_require_ref!(wet, email_service)?;
        
        // Validate age
        if age < 13 {
            return Err(OpError::ExecutionFailed("User must be at least 13 years old".to_string()));
        }
        
        // Create user in database
        let sql = format!("INSERT INTO users (username, email, age, is_premium) VALUES ('{}', '{}', {}, {})", 
                         username, email, age, is_premium);
        let _db_result = db_service.query(&sql).await;
        
        // Send welcome email
        let subject = if is_premium { "Welcome to Premium!" } else { "Welcome!" };
        let body = format!("Hello {}, welcome to our platform!", username);
        let email_sent = email_service.send_email(&email, subject, &body).await;
        
        if !email_sent {
            return Err(OpError::ExecutionFailed("Failed to send welcome email".to_string()));
        }
        
        // Return user ID
        let user_id = format!("user_{}", username);
        Ok(user_id)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("UserRegistrationOp")
            .description("Registers a new user with database and email confirmation")
            .input_schema(json!({
                "type": "object",
                "properties": {
                    "username": {
                        "type": "string",
                        "minLength": 3,
                        "maxLength": 20,
                        "pattern": "^[a-zA-Z0-9_]+$",
                        "description": "Unique username for the account"
                    },
                    "email": {
                        "type": "string",
                        "format": "email",
                        "description": "User's email address for notifications"
                    },
                    "age": {
                        "type": "integer",
                        "minimum": 13,
                        "maximum": 120,
                        "description": "User's age for compliance purposes"
                    },
                    "is_premium": {
                        "type": "boolean",
                        "default": false,
                        "description": "Whether the user has premium features"
                    }
                },
                "required": ["username", "email", "age", "is_premium"],
                "additionalProperties": false
            }))
            .reference_schema(json!({
                "type": "object",
                "properties": {
                    "db_service": {
                        "type": "DatabaseService",
                        "description": "Database service for user storage"
                    },
                    "email_service": {
                        "type": "EmailService", 
                        "description": "Email service for notifications"
                    }
                },
                "required": ["db_service", "email_service"],
                "additionalProperties": false
            }))
            .output_schema(json!({
                "type": "string",
                "pattern": "^user_[a-zA-Z0-9_]+$",
                "description": "Generated user ID for the new account"
            }))
            .build()
    }
}

// Another example with optional fields and complex validation
struct DataProcessingOp;

#[async_trait]
impl Op<serde_json::Value> for DataProcessingOp {
    async fn perform(&self, dry: &DryContext, wet: &WetContext) -> OpResult<serde_json::Value> {
        // Required fields
        let input_data: String = dry_require!(dry, input_data)?;
        let processing_type: String = dry_require!(dry, processing_type)?;
        
        // Optional fields with defaults
        let batch_size: i32 = dry.get("batch_size").unwrap_or(100);
        let timeout_seconds: i32 = dry.get("timeout_seconds").unwrap_or(30);
        let enable_logging: bool = dry.get("enable_logging").unwrap_or(true);
        
        // Required service
        let _db_service: Arc<DatabaseService> = wet_require_ref!(wet, db_service)?;
        
        if enable_logging {
            println!("Processing {} with type {} (batch_size: {}, timeout: {}s)", 
                    input_data, processing_type, batch_size, timeout_seconds);
        }
        
        // Simulate processing
        let result = match processing_type.as_str() {
            "transform" => json!({
                "transformed_data": input_data.to_uppercase(),
                "batch_size": batch_size,
                "processed_at": chrono::Utc::now().to_rfc3339()
            }),
            "analyze" => json!({
                "analysis": format!("Data '{}' has {} characters", input_data, input_data.len()),
                "complexity": if input_data.len() > 100 { "high" } else { "low" },
                "processed_at": chrono::Utc::now().to_rfc3339()
            }),
            _ => return Err(OpError::ExecutionFailed(format!("Unknown processing type: {}", processing_type)))
        };
        
        Ok(result)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("DataProcessingOp")
            .description("Processes data with configurable options and type-specific logic")
            .input_schema(json!({
                "type": "object",
                "properties": {
                    "input_data": {
                        "type": "string",
                        "minLength": 1,
                        "maxLength": 10000,
                        "description": "The data to process"
                    },
                    "processing_type": {
                        "type": "string",
                        "enum": ["transform", "analyze"],
                        "description": "Type of processing to perform"
                    },
                    "batch_size": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 1000,
                        "default": 100,
                        "description": "Number of items to process in each batch (optional)"
                    },
                    "timeout_seconds": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 300,
                        "default": 30,
                        "description": "Processing timeout in seconds (optional)"
                    },
                    "enable_logging": {
                        "type": "boolean",
                        "default": true,
                        "description": "Whether to enable verbose logging (optional)"
                    }
                },
                "required": ["input_data", "processing_type"],
                "additionalProperties": false
            }))
            .reference_schema(json!({
                "type": "object",
                "properties": {
                    "db_service": {
                        "type": "DatabaseService",
                        "description": "Database service for data storage and retrieval"
                    }
                },
                "required": ["db_service"],
                "additionalProperties": false
            }))
            .output_schema(json!({
                "type": "object",
                "properties": {
                    "transformed_data": {
                        "type": "string",
                        "description": "Transformed data (for transform type)"
                    },
                    "analysis": {
                        "type": "string", 
                        "description": "Analysis result (for analyze type)"
                    },
                    "complexity": {
                        "type": "string",
                        "enum": ["low", "medium", "high"],
                        "description": "Complexity assessment (for analyze type)"
                    },
                    "batch_size": {
                        "type": "integer",
                        "description": "Batch size used for processing"
                    },
                    "processed_at": {
                        "type": "string",
                        "format": "date-time",
                        "description": "ISO 8601 timestamp of when processing completed"
                    }
                },
                "additionalProperties": false
            }))
            .build()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    println!("=== Complete Metadata Demo ===\n");
    
    // Example 1: UserRegistrationOp with full validation
    println!("1. User Registration Op");
    println!("   Input Schema: username, email, age, is_premium (all required)");
    println!("   Reference Schema: db_service, email_service (both required)");
    println!("   Output Schema: user ID string with pattern validation\n");
    
    let user_op = UserRegistrationOp;
    let user_metadata = user_op.metadata();
    
    // Create and validate dry context
    let mut user_dry = DryContext::new();
    let username = "john_doe".to_string();
    let email = "john@example.com".to_string();
    let age = 25;
    let is_premium = true;
    
    dry_put!(user_dry, username);
    dry_put!(user_dry, email);
    dry_put!(user_dry, age);
    dry_put!(user_dry, is_premium);
    
    // Validate dry context against schema
    let dry_validation = user_metadata.validate_dry_context(&user_dry)?;
    if dry_validation.is_valid {
        println!("✓ Dry context validation passed");
    } else {
        println!("✗ Dry context validation failed: {:?}", dry_validation.errors);
        return Ok(());
    }
    
    // Create and validate wet context
    let mut user_wet = WetContext::new();
    let db_service = DatabaseService::new("postgres://localhost/users");
    let email_service = EmailService::new("smtp.example.com");
    
    wet_put_ref!(user_wet, db_service);
    wet_put_ref!(user_wet, email_service);
    
    let wet_validation = user_metadata.validate_wet_context(&user_wet)?;
    if wet_validation.is_valid {
        println!("✓ Wet context validation passed");
    } else {
        println!("✗ Wet context validation failed: {:?}", wet_validation.errors);
        return Ok(());
    }
    
    // Execute the op
    let user_result = user_op.perform(&user_dry, &user_wet).await?;
    println!("User registration result: {}", user_result);
    
    // Validate output
    let output_validation = user_metadata.validate_output(&user_result)?;
    if output_validation.is_valid {
        println!("✓ Output validation passed\n");
    } else {
        println!("✗ Output validation failed: {:?}\n", output_validation.errors);
    }
    
    // Example 2: DataProcessingOp with optional fields
    println!("2. Data Processing Op");
    println!("   Input Schema: input_data, processing_type (required) + optional fields");
    println!("   Reference Schema: db_service (required)");
    println!("   Output Schema: flexible object based on processing type\n");
    
    let data_op = DataProcessingOp;
    let data_metadata = data_op.metadata();
    
    // Create context with required and optional fields
    let mut data_dry = DryContext::new();
    let input_data = "Hello World Data".to_string();
    let processing_type = "analyze".to_string();
    let batch_size = 50; // Optional field
    
    dry_put!(data_dry, input_data);
    dry_put!(data_dry, processing_type);
    dry_put!(data_dry, batch_size);
    // Note: timeout_seconds and enable_logging will use defaults
    
    let data_dry_validation = data_metadata.validate_dry_context(&data_dry)?;
    println!("Dry context validation: {}", if data_dry_validation.is_valid { "✓ passed" } else { "✗ failed" });
    
    // Create database service for data processing
    let mut data_wet = WetContext::new();
    let db_service = DatabaseService::new("postgres://localhost/analytics");
    wet_put_ref!(data_wet, db_service);
    
    let data_wet_validation = data_metadata.validate_wet_context(&data_wet)?;
    println!("Wet context validation: {}", if data_wet_validation.is_valid { "✓ passed" } else { "✗ failed" });
    
    // Execute the op
    let data_result = data_op.perform(&data_dry, &data_wet).await?;
    println!("Data processing result: {}", serde_json::to_string_pretty(&data_result)?);
    
    // Validate output
    let data_output_validation = data_metadata.validate_output(&data_result)?;
    println!("Output validation: {}", if data_output_validation.is_valid { "✓ passed" } else { "✗ failed" });
    
    // Example 3: Demonstrate OpRequest with metadata
    println!("\n3. OpRequest with Metadata");
    
    let op_request = OpRequest::new("UserRegistrationOp")
        .with_data("username", "alice_smith")
        .with_data("email", "alice@example.com")
        .with_data("age", 30)
        .with_data("is_premium", false)
        .with_metadata(user_metadata);
        
    println!("OpRequest created with ID: {}", op_request.id);
    println!("OpRequest validation and context extraction...");
    
    let validated_dry = op_request.validate_and_get_dry_context()?;
    println!("✓ OpRequest dry context validated successfully");
    
    // Show how to reconstruct and execute later
    println!("Later execution with different services...");
    let mut later_wet = WetContext::new();
    let db_service = DatabaseService::new("postgres://backup-server/users");
    let email_service = EmailService::new("backup-smtp.example.com");
    
    wet_put_ref!(later_wet, db_service);
    wet_put_ref!(later_wet, email_service);
    
    let later_result = user_op.perform(&validated_dry, &later_wet).await?;
    println!("Later execution result: {}", later_result);
    
    // Example 4: Show schema information
    println!("\n4. Schema Information");
    println!("UserRegistrationOp schemas:");
    if let Some(input_schema) = &user_op.metadata().input_schema {
        println!("  Input Schema: {}", serde_json::to_string_pretty(input_schema)?);
    }
    if let Some(ref_schema) = &user_op.metadata().reference_schema {
        println!("  Reference Schema: {}", serde_json::to_string_pretty(ref_schema)?);
    }
    if let Some(output_schema) = &user_op.metadata().output_schema {
        println!("  Output Schema: {}", serde_json::to_string_pretty(output_schema)?);
    }
    
    Ok(())
}
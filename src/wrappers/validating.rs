use crate::prelude::*;
use crate::op::Op;
use crate::{DryContext, WetContext, OpMetadata};
use crate::error::OpError;
use async_trait::async_trait;
use jsonschema::{JSONSchema, Draft};

pub struct ValidatingWrapper<T> {
    wrapped_op: Box<dyn Op<T>>,
    validate_input: bool,
    validate_output: bool,
}

impl<T> ValidatingWrapper<T> 
where
    T: Send + Sync + 'static + serde::Serialize,
{
    /// Create new validating wrapper that validates both input and output
    pub fn new(op: Box<dyn Op<T>>) -> Self {
        Self {
            wrapped_op: op,
            validate_input: true,
            validate_output: true,
        }
    }

    /// Create validating wrapper that only validates input
    pub fn input_only(op: Box<dyn Op<T>>) -> Self {
        Self {
            wrapped_op: op,
            validate_input: true,
            validate_output: false,
        }
    }

    /// Create validating wrapper that only validates output
    pub fn output_only(op: Box<dyn Op<T>>) -> Self {
        Self {
            wrapped_op: op,
            validate_input: false,
            validate_output: true,
        }
    }

    /// Validate input against schema
    fn validate_input_schema(&self, dry: &DryContext, metadata: &OpMetadata) -> OpResult<()> {
        if !self.validate_input {
            return Ok(());
        }

        if let Some(ref schema) = metadata.input_schema {
            // Compile the schema
            let compiled = JSONSchema::options()
                .with_draft(Draft::Draft7)
                .compile(schema)
                .map_err(|e| OpError::Context(format!("Invalid input schema for {}: {}", metadata.name, e)))?;

            // Get all context values as a JSON object for validation
            let context_json = serde_json::json!(dry.values());

            // Validate
            let validation_result = compiled.validate(&context_json);
            if let Err(errors) = validation_result {
                let error_messages: Vec<String> = errors
                    .map(|e| format!("{}: {}", e.instance_path, e))
                    .collect();
                
                return Err(OpError::Context(format!(
                    "Input validation failed for {}: {}",
                    metadata.name,
                    error_messages.join(", ")
                )));
            }
        }
        Ok(())
    }

    /// Validate references against schema
    fn validate_references_schema(&self, wet: &WetContext, metadata: &OpMetadata) -> OpResult<()> {
        // References are always validated when a reference_schema is present —
        // they are pre-conditions of the op regardless of input/output validation mode.

        if let Some(ref schema) = metadata.reference_schema {
            // For reference schema validation, we check that required references exist
            // The schema should define required reference keys
            if let Some(required_refs) = schema.get("required") {
                if let Some(required_array) = required_refs.as_array() {
                    for required_ref in required_array {
                        if let Some(ref_name) = required_ref.as_str() {
                            if !wet.contains(ref_name) {
                                return Err(OpError::Context(format!(
                                    "Required reference '{}' not found in WetContext for op '{}'",
                                    ref_name, metadata.name
                                )));
                            }
                        }
                    }
                }
            }

            // Optionally validate reference types if specified in the schema
            if let Some(properties) = schema.get("properties") {
                if let Some(props_obj) = properties.as_object() {
                    for (ref_name, ref_schema) in props_obj {
                        if wet.contains(ref_name) {
                            // If the reference exists, we could validate its type
                            // For now, we just check existence since types are checked at runtime
                            if let Some(ref_type) = ref_schema.get("type") {
                                if let Some(type_str) = ref_type.as_str() {
                                    // Log that we're skipping type validation for references
                                    tracing::debug!(
                                        "Reference '{}' exists but type validation ('{}') is skipped for runtime safety",
                                        ref_name, type_str
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Validate output against schema
    fn validate_output_schema(&self, output: &T, metadata: &OpMetadata) -> OpResult<()> {
        if !self.validate_output {
            return Ok(());
        }

        if let Some(ref schema) = metadata.output_schema {
            // Compile the schema
            let compiled = JSONSchema::options()
                .with_draft(Draft::Draft7)
                .compile(schema)
                .map_err(|e| OpError::Context(format!("Invalid output schema for {}: {}", metadata.name, e)))?;

            // Serialize the output
            let output_json = serde_json::to_value(output)
                .map_err(|e| OpError::Context(format!("Failed to serialize output for validation: {}", e)))?;

            // Validate
            let validation_result = compiled.validate(&output_json);
            if let Err(errors) = validation_result {
                let error_messages: Vec<String> = errors
                    .map(|e| format!("{}: {}", e.instance_path, e))
                    .collect();
                
                return Err(OpError::Context(format!(
                    "Output validation failed for {}: {}",
                    metadata.name,
                    error_messages.join(", ")
                )));
            }
        }
        Ok(())
    }
}

#[async_trait]
impl<T> Op<T> for ValidatingWrapper<T>
where
    T: Send + Sync + 'static + serde::Serialize,
{
    async fn perform(&self, dry: &mut DryContext, wet: &mut WetContext) -> OpResult<T> {
        let metadata = self.wrapped_op.metadata();
        
        // Validate input before execution
        self.validate_input_schema(dry, &metadata)?;
        
        // Validate references before execution
        self.validate_references_schema(wet, &metadata)?;
        
        // Execute wrapped op
        let result = self.wrapped_op.perform(dry, wet).await?;
        
        // Validate output after execution
        self.validate_output_schema(&result, &metadata)?;
        
        Ok(result)
    }
    
    fn metadata(&self) -> OpMetadata {
        // Pass through metadata from wrapped op
        self.wrapped_op.metadata()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct TestOutput {
        value: i32,
    }

    struct ValidatedOp;

    #[async_trait]
    impl Op<TestOutput> for ValidatedOp {
        async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<TestOutput> {
            let value = dry.get_required::<i32>("value")?;
            Ok(TestOutput { value })
        }

        fn metadata(&self) -> OpMetadata {
            OpMetadata::builder("ValidatedOp")
                .description("Op with schema validation")
                .input_schema(json!({
                    "type": "object",
                    "properties": {
                        "value": { "type": "integer", "minimum": 0, "maximum": 100 }
                    },
                    "required": ["value"]
                }))
                .output_schema(json!({
                    "type": "object",
                    "properties": {
                        "value": { "type": "integer" }
                    },
                    "required": ["value"]
                }))
                .build()
        }
    }

    // TEST038: Run ValidatingWrapper with a valid input and verify the op executes and returns the result
    #[tokio::test]
    async fn test_038_valid_input_output() {
        let validator = ValidatingWrapper::new(Box::new(ValidatedOp));
        
        let mut dry = DryContext::new();
        dry.insert("value", 42);
        let mut wet = WetContext::new();
        
        let result = validator.perform(&mut dry, &mut wet).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value, 42);
    }

    // TEST039: Run ValidatingWrapper without a required input field and verify a Context validation error
    #[tokio::test]
    async fn test_039_invalid_input_missing_required() {
        let validator = ValidatingWrapper::new(Box::new(ValidatedOp));
        
        let mut dry = DryContext::new();
        // Missing required "value" field
        let mut wet = WetContext::new();
        
        let result = validator.perform(&mut dry, &mut wet).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            OpError::Context(msg) => assert!(msg.contains("Input validation failed")),
            _ => panic!("Expected Context error"),
        }
    }

    // TEST040: Run ValidatingWrapper with an input exceeding the schema maximum and verify a validation error
    #[tokio::test]
    async fn test_040_invalid_input_out_of_range() {
        let validator = ValidatingWrapper::new(Box::new(ValidatedOp));
        
        let mut dry = DryContext::new();
        dry.insert("value", 150); // Exceeds maximum of 100
        let mut wet = WetContext::new();
        
        let result = validator.perform(&mut dry, &mut wet).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            OpError::Context(msg) => assert!(msg.contains("maximum")),
            _ => panic!("Expected Context error"),
        }
    }

    // TEST041: Use ValidatingWrapper::input_only and confirm input is validated while output is not
    #[tokio::test]
    async fn test_041_input_only_validation() {
        struct NoOutputSchemaOp;

        #[async_trait]
        impl Op<i32> for NoOutputSchemaOp {
            async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<i32> {
                dry.get_required::<i32>("value")
            }

            fn metadata(&self) -> OpMetadata {
                OpMetadata::builder("NoOutputSchemaOp")
                    .input_schema(json!({
                        "type": "object",
                        "properties": {
                            "value": { "type": "integer" }
                        },
                        "required": ["value"]
                    }))
                    .build()
            }
        }

        let validator = ValidatingWrapper::input_only(Box::new(NoOutputSchemaOp));
        
        let mut dry = DryContext::new();
        dry.insert("value", 42);
        let mut wet = WetContext::new();
        
        let result = validator.perform(&mut dry, &mut wet).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    // TEST042: Use ValidatingWrapper::output_only and confirm output is validated while input is not
    #[tokio::test]
    async fn test_042_output_only_validation() {
        struct NoInputSchemaOp;

        #[async_trait]
        impl Op<TestOutput> for NoInputSchemaOp {
            async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<TestOutput> {
                Ok(TestOutput { value: 99 })
            }

            fn metadata(&self) -> OpMetadata {
                OpMetadata::builder("NoInputSchemaOp")
                    .output_schema(json!({
                        "type": "object",
                        "properties": {
                            "value": { "type": "integer", "maximum": 100 }
                        },
                        "required": ["value"]
                    }))
                    .build()
            }
        }

        let validator = ValidatingWrapper::output_only(Box::new(NoInputSchemaOp));
        
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        let result = validator.perform(&mut dry, &mut wet).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value, 99);
    }

    // TEST043: Wrap an op with no schemas in ValidatingWrapper and confirm it still succeeds
    #[tokio::test]
    async fn test_043_no_schema_validation() {
        struct NoSchemaOp;

        #[async_trait]
        impl Op<i32> for NoSchemaOp {
            async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<i32> {
                Ok(123)
            }

            fn metadata(&self) -> OpMetadata {
                OpMetadata::builder("NoSchemaOp").build()
            }
        }

        let validator = ValidatingWrapper::new(Box::new(NoSchemaOp));
        
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        // Should succeed even without schemas
        let result = validator.perform(&mut dry, &mut wet).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 123);
    }

    // TEST044: Verify ValidatingWrapper::metadata() delegates to the inner op's metadata unchanged
    #[tokio::test]
    async fn test_044_metadata_transparency() {
        let validator = ValidatingWrapper::new(Box::new(ValidatedOp));
        let metadata = validator.metadata();
        
        assert_eq!(metadata.name, "ValidatedOp");
        assert_eq!(metadata.description, Some("Op with schema validation".to_string()));
        assert!(metadata.input_schema.is_some());
        assert!(metadata.output_schema.is_some());
    }

    // TEST045: Verify ValidatingWrapper checks reference_schema and rejects when required refs are missing
    #[tokio::test]
    async fn test_045_reference_validation() {
        struct ServiceRequiringOp;

        #[async_trait]
        impl Op<String> for ServiceRequiringOp {
            async fn perform(&self, _dry: &mut DryContext, wet: &mut WetContext) -> OpResult<String> {
                let service = wet.get_required::<String>("database")?;
                Ok(format!("Used service: {}", service))
            }

            fn metadata(&self) -> OpMetadata {
                OpMetadata::builder("ServiceRequiringOp")
                    .reference_schema(json!({
                        "type": "object",
                        "required": ["database", "cache"],
                        "properties": {
                            "database": { "type": "string" },
                            "cache": { "type": "string" }
                        }
                    }))
                    .build()
            }
        }

        let validator = ValidatingWrapper::new(Box::new(ServiceRequiringOp));
        
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();
        
        // Missing required references
        let result = validator.perform(&mut dry, &mut wet).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            OpError::Context(msg) => assert!(msg.contains("Required reference 'database' not found")),
            _ => panic!("Expected Context error"),
        }
        
        // Add one reference but not all
        wet.insert_ref("database", "postgresql".to_string());
        let result = validator.perform(&mut dry, &mut wet).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            OpError::Context(msg) => assert!(msg.contains("Required reference 'cache' not found")),
            _ => panic!("Expected Context error"),
        }
        
        // Add all required references
        wet.insert_ref("cache", "redis".to_string());
        let result = validator.perform(&mut dry, &mut wet).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Used service: postgresql");
    }

    // TEST046: Wrap an op with no reference schema in ValidatingWrapper and confirm it succeeds
    #[tokio::test]
    async fn test_046_no_reference_schema() {
        struct NoRefSchemaOp;

        #[async_trait]
        impl Op<i32> for NoRefSchemaOp {
            async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<i32> {
                Ok(456)
            }

            fn metadata(&self) -> OpMetadata {
                OpMetadata::builder("NoRefSchemaOp").build()
            }
        }

        let validator = ValidatingWrapper::new(Box::new(NoRefSchemaOp));

        let mut dry = DryContext::new();
        let mut wet = WetContext::new();

        // Should succeed even without reference schema
        let result = validator.perform(&mut dry, &mut wet).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 456);
    }

    // TEST112: Verify ValidatingWrapper::output_only validates references even when input validation is disabled
    #[tokio::test]
    async fn test_112_output_only_still_validates_references() {
        struct RefRequiringOp;

        #[async_trait]
        impl Op<i32> for RefRequiringOp {
            async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<i32> {
                Ok(42)
            }
            fn metadata(&self) -> OpMetadata {
                OpMetadata::builder("RefRequiringOp")
                    .reference_schema(json!({
                        "type": "object",
                        "required": ["database"],
                        "properties": {
                            "database": { "type": "string" }
                        }
                    }))
                    .build()
            }
        }

        let validator = ValidatingWrapper::output_only(Box::new(RefRequiringOp));
        let mut dry = DryContext::new();
        let mut wet = WetContext::new();

        // Missing required reference — must be rejected even though input validation is off
        let result = validator.perform(&mut dry, &mut wet).await;
        assert!(result.is_err(), "output_only must still validate references");
        match result.unwrap_err() {
            OpError::Context(msg) => assert!(msg.contains("database")),
            e => panic!("expected Context error, got {:?}", e),
        }

        // With the reference present — must succeed
        wet.insert_ref("database", "postgres://localhost".to_string());
        let result = validator.perform(&mut dry, &mut wet).await;
        assert!(result.is_ok(), "should succeed when required reference is present");
    }
}
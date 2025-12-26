use crate::prelude::*;
use crate::{DryContext, WetContext};

use serde_json::{json, Value};
use serde_json::Value as JsonValue;

/// Metadata describing an op's requirements and schemas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpMetadata {
    pub name: String,
    pub input_schema: Option<JsonValue>,
    pub reference_schema: Option<JsonValue>,
    pub output_schema: Option<JsonValue>,
    pub description: Option<String>,
}

impl OpMetadata {
    pub fn builder(name: impl Into<String>) -> OpMetadataBuilder {
        OpMetadataBuilder {
            name: name.into(),
            input_schema: None,
            reference_schema: None,
            output_schema: None,
            description: None,
        }
    }

    /// Validate a dry context against the input schema
    pub fn validate_dry_context(&self, ctx: &DryContext) -> Result<ValidationReport, OpError> {
        if let Some(schema) = &self.input_schema {
            let values = ctx.values();
            let context_json = serde_json::to_value(values)?;
            validate_against_schema(&context_json, schema)
        } else {
            Ok(ValidationReport::success())
        }
    }

    /// Validate a wet context against the reference schema
    pub fn validate_wet_context(&self, ctx: &WetContext) -> Result<ValidationReport, OpError> {
        if let Some(schema) = &self.reference_schema {
            // For wet context, we can only validate that required keys exist
            // since we can't serialize the actual references
            let mut context_keys = serde_json::Map::new();
            for key in ctx.keys() {
                context_keys.insert(key.clone(), serde_json::Value::String("present".to_string()));
            }
            let context_json = serde_json::Value::Object(context_keys);
            validate_reference_schema(&context_json, schema)
        } else {
            Ok(ValidationReport::success())
        }
    }

    /// Validate contexts together
    pub fn validate_contexts(
        &self,
        dry: &DryContext,
        wet: &WetContext,
    ) -> Result<ValidationReport, OpError> {
        let dry_report = self.validate_dry_context(dry)?;
        let wet_report = self.validate_wet_context(wet)?;
        
        Ok(ValidationReport {
            is_valid: dry_report.is_valid && wet_report.is_valid,
            errors: [dry_report.errors, wet_report.errors].concat(),
            warnings: [dry_report.warnings, wet_report.warnings].concat(),
        })
    }

    /// Validate output against the output schema
    pub fn validate_output<T: Serialize>(&self, output: &T) -> Result<ValidationReport, OpError> {
        if let Some(schema) = &self.output_schema {
            let output_json = serde_json::to_value(output)?;
            validate_against_schema(&output_json, schema)
        } else {
            Ok(ValidationReport::success())
        }
    }
}

pub struct OpMetadataBuilder {
    name: String,
    input_schema: Option<JsonValue>,
    reference_schema: Option<JsonValue>,
    output_schema: Option<JsonValue>,
    description: Option<String>,
}

impl OpMetadataBuilder {
    pub fn input_schema(mut self, schema: JsonValue) -> Self {
        self.input_schema = Some(schema);
        self
    }

    pub fn reference_schema(mut self, schema: JsonValue) -> Self {
        self.reference_schema = Some(schema);
        self
    }

    pub fn output_schema(mut self, schema: JsonValue) -> Self {
        self.output_schema = Some(schema);
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn build(self) -> OpMetadata {
        OpMetadata {
            name: self.name,
            input_schema: self.input_schema,
            reference_schema: self.reference_schema,
            output_schema: self.output_schema,
            description: self.description,
        }
    }
}

/// Result of schema validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

impl ValidationReport {
    pub fn success() -> Self {
        Self {
            is_valid: true,
            errors: vec![],
            warnings: vec![],
        }
    }

    pub fn is_fully_valid(&self) -> bool {
        self.is_valid && self.warnings.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub field: String,
    pub message: String,
}

/// TriggerFuse represents a saved request to execute an op later
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerFuse {
    pub id: String,
    pub trigger_name: String,
    pub dry_context: DryContext,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub metadata: Option<OpMetadata>,
}

impl TriggerFuse {
    pub fn new(trigger_name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            trigger_name: trigger_name.into(),
            dry_context: DryContext::new(),
            created_at: chrono::Utc::now(),
            metadata: None,
        }
    }

    pub fn with_data<T: Serialize>(mut self, key: impl Into<String>, value: T) -> Self {
        self.dry_context.insert(key, value);
        self
    }

    pub fn with_metadata(mut self, metadata: OpMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn validate_and_get_dry_context(&self) -> Result<DryContext, OpError> {
        if let Some(metadata) = &self.metadata {
            let report = metadata.validate_dry_context(&self.dry_context)?;
            if !report.is_valid {
                return Err(OpError::Context(format!(
                    "Invalid dry context: {:?}",
                    report.errors
                )));
            }
        }
        Ok(self.dry_context.clone())
    }

    pub fn save(&self) -> Result<(), OpError> {
        // TODO: Implement persistence
        // For now, this is a placeholder
        debug!("Saving TriggerFuse: {}", self.id);
        Ok(())
    }

    pub fn load(_id: &str) -> Result<Self, OpError> {
        // TODO: Implement persistence loading
        // For now, this is a placeholder
        Err(OpError::Context("TriggerFuse loading not yet implemented".to_string()))
    }
}

// Placeholder validation functions - these would use a proper JSON Schema validator
fn validate_against_schema(value: &JsonValue, schema: &JsonValue) -> Result<ValidationReport, OpError> {
    // TODO: Implement actual JSON Schema validation using jsonschema crate
    // For now, just do basic type checking
    
    let mut errors = vec![];
    let warnings = vec![];
    
    if let (Some(schema_obj), Some(value_obj)) = (schema.as_object(), value.as_object()) {
        // Check required fields
        if let Some(required) = schema_obj.get("required").and_then(|r| r.as_array()) {
            for req_field in required {
                if let Some(field_name) = req_field.as_str() {
                    if !value_obj.contains_key(field_name) {
                        errors.push(ValidationError {
                            field: field_name.to_string(),
                            message: format!("Required field '{}' is missing", field_name),
                        });
                    }
                }
            }
        }
    }
    
    Ok(ValidationReport {
        is_valid: errors.is_empty(),
        errors,
        warnings,
    })
}

fn validate_reference_schema(value: &JsonValue, schema: &JsonValue) -> Result<ValidationReport, OpError> {
    // Special validation for reference schemas - just check that required keys exist
    let mut errors = vec![];
    
    if let (Some(schema_obj), Some(value_obj)) = (schema.as_object(), value.as_object()) {
        if let Some(required) = schema_obj.get("required").and_then(|r| r.as_array()) {
            for req_field in required {
                if let Some(field_name) = req_field.as_str() {
                    if !value_obj.contains_key(field_name) {
                        errors.push(ValidationError {
                            field: field_name.to_string(),
                            message: format!("Required reference '{}' is missing", field_name),
                        });
                    }
                }
            }
        }
    }
    
    Ok(ValidationReport {
        is_valid: errors.is_empty(),
        errors,
        warnings: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_metadata_builder() {
        let metadata = OpMetadata::builder("TestOp")
            .description("A test operation")
            .input_schema(json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" }
                },
                "required": ["name"]
            }))
            .output_schema(json!({
                "type": "string"
            }))
            .build();

        assert_eq!(metadata.name, "TestOp");
        assert_eq!(metadata.description.as_deref(), Some("A test operation"));
        assert!(metadata.input_schema.is_some());
        assert!(metadata.output_schema.is_some());
    }

    #[test]
    fn test_trigger_fuse() {
        let request = TriggerFuse::new("ProcessImage")
            .with_data("image_path", "/tmp/test.jpg")
            .with_data("width", 800);

        assert_eq!(request.trigger_name, "ProcessImage");
        assert_eq!(
            request.dry_context.get::<String>("image_path").unwrap(),
            "/tmp/test.jpg"
        );
        assert_eq!(request.dry_context.get::<i32>("width").unwrap(), 800);
    }

    #[test]
    fn test_basic_validation() {
        let metadata = OpMetadata::builder("TestOp")
            .input_schema(json!({
                "type": "object",
                "required": ["name"]
            }))
            .build();

        let ctx = DryContext::new().with_value("name", "test");
        let report = metadata.validate_dry_context(&ctx).unwrap();
        assert!(report.is_valid);

        let empty_ctx = DryContext::new();
        let report = metadata.validate_dry_context(&empty_ctx).unwrap();
        assert!(!report.is_valid);
        assert_eq!(report.errors.len(), 1);
    }
}
use crate::prelude::*;
use std::collections::{HashMap, HashSet};
use serde_json::{json, Value as JsonValue, Map};

/// Analyzes and constructs metadata for batch operations by understanding data flow
pub struct BatchMetadataBuilder {
    ops_metadata: Vec<OpMetadata>,
}

impl BatchMetadataBuilder {
    pub fn new<T>(ops: &[Arc<dyn Op<T>>]) -> Self {
        Self {
            ops_metadata: ops.iter().map(|op| op.metadata()).collect(),
        }
    }
    
    /// Build the complete metadata for a batch operation
    pub fn build(&self) -> OpMetadata {
        let (input_schema, outputs_by_index) = self.analyze_input_requirements();
        let reference_schema = self.merge_reference_schemas();
        let output_schema = self.construct_output_schema(&outputs_by_index);
        
        OpMetadata::builder("BatchOp")
            .description(format!("Batch of {} operations with data flow analysis", self.ops_metadata.len()))
            .input_schema(input_schema)
            .reference_schema(reference_schema)
            .output_schema(output_schema)
            .build()
    }
    
    /// Analyze input requirements considering data flow between ops
    /// Returns the final input schema and a map of outputs by op index
    fn analyze_input_requirements(&self) -> (JsonValue, HashMap<usize, HashSet<String>>) {
        let mut required_inputs = HashSet::new();
        let mut available_outputs = HashSet::new();
        let mut outputs_by_index = HashMap::new();
        
        // Analyze each op in sequence
        for (index, metadata) in self.ops_metadata.iter().enumerate() {
            // Collect this op's required inputs
            if let Some(input_schema) = &metadata.input_schema {
                if let Some(required) = self.extract_required_fields(input_schema) {
                    for field in required {
                        // Only add to required inputs if not produced by a previous op
                        if !available_outputs.contains(&field) {
                            required_inputs.insert(field);
                        }
                    }
                }
            }
            
            // Collect this op's outputs
            let op_outputs = self.extract_output_fields(&metadata.output_schema);
            outputs_by_index.insert(index, op_outputs.clone());
            available_outputs.extend(op_outputs);
        }
        
        // Build the final input schema with only unsatisfied requirements
        let schema = self.build_input_schema_from_requirements(required_inputs);
        (schema, outputs_by_index)
    }
    
    /// Extract required fields from an input schema
    fn extract_required_fields(&self, schema: &JsonValue) -> Option<Vec<String>> {
        schema.as_object()
            .and_then(|obj| obj.get("required"))
            .and_then(|req| req.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
    }
    
    /// Extract output field names from an output schema
    fn extract_output_fields(&self, schema: &Option<JsonValue>) -> HashSet<String> {
        let mut fields = HashSet::new();
        
        if let Some(schema) = schema {
            // Handle different schema types
            if let Some(obj) = schema.as_object() {
                // Object schema with properties
                if let Some(properties) = obj.get("properties").and_then(|p| p.as_object()) {
                    fields.extend(properties.keys().cloned());
                } else if obj.get("type").and_then(|t| t.as_str()) == Some("string") {
                    // Single string output - we'll need to infer the field name from op name
                    // This is a simplification; in practice, you might want a naming convention
                    fields.insert("result".to_string());
                }
            }
        }
        
        fields
    }
    
    /// Build input schema from required fields
    fn build_input_schema_from_requirements(&self, required_fields: HashSet<String>) -> JsonValue {
        let mut properties = Map::new();
        let mut required = Vec::new();
        
        // Collect properties from all ops that match required fields
        for metadata in &self.ops_metadata {
            if let Some(input_schema) = &metadata.input_schema {
                if let Some(schema_props) = input_schema.as_object()
                    .and_then(|obj| obj.get("properties"))
                    .and_then(|p| p.as_object()) {
                    
                    for (field_name, field_schema) in schema_props {
                        if required_fields.contains(field_name) && !properties.contains_key(field_name) {
                            properties.insert(field_name.clone(), field_schema.clone());
                            required.push(JsonValue::String(field_name.clone()));
                        }
                    }
                }
            }
        }
        
        json!({
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false
        })
    }
    
    /// Merge reference schemas from all ops (union of all requirements)
    fn merge_reference_schemas(&self) -> JsonValue {
        let mut all_properties = Map::new();
        let mut all_required = HashSet::new();
        
        for metadata in &self.ops_metadata {
            if let Some(ref_schema) = &metadata.reference_schema {
                if let Some(obj) = ref_schema.as_object() {
                    // Merge properties
                    if let Some(properties) = obj.get("properties").and_then(|p| p.as_object()) {
                        for (key, value) in properties {
                            if !all_properties.contains_key(key) {
                                all_properties.insert(key.clone(), value.clone());
                            }
                        }
                    }
                    
                    // Merge required fields
                    if let Some(required) = obj.get("required").and_then(|r| r.as_array()) {
                        for req in required {
                            if let Some(field) = req.as_str() {
                                all_required.insert(field.to_string());
                            }
                        }
                    }
                }
            }
        }
        
        let required: Vec<JsonValue> = all_required.into_iter()
            .map(JsonValue::String)
            .collect();
        
        json!({
            "type": "object",
            "properties": all_properties,
            "required": required,
            "additionalProperties": false
        })
    }
    
    /// Construct output schema as array of all distinct outputs
    fn construct_output_schema(&self, _outputs_by_index: &HashMap<usize, HashSet<String>>) -> JsonValue {
        // For batch ops, the output is a Vec<T>, so we return an array schema
        // This is a simplified version - in practice, you might want to preserve
        // the structure of individual outputs
        json!({
            "type": "array",
            "items": {
                "type": "object",
                "description": "Output from individual ops in the batch"
            },
            "minItems": self.ops_metadata.len(),
            "maxItems": self.ops_metadata.len()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    struct ProducerOp;
    struct ConsumerOp;
    
    #[async_trait]
    impl Op<String> for ProducerOp {
        async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<String> {
            let input = dry.get_required::<String>("initial_input")?;
            Ok(format!("produced_{}", input))
        }
        
        fn metadata(&self) -> OpMetadata {
            OpMetadata::builder("ProducerOp")
                .input_schema(json!({
                    "type": "object",
                    "properties": {
                        "initial_input": { "type": "string" }
                    },
                    "required": ["initial_input"]
                }))
                .output_schema(json!({
                    "type": "object",
                    "properties": {
                        "produced_value": { "type": "string" }
                    }
                }))
                .build()
        }
    }
    
    #[async_trait]
    impl Op<String> for ConsumerOp {
        async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<String> {
            let value = dry.get_required::<String>("produced_value")?;
            Ok(format!("consumed_{}", value))
        }
        
        fn metadata(&self) -> OpMetadata {
            OpMetadata::builder("ConsumerOp")
                .input_schema(json!({
                    "type": "object",
                    "properties": {
                        "produced_value": { "type": "string" }
                    },
                    "required": ["produced_value"]
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
    
    #[test]
    fn test_batch_metadata_with_data_flow() {
        let ops: Vec<Arc<dyn Op<String>>> = vec![
            Arc::new(ProducerOp),
            Arc::new(ConsumerOp),
        ];
        
        let builder = BatchMetadataBuilder::new(&ops);
        let metadata = builder.build();
        
        // The batch should only require initial_input since produced_value
        // is satisfied by the ProducerOp
        if let Some(input_schema) = metadata.input_schema {
            let required = input_schema.get("required")
                .and_then(|r| r.as_array())
                .unwrap();
            assert_eq!(required.len(), 1);
            assert_eq!(required[0].as_str().unwrap(), "initial_input");
        }
    }
    
    #[test]
    fn test_reference_schema_merging() {
        struct ServiceAOp;
        struct ServiceBOp;
        
        #[async_trait]
        impl Op<()> for ServiceAOp {
            async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
                Ok(())
            }
            
            fn metadata(&self) -> OpMetadata {
                OpMetadata::builder("ServiceAOp")
                    .reference_schema(json!({
                        "type": "object",
                        "properties": {
                            "service_a": { "type": "ServiceA" }
                        },
                        "required": ["service_a"]
                    }))
                    .build()
            }
        }
        
        #[async_trait]
        impl Op<()> for ServiceBOp {
            async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
                Ok(())
            }
            
            fn metadata(&self) -> OpMetadata {
                OpMetadata::builder("ServiceBOp")
                    .reference_schema(json!({
                        "type": "object",
                        "properties": {
                            "service_b": { "type": "ServiceB" }
                        },
                        "required": ["service_b"]
                    }))
                    .build()
            }
        }
        
        let ops: Vec<Arc<dyn Op<()>>> = vec![
            Arc::new(ServiceAOp),
            Arc::new(ServiceBOp),
        ];
        
        let builder = BatchMetadataBuilder::new(&ops);
        let metadata = builder.build();
        
        // The batch should require both services
        if let Some(ref_schema) = metadata.reference_schema {
            let required = ref_schema.get("required")
                .and_then(|r| r.as_array())
                .unwrap();
            assert_eq!(required.len(), 2);
            
            let required_strs: Vec<&str> = required.iter()
                .filter_map(|v| v.as_str())
                .collect();
            assert!(required_strs.contains(&"service_a"));
            assert!(required_strs.contains(&"service_b"));
        }
    }
}
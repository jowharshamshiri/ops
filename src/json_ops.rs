// JSON operation examples - equivalent to Java op/ directory
// Implements DeserializeJsonOp and SerializeToJsonOp with serde_json

use crate::operation::Operation;
use crate::context::OperationalContext;
use crate::error::OperationError;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use serde_json;

/// JSON deserialization operation
/// Equivalent to Java DeserializeJsonOp.java
pub struct DeserializeJsonOperation<T> 
where 
    T: for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    json_string: String,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> DeserializeJsonOperation<T>
where
    T: for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    pub fn new(json_string: String) -> Self {
        Self {
            json_string,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn from_str(json_str: &str) -> Self {
        Self::new(json_str.to_string())
    }
}

#[async_trait]
impl<T> Operation<T> for DeserializeJsonOperation<T>
where
    T: for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    async fn perform(&self, _context: &mut OperationalContext) -> Result<T, OperationError> {
        serde_json::from_str(&self.json_string)
            .map_err(|e| OperationError::ExecutionFailed(
                format!("JSON deserialization failed: {}", e)
            ))
    }
}

/// JSON serialization operation  
/// Equivalent to Java SerializeToJsonOp.java
pub struct SerializeToJsonOperation<T>
where
    T: Serialize + Send + Sync,
{
    data: T,
}

impl<T> SerializeToJsonOperation<T>
where
    T: Serialize + Send + Sync,
{
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

#[async_trait]
impl<T> Operation<String> for SerializeToJsonOperation<T>
where
    T: Serialize + Send + Sync,
{
    async fn perform(&self, _context: &mut OperationalContext) -> Result<String, OperationError> {
        serde_json::to_string(&self.data)
            .map_err(|e| OperationError::ExecutionFailed(
                format!("JSON serialization failed: {}", e)
            ))
    }
}

/// Pretty-print JSON serialization operation
/// Enhancement over Java version
pub struct SerializeToPrettyJsonOperation<T>
where
    T: Serialize + Send + Sync,
{
    data: T,
}

impl<T> SerializeToPrettyJsonOperation<T>
where
    T: Serialize + Send + Sync,
{
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

#[async_trait]
impl<T> Operation<String> for SerializeToPrettyJsonOperation<T>
where
    T: Serialize + Send + Sync,
{
    async fn perform(&self, _context: &mut OperationalContext) -> Result<String, OperationError> {
        serde_json::to_string_pretty(&self.data)
            .map_err(|e| OperationError::ExecutionFailed(
                format!("Pretty JSON serialization failed: {}", e)
            ))
    }
}

/// JSON roundtrip operation (serialize then deserialize for validation)
/// Enhanced operation not in Java version
pub struct JsonRoundtripOperation<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    data: T,
}

impl<T> JsonRoundtripOperation<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

#[async_trait]
impl<T> Operation<T> for JsonRoundtripOperation<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    async fn perform(&self, _context: &mut OperationalContext) -> Result<T, OperationError> {
        // First serialize to JSON
        let json_string = serde_json::to_string(&self.data)
            .map_err(|e| OperationError::ExecutionFailed(
                format!("JSON serialization failed in roundtrip: {}", e)
            ))?;

        // Then deserialize back
        serde_json::from_str(&json_string)
            .map_err(|e| OperationError::ExecutionFailed(
                format!("JSON deserialization failed in roundtrip: {}", e)
            ))
    }
}

/// Convenience functions for creating JSON operations
/// Similar to Java static factory methods

/// Create a deserialization operation from JSON string
pub fn deserialize_json<T>(json_string: String) -> DeserializeJsonOperation<T>
where
    T: for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    DeserializeJsonOperation::new(json_string)
}

/// Create a serialization operation from data
pub fn serialize_to_json<T>(data: T) -> SerializeToJsonOperation<T>
where
    T: Serialize + Send + Sync,
{
    SerializeToJsonOperation::new(data)
}

/// Create a pretty serialization operation from data
pub fn serialize_to_pretty_json<T>(data: T) -> SerializeToPrettyJsonOperation<T>
where
    T: Serialize + Send + Sync,
{
    SerializeToPrettyJsonOperation::new(data)
}

/// Create a roundtrip validation operation
pub fn json_roundtrip<T>(data: T) -> JsonRoundtripOperation<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    JsonRoundtripOperation::new(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Serialize, Deserialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    struct TestData {
        name: String,
        age: u32,
        active: bool,
    }

    #[tokio::test]
    async fn test_deserialize_json_operation() {
        let json_str = r#"{"name":"John","age":30,"active":true}"#;
        let op = DeserializeJsonOperation::<TestData>::from_str(json_str);
        let mut context = OperationalContext::new();
        
        let result = op.perform(&mut context).await;
        assert!(result.is_ok());
        
        let data = result.unwrap();
        assert_eq!(data.name, "John");
        assert_eq!(data.age, 30);
        assert_eq!(data.active, true);
    }

    #[tokio::test]
    async fn test_serialize_json_operation() {
        let data = TestData {
            name: "Alice".to_string(),
            age: 25,
            active: false,
        };
        
        let op = SerializeToJsonOperation::new(data);
        let mut context = OperationalContext::new();
        
        let result = op.perform(&mut context).await;
        assert!(result.is_ok());
        
        let json_string = result.unwrap();
        assert!(json_string.contains("Alice"));
        assert!(json_string.contains("25"));
        assert!(json_string.contains("false"));
    }

    #[tokio::test]
    async fn test_pretty_json_operation() {
        let data = TestData {
            name: "Bob".to_string(),
            age: 35,
            active: true,
        };
        
        let op = SerializeToPrettyJsonOperation::new(data);
        let mut context = OperationalContext::new();
        
        let result = op.perform(&mut context).await;
        assert!(result.is_ok());
        
        let pretty_json = result.unwrap();
        assert!(pretty_json.contains("Bob"));
        assert!(pretty_json.contains('\n')); // Should have newlines for pretty printing
        assert!(pretty_json.contains("  ")); // Should have indentation
    }

    #[tokio::test]
    async fn test_json_roundtrip_operation() {
        let original_data = TestData {
            name: "Charlie".to_string(),
            age: 40,
            active: true,
        };
        
        let op = JsonRoundtripOperation::new(original_data.clone());
        let mut context = OperationalContext::new();
        
        let result = op.perform(&mut context).await;
        assert!(result.is_ok());
        
        let roundtrip_data = result.unwrap();
        assert_eq!(original_data, roundtrip_data);
    }

    #[tokio::test]
    async fn test_deserialize_invalid_json() {
        let invalid_json = r#"{"name":"John","age":invalid}"#;
        let op = DeserializeJsonOperation::<TestData>::from_str(invalid_json);
        let mut context = OperationalContext::new();
        
        let result = op.perform(&mut context).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            OperationError::ExecutionFailed(msg) => {
                assert!(msg.contains("JSON deserialization failed"));
            },
            _ => panic!("Expected ExecutionFailed error"),
        }
    }

    #[tokio::test]
    async fn test_convenience_functions() {
        let json_str = r#"{"name":"Dave","age":28,"active":false}"#.to_string();
        let mut context = OperationalContext::new();
        
        // Test deserialize convenience function
        let deserialize_op = deserialize_json::<TestData>(json_str);
        let data = deserialize_op.perform(&mut context).await.unwrap();
        assert_eq!(data.name, "Dave");
        
        // Test serialize convenience function  
        let serialize_op = serialize_to_json(data.clone());
        let json_result = serialize_op.perform(&mut context).await.unwrap();
        assert!(json_result.contains("Dave"));
        
        // Test pretty serialize convenience function
        let pretty_op = serialize_to_pretty_json(data.clone());
        let pretty_result = pretty_op.perform(&mut context).await.unwrap();
        assert!(pretty_result.contains("Dave"));
        assert!(pretty_result.contains('\n'));
        
        // Test roundtrip convenience function
        let roundtrip_op = json_roundtrip(data.clone());
        let roundtrip_result = roundtrip_op.perform(&mut context).await.unwrap();
        assert_eq!(data, roundtrip_result);
    }

    #[tokio::test]
    async fn test_complex_data_structures() {
        #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
        struct ComplexData {
            users: Vec<TestData>,
            metadata: std::collections::HashMap<String, serde_json::Value>,
        }

        let mut metadata = std::collections::HashMap::new();
        metadata.insert("version".to_string(), serde_json::json!("1.0"));
        metadata.insert("count".to_string(), serde_json::json!(2));

        let complex_data = ComplexData {
            users: vec![
                TestData { name: "User1".to_string(), age: 20, active: true },
                TestData { name: "User2".to_string(), age: 30, active: false },
            ],
            metadata,
        };

        // Test roundtrip with complex structure
        let op = JsonRoundtripOperation::new(complex_data.clone());
        let mut context = OperationalContext::new();
        
        let result = op.perform(&mut context).await;
        assert!(result.is_ok());
        
        let roundtrip_data = result.unwrap();
        assert_eq!(complex_data, roundtrip_data);
    }
}
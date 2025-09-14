// JSON op examples - equivalent to Java op/ directory
// Implements DeserializeJsonOp and SerializeToJsonOp with serde_json

use crate::op::Op;
use crate::context::OpContext;
use crate::error::OpError;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use serde_json;

/// JSON deserialization op
/// Equivalent to Java DeserializeJsonOp.java
pub struct DeserializeJsonOp<T> 
where 
    T: for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    json_string: String,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> DeserializeJsonOp<T>
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
impl<T> Op<T> for DeserializeJsonOp<T>
where
    T: for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    async fn perform(&self, _context: &mut OpContext) -> Result<T, OpError> {
        serde_json::from_str(&self.json_string)
            .map_err(|e| OpError::ExecutionFailed(
                format!("JSON deserialization failed: {}", e)
            ))
    }
}

/// JSON serialization op  
/// Equivalent to Java SerializeToJsonOp.java
pub struct SerializeToJsonOp<T>
where
    T: Serialize + Send + Sync,
{
    data: T,
}

impl<T> SerializeToJsonOp<T>
where
    T: Serialize + Send + Sync,
{
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

#[async_trait]
impl<T> Op<String> for SerializeToJsonOp<T>
where
    T: Serialize + Send + Sync,
{
    async fn perform(&self, _context: &mut OpContext) -> Result<String, OpError> {
        serde_json::to_string(&self.data)
            .map_err(|e| OpError::ExecutionFailed(
                format!("JSON serialization failed: {}", e)
            ))
    }
}

/// Pretty-print JSON serialization op
/// Enhancement over Java version
pub struct SerializeToPrettyJsonOp<T>
where
    T: Serialize + Send + Sync,
{
    data: T,
}

impl<T> SerializeToPrettyJsonOp<T>
where
    T: Serialize + Send + Sync,
{
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

#[async_trait]
impl<T> Op<String> for SerializeToPrettyJsonOp<T>
where
    T: Serialize + Send + Sync,
{
    async fn perform(&self, _context: &mut OpContext) -> Result<String, OpError> {
        serde_json::to_string_pretty(&self.data)
            .map_err(|e| OpError::ExecutionFailed(
                format!("Pretty JSON serialization failed: {}", e)
            ))
    }
}

/// JSON roundtrip op (serialize then deserialize for validation)
/// Enhanced op not in Java version
pub struct JsonRoundtripOp<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    data: T,
}

impl<T> JsonRoundtripOp<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

#[async_trait]
impl<T> Op<T> for JsonRoundtripOp<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    async fn perform(&self, _context: &mut OpContext) -> Result<T, OpError> {
        // First serialize to JSON
        let json_string = serde_json::to_string(&self.data)
            .map_err(|e| OpError::ExecutionFailed(
                format!("JSON serialization failed in roundtrip: {}", e)
            ))?;

        // Then deserialize back
        serde_json::from_str(&json_string)
            .map_err(|e| OpError::ExecutionFailed(
                format!("JSON deserialization failed in roundtrip: {}", e)
            ))
    }
}

/// Convenience functions for creating JSON ops
/// Similar to Java static factory methods

/// Create a deserialization op from JSON string
pub fn deserialize_json<T>(json_string: String) -> DeserializeJsonOp<T>
where
    T: for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    DeserializeJsonOp::new(json_string)
}

/// Create a serialization op from data
pub fn serialize_to_json<T>(data: T) -> SerializeToJsonOp<T>
where
    T: Serialize + Send + Sync,
{
    SerializeToJsonOp::new(data)
}

/// Create a pretty serialization op from data
pub fn serialize_to_pretty_json<T>(data: T) -> SerializeToPrettyJsonOp<T>
where
    T: Serialize + Send + Sync,
{
    SerializeToPrettyJsonOp::new(data)
}

/// Create a roundtrip validation op
pub fn json_roundtrip<T>(data: T) -> JsonRoundtripOp<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    JsonRoundtripOp::new(data)
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
    async fn test_deserialize_json_op() {
        let json_str = r#"{"name":"John","age":30,"active":true}"#;
        let op = DeserializeJsonOp::<TestData>::from_str(json_str);
        let mut context = OpContext::new();
        
        let result = op.perform(&mut context).await;
        assert!(result.is_ok());
        
        let data = result.unwrap();
        assert_eq!(data.name, "John");
        assert_eq!(data.age, 30);
        assert_eq!(data.active, true);
    }

    #[tokio::test]
    async fn test_serialize_json_op() {
        let data = TestData {
            name: "Alice".to_string(),
            age: 25,
            active: false,
        };
        
        let op = SerializeToJsonOp::new(data);
        let mut context = OpContext::new();
        
        let result = op.perform(&mut context).await;
        assert!(result.is_ok());
        
        let json_string = result.unwrap();
        assert!(json_string.contains("Alice"));
        assert!(json_string.contains("25"));
        assert!(json_string.contains("false"));
    }

    #[tokio::test]
    async fn test_pretty_json_op() {
        let data = TestData {
            name: "Bob".to_string(),
            age: 35,
            active: true,
        };
        
        let op = SerializeToPrettyJsonOp::new(data);
        let mut context = OpContext::new();
        
        let result = op.perform(&mut context).await;
        assert!(result.is_ok());
        
        let pretty_json = result.unwrap();
        assert!(pretty_json.contains("Bob"));
        assert!(pretty_json.contains('\n')); // Should have newlines for pretty printing
        assert!(pretty_json.contains("  ")); // Should have indentation
    }

    #[tokio::test]
    async fn test_json_roundtrip_op() {
        let original_data = TestData {
            name: "Charlie".to_string(),
            age: 40,
            active: true,
        };
        
        let op = JsonRoundtripOp::new(original_data.clone());
        let mut context = OpContext::new();
        
        let result = op.perform(&mut context).await;
        assert!(result.is_ok());
        
        let roundtrip_data = result.unwrap();
        assert_eq!(original_data, roundtrip_data);
    }

    #[tokio::test]
    async fn test_deserialize_invalid_json() {
        let invalid_json = r#"{"name":"John","age":invalid}"#;
        let op = DeserializeJsonOp::<TestData>::from_str(invalid_json);
        let mut context = OpContext::new();
        
        let result = op.perform(&mut context).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            OpError::ExecutionFailed(msg) => {
                assert!(msg.contains("JSON deserialization failed"));
            },
            _ => panic!("Expected ExecutionFailed error"),
        }
    }

    #[tokio::test]
    async fn test_convenience_functions() {
        let json_str = r#"{"name":"Dave","age":28,"active":false}"#.to_string();
        let mut context = OpContext::new();
        
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
        let op = JsonRoundtripOp::new(complex_data.clone());
        let mut context = OpContext::new();
        
        let result = op.perform(&mut context).await;
        assert!(result.is_ok());
        
        let roundtrip_data = result.unwrap();
        assert_eq!(complex_data, roundtrip_data);
    }
}
use crate::prelude::*;
use std::collections::HashMap;

/// Control flow flags for ops execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlFlags {
    pub aborted: bool,
    pub abort_reason: Option<String>,
}

impl Default for ControlFlags {
    fn default() -> Self {
        Self {
            aborted: false,
            abort_reason: None,
        }
    }
}

/// DryContext contains only serializable data values
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DryContext {
    values: HashMap<String, serde_json::Value>,
    control_flags: ControlFlags,
}

impl DryContext {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            control_flags: ControlFlags::default(),
        }
    }

    pub fn with_value<T: Serialize>(mut self, key: impl Into<String>, value: T) -> Self {
        self.insert(key, value);
        self
    }

    pub fn insert<T: Serialize>(&mut self, key: impl Into<String>, value: T) {
        self.values.insert(
            key.into(),
            serde_json::to_value(value).expect("Failed to serialize value"),
        );
    }

    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.values
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    pub fn get_required<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<T, OpError> {
        match self.values.get(key) {
            None => Err(OpError::Context(format!("Required dry context key '{}' not found", key))),
            Some(value) => {
                match serde_json::from_value::<T>(value.clone()) {
                    Ok(parsed) => Ok(parsed),
                    Err(_) => {
                        let actual_type = match value {
                            serde_json::Value::Null => "null",
                            serde_json::Value::Bool(_) => "boolean",
                            serde_json::Value::Number(_) => "number",
                            serde_json::Value::String(_) => "string",
                            serde_json::Value::Array(_) => "array",
                            serde_json::Value::Object(_) => "object",
                        };
                        let expected_type = std::any::type_name::<T>();
                        Err(OpError::Context(format!(
                            "Type mismatch for dry context key '{}': expected type '{}', but found '{}' value: {}",
                            key, expected_type, actual_type, value
                        )))
                    }
                }
            }
        }
    }

    pub fn contains(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.values.keys()
    }

    pub fn values(&self) -> &HashMap<String, serde_json::Value> {
        &self.values
    }

    pub fn merge(&mut self, other: DryContext) {
        self.values.extend(other.values);
        // Only merge control flags if they are set in other and not already set in self
        if other.control_flags.aborted && !self.control_flags.aborted {
            self.control_flags.aborted = true;
            self.control_flags.abort_reason = other.control_flags.abort_reason;
        }
    }
    
    /// Set abort flag with optional reason
    pub fn set_abort(&mut self, reason: Option<String>) {
        self.control_flags.aborted = true;
        self.control_flags.abort_reason = reason;
    }
    
    /// Check if abort flag is set
    pub fn is_aborted(&self) -> bool {
        self.control_flags.aborted
    }
    
    /// Get abort reason if set
    pub fn abort_reason(&self) -> Option<&String> {
        self.control_flags.abort_reason.as_ref()
    }
    
    
    
    /// Clear all control flags
    pub fn clear_control_flags(&mut self) {
        self.control_flags = ControlFlags::default();
    }
}

/// WetContext contains runtime references (services, connections, etc.)
#[derive(Debug, Default)]
pub struct WetContext {
    references: HashMap<String, Arc<dyn Any + Send + Sync>>,
}

// WetContext is Send and Sync because all its contents are Send + Sync
unsafe impl Send for WetContext {}
unsafe impl Sync for WetContext {}

impl WetContext {
    pub fn new() -> Self {
        Self {
            references: HashMap::new(),
        }
    }

    pub fn with_ref<T: Any + Send + Sync>(mut self, key: impl Into<String>, value: T) -> Self {
        self.insert_ref(key, value);
        self
    }

    pub fn insert_ref<T: Any + Send + Sync>(&mut self, key: impl Into<String>, value: T) {
        self.references.insert(key.into(), Arc::new(value));
    }

    pub fn insert_arc(&mut self, key: impl Into<String>, value: Arc<dyn Any + Send + Sync>) {
        self.references.insert(key.into(), value);
    }

    pub fn get_ref<T: Any + Send + Sync>(&self, key: &str) -> Option<Arc<T>> {
        self.references
            .get(key)
            .and_then(|any_ref| any_ref.clone().downcast::<T>().ok())
    }

    pub fn get_required<T: Any + Send + Sync>(&self, key: &str) -> Result<Arc<T>, OpError> {
        match self.references.get(key) {
            None => Err(OpError::Context(format!("Required wet context reference '{}' not found", key))),
            Some(any_ref) => {
                match any_ref.clone().downcast::<T>() {
                    Ok(typed_ref) => Ok(typed_ref),
                    Err(_) => {
                        let expected_type = std::any::type_name::<T>();
                        Err(OpError::Context(format!(
                            "Type mismatch for wet context reference '{}': expected type '{}', but found a different type",
                            key, expected_type
                        )))
                    }
                }
            }
        }
    }

    pub fn contains(&self, key: &str) -> bool {
        self.references.contains_key(key)
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.references.keys()
    }

    pub fn merge(&mut self, other: WetContext) {
        self.references.extend(other.references);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dry_context_basic_operations() {
        let mut ctx = DryContext::new();
        ctx.insert("name", "test");
        ctx.insert("count", 42);

        assert_eq!(ctx.get::<String>("name").unwrap(), "test");
        assert_eq!(ctx.get::<i32>("count").unwrap(), 42);
        assert!(ctx.contains("name"));
        assert!(!ctx.contains("missing"));
    }

    #[test]
    fn test_dry_context_builder() {
        let ctx = DryContext::new()
            .with_value("key1", "value1")
            .with_value("key2", 123);

        assert_eq!(ctx.get::<String>("key1").unwrap(), "value1");
        assert_eq!(ctx.get::<i32>("key2").unwrap(), 123);
    }

    #[test]
    fn test_wet_context_basic_operations() {
        #[derive(Debug)]
        struct TestService {
            name: String,
        }

        let mut ctx = WetContext::new();
        let service = TestService {
            name: "test".to_string(),
        };
        ctx.insert_ref("service", service);

        let retrieved = ctx.get_ref::<TestService>("service").unwrap();
        assert_eq!(retrieved.name, "test");
    }

    #[test]
    fn test_wet_context_builder() {
        struct Service1;
        struct Service2;

        let ctx = WetContext::new()
            .with_ref("service1", Service1)
            .with_ref("service2", Service2);

        assert!(ctx.contains("service1"));
        assert!(ctx.contains("service2"));
    }

    #[test]
    fn test_required_values() {
        let ctx = DryContext::new().with_value("exists", 42);

        assert_eq!(ctx.get_required::<i32>("exists").unwrap(), 42);
        assert!(ctx.get_required::<i32>("missing").is_err());
    }

    #[test]
    fn test_context_merge() {
        let mut ctx1 = DryContext::new().with_value("a", 1);
        let ctx2 = DryContext::new().with_value("b", 2);

        ctx1.merge(ctx2);
        assert_eq!(ctx1.get::<i32>("a").unwrap(), 1);
        assert_eq!(ctx1.get::<i32>("b").unwrap(), 2);
    }

    #[test]
    fn test_dry_context_type_mismatch_error() {
        let ctx = DryContext::new()
            .with_value("count", "not_a_number")
            .with_value("flag", 123);
        
        // String value, expecting i32
        let result = ctx.get_required::<i32>("count");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Type mismatch"));
        assert!(err.contains("expected type 'i32'"));
        assert!(err.contains("found 'string' value"));
        
        // Number value, expecting bool
        let result = ctx.get_required::<bool>("flag");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Type mismatch"));
        assert!(err.contains("expected type 'bool'"));
        assert!(err.contains("found 'number' value"));
        
        // Missing key still gives "not found"
        let result = ctx.get_required::<i32>("missing");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not found"));
        assert!(!err.contains("Type mismatch"));
    }

    #[test]
    fn test_wet_context_type_mismatch_error() {
        #[derive(Debug)]
        struct ServiceA {
            _name: String,
        }
        #[derive(Debug)]
        struct ServiceB {
            _id: i32,
        }
        
        let mut ctx = WetContext::new();
        ctx.insert_ref("service", ServiceA { _name: "test".to_string() });
        
        // Wrong type
        let result = ctx.get_required::<ServiceB>("service");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Type mismatch"));
        assert!(err.contains("expected type"));
        assert!(err.contains("ServiceB"));
        
        // Missing key
        let result = ctx.get_required::<ServiceA>("missing");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not found"));
        assert!(!err.contains("Type mismatch"));
    }

    #[test]
    fn test_control_flags() {
        let mut ctx = DryContext::new();
        
        // Test abort functionality
        assert!(!ctx.is_aborted());
        assert_eq!(ctx.abort_reason(), None);
        
        ctx.set_abort(Some("Test abort reason".to_string()));
        assert!(ctx.is_aborted());
        assert_eq!(ctx.abort_reason(), Some(&"Test abort reason".to_string()));
        
        // Test clearing all flags
        ctx.set_abort(Some("Another reason".to_string()));
        assert!(ctx.is_aborted());
        
        ctx.clear_control_flags();
        assert!(!ctx.is_aborted());
        assert_eq!(ctx.abort_reason(), None);
    }

    #[test]
    fn test_control_flags_merge() {
        let mut ctx1 = DryContext::new();
        let mut ctx2 = DryContext::new();
        
        // Set flags in ctx2
        ctx2.set_abort(Some("Merged abort".to_string()));
        
        // Merge ctx2 into ctx1
        ctx1.merge(ctx2);
        
        assert!(ctx1.is_aborted());
        assert_eq!(ctx1.abort_reason(), Some(&"Merged abort".to_string()));
        
        // Test that merge doesn't override existing abort
        let mut ctx3 = DryContext::new();
        ctx3.set_abort(Some("Original abort".to_string()));
        
        let mut ctx4 = DryContext::new();
        ctx4.set_abort(Some("New abort".to_string()));
        
        ctx3.merge(ctx4);
        // Should keep the original abort reason since ctx3 was already aborted
        assert_eq!(ctx3.abort_reason(), Some(&"Original abort".to_string()));
    }
}
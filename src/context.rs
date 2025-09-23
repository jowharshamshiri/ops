use crate::prelude::*;

pub trait RequirementFactory<T>: Send + Sync {
    fn create(&self) -> Result<T, OpError>;
}

/// Functional implementation of RequirementFactory
pub struct ClosureFactory<T, F>
where
    F: Fn() -> Result<T, OpError> + Send + Sync,
{
    closure: F,
}

impl<T, F> ClosureFactory<T, F>
where
    F: Fn() -> Result<T, OpError> + Send + Sync,
{
    pub fn new(closure: F) -> Self {
        Self { closure }
    }
}

impl<T, F> RequirementFactory<T> for ClosureFactory<T, F>
where
    T: serde::Serialize,
    F: Fn() -> Result<T, OpError> + Send + Sync,
{
    fn create(&self) -> Result<T, OpError> {
        (self.closure)()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpContext {
    values: HashMap<String, serde_json::Value>,
    #[serde(skip)]
    references: HashMap<String, Arc<dyn Any + Send + Sync>>,
}

impl OpContext {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            references: HashMap::new(),
        }
    }

    /// Builder pattern: put value and return self for chaining
    /// Equivalent to Java OpContext.build(key, value)
    pub fn build<T: serde::Serialize>(mut self, key: &str, value: T) -> Self {
        let _ = self.put(key, value);
        self
    }

    /// Fluent put that returns self for chaining
    pub fn with<T: serde::Serialize>(mut self, key: &str, value: T) -> Self {
        let _ = self.put(key, value);
        self
    }

    pub fn put<T: serde::Serialize>(&mut self, key: &str, value: T) -> OpResult<()> {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.values.insert(key.to_string(), json_value);
            Ok(())
        } else {
            Err(OpError::Context("Failed to serialize value".to_string()))
        }
    }

    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.values
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Require a value with lazy initialization
    /// Equivalent to Java require(String key, RequirementFactory<T> factory)
    pub fn require<T>(&mut self, key: &str, factory: Box<dyn RequirementFactory<T>>) -> Result<T, OpError>
    where
        T: serde::Serialize + serde::de::DeserializeOwned + Clone,
    {
        // Check if value already exists
        if let Some(existing_value) = self.get::<T>(key) {
            return Ok(existing_value);
        }
        
        // Create value using factory
        let value = factory.create()?;
        
        // Store for future use
        self.put(key, value.clone())?;
        
        Ok(value)
    }

    /// Convenience method for closure-based requirement factories
    pub fn require_with<T, F>(&mut self, key: &str, factory_fn: F) -> Result<T, OpError>
    where
        T: serde::Serialize + serde::de::DeserializeOwned + Clone + 'static,
        F: Fn() -> Result<T, OpError> + Send + Sync + 'static,
    {
        let factory = Box::new(ClosureFactory::new(factory_fn));
        self.require(key, factory)
    }

    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        self.values.remove(key)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.values.keys()
    }

    /// Access to internal values for persistence ops
    pub fn values(&self) -> &HashMap<String, serde_json::Value> {
        &self.values
    }

    /// Insert method for test compatibility
    pub fn insert<T: serde::Serialize>(&mut self, key: String, value: T) -> OpResult<()> {
        self.put(&key, value)
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Get raw JSON value without deserialization
    pub fn get_raw(&self, key: &str) -> Option<&serde_json::Value> {
        self.values.get(key)
    }

    /// Set raw JSON value directly  
    pub fn set(&mut self, key: String, value: serde_json::Value) {
        self.values.insert(key, value);
    }

    /// Store a reference to a value without serialization
    /// The value must be Send + Sync and will be wrapped in Arc
    pub fn put_ref<T>(&mut self, key: &str, value: T) 
    where
        T: Any + Send + Sync + 'static,
    {
        self.references.insert(key.to_string(), Arc::new(value));
    }

    /// Store an already Arc-wrapped reference
    pub fn put_arc<T>(&mut self, key: &str, value: Arc<T>) 
    where
        T: Any + Send + Sync + 'static,
    {
        self.references.insert(key.to_string(), value);
    }

    /// Get a reference to a stored value
    /// Returns None if the key doesn't exist or the type doesn't match
    pub fn get_ref<T>(&self, key: &str) -> Option<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        self.references
            .get(key)
            .and_then(|arc_any| arc_any.clone().downcast::<T>().ok())
    }

    /// Check if a reference exists for the given key
    pub fn contains_ref(&self, key: &str) -> bool {
        self.references.contains_key(key)
    }

    /// Remove a reference from the context
    pub fn remove_ref(&mut self, key: &str) -> bool {
        self.references.remove(key).is_some()
    }

    /// Builder pattern for references
    pub fn with_ref<T>(mut self, key: &str, value: T) -> Self 
    where
        T: Any + Send + Sync + 'static,
    {
        self.put_ref(key, value);
        self
    }
}

/// HOLLOW context pattern - singleton no-op context for testing
/// Equivalent to Java OpContext.HOLLOW
pub struct HollowOpContext;

impl HollowOpContext {
    /// Get the singleton HOLLOW instance
    pub const HOLLOW: HollowOpContext = HollowOpContext;
    
    /// Convert to OpContext (warns about hollow usage)
    pub fn to_context(self) -> OpContext {
        warn!("Using HOLLOW context - ops may not have required values");
        OpContext::new()
    }
    
    /// Create a new hollow context with warning
    pub fn new() -> Self {
        warn!("Creating HOLLOW context for testing/debugging");
        HollowOpContext
    }
}

/// Trait to enable hollow context pattern
pub trait ContextProvider {
    fn get_context(&mut self) -> &mut OpContext;
    fn is_hollow(&self) -> bool { false }
}

impl ContextProvider for OpContext {
    fn get_context(&mut self) -> &mut OpContext {
        self
    }
}

impl ContextProvider for HollowOpContext {
    fn get_context(&mut self) -> &mut OpContext {
        warn!("Hollow context accessed - returning empty context");
        // Note: This is a design limitation - we can't return a mutable reference
        // from a hollow context. In practice, ops using hollow context
        // should handle this gracefully.
        use std::sync::OnceLock;
        static HOLLOW_STORAGE: OnceLock<std::sync::Mutex<OpContext>> = OnceLock::new();
        
        let context = HOLLOW_STORAGE.get_or_init(|| {
            std::sync::Mutex::new(OpContext::new())
        });
        
        // This is unsafe but needed for the hollow context pattern
        // In practice, hollow contexts should be avoided
        unsafe {
            let mutex_ptr = context as *const std::sync::Mutex<OpContext> as *mut std::sync::Mutex<OpContext>;
            let guard = (*mutex_ptr).get_mut().unwrap();
            guard
        }
    }
    
    fn is_hollow(&self) -> bool { true }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_builder() {
        let ctx = OpContext::new()
            .build("key1", "value1")
            .build("key2", 42);
        
        assert_eq!(ctx.get::<String>("key1"), Some("value1".to_string()));
        assert_eq!(ctx.get::<i32>("key2"), Some(42));
    }

    #[test]
    fn test_context_ops() {
        let mut ctx = OpContext::new();
        
        assert!(ctx.put("test_key", "test_value").is_ok());
        assert_eq!(ctx.get::<String>("test_key"), Some("test_value".to_string()));
        
        assert!(ctx.contains_key("test_key"));
        assert!(!ctx.is_empty());
        
        assert!(ctx.remove("test_key").is_some());
        assert!(!ctx.contains_key("test_key"));
    }

    #[test]
    fn test_requirement_factory() {
        let mut ctx = OpContext::new();
        
        // Test lazy initialization
        let result = ctx.require_with("expensive_value", || {
            Ok("computed_value".to_string())
        });
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "computed_value");
        
        // Test that value is cached
        assert_eq!(ctx.get::<String>("expensive_value"), Some("computed_value".to_string()));
        
        // Second call should use cached value
        let result2 = ctx.require_with("expensive_value", || {
            Ok("should_not_be_called".to_string())
        });
        
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap(), "computed_value"); // Still the original value
    }

    #[test]
    fn test_hollow_context() {
        let hollow = HollowOpContext::new();
        let ctx = hollow.to_context();
        
        assert!(ctx.is_empty());
        assert!(!ctx.contains_key("any_key"));
    }

    #[test]
    fn test_hollow_singleton() {
        let hollow1 = HollowOpContext::HOLLOW;
        let hollow2 = HollowOpContext::HOLLOW;
        
        // Both should be hollow contexts
        assert!(hollow1.is_hollow());
        assert!(hollow2.is_hollow());
    }

    #[test]
    fn test_fluent_interface() {
        let ctx = OpContext::new()
            .with("name", "test")
            .with("age", 25)
            .with("active", true);
        
        assert_eq!(ctx.get::<String>("name"), Some("test".to_string()));
        assert_eq!(ctx.get::<i32>("age"), Some(25));
        assert_eq!(ctx.get::<bool>("active"), Some(true));
    }

    #[test] 
    fn test_requirement_factory_error_handling() {
        let mut ctx = OpContext::new();
        
        let result: OpResult<String> = ctx.require_with("failing_value", || {
            Err(OpError::Context("Factory failed".to_string()))
        });
        
        assert!(result.is_err());
        assert!(!ctx.contains_key("failing_value"));
    }

    #[test]
    fn test_reference_storage() {
        let mut ctx = OpContext::new();
        
        // Test storing a reference
        let data = vec![1, 2, 3, 4, 5];
        ctx.put_ref("numbers", data.clone());
        
        // Test retrieving the reference
        let retrieved: Option<Arc<Vec<i32>>> = ctx.get_ref("numbers");
        assert!(retrieved.is_some());
        assert_eq!(*retrieved.unwrap(), data);
        
        // Test contains_ref
        assert!(ctx.contains_ref("numbers"));
        assert!(!ctx.contains_ref("nonexistent"));
    }

    #[test]
    fn test_reference_type_safety() {
        let mut ctx = OpContext::new();
        
        // Store a String
        ctx.put_ref("text", "hello".to_string());
        
        // Try to retrieve as wrong type
        let wrong_type: Option<Arc<i32>> = ctx.get_ref("text");
        assert!(wrong_type.is_none());
        
        // Retrieve as correct type
        let correct_type: Option<Arc<String>> = ctx.get_ref("text");
        assert!(correct_type.is_some());
        assert_eq!(*correct_type.unwrap(), "hello");
    }

    #[test]
    fn test_arc_storage() {
        let mut ctx = OpContext::new();
        
        // Test with pre-wrapped Arc
        let data = Arc::new(vec![10, 20, 30]);
        let data_clone = data.clone();
        ctx.put_arc("shared_data", data);
        
        // Retrieve and verify it's the same Arc
        let retrieved: Option<Arc<Vec<i32>>> = ctx.get_ref("shared_data");
        assert!(retrieved.is_some());
        let retrieved_arc = retrieved.unwrap();
        assert_eq!(*retrieved_arc, *data_clone);
        
        // Verify they point to the same memory
        assert!(Arc::ptr_eq(&retrieved_arc, &data_clone));
    }

    #[test]
    fn test_reference_builder_pattern() {
        let data1 = vec![1, 2, 3];
        let data2 = "test string".to_string();
        
        let ctx = OpContext::new()
            .with_ref("list", data1.clone())
            .with_ref("text", data2.clone());
        
        let retrieved_list: Option<Arc<Vec<i32>>> = ctx.get_ref("list");
        let retrieved_text: Option<Arc<String>> = ctx.get_ref("text");
        
        assert!(retrieved_list.is_some());
        assert!(retrieved_text.is_some());
        assert_eq!(*retrieved_list.unwrap(), data1);
        assert_eq!(*retrieved_text.unwrap(), data2);
    }

    #[test]
    fn test_reference_removal() {
        let mut ctx = OpContext::new();
        
        ctx.put_ref("temp", vec![1, 2, 3]);
        assert!(ctx.contains_ref("temp"));
        
        let removed = ctx.remove_ref("temp");
        assert!(removed);
        assert!(!ctx.contains_ref("temp"));
        
        // Try to remove again
        let removed_again = ctx.remove_ref("temp");
        assert!(!removed_again);
    }
}
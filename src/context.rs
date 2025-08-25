use std::collections::HashMap;
use serde_json;
use serde::{Serialize, Deserialize};
use log::warn;
use crate::OperationError;

/// Trait for lazy initialization of context values
/// Equivalent to Java RequirementFactory<T>
pub trait RequirementFactory<T>: Send + Sync {
    fn create(&self) -> Result<T, OperationError>;
}

/// Functional implementation of RequirementFactory
pub struct ClosureFactory<T, F>
where
    F: Fn() -> Result<T, OperationError> + Send + Sync,
{
    closure: F,
}

impl<T, F> ClosureFactory<T, F>
where
    F: Fn() -> Result<T, OperationError> + Send + Sync,
{
    pub fn new(closure: F) -> Self {
        Self { closure }
    }
}

impl<T, F> RequirementFactory<T> for ClosureFactory<T, F>
where
    T: serde::Serialize,
    F: Fn() -> Result<T, OperationError> + Send + Sync,
{
    fn create(&self) -> Result<T, OperationError> {
        (self.closure)()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OperationalContext {
    values: HashMap<String, serde_json::Value>,
}

impl OperationalContext {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
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

    pub fn put<T: serde::Serialize>(&mut self, key: &str, value: T) -> Result<(), OperationError> {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.values.insert(key.to_string(), json_value);
            Ok(())
        } else {
            Err(OperationError::Context("Failed to serialize value".to_string()))
        }
    }

    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.values
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Require a value with lazy initialization
    /// Equivalent to Java require(String key, RequirementFactory<T> factory)
    pub fn require<T>(&mut self, key: &str, factory: Box<dyn RequirementFactory<T>>) -> Result<T, OperationError>
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
    pub fn require_with<T, F>(&mut self, key: &str, factory_fn: F) -> Result<T, OperationError>
    where
        T: serde::Serialize + serde::de::DeserializeOwned + Clone + 'static,
        F: Fn() -> Result<T, OperationError> + Send + Sync + 'static,
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

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// HOLLOW context pattern - singleton no-op context for testing
/// Equivalent to Java OpContext.HOLLOW
pub struct HollowOpContext;

impl HollowOpContext {
    /// Get the singleton HOLLOW instance
    pub const HOLLOW: HollowOpContext = HollowOpContext;
    
    /// Convert to OperationalContext (warns about hollow usage)
    pub fn to_context(self) -> OperationalContext {
        warn!("Using HOLLOW context - operations may not have required values");
        OperationalContext::new()
    }
    
    /// Create a new hollow context with warning
    pub fn new() -> Self {
        warn!("Creating HOLLOW context for testing/debugging");
        HollowOpContext
    }
}

/// Trait to enable hollow context pattern
pub trait ContextProvider {
    fn get_context(&mut self) -> &mut OperationalContext;
    fn is_hollow(&self) -> bool { false }
}

impl ContextProvider for OperationalContext {
    fn get_context(&mut self) -> &mut OperationalContext {
        self
    }
}

impl ContextProvider for HollowOpContext {
    fn get_context(&mut self) -> &mut OperationalContext {
        warn!("Hollow context accessed - returning empty context");
        // Note: This is a design limitation - we can't return a mutable reference
        // from a hollow context. In practice, operations using hollow context
        // should handle this gracefully.
        static mut HOLLOW_STORAGE: Option<OperationalContext> = None;
        unsafe {
            if HOLLOW_STORAGE.is_none() {
                HOLLOW_STORAGE = Some(OperationalContext::new());
            }
            HOLLOW_STORAGE.as_mut().unwrap()
        }
    }
    
    fn is_hollow(&self) -> bool { true }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_builder() {
        let ctx = OperationalContext::new()
            .build("key1", "value1")
            .build("key2", 42);
        
        assert_eq!(ctx.get::<String>("key1"), Some("value1".to_string()));
        assert_eq!(ctx.get::<i32>("key2"), Some(42));
    }

    #[test]
    fn test_context_operations() {
        let mut ctx = OperationalContext::new();
        
        assert!(ctx.put("test_key", "test_value").is_ok());
        assert_eq!(ctx.get::<String>("test_key"), Some("test_value".to_string()));
        
        assert!(ctx.contains_key("test_key"));
        assert!(!ctx.is_empty());
        
        assert!(ctx.remove("test_key").is_some());
        assert!(!ctx.contains_key("test_key"));
    }

    #[test]
    fn test_requirement_factory() {
        let mut ctx = OperationalContext::new();
        
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
        let ctx = OperationalContext::new()
            .with("name", "test")
            .with("age", 25)
            .with("active", true);
        
        assert_eq!(ctx.get::<String>("name"), Some("test".to_string()));
        assert_eq!(ctx.get::<i32>("age"), Some(25));
        assert_eq!(ctx.get::<bool>("active"), Some(true));
    }

    #[test] 
    fn test_requirement_factory_error_handling() {
        let mut ctx = OperationalContext::new();
        
        let result: Result<String, OperationError> = ctx.require_with("failing_value", || {
            Err(OperationError::Context("Factory failed".to_string()))
        });
        
        assert!(result.is_err());
        assert!(!ctx.contains_key("failing_value"));
    }
}
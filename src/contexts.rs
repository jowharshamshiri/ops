use crate::prelude::*;
use std::collections::HashMap;

/// DryContext contains only serializable data values
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DryContext {
    values: HashMap<String, serde_json::Value>,
}

impl DryContext {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
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
        self.get(key)
            .ok_or_else(|| OpError::Context(format!("Required dry context key '{}' not found", key)))
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
        self.get_ref(key)
            .ok_or_else(|| OpError::Context(format!("Required wet context reference '{}' not found", key)))
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
}
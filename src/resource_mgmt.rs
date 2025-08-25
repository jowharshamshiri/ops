// Resource Management - Automatic resource cleanup and management
// Leverages Rust's ownership system for superior resource handling

use crate::operation::Operation;
use crate::context::OperationalContext;
use crate::error::OperationError;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use log::{info, warn, error};

/// Resource trait for managed resources
pub trait ManagedResource: Send + Sync {
    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn is_healthy(&self) -> bool;
    fn resource_type(&self) -> &'static str;
}

/// Resource pool for managing shared resources
pub struct ResourcePool<R> 
where
    R: ManagedResource + 'static,
{
    resources: Arc<Mutex<Vec<Arc<Mutex<R>>>>>,
    max_size: usize,
    created_count: Arc<Mutex<usize>>,
    resource_factory: Box<dyn Fn() -> R + Send + Sync>,
}

impl<R> std::fmt::Debug for ResourcePool<R>
where
    R: ManagedResource + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourcePool")
            .field("max_size", &self.max_size)
            .field("created_count", &self.created_count)
            .finish()
    }
}

impl<R> ResourcePool<R>
where
    R: ManagedResource + 'static,
{
    pub fn new<F>(max_size: usize, factory: F) -> Self 
    where
        F: Fn() -> R + Send + Sync + 'static,
    {
        Self {
            resources: Arc::new(Mutex::new(Vec::new())),
            max_size,
            created_count: Arc::new(Mutex::new(0)),
            resource_factory: Box::new(factory),
        }
    }

    pub fn acquire(&self) -> Result<ManagedResourceHandle<R>, OperationError> {
        // Try to get existing resource
        if let Ok(mut resources) = self.resources.lock() {
            if let Some(resource) = resources.pop() {
                return Ok(ManagedResourceHandle::new(resource, self.resources.clone()));
            }
        }

        // Create new resource if under limit
        if let Ok(mut count) = self.created_count.lock() {
            if *count < self.max_size {
                let mut new_resource = (self.resource_factory)();
                if let Err(e) = new_resource.initialize() {
                    return Err(OperationError::ExecutionFailed(
                        format!("Failed to initialize resource: {}", e)
                    ));
                }
                
                *count += 1;
                let resource = Arc::new(Mutex::new(new_resource));
                return Ok(ManagedResourceHandle::new(resource, self.resources.clone()));
            }
        }

        Err(OperationError::ExecutionFailed(
            "Resource pool exhausted".to_string()
        ))
    }

    pub fn size(&self) -> usize {
        self.resources.lock().unwrap().len()
    }

    pub fn created_count(&self) -> usize {
        *self.created_count.lock().unwrap()
    }

    pub fn cleanup_unhealthy(&self) -> usize {
        if let Ok(mut resources) = self.resources.lock() {
            let original_size = resources.len();
            resources.retain(|resource| {
                if let Ok(r) = resource.lock() {
                    r.is_healthy()
                } else {
                    false
                }
            });
            original_size - resources.len()
        } else {
            0
        }
    }
}

/// RAII handle for managed resources
pub struct ManagedResourceHandle<R>
where
    R: ManagedResource,
{
    resource: Arc<Mutex<R>>,
    pool: Arc<Mutex<Vec<Arc<Mutex<R>>>>>,
    returned: bool,
}

impl<R> ManagedResourceHandle<R>
where
    R: ManagedResource,
{
    fn new(resource: Arc<Mutex<R>>, pool: Arc<Mutex<Vec<Arc<Mutex<R>>>>>) -> Self {
        Self {
            resource,
            pool,
            returned: false,
        }
    }

    pub fn get(&self) -> Result<std::sync::MutexGuard<R>, OperationError> {
        self.resource.lock().map_err(|_| {
            OperationError::ExecutionFailed("Failed to acquire resource lock".to_string())
        })
    }

    pub fn resource_type(&self) -> Result<&'static str, OperationError> {
        Ok(self.get()?.resource_type())
    }

    pub fn is_healthy(&self) -> bool {
        if let Ok(resource) = self.resource.lock() {
            resource.is_healthy()
        } else {
            false
        }
    }
}

impl<R> Drop for ManagedResourceHandle<R>
where
    R: ManagedResource,
{
    fn drop(&mut self) {
        if !self.returned && self.is_healthy() {
            if let Ok(mut pool) = self.pool.lock() {
                pool.push(self.resource.clone());
                self.returned = true;
            }
        }
        // If unhealthy or can't return to pool, resource will be dropped
    }
}

/// Resource-managed operation wrapper
pub struct ResourceManagedOperation<T, R>
where
    R: ManagedResource + 'static,
{
    operation: Box<dyn Operation<T>>,
    resource_pool: Arc<ResourcePool<R>>,
    operation_name: String,
    acquire_timeout: Duration,
}

impl<T, R> ResourceManagedOperation<T, R>
where
    T: Send + 'static,
    R: ManagedResource + 'static,
{
    pub fn new(operation: Box<dyn Operation<T>>, resource_pool: Arc<ResourcePool<R>>) -> Self {
        Self {
            operation,
            resource_pool,
            operation_name: "ResourceManagedOperation".to_string(),
            acquire_timeout: Duration::from_secs(30),
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.operation_name = name;
        self
    }

    pub fn with_acquire_timeout(mut self, timeout: Duration) -> Self {
        self.acquire_timeout = timeout;
        self
    }

    async fn acquire_resource_with_timeout(&self) -> Result<ManagedResourceHandle<R>, OperationError> {
        let start = Instant::now();
        
        loop {
            match self.resource_pool.acquire() {
                Ok(handle) => return Ok(handle),
                Err(_) if start.elapsed() < self.acquire_timeout => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                },
                Err(e) => return Err(e),
            }
        }
    }
}

#[async_trait]
impl<T, R> Operation<T> for ResourceManagedOperation<T, R>
where
    T: Send + 'static,
    R: ManagedResource + 'static,
{
    async fn perform(&self, context: &mut OperationalContext) -> Result<T, OperationError> {
        info!("Acquiring resource for operation '{}'", self.operation_name);
        
        let resource_handle = self.acquire_resource_with_timeout().await?;
        let resource_type = resource_handle.resource_type()?;
        
        info!("Acquired {} resource for operation '{}'", resource_type, self.operation_name);
        
        // Store resource handle in context for operation use
        context.put("resource_handle", format!("{}:{}", resource_type, self.operation_name))?;
        
        let result = self.operation.perform(context).await;
        
        match &result {
            Ok(_) => info!("Operation '{}' completed, resource will be returned to pool", self.operation_name),
            Err(e) => warn!("Operation '{}' failed: {:?}, resource will be returned to pool", self.operation_name, e),
        }
        
        // Resource is automatically returned to pool when handle drops
        result
    }
}

/// Connection pool for database/network connections
pub struct ConnectionResource {
    connection_string: String,
    connected: bool,
    last_health_check: Instant,
    connection_id: u64,
}

impl ConnectionResource {
    pub fn new(connection_string: String, connection_id: u64) -> Self {
        Self {
            connection_string,
            connected: false,
            last_health_check: Instant::now(),
            connection_id,
        }
    }

    pub fn connection_id(&self) -> u64 {
        self.connection_id
    }

    pub fn connection_string(&self) -> &str {
        &self.connection_string
    }

    pub fn simulate_query(&mut self, _query: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if !self.connected {
            return Err("Connection not established".into());
        }
        
        // Simulate query execution
        std::thread::sleep(std::time::Duration::from_millis(10));
        Ok("Query result".to_string())
    }
}

impl ManagedResource for ConnectionResource {
    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Initializing connection {} to {}", self.connection_id, self.connection_string);
        // Simulate connection establishment
        std::thread::sleep(Duration::from_millis(100));
        self.connected = true;
        self.last_health_check = Instant::now();
        Ok(())
    }

    fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Cleaning up connection {}", self.connection_id);
        self.connected = false;
        Ok(())
    }

    fn is_healthy(&self) -> bool {
        self.connected && self.last_health_check.elapsed() < Duration::from_secs(300)
    }

    fn resource_type(&self) -> &'static str {
        "DatabaseConnection"
    }
}

/// File handle resource for managed file operations
pub struct FileResource {
    file_path: String,
    opened: bool,
    read_only: bool,
}

impl FileResource {
    pub fn new(file_path: String, read_only: bool) -> Self {
        Self {
            file_path,
            opened: false,
            read_only,
        }
    }

    pub fn file_path(&self) -> &str {
        &self.file_path
    }

    pub fn simulate_read(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if !self.opened {
            return Err("File not opened".into());
        }
        Ok(format!("Contents of {}", self.file_path))
    }

    pub fn simulate_write(&mut self, _data: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.opened {
            return Err("File not opened".into());
        }
        if self.read_only {
            return Err("File opened in read-only mode".into());
        }
        Ok(())
    }
}

impl ManagedResource for FileResource {
    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Opening file: {} (read-only: {})", self.file_path, self.read_only);
        // Simulate file opening
        self.opened = true;
        Ok(())
    }

    fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Closing file: {}", self.file_path);
        self.opened = false;
        Ok(())
    }

    fn is_healthy(&self) -> bool {
        self.opened
    }

    fn resource_type(&self) -> &'static str {
        "FileHandle"
    }
}

/// Convenience functions for creating resource-managed operations

pub fn with_connection_pool<T>(
    operation: Box<dyn Operation<T>>, 
    connection_strings: Vec<String>,
    pool_size: usize
) -> ResourceManagedOperation<T, ConnectionResource>
where
    T: Send + 'static,
{
    use std::sync::atomic::{AtomicU64, Ordering};
    let connection_id = Arc::new(AtomicU64::new(0));
    let connection_id_clone = connection_id.clone();
    
    let pool = Arc::new(ResourcePool::new(pool_size, move || {
        let id = connection_id_clone.fetch_add(1, Ordering::SeqCst);
        let conn_str = connection_strings[id as usize % connection_strings.len()].clone();
        ConnectionResource::new(conn_str, id + 1)
    }));
    
    ResourceManagedOperation::new(operation, pool)
}

pub fn with_file_pool<T>(
    operation: Box<dyn Operation<T>>,
    file_paths: Vec<String>,
    read_only: bool,
    pool_size: usize
) -> ResourceManagedOperation<T, FileResource>
where
    T: Send + 'static,
{
    use std::sync::atomic::{AtomicUsize, Ordering};
    let file_index = Arc::new(AtomicUsize::new(0));
    let file_index_clone = file_index.clone();
    
    let pool = Arc::new(ResourcePool::new(pool_size, move || {
        let idx = file_index_clone.fetch_add(1, Ordering::SeqCst);
        let file_path = file_paths[idx % file_paths.len()].clone();
        FileResource::new(file_path, read_only)
    }));
    
    ResourceManagedOperation::new(operation, pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation::ClosureOperation;

    #[test]
    fn test_connection_resource() {
        let mut conn = ConnectionResource::new("test://localhost".to_string(), 1);
        assert!(!conn.is_healthy());
        
        conn.initialize().unwrap();
        assert!(conn.is_healthy());
        assert_eq!(conn.connection_id(), 1);
        assert_eq!(conn.connection_string(), "test://localhost");
        
        let result = conn.simulate_query("SELECT 1");
        assert!(result.is_ok());
        
        conn.cleanup().unwrap();
        assert!(!conn.is_healthy());
    }

    #[test] 
    fn test_file_resource() {
        let mut file = FileResource::new("/test/file.txt".to_string(), false);
        assert!(!file.is_healthy());
        
        file.initialize().unwrap();
        assert!(file.is_healthy());
        assert_eq!(file.file_path(), "/test/file.txt");
        
        let read_result = file.simulate_read();
        assert!(read_result.is_ok());
        
        let write_result = file.simulate_write("test data");
        assert!(write_result.is_ok());
        
        file.cleanup().unwrap();
        assert!(!file.is_healthy());
    }

    #[test]
    fn test_resource_pool() {
        let pool = ResourcePool::new(2, || {
            ConnectionResource::new("test://pool".to_string(), 1)
        });

        assert_eq!(pool.size(), 0);
        assert_eq!(pool.created_count(), 0);

        // Acquire first resource
        let handle1 = pool.acquire().unwrap();
        assert_eq!(pool.created_count(), 1);
        assert_eq!(pool.size(), 0);

        // Acquire second resource
        let _handle2 = pool.acquire().unwrap();
        assert_eq!(pool.created_count(), 2);
        assert_eq!(pool.size(), 0);

        // Pool should be exhausted
        let handle3_result = pool.acquire();
        assert!(handle3_result.is_err());

        // Drop first handle, should return to pool
        drop(handle1);
        assert_eq!(pool.size(), 1);

        // Should be able to acquire again
        let handle3 = pool.acquire().unwrap();
        assert!(handle3.is_healthy());
        assert_eq!(pool.size(), 0);
    }

    #[test]
    fn test_resource_handle() {
        let pool = ResourcePool::new(1, || {
            ConnectionResource::new("test://handle".to_string(), 42)
        });

        let handle = pool.acquire().unwrap();
        assert!(handle.is_healthy());
        assert_eq!(handle.resource_type().unwrap(), "DatabaseConnection");

        {
            let resource = handle.get().unwrap();
            assert_eq!(resource.connection_id(), 42);
            assert_eq!(resource.connection_string(), "test://handle");
        }

        // Resource should still be healthy
        assert!(handle.is_healthy());
    }

    #[tokio::test]
    async fn test_resource_managed_operation_success() {
        let operation = Box::new(ClosureOperation::new(|_ctx| {
            Box::pin(async move {
                Ok("Operation completed with resource".to_string())
            })
        }));

        let pool = Arc::new(ResourcePool::new(1, || {
            ConnectionResource::new("test://managed".to_string(), 100)
        }));

        let resource_op = ResourceManagedOperation::new(operation, pool.clone())
            .with_name("TestResourceOp".to_string());

        let mut context = OperationalContext::new();
        let result = resource_op.perform(&mut context).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Operation completed with resource");
        assert_eq!(pool.size(), 1); // Resource returned to pool
    }

    #[tokio::test]
    async fn test_resource_managed_operation_timeout() {
        let operation = Box::new(ClosureOperation::new(|_ctx| {
            Box::pin(async move { Ok("Should not reach here".to_string()) })
        }));

        // Create pool with no capacity
        let pool = Arc::new(ResourcePool::new(0, || {
            ConnectionResource::new("test://timeout".to_string(), 1)
        }));

        let resource_op = ResourceManagedOperation::new(operation, pool)
            .with_name("TimeoutTest".to_string())
            .with_acquire_timeout(Duration::from_millis(50));

        let mut context = OperationalContext::new();
        let result = resource_op.perform(&mut context).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            OperationError::ExecutionFailed(msg) => {
                assert!(msg.contains("exhausted"));
            },
            _ => panic!("Expected ExecutionFailed error"),
        }
    }

    #[test]
    fn test_connection_pool_convenience() {
        let operation = Box::new(ClosureOperation::new(|_ctx| {
            Box::pin(async move { Ok(42) })
        }));

        let connections = vec![
            "postgres://db1".to_string(),
            "postgres://db2".to_string(),
        ];

        let resource_op = with_connection_pool(operation, connections, 2)
            .with_name("ConnectionTest".to_string());

        // Should create resource-managed operation
        assert_eq!(resource_op.operation_name, "ConnectionTest");
    }

    #[test]
    fn test_file_pool_convenience() {
        let operation = Box::new(ClosureOperation::new(|_ctx| {
            Box::pin(async move { Ok("file content".to_string()) })
        }));

        let files = vec![
            "/tmp/file1.txt".to_string(),
            "/tmp/file2.txt".to_string(),
        ];

        let resource_op = with_file_pool(operation, files, true, 2)
            .with_name("FileTest".to_string());

        assert_eq!(resource_op.operation_name, "FileTest");
    }
}
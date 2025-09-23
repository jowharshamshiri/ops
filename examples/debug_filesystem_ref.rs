use std::sync::Arc;
use ops::{OpContext, BatchOp, Op, OpResult, ctx_put_ref, ctx_require_ref, OpError};
use async_trait::async_trait;

// Mock the FileSystemServiceImpl
#[derive(Clone)]
struct FileSystemServiceImpl {
    base_path: String,
}

impl FileSystemServiceImpl {
    fn new(base_path: String) -> Self {
        Self { base_path }
    }
}

struct ValidateFolderPathOp;
struct ScanDirectoryOp;

#[async_trait]
impl Op<()> for ValidateFolderPathOp {
    async fn perform(&self, context: &mut OpContext) -> OpResult<()> {
        println!("ValidateFolderPathOp: Checking for filesystem_service...");
        
        // Check if reference exists
        if context.contains_ref("filesystem_service") {
            println!("✓ filesystem_service reference found in context");
            
            let filesystem_service: Arc<FileSystemServiceImpl> = ctx_require_ref!(context, filesystem_service)?;
            println!("✓ Successfully retrieved filesystem_service: {}", filesystem_service.base_path);
        } else {
            println!("✗ filesystem_service reference NOT found in context");
            return Err(OpError::ExecutionFailed("filesystem_service not found".to_string()));
        }
        
        Ok(())
    }
}

#[async_trait]
impl Op<()> for ScanDirectoryOp {
    async fn perform(&self, context: &mut OpContext) -> OpResult<()> {
        println!("ScanDirectoryOp: Checking for filesystem_service...");
        
        // Check if reference exists
        if context.contains_ref("filesystem_service") {
            println!("✓ filesystem_service reference found in context");
            
            let filesystem_service: Arc<FileSystemServiceImpl> = ctx_require_ref!(context, filesystem_service)?;
            println!("✓ Successfully retrieved filesystem_service: {}", filesystem_service.base_path);
            
            // Simulate adding discovered_file_paths to context
            let discovered_file_paths = vec![
                "/path/to/file1.txt".to_string(),
                "/path/to/file2.txt".to_string(),
            ];
            context.put("discovered_file_paths", discovered_file_paths)?;
            println!("✓ Added discovered_file_paths to context");
        } else {
            println!("✗ filesystem_service reference NOT found in context");
            return Err(OpError::ExecutionFailed("filesystem_service not found".to_string()));
        }
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Debug Filesystem Reference Issue ===");
    
    let mut context = OpContext::new();
    
    // Simulate your exact code pattern
    let filesystem_service = Arc::new(FileSystemServiceImpl::new("/base/path".to_string()));
    
    println!("1. Storing filesystem_service in context...");
    // The issue is that ctx_put_ref! moves the value, but you already have an Arc
    // So we should use put_arc instead, or clone the Arc
    println!("   - Using context.put_arc() directly");
    context.put_arc("filesystem_service", filesystem_service.clone());
    println!("   - Put_arc call completed");
    
    // Verify it was stored
    println!("2. Verifying storage...");
    println!("   - Checking context.contains_ref(\"filesystem_service\")");
    if context.contains_ref("filesystem_service") {
        println!("✓ filesystem_service reference confirmed in context");
    } else {
        println!("✗ filesystem_service reference NOT found after storage!");
        
        return Ok(());
    }
    
    // Test direct access
    println!("3. Testing direct access...");
    let direct_access: Arc<FileSystemServiceImpl> = ctx_require_ref!(context, filesystem_service)?;
    println!("✓ Direct access successful: {}", direct_access.base_path);
    
    // Create discovery batch exactly like your code
    println!("4. Creating discovery batch...");
    let discovery_batch = BatchOp::new(vec![
        Arc::new(ValidateFolderPathOp) as Arc<dyn Op<()>>,
        Arc::new(ScanDirectoryOp) as Arc<dyn Op<()>>,
    ]);
    
    // Execute discovery batch
    println!("5. Executing discovery batch...");
    discovery_batch
        .perform(&mut context)
        .await
        .map_err(|e| OpError::ExecutionFailed(format!("Directory discovery failed: {}", e)))?;
    
    println!("✓ Discovery batch completed successfully!");
    
    // Test retrieving the discovered files
    println!("6. Retrieving discovered file paths...");
    let discovered_file_paths: Vec<String> = context.get("discovered_file_paths")
        .ok_or_else(|| OpError::ExecutionFailed("Failed to get discovered file paths".to_string()))?;
    
    println!("✓ Found {} discovered files:", discovered_file_paths.len());
    for path in &discovered_file_paths {
        println!("  - {}", path);
    }
    
    println!("\n=== Test Completed Successfully ===");
    Ok(())
}
use std::sync::Arc;
use ops::{WetContext, DryContext, BatchOp, Op, OpResult, OpMetadata, wet_put_arc, wet_require_ref, OpError};
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
    async fn perform(&self, dry: &mut DryContext, wet: &mut WetContext) -> OpResult<()> {
        println!("ValidateFolderPathOp: Checking for filesystem_service...");
        
        // Check if reference exists
        if wet.contains("filesystem_service") {
            println!("✓ filesystem_service reference found in wet context");
            
            let filesystem_service: Arc<FileSystemServiceImpl> = wet_require_ref!(wet, filesystem_service)?;
            println!("✓ Successfully retrieved filesystem_service: {}", filesystem_service.base_path);
        } else {
            println!("✗ filesystem_service reference NOT found in wet context");
            return Err(OpError::ExecutionFailed("filesystem_service not found".to_string()));
        }
        
        Ok(())
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("ValidateFolderPathOp")
            .description("Validates folder path using filesystem service")
            .build()
    }
}

#[async_trait]
impl Op<Vec<String>> for ScanDirectoryOp {
    async fn perform(&self, dry: &mut DryContext, wet: &mut WetContext) -> OpResult<Vec<String>> {
        println!("ScanDirectoryOp: Checking for filesystem_service...");
        
        // Check if reference exists
        if wet.contains("filesystem_service") {
            println!("✓ filesystem_service reference found in wet context");
            
            let filesystem_service: Arc<FileSystemServiceImpl> = wet_require_ref!(wet, filesystem_service)?;
            println!("✓ Successfully retrieved filesystem_service: {}", filesystem_service.base_path);
            
            // Simulate discovering files
            let discovered_file_paths = vec![
                "/path/to/file1.txt".to_string(),
                "/path/to/file2.txt".to_string(),
            ];
            println!("✓ Discovered {} files", discovered_file_paths.len());
            
            Ok(discovered_file_paths)
        } else {
            println!("✗ filesystem_service reference NOT found in wet context");
            Err(OpError::ExecutionFailed("filesystem_service not found".to_string()))
        }
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("ScanDirectoryOp")
            .description("Scans directory for files using filesystem service")
            .build()
    }
}

// Wrapper op that only returns unit but stores the discovered files in dry context
struct ScanDirectoryWrapperOp;

#[async_trait]
impl Op<()> for ScanDirectoryWrapperOp {
    async fn perform(&self, dry: &mut DryContext, wet: &mut WetContext) -> OpResult<()> {
        let scan_op = ScanDirectoryOp;
        let discovered_files = scan_op.perform(dry, wet).await?;
        
        // In a real implementation, you'd need a way to store this back to the dry context
        // For now, just print the results
        println!("✓ ScanDirectoryWrapper found {} files:", discovered_files.len());
        for file in &discovered_files {
            println!("  - {}", file);
        }
        
        Ok(())
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("ScanDirectoryWrapperOp")
            .description("Wrapper that scans directory and handles results")
            .build()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Debug Filesystem Reference Issue with Dry/Wet Contexts ===");
    
    let mut dry = DryContext::new();
    let mut wet = WetContext::new();
    
    // Simulate your exact code pattern
    let filesystem_service = Arc::new(FileSystemServiceImpl::new("/base/path".to_string()));
    
    println!("1. Storing filesystem_service in wet context...");
    println!("   - Using wet_put_arc! macro");
    wet_put_arc!(wet, filesystem_service);
    println!("   - wet_put_arc! call completed");
    
    // Verify it was stored
    println!("2. Verifying storage...");
    println!("   - Checking wet.contains(\"filesystem_service\")");
    if wet.contains("filesystem_service") {
        println!("✓ filesystem_service reference confirmed in wet context");
    } else {
        println!("✗ filesystem_service reference NOT found after storage!");
        return Ok(());
    }
    
    // Test direct access
    println!("3. Testing direct access...");
    let direct_access: Arc<FileSystemServiceImpl> = wet_require_ref!(wet, filesystem_service)?;
    println!("✓ Direct access successful: {}", direct_access.base_path);
    
    // Create discovery batch exactly like your code but with new op signature
    println!("4. Creating discovery batch...");
    let discovery_batch = BatchOp::new(vec![
        Arc::new(ValidateFolderPathOp) as Arc<dyn Op<()>>,
        Arc::new(ScanDirectoryWrapperOp) as Arc<dyn Op<()>>,
    ]);
    
    // Execute discovery batch
    println!("5. Executing discovery batch...");
    discovery_batch
        .perform(&mut dry, &mut wet)
        .await
        .map_err(|e| OpError::ExecutionFailed(format!("Directory discovery failed: {}", e)))?;
    
    println!("✓ Discovery batch completed successfully!");
    
    // Test individual op execution
    println!("6. Testing individual op execution...");
    let scan_op = ScanDirectoryOp;
    let discovered_files = scan_op.perform(&mut dry, &mut wet).await?;
    
    println!("✓ Found {} discovered files:", discovered_files.len());
    for path in &discovered_files {
        println!("  - {}", path);
    }
    
    // Show op metadata
    println!("\n7. Op Metadata:");
    println!("- {}: {}", 
             ValidateFolderPathOp.metadata().name, 
             ValidateFolderPathOp.metadata().description.unwrap_or("No description".to_string()));
    println!("- {}: {}", 
             ScanDirectoryOp.metadata().name, 
             ScanDirectoryOp.metadata().description.unwrap_or("No description".to_string()));
    
    println!("\n=== Test Completed Successfully ===");
    Ok(())
}
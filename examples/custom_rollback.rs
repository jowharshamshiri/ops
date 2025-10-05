use ops::prelude::*;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// Example demonstrating custom rollback logic for non-transactional operations
/// like search index updates that need custom cleanup when a batch fails.

// Simulated search index
type SearchIndex = Arc<Mutex<HashMap<String, String>>>;

struct UpdateSearchIndexOp {
    document_id: String,
    content: String,
    index: SearchIndex,
}

#[async_trait]
impl Op<()> for UpdateSearchIndexOp {
    async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        // Simulate updating search index
        self.index.lock().unwrap().insert(self.document_id.clone(), self.content.clone());
        println!("Updated search index for document: {}", self.document_id);
        Ok(())
    }
    
    async fn rollback(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        // Custom rollback: remove the document from search index
        self.index.lock().unwrap().remove(&self.document_id);
        println!("Rolled back search index for document: {}", self.document_id);
        Ok(())
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("UpdateSearchIndexOp")
            .description("Updates search index with document content")
            .build()
    }
}

struct DatabaseOp {
    document_id: String,
    should_fail: bool,
}

#[async_trait]
impl Op<()> for DatabaseOp {
    async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        if self.should_fail {
            return Err(OpError::ExecutionFailed("Database operation failed".to_string()));
        }
        
        println!("Database operation succeeded for document: {}", self.document_id);
        Ok(())
    }
    
    // Database operations use transaction rollback, so no custom rollback needed
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("DatabaseOp")
            .description("Performs database operation")
            .build()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Custom Rollback Example ===\n");
    
    // Create a shared search index
    let search_index: SearchIndex = Arc::new(Mutex::new(HashMap::new()));
    
    // Create operations that will succeed initially but fail later
    let ops = vec![
        Arc::new(UpdateSearchIndexOp {
            document_id: "doc1".to_string(),
            content: "First document content".to_string(),
            index: search_index.clone(),
        }) as Arc<dyn Op<()>>,
        Arc::new(UpdateSearchIndexOp {
            document_id: "doc2".to_string(),
            content: "Second document content".to_string(),
            index: search_index.clone(),
        }) as Arc<dyn Op<()>>,
        Arc::new(DatabaseOp {
            document_id: "doc1".to_string(),
            should_fail: true, // This will fail and trigger rollback
        }) as Arc<dyn Op<()>>,
    ];
    
    println!("Initial search index state:");
    println!("{:?}\n", *search_index.lock().unwrap());
    
    // Create and execute batch
    let batch = ops::BatchOp::new(ops);
    let mut dry = DryContext::new();
    let mut wet = WetContext::new();
    
    println!("Executing batch operations...\n");
    
    match batch.perform(&mut dry, &mut wet).await {
        Ok(_) => println!("Batch succeeded (unexpected)"),
        Err(e) => println!("Batch failed as expected: {}\n", e),
    }
    
    println!("Final search index state (should be empty due to rollback):");
    println!("{:?}", *search_index.lock().unwrap());
    
    println!("\n=== Summary ===");
    println!("This example demonstrates how:");
    println!("1. UpdateSearchIndexOp implements custom rollback logic");
    println!("2. When DatabaseOp fails, the batch automatically rolls back");
    println!("3. Search index entries are cleaned up via custom rollback");
    println!("4. Transaction-based ops (DatabaseOp) can rely on transaction rollback");
    
    Ok(())
}
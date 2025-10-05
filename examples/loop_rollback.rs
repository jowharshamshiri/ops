use ops::prelude::*;
use ops::LoopOp;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// Example demonstrating rollback functionality in loop operations
/// Each iteration of a loop behaves like a mini-batch with automatic rollback

// Simulated database and search index
type Database = Arc<Mutex<HashMap<String, String>>>;
type SearchIndex = Arc<Mutex<HashMap<String, String>>>;

struct DatabaseInsertOp {
    database: Database,
}

impl DatabaseInsertOp {
    fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl Op<()> for DatabaseInsertOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        let iteration: usize = dry.get("iteration_counter").unwrap_or(0);
        let key = format!("doc_{}", iteration);
        let value = format!("iteration_{}_value", iteration);
        
        self.database.lock().unwrap().insert(key.clone(), value.clone());
        println!("✓ Inserted into database: {} = {}", key, value);
        Ok(())
    }
    
    async fn rollback(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        let iteration: usize = dry.get("iteration_counter").unwrap_or(0);
        let key = format!("doc_{}", iteration);
        
        self.database.lock().unwrap().remove(&key);
        println!("↶ Rolled back database insert: {}", key);
        Ok(())
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("DatabaseInsertOp").build()
    }
}

struct SearchIndexUpdateOp {
    index: SearchIndex,
    should_fail_on_iteration: Option<usize>,
}

impl SearchIndexUpdateOp {
    fn new(index: SearchIndex, should_fail_on_iteration: Option<usize>) -> Self {
        Self { index, should_fail_on_iteration }
    }
}

#[async_trait]
impl Op<()> for SearchIndexUpdateOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        let iteration: usize = dry.get("iteration_counter").unwrap_or(0);
        
        if let Some(fail_iteration) = self.should_fail_on_iteration {
            if iteration == fail_iteration {
                println!("✗ Search index update failed on iteration {}", iteration);
                return Err(OpError::ExecutionFailed("Search index service unavailable".to_string()));
            }
        }
        
        let key = format!("doc_{}", iteration);
        let content = format!("searchable_content_{}", iteration);
        
        self.index.lock().unwrap().insert(key.clone(), content.clone());
        println!("✓ Updated search index: {} = {}", key, content);
        Ok(())
    }
    
    async fn rollback(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        let iteration: usize = dry.get("iteration_counter").unwrap_or(0);
        let key = format!("doc_{}", iteration);
        
        self.index.lock().unwrap().remove(&key);
        println!("↶ Rolled back search index update: {}", key);
        Ok(())
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("SearchIndexUpdateOp").build()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Loop Operation Rollback Example ===\n");
    
    // Create shared resources
    let database: Database = Arc::new(Mutex::new(HashMap::new()));
    let search_index: SearchIndex = Arc::new(Mutex::new(HashMap::new()));
    
    println!("Initial state:");
    println!("Database: {:?}", *database.lock().unwrap());
    println!("Search Index: {:?}\n", *search_index.lock().unwrap());
    
    // Create operations for each iteration
    // In each iteration: insert to database, then update search index
    let ops = vec![
        Arc::new(DatabaseInsertOp::new(database.clone())) as Arc<dyn Op<()>>,
        Arc::new(SearchIndexUpdateOp::new(
            search_index.clone(),
            Some(2), // Fail on iteration 2
        )) as Arc<dyn Op<()>>,
    ];
    
    // Create loop that will run 4 iterations
    let loop_op = LoopOp::new("iteration_counter".to_string(), 4, ops);
    let mut dry = DryContext::new();
    let mut wet = WetContext::new();
    
    println!("Executing loop with 4 iterations...\n");
    
    match loop_op.perform(&mut dry, &mut wet).await {
        Ok(_) => println!("Loop completed successfully (unexpected)"),
        Err(e) => println!("Loop failed as expected: {}\n", e),
    }
    
    println!("Final state:");
    println!("Database: {:?}", *database.lock().unwrap());
    println!("Search Index: {:?}\n", *search_index.lock().unwrap());
    
    println!("=== Summary ===");
    println!("This example demonstrates how:");
    println!("1. Each iteration of a loop op behaves like a mini-batch");
    println!("2. When an op fails within an iteration, only that iteration's succeeded ops are rolled back");
    println!("3. Previous successful iterations remain committed");
    println!("4. Database inserts from iterations 0 and 1 should remain");
    println!("5. Search index updates from iteration 2 should be rolled back");
    
    // Verify expected state
    let db_state = database.lock().unwrap();
    let index_state = search_index.lock().unwrap();
    
    println!("\nVerification:");
    println!("✓ Database should have entries from successful iterations: {}", !db_state.is_empty());
    println!("✓ Search index should be empty due to rollback: {}", index_state.is_empty());
    
    Ok(())
}
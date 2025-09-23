use ops::{OpContext, ctx_put_ref, ctx_get_ref, ctx_require_ref};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Reference Context Demo ===");
    
    let mut ctx = OpContext::new();
    
    // Example 1: Store a large data structure without serialization
    println!("\n1. Storing large data structure as reference...");
    let large_dataset = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]; // Imagine this is huge
    ctx_put_ref!(ctx, large_dataset, large_dataset.clone());
    
    // Retrieve the reference
    let dataset_ref: Option<Arc<Vec<i32>>> = ctx_get_ref!(ctx, large_dataset);
    if let Some(data) = dataset_ref {
        println!("Retrieved dataset: {:?}", &data[..5]); // Show first 5 elements
        println!("Dataset size: {}", data.len());
    }
    
    // Example 2: Share expensive-to-compute objects
    println!("\n2. Sharing expensive computed objects...");
    
    #[derive(Debug, Clone)]
    struct ComputationResult {
        matrix: Vec<Vec<f64>>,
        computation_time_ms: u64,
    }
    
    let expensive_result = ComputationResult {
        matrix: vec![vec![1.0, 2.0, 3.0]; 100], // 100x3 matrix
        computation_time_ms: 5000, // Pretend this took 5 seconds
    };
    
    // Store without needing Serialize
    ctx.put_ref("computation_result", expensive_result.clone());
    
    // Multiple operations can access the same reference
    for i in 0..3 {
        let result_ref: Option<Arc<ComputationResult>> = ctx.get_ref("computation_result");
        if let Some(result) = result_ref {
            println!("Operation {} accessed result - matrix size: {}x{}", 
                     i + 1, result.matrix.len(), result.matrix[0].len());
        }
    }
    
    // Example 3: Using the require macro for error handling
    println!("\n3. Using require macro for mandatory references...");
    
    let file_paths = vec![
        "/tmp/file1.txt".to_string(),
        "/tmp/file2.txt".to_string(),
        "/tmp/file3.txt".to_string(),
    ];
    ctx_put_ref!(ctx, file_paths);
    
    // This will succeed
    let file_paths_result: Result<Arc<Vec<String>>, _> = ctx_require_ref!(ctx, file_paths);
    match file_paths_result {
        Ok(paths) => {
            println!("Required file paths found: {} files", paths.len());
            for path in paths.iter() {
                println!("  - {}", path);
            }
        }
        Err(e) => println!("Error: {}", e),
    }
    
    // This will fail with proper error
    let missing_result: Result<Arc<Vec<String>>, _> = ctx_require_ref!(ctx, missing_paths);
    match missing_result {
        Ok(_) => println!("This shouldn't happen"),
        Err(e) => println!("Expected error: {}", e),
    }
    
    // Example 4: Type safety demonstration
    println!("\n4. Type safety with references...");
    
    ctx.put_ref("string_data", "Hello, World!".to_string());
    
    // Correct type retrieval
    let string_ref: Option<Arc<String>> = ctx.get_ref("string_data");
    if let Some(s) = string_ref {
        println!("String data: {}", s);
    }
    
    // Wrong type retrieval returns None
    let wrong_type: Option<Arc<i32>> = ctx.get_ref("string_data");
    println!("Wrong type retrieval result: {:?}", wrong_type);
    
    // Example 5: Memory sharing demonstration
    println!("\n5. Memory sharing demonstration...");
    
    let shared_data = Arc::new(vec![100, 200, 300, 400, 500]);
    let original_ptr = Arc::as_ptr(&shared_data);
    
    // Store the Arc directly
    ctx.put_arc("shared_ptr", shared_data.clone());
    
    // Retrieve and verify it's the same memory location
    let retrieved: Option<Arc<Vec<i32>>> = ctx.get_ref("shared_ptr");
    if let Some(retrieved_data) = retrieved {
        let retrieved_ptr = Arc::as_ptr(&retrieved_data);
        println!("Original ptr: {:p}", original_ptr);
        println!("Retrieved ptr: {:p}", retrieved_ptr);
        println!("Same memory location: {}", original_ptr == retrieved_ptr);
        println!("Reference count: {}", Arc::strong_count(&retrieved_data));
    }
    
    println!("\n=== Demo Complete ===");
    Ok(())
}
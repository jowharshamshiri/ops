use ops::{WetContext, wet_put_ref, wet_get_ref, wet_require_ref, wet_put_arc};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Reference Context Demo ===");
    
    let mut wet = WetContext::new();
    
    // Example 1: Store a large data structure without serialization
    println!("\n1. Storing large data structure as reference...");
    let large_dataset = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]; // Imagine this is huge
    wet_put_ref!(wet, large_dataset);
    
    // Retrieve the reference
    let dataset_ref: Option<Arc<Vec<i32>>> = wet_get_ref!(wet, large_dataset);
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
    wet.insert_ref("computation_result", expensive_result.clone());
    
    // Multiple operations can access the same reference
    for i in 0..3 {
        let result_ref: Option<Arc<ComputationResult>> = wet.get_ref("computation_result");
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
    wet_put_ref!(wet, file_paths);
    
    // This will succeed
    let file_paths_result: Result<Arc<Vec<String>>, _> = wet_require_ref!(wet, file_paths);
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
    let missing_result: Result<Arc<Vec<String>>, _> = wet_require_ref!(wet, missing_paths);
    match missing_result {
        Ok(_) => println!("This shouldn't happen"),
        Err(e) => println!("Expected error: {}", e),
    }
    
    // Example 4: Type safety demonstration
    println!("\n4. Type safety with references...");
    
    wet.insert_ref("string_data", "Hello, World!".to_string());
    
    // Correct type retrieval
    let string_ref: Option<Arc<String>> = wet.get_ref("string_data");
    if let Some(s) = string_ref {
        println!("String data: {}", s);
    }
    
    // Wrong type retrieval returns None
    let wrong_type: Option<Arc<i32>> = wet.get_ref("string_data");
    println!("Wrong type retrieval result: {:?}", wrong_type);
    
    // Example 5: Memory sharing demonstration
    println!("\n5. Memory sharing demonstration...");
    
    let shared_data = Arc::new(vec![100, 200, 300, 400, 500]);
    let original_ptr = Arc::as_ptr(&shared_data);
    
    // Store the Arc directly
    wet_put_arc!(wet, shared_data);
    
    // Retrieve and verify it's the same memory location
    let retrieved: Option<Arc<Vec<i32>>> = wet_get_ref!(wet, shared_data);
    if let Some(retrieved_data) = retrieved {
        let retrieved_ptr = Arc::as_ptr(&retrieved_data);
        println!("Original ptr: {:p}", original_ptr);
        println!("Retrieved ptr: {:p}", retrieved_ptr);
        println!("Same memory location: {}", original_ptr == retrieved_ptr);
        println!("Reference count: {}", Arc::strong_count(&retrieved_data));
    }
    
    // Example 6: Demonstrating wet context use in ops
    println!("\n6. Using wet context in an op...");
    
    use ops::prelude::*;
    
    struct DataProcessorOp;
    
    #[async_trait]
    impl Op<String> for DataProcessorOp {
        async fn perform(&self, _dry: &DryContext, wet: &WetContext) -> OpResult<String> {
            // Use wet_require_ref macro in an op
            let data: Arc<Vec<i32>> = wet_require_ref!(wet, large_dataset)?;
            let sum: i32 = data.iter().sum();
            Ok(format!("Sum of {} elements: {}", data.len(), sum))
        }
        
        fn metadata(&self) -> OpMetadata {
            OpMetadata::builder("DataProcessorOp")
                .description("Processes data from wet context")
                .build()
        }
    }
    
    // Execute the op using the wet context
    let dry = DryContext::new();
    let processor = DataProcessorOp;
    
    tokio::runtime::Runtime::new()?.block_on(async {
        match processor.perform(&dry, &wet).await {
            Ok(result) => println!("Op result: {}", result),
            Err(e) => println!("Op error: {}", e),
        }
    });
    
    println!("\n=== Demo Complete ===");
    Ok(())
}
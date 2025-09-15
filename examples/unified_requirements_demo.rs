use ops::{op, perform, execute_ops, OpError};
use std::sync::atomic::{AtomicU32, Ordering};

// Mock database and config functions for demonstration
fn connect_to_db() -> Result<String, OpError> {
    Ok("Database connection established".to_string())
}

fn load_config() -> Result<i32, OpError> {
    Ok(42) // Magic config value
}

static COUNTER: AtomicU32 = AtomicU32::new(0);
fn get_counter() -> Result<u32, OpError> {
    Ok(COUNTER.fetch_add(1, Ordering::SeqCst))
}

// Define some ops that can use both static values and requirements
op!(double_number(x: i32) -> i32 {
    Ok(x * 2)
});

op!(add_config(x: i32, config: i32) -> i32 {
    Ok(x + config)
});

op!(save_to_database(x: i32, database: String) -> String {
    Ok(format!("Saved {} to {}", x, database))
});

op!(increment_counter(base: i32, counter: u32) -> i32 {
    Ok(base + counter as i32)
});

#[tokio::main]
async fn main() -> Result<(), OpError> {
    tracing_subscriber::fmt::init();

    println!("=== Testing Unified Requirements Syntax ===\n");

    // Example 1: Mixed static values and factory functions
    println!("1. Mixed static values and factory functions:");
    let context = perform!(
        [("x", 10),                          // Static value
         ("database", || connect_to_db()),   // Factory function
         ("config", || load_config())],      // Factory function
        DoubleNumber,       // x=10 -> 20
        AddConfig,          // x=20, config=42 -> 62  
        SaveToDatabase      // data=62, database="Database..." -> formatted string
    ).await?;
    
    if let Some(result) = context.get_raw("result") {
        println!("Result: {}\n", result.as_str().unwrap_or("error"));
    }

    // Example 2: Only static values (should work the same)
    println!("2. Only static values:");
    let context = perform!(
        [("x", 5)],
        DoubleNumber        // x=5 -> 10
    ).await?;
    
    if let Some(result) = context.get_raw("result") {
        println!("Result: {}\n", result.as_i64().unwrap_or(0));
    }

    // Example 3: Only factory functions  
    println!("3. Only factory functions:");
    let context = perform!(
        [("base", || load_config()),      // Use config value as base
         ("counter", || get_counter())],
        IncrementCounter     // base=42, counter=0 -> 42
    ).await?;
    
    if let Some(result) = context.get_raw("result") {
        println!("Result: {}\n", result.as_i64().unwrap_or(0));
    }

    // Example 4: Factory functions are lazy - only called when needed
    println!("4. Factory functions are lazy (counter should increment):");
    let context = perform!(
        [("base", 100),
         ("counter", || get_counter())],     // Counter will be 1 now
        IncrementCounter
    ).await?;
    
    if let Some(result) = context.get_raw("result") {
        println!("Result: {}\n", result.as_i64().unwrap_or(0));
    }

    // Example 5: Using execute_ops with mixed syntax
    println!("5. Ultra-simple syntax with execute_ops:");
    let result = execute_ops!(
        [("x", 3),
         ("config", || load_config())],
        DoubleNumber,     // x=3 -> 6
        AddConfig         // x=6, config=42 -> 48
    ).await?;
    
    if let Some(value) = result {
        println!("Result: {}", value.as_i64().unwrap_or(0));
    }

    Ok(())
}
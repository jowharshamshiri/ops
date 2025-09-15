use ops::{op, OpContext, OpError, Op};
use serde_json::json;

// New ergonomic ops that use OpContext for input/output
op!(hello_operation(name: String) -> String {
    Ok(format!("Hello, {}!", name))
});

op!(math_operation(x: i32) -> i32 {
    Ok(x * 2 + 1)
});

op!(complex_operation(data: Vec<String>) -> String {
    if data.is_empty() {
        Err(OpError::ExecutionFailed("Empty input".to_string()))
    } else {
        Ok(data.join(", "))
    }
});

// Multi-parameter operation
op!(add_operation(a: i32, b: i32) -> i32 {
    Ok(a + b)
});

#[tokio::main]
async fn main() -> Result<(), OpError> {
    env_logger::init();
    let mut context = OpContext::new();

    println!("=== Testing op! macro with OpContext ===");
    
    // Test hello_operation
    context.set("name".to_string(), json!("World"));
    let hello_op = HelloOperation::new();
    hello_op.perform(&mut context).await?;
    
    if let Some(result) = context.get_raw("result") {
        println!("HelloOperation result: {}", result.as_str().unwrap_or("error"));
    }
    
    // Test math_operation
    context.set("x".to_string(), json!(5));
    let math_op = MathOperation::new();
    math_op.perform(&mut context).await?;
    
    if let Some(result) = context.get_raw("result") {
        println!("MathOperation result: {}", result.as_i64().unwrap_or(0));
    }
    
    // Test complex_operation with data
    context.set("data".to_string(), json!(["a", "b", "c"]));
    let complex_op = ComplexOperation::new();
    complex_op.perform(&mut context).await?;
    
    if let Some(result) = context.get_raw("result") {
        println!("ComplexOperation result: {}", result.as_str().unwrap_or("error"));
    }
    
    // Test complex_operation with empty data
    context.set("data".to_string(), json!([]));
    match complex_op.perform(&mut context).await {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("Expected error: {}", e),
    }
    
    // Test multi-parameter operation
    context.set("a".to_string(), json!(10));
    context.set("b".to_string(), json!(20));
    let add_op = AddOperation::new();
    add_op.perform(&mut context).await?;
    
    if let Some(result) = context.get_raw("result") {
        println!("AddOperation result: {}", result.as_i64().unwrap_or(0));
    }
    
    // Test error case - missing input
    let mut empty_context = OpContext::new();
    match hello_op.perform(&mut empty_context).await {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("Expected missing input error: {}", e),
    }
    
    // Test error case - wrong type
    context.set("name".to_string(), json!(42)); // Should be string, not number
    match hello_op.perform(&mut context).await {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("Expected type error: {}", e),
    }

    Ok(())
}
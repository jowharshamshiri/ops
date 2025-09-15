use ops::{op, run_ops, execute_ops, OpError};
use serde_json::json;

// Define some ops that can work in a daisy chain
op!(double_number(x: i32) -> i32 {
    Ok(x * 2)
});

op!(add_ten(x: i32) -> i32 {
    Ok(x + 10)
});

op!(format_result(x: i32) -> String {
    Ok(format!("Final result: {}", x))
});

op!(greet_person(name: String) -> String {
    Ok(format!("Hello, {}!", name))
});

op!(append_suffix(text: String) -> String {
    Ok(format!("{} Have a great day!", text))
});

// Op that can reference a specific previous op by name
op!(use_double_result(doubled_value: i32) -> String {
    Ok(format!("The doubled value was: {}", doubled_value))
});

#[tokio::main]
async fn main() -> Result<(), OpError> {
    env_logger::init();

    println!("=== Testing Daisy Chain Ops ===\n");

    // Example 1: Simple number processing chain using "result" daisy chaining
    println!("1. Number processing chain: 5 -> double -> add 10 -> format");
    let context = run_ops!(
        [("x", json!(5))],
        DoubleNumber,    // Takes x=5, outputs 10, stores as "DoubleNumber" and "result"
        AddTen,          // Takes x from "result"=10, outputs 20, stores as "AddTen" and "result"
        FormatResult     // Takes x from "result"=20, outputs "Final result: 20"
    ).await?;
    
    if let Some(result) = context.get_raw("result") {
        println!("Final result: {}\n", result.as_str().unwrap_or("error"));
    }

    // Example 2: Text processing chain using "result" daisy chaining
    println!("2. Text processing chain: World -> greet -> append suffix");
    let context = run_ops!(
        [("name", json!("World"))],
        GreetPerson,     // Takes name="World", outputs "Hello, World!", stores as "GreetPerson" and "result"
        AppendSuffix     // Takes text from "result", outputs "Hello, World! Have a great day!"
    ).await?;
    
    if let Some(result) = context.get_raw("result") {
        println!("Final result: {}\n", result.as_str().unwrap_or("error"));
    }

    // Example 3: Multiple operations with named results
    println!("3. Multiple operations storing named results:");
    let context = run_ops!(
        [("x", json!(7))],
        DoubleNumber,        // Stores result as "DoubleNumber" (14)
        AddTen,              // Uses "result" (14), stores as "AddTen" (24)  
        FormatResult         // Uses "result" (24), stores formatted result
    ).await?;
    
    if let Some(result) = context.get_raw("result") {
        println!("Result: {}\n", result.as_str().unwrap_or("error"));
    }

    // Example 4: Show all stored values
    println!("4. All stored values in context:");
    let context = run_ops!(
        [("x", json!(3))],
        DoubleNumber,
        AddTen
    ).await?;
    
    println!("DoubleNumber result: {}", 
        context.get_raw("DoubleNumber").unwrap().as_i64().unwrap_or(0));
    println!("AddTen result: {}", 
        context.get_raw("AddTen").unwrap().as_i64().unwrap_or(0));
    println!("Latest result: {}\n", 
        context.get_raw("result").unwrap().as_i64().unwrap_or(0));

    // Example 5: Error handling - missing input
    println!("5. Error handling - missing input:");
    match run_ops!(
        [("wrong_key", json!(10))],  // x is missing, result is missing
        DoubleNumber
    ).await {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("Expected error: {}\n", e),
    }

    // Example 6: Using execute_ops for simple one-liner
    println!("6. Ultra-simple syntax with execute_ops:");
    let result = execute_ops!(
        [("x", json!(4))],
        DoubleNumber,
        AddTen
    ).await?;
    
    if let Some(value) = result {
        println!("Result: {}", value.as_i64().unwrap_or(0));
    }

    Ok(())
}
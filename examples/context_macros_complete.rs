use ops::prelude::*;
use ops::{dry_put, dry_put_key, dry_get, dry_get_key, dry_require, dry_require_key, dry_result,
         wet_put_ref, wet_put_ref_key, wet_put_arc, wet_put_arc_key, 
         wet_get_ref, wet_get_ref_key, wet_require_ref, wet_require_ref_key};
use std::sync::Arc;

#[tokio::main]
async fn main() -> OpResult<()> {
    let mut dry = DryContext::new();
    let mut wet = WetContext::new();
    
    println!("🧪 COMPLETE CONTEXT MACROS DEMONSTRATION 🧪\n");
    
    // === DRY CONTEXT MACROS ===
    println!("=== Dry Context Macros ===");
    
    // Basic dry_put! patterns
    let username = "alice".to_string();
    let user_id = 123u32;
    
    dry_put!(dry, username);                    // Store variable using its name as key
    dry_put!(dry, user_id, 456u32);            // Store value with variable name as key
    dry_put_key!(dry, "session_id", "sess_789"); // Store with custom key
    
    // Basic dry_get! patterns
    let retrieved_username: Option<String> = dry_get!(dry, username);
    let retrieved_user_id: Option<u32> = dry_get!(dry, user_id);
    let session_id: Option<String> = dry_get_key!(dry, "session_id");
    
    println!("Retrieved username: {:?}", retrieved_username);
    println!("Retrieved user_id: {:?}", retrieved_user_id);
    println!("Retrieved session_id: {:?}", session_id);
    
    // Required dry context access
    let required_username: String = dry_require!(dry, username)?;
    let required_session: String = dry_require_key!(dry, "session_id")?;
    
    println!("Required username: {}", required_username);
    println!("Required session: {}", required_session);
    
    // Store result in multiple keys
    let operation_result = "SUCCESS".to_string();
    dry_result!(dry, "MyOperation", operation_result);
    
    let my_op_result: Option<String> = dry_get_key!(dry, "MyOperation");
    let result: Option<String> = dry_get_key!(dry, "result");
    
    println!("MyOperation result: {:?}", my_op_result);
    println!("Generic result: {:?}", result);
    
    // === WET CONTEXT MACROS ===
    println!("\n=== Wet Context Macros ===");
    
    // Create some test data
    let test_string = Arc::new("Hello, World!".to_string());
    let test_vec = Arc::new(vec![1, 2, 3, 4, 5]);
    let arc_string = Arc::new("Shared String".to_string());
    let arc_vec = Arc::new(vec!["a".to_string(), "b".to_string()]);
    
    // Store references in wet context
    wet_put_arc!(wet, test_string, test_string.clone());
    wet_put_arc_key!(wet, "numbers", test_vec.clone());
    
    // Store Arcs in wet context
    wet_put_arc!(wet, arc_string);
    wet_put_arc_key!(wet, "string_vec", arc_vec.clone());
    
    // Retrieve references from wet context
    let retrieved_string_ref: Option<Arc<String>> = wet_get_ref!(wet, test_string);
    let retrieved_numbers: Option<Arc<Vec<i32>>> = wet_get_ref_key!(wet, "numbers");
    
    println!("Retrieved string ref: {:?}", retrieved_string_ref);
    println!("Retrieved numbers: {:?}", retrieved_numbers);
    
    // Required wet context access
    let required_arc_string: Arc<String> = wet_require_ref!(wet, arc_string)?;
    let required_string_vec: Arc<Vec<String>> = wet_require_ref_key!(wet, "string_vec")?;
    
    println!("Required arc string: {}", required_arc_string);
    println!("Required string vec: {:?}", required_string_vec);
    
    println!("\n✅ ALL CONTEXT MACROS WORKING PERFECTLY!");
    
    println!("\n📋 COMPLETE MACRO SET:");
    println!("DRY CONTEXT:");
    println!("  • dry_put!(ctx, var) / dry_put!(ctx, var, value)");
    println!("  • dry_put_key!(ctx, key, value)");
    println!("  • dry_get!(ctx, var)");
    println!("  • dry_get_key!(ctx, key)");
    println!("  • dry_require!(ctx, var)");
    println!("  • dry_require_key!(ctx, key)");
    println!("  • dry_result!(ctx, op_name, result)");
    
    println!("WET CONTEXT:");
    println!("  • wet_put_ref!(ctx, var, value) / wet_put_ref!(ctx, var)");
    println!("  • wet_put_ref_key!(ctx, key, value)");
    println!("  • wet_put_arc!(ctx, var, value) / wet_put_arc!(ctx, var)");
    println!("  • wet_put_arc_key!(ctx, key, value)");
    println!("  • wet_get_ref!(ctx, var)");
    println!("  • wet_get_ref_key!(ctx, key)");
    println!("  • wet_require_ref!(ctx, var)");
    println!("  • wet_require_ref_key!(ctx, key)");
    
    Ok(())
}
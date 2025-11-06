use ops::prelude::*;
use ops::{repeat_until, break_loop, continue_loop, abort};

// Example operation that can break, continue, or abort
#[derive(Debug)]
struct ControlFlowOp {
    id: i32,
    action: ControlAction,
}

#[derive(Debug)]
enum ControlAction {
    Continue,  // Process normally
    Skip,      // Skip to next iteration (continue_loop!)
    Break,     // Exit the loop entirely (break_loop!)
    Abort,     // Abort the entire operation (abort!)
}

impl ControlFlowOp {
    fn new(id: i32, action: ControlAction) -> Self {
        Self { id, action }
    }
}

#[async_trait]
impl Op<String> for ControlFlowOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<String> {
        let iteration = dry.get::<usize>("iteration").unwrap_or(0);
        println!("Op {} executing in iteration {}", self.id, iteration);
        
        match self.action {
            ControlAction::Continue => {
                println!("  → Processing normally");
                Ok(format!("Op{}-Iter{}", self.id, iteration))
            }
            ControlAction::Skip => {
                println!("  → Skipping rest of this iteration (continue_loop!)");
                continue_loop!(dry);
            }
            ControlAction::Break => {
                println!("  → Breaking out of entire loop (break_loop!)");
                break_loop!(dry);
            }
            ControlAction::Abort => {
                println!("  → Aborting entire operation (abort!)");
                abort!(dry, "Operation aborted by ControlFlowOp");
            }
        }
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder(&format!("ControlFlowOp-{}", self.id)).build()
    }
}

// Example condition-checking operation
#[derive(Debug)]
struct ConditionCheckOp {
    max_iterations: usize,
}

impl ConditionCheckOp {
    fn new(max_iterations: usize) -> Self {
        Self { max_iterations }
    }
}

#[async_trait]
impl Op<String> for ConditionCheckOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<String> {
        let iteration = dry.get::<usize>("iteration").unwrap_or(0);
        let should_continue = iteration < self.max_iterations;
        
        println!("ConditionCheck: iteration {} < {}: {}", 
                iteration, self.max_iterations, should_continue);
        
        // Update the condition for the next iteration
        dry.insert("should_continue", should_continue);
        
        Ok(format!("ConditionCheck-Iter{}-Continue{}", iteration, should_continue))
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("ConditionCheckOp").build()
    }
}

// Example 1: Normal loop with continue
repeat_until! {
    NormalLoopWithContinue<String> = {
        counter: "iteration",
        condition: "should_continue",
        max_iterations: 5,
        ops: [
            ControlFlowOp::new(1, ControlAction::Continue),
            ControlFlowOp::new(2, ControlAction::Skip),      // This will skip iteration
            ControlFlowOp::new(3, ControlAction::Continue),  // This won't execute due to skip
            ConditionCheckOp::new(3)
        ]
    }
}

// Example 2: Loop with break
repeat_until! {
    LoopWithBreak<String> = {
        counter: "iteration", 
        condition: "should_continue",
        max_iterations: 10,
        ops: [
            ControlFlowOp::new(1, ControlAction::Continue),
            ControlFlowOp::new(2, ControlAction::Break),     // This will break the entire loop
            ControlFlowOp::new(3, ControlAction::Continue),  // This won't execute due to break
            ConditionCheckOp::new(10)
        ]
    }
}

// Example 3: Loop with abort
repeat_until! {
    LoopWithAbort<String> = {
        counter: "iteration",
        condition: "should_continue",
        max_iterations: 10,
        ops: [
            ControlFlowOp::new(1, ControlAction::Continue),
            ControlFlowOp::new(2, ControlAction::Abort),     // This will abort everything
            ControlFlowOp::new(3, ControlAction::Continue),  // This won't execute due to abort
            ConditionCheckOp::new(10)
        ]
    }
}

// Example 4: Complex control flow demonstration
repeat_until! {
    ComplexControlFlow<String> -> last = {
        counter: "iteration",
        condition: "should_continue",
        max_iterations: 20,
        ops: [
            ControlFlowOp::new(1, ControlAction::Continue),
            ConditionCheckOp::new(5)  // This controls when to stop
        ]
    }
}

#[tokio::main]
async fn main() -> OpResult<()> {
    let mut dry = DryContext::new();
    let mut wet = WetContext::new();
    
    println!("🔄 LOOP CONTROL FLOW DEMONSTRATIONS 🔄\n");
    
    // Example 1: continue_loop demonstration
    println!("=== Example 1: continue_loop! behavior ===");
    dry.insert("should_continue", true);
    let normal_loop = NormalLoopWithContinue::new();
    match normal_loop.perform(&mut dry, &mut wet).await {
        Ok(results) => {
            println!("✅ Normal loop completed with {} results:", results.len());
            for (i, result) in results.iter().enumerate() {
                println!("  Result {}: {}", i, result);
            }
        }
        Err(e) => println!("❌ Error: {}", e),
    }
    
    println!("\n=== Example 2: break_loop! behavior ===");
    dry.clear_control_flags();
    dry.insert("should_continue", true);
    let break_loop = LoopWithBreak::new();
    match break_loop.perform(&mut dry, &mut wet).await {
        Ok(results) => {
            println!("✅ Loop with break completed with {} results:", results.len());
            for (i, result) in results.iter().enumerate() {
                println!("  Result {}: {}", i, result);
            }
        }
        Err(e) => println!("❌ Error: {}", e),
    }
    
    println!("\n=== Example 3: abort! behavior ===");
    dry.clear_control_flags();
    dry.insert("should_continue", true);
    let abort_loop = LoopWithAbort::new();
    match abort_loop.perform(&mut dry, &mut wet).await {
        Ok(results) => {
            println!("✅ Loop with abort completed with {} results:", results.len());
            for (i, result) in results.iter().enumerate() {
                println!("  Result {}: {}", i, result);
            }
        }
        Err(e) => println!("❌ Expected abort error: {}", e),
    }
    
    println!("\n=== Example 4: Condition-based loop ===");
    dry.clear_control_flags();
    dry.insert("should_continue", true);
    let complex_loop = ComplexControlFlow::new();
    match complex_loop.perform(&mut dry, &mut wet).await {
        Ok(result) => {
            println!("✅ Complex loop completed with result: {}", result);
        }
        Err(e) => println!("❌ Error: {}", e),
    }
    
    println!("\n🎯 SUMMARY:");
    println!("• continue_loop!: Skips rest of current iteration, moves to next");
    println!("• break_loop!: Exits the entire loop immediately");
    println!("• abort!: Stops everything and returns an error");
    println!("• condition variable: Controls whether loop continues");
    
    Ok(())
}
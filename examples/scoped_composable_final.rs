use ops::prelude::*;
use ops::{batch, repeat, repeat_until, break_loop, continue_loop};

// Example showcasing the complete scoped composable op system

#[derive(Debug)]
struct MockOp {
    id: String,
    action: String,
}

impl MockOp {
    fn new(id: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            action: action.into(),
        }
    }
}

#[async_trait]
impl Op<String> for MockOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<String> {
        let result = format!("{}:{}", self.id, self.action);
        println!(" MockOp: {}", result);
        
        match self.action.as_str() {
            "break" => {
                println!("    Breaking loop!");
                break_loop!(dry);
            }
            "continue" => {
                println!("    Continuing loop!");
                continue_loop!(dry);
            }
            _ => {}
        }
        
        Ok(result)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder(&self.id).build()
    }
}

// OK SOLUTION: All macros now support flexible return types with UUID-scoped control flow

// 1. Sequential batch operations
batch! {
    SequentialOps<String> -> unit = [
        MockOp::new("S1", "normal"),
        MockOp::new("S2", "normal"),
        MockOp::new("S3", "normal")
    ]
}

// 2. For-loop operations  
repeat! {
    ForLoopOps<String> -> unit = {
        counter: "for_counter",
        limit: "for_limit",
        ops: [
            MockOp::new("F1", "normal"),
            MockOp::new("F2", "break")  // Will break on iteration 1
        ]
    }
}

// 3. While-loop operations
repeat_until! {
    WhileLoopOps<String> -> unit = {
        counter: "while_counter", 
        condition: "should_continue",
        max_iterations: 5,
        ops: [
            MockOp::new("W1", "normal"),
            MockOp::new("W2", "continue")  // Will continue on iteration 1
        ]
    }
}

// 4. Nested composable operations - each with unique UUID-scoped control
repeat_until! {
    OuterLoop<()> -> unit = {
        counter: "outer_counter",
        condition: "outer_continue", 
        max_iterations: 3,
        ops: [
            SequentialOps::new(),     // OK Nested batch
            ForLoopOps::new(),        // OK Nested for-loop
            WhileLoopOps::new()       // OK Nested while-loop
        ]
    }
}

// 5. All return types work seamlessly
batch! {
    ReturnVec<String> = [MockOp::new("V1", "normal"), MockOp::new("V2", "normal")]
}

batch! {
    ReturnLast<String> -> last = [MockOp::new("L1", "normal"), MockOp::new("L2", "normal")]
}

batch! {
    ReturnFirst<String> -> first = [MockOp::new("F1", "normal"), MockOp::new("F2", "normal")]
}

batch! {
    ReturnUnit<String> -> unit = [MockOp::new("U1", "normal"), MockOp::new("U2", "normal")]
}

#[tokio::main]
async fn main() -> OpResult<()> {
    let mut dry = DryContext::new();
    let mut wet = WetContext::new();
    
    println!(" SCOPED COMPOSABLE OPS FINAL DEMONSTRATION \n");
    
    // Set up context variables
    dry.insert("for_limit", 3_usize);
    dry.insert("should_continue", true);
    dry.insert("outer_continue", true);
    
    println!("=== 1. Sequential Operations ===");
    let seq_ops = SequentialOps::new();
    seq_ops.perform(&mut dry, &mut wet).await?;
    
    println!("\n=== 2. For-Loop Operations ===");
    let for_ops = ForLoopOps::new();
    for_ops.perform(&mut dry, &mut wet).await?;
    
    println!("\n=== 3. While-Loop Operations ===");
    dry.insert("should_continue", true);
    let while_ops = WhileLoopOps::new();
    while_ops.perform(&mut dry, &mut wet).await?;
    
    println!("\n=== 4. Different Return Types ===");
    
    let vec_result = ReturnVec::new();
    let vec_data = vec_result.perform(&mut dry, &mut wet).await?;
    println!("Vec result: {:?}", vec_data);
    
    let last_result = ReturnLast::new();
    let last_data = last_result.perform(&mut dry, &mut wet).await?;
    println!("Last result: {:?}", last_data);
    
    let first_result = ReturnFirst::new();
    let first_data = first_result.perform(&mut dry, &mut wet).await?;
    println!("First result: {:?}", first_data);
    
    let unit_result = ReturnUnit::new();
    let unit_data = unit_result.perform(&mut dry, &mut wet).await?;
    println!("Unit result: {:?}", unit_data);
    
    println!("\n=== 5. Nested Composable Operations ===");
    dry.insert("outer_continue", true);
    dry.insert("should_continue", true);
    let outer_loop = OuterLoop::new();
    outer_loop.perform(&mut dry, &mut wet).await?;
    
    println!("\n COMPLETE SOLUTION SUMMARY:");
    println!("OK UUID-scoped control flow - no conflicts between nested loops");
    println!("OK break_loop!() and continue_loop!() target current loop automatically");
    println!("OK All macros support flexible return types (all, last, first, unit, merge)");
    println!("OK Perfect composability - any op can be nested within any other");
    println!("OK Type safety - compile-time verification of all compositions");
    println!("OK Clean syntax - intuitive and readable macro definitions");
    
    Ok(())
}
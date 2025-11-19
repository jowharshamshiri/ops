use ops::prelude::*;
use ops::{batch, repeat, repeat_until};

// Mock ContentSelectionOp-like operations
#[derive(Debug)]
struct MockOp(String);

impl MockOp {
    fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

#[async_trait]
impl Op<()> for MockOp {
    async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        println!("Executing: {}", self.0);
        Ok(())
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder(&self.0).build()
    }
}

/// SOLUTION: Enhanced Composable Macros
/// 
/// Problem: repeat! macro created Op<Vec<T>> but batch! expected Op<T>
/// Solution: All macros now support flexible return types with aggregation strategies

// OPTION 1: Use batch! with unit aggregation (recommended for sequential execution)
batch! {
    ContentSelectionOp<()> -> unit = [
        MockOp::new("LoadContentOp"),
        MockOp::new("InsertDataOp"),
        MockOp::new("BitChoiceOp"),
        MockOp::new("ReactToContentSelectionResponse")
    ]
}

// OPTION 2: Use repeat! with unit aggregation (for looping behavior)
repeat! {
    ContentSelectionLoopOp<()> -> unit = {
        counter: "cso",
        limit: "cso_limit",
        ops: [
            MockOp::new("LoadContentOp"),
            MockOp::new("InsertDataOp"),
            MockOp::new("BitChoiceOp"),
            MockOp::new("ReactToContentSelectionResponse")
        ]
    }
}

// OPTION 3: Use repeat_until! for condition-based looping
repeat_until! {
    ContentSelectionWhileOp<()> -> unit = {
        counter: "cso",
        condition: "should_continue",
        max_iterations: 10,
        ops: [
            MockOp::new("LoadContentOp"),
            MockOp::new("InsertDataOp"),
            MockOp::new("BitChoiceOp"),
            MockOp::new("ReactToContentSelectionResponse")
        ]
    }
}

// The original batch that was failing - now works!
batch! {
    SkimPassOpBatch<()> -> unit = [
        MockOp::new("StartTransactionOp"),
        ContentSelectionOp::new(),           // ✅ Now implements Op<()>
        ContentSelectionLoopOp::new(),       // ✅ Also implements Op<()>
        ContentSelectionWhileOp::new()       // ✅ Also implements Op<()>
    ]
}

/// All aggregation strategies available:
/// - `-> all` (default): Returns Vec<T>
/// - `-> last`: Returns T (last result)
/// - `-> first`: Returns T (first result)  
/// - `-> unit`: Returns () (ignores results)
/// - `-> merge`: Returns T (merged result, requires Default + Clone)

// Example: Different strategies with same operations
batch! {
    ProcessingPipelineAll<()> = [MockOp::new("step1"), MockOp::new("step2")]
}

batch! {
    ProcessingPipelineLast<()> -> last = [MockOp::new("step1"), MockOp::new("step2")]
}

batch! {
    ProcessingPipelineUnit<()> -> unit = [MockOp::new("step1"), MockOp::new("step2")]
}

#[tokio::main]
async fn main() -> OpResult<()> {
    let mut dry = DryContext::new();
    let mut wet = WetContext::new();
    
    // Set up context
    dry.insert("cso_limit", 2_usize);
    dry.insert("should_continue", true);
    
    println!("🎉 SOLUTION DEMONSTRATION 🎉\n");
    
    println!("1. Sequential Content Selection:");
    let op1 = ContentSelectionOp::new();
    op1.perform(&mut dry, &mut wet).await?;
    
    println!("\n2. Loop Content Selection:");
    let op2 = ContentSelectionLoopOp::new();
    op2.perform(&mut dry, &mut wet).await?;
    
    println!("\n3. While Loop Content Selection:");
    dry.insert("should_continue", true);
    let op3 = ContentSelectionWhileOp::new();
    op3.perform(&mut dry, &mut wet).await?;
    
    println!("\n4. The Original Failing Batch - Now Working!");
    let extraction_batch = SkimPassOpBatch::new();
    extraction_batch.perform(&mut dry, &mut wet).await?;
    
    println!("\n✅ All ops are now fully composable and interchangeable!");
    println!("✅ No more 'trait bound not satisfied' errors!");
    println!("✅ Clean, type-safe, and powerful macro system!");
    
    Ok(())
}
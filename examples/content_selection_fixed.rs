use ops::prelude::*;
use ops::{batch, repeat, repeat_until};

// Mock ops to simulate the user's scenario
#[derive(Debug)]
struct StartTransactionOp;

#[async_trait]
impl Op<()> for StartTransactionOp {
    async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        println!("Transaction started");
        Ok(())
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("StartTransactionOp").build()
    }
}

#[derive(Debug)]
struct LoadContentOp {
    prefix: String,
}

impl LoadContentOp {
    fn new(prefix: String) -> Self {
        Self { prefix }
    }
}

#[async_trait]
impl Op<()> for LoadContentOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        let iteration = dry.get::<usize>(&self.prefix).unwrap_or(0);
        println!("Loading content for iteration {}", iteration);
        Ok(())
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("LoadContentOp").build()
    }
}

#[derive(Debug)]
struct InsertDataOp {
    config: String,
}

impl InsertDataOp {
    fn with(config: impl Into<String>) -> Self {
        Self { config: config.into() }
    }
}

#[async_trait]
impl Op<()> for InsertDataOp {
    async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        println!("Inserting data with config: {}", self.config);
        Ok(())
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("InsertDataOp").build()
    }
}

#[derive(Debug)]
struct BitChoiceOp;

#[async_trait]
impl Op<()> for BitChoiceOp {
    async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        println!("Making bit choice");
        Ok(())
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("BitChoiceOp").build()
    }
}

#[derive(Debug)]
struct ReactToContentSelectionResponse;

#[async_trait]
impl Op<()> for ReactToContentSelectionResponse {
    async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        println!("Reacting to content selection response");
        Ok(())
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("ReactToContentSelectionResponse").build()
    }
}

// SOLUTION 1: Using batch macro with unit aggregation
// This is the cleanest solution if you want sequential execution
batch! {
    ContentSelectionOp<()> -> unit = [
        LoadContentOp::new("cso".to_string()),
        InsertDataOp::with("default_config"),
        BitChoiceOp,
        ReactToContentSelectionResponse
    ]
}

// SOLUTION 2: Using repeat macro with unit aggregation  
// This keeps the looping behavior but returns ()
repeat! {
    ContentSelectionLoopOp<()> -> unit = {
        counter: "cso",
        limit: "cso_limit",
        ops: [
            LoadContentOp::new("cso".to_string()),
            InsertDataOp::with("default_config"),
            BitChoiceOp,
            ReactToContentSelectionResponse
        ]
    }
}

// SOLUTION 3: Using while loop for condition-based content selection
repeat_until! {
    ContentSelectionWhileOp<()> -> unit = {
        counter: "cso",
        condition: "has_more_content",
        max_iterations: 100,
        ops: [
            LoadContentOp::new("cso".to_string()),
            InsertDataOp::with("default_config"),
            BitChoiceOp,
            ReactToContentSelectionResponse
        ]
    }
}

// Now all three ops implement Op<()> and can be used interchangeably!

// The batch operation that had the original error - now works!
batch! {
    CloseReadOpBatch<()> -> unit = [
        StartTransactionOp,
        ContentSelectionOp::new(),           // OK Now implements Op<()>
        ContentSelectionLoopOp::new(),       // OK Also implements Op<()>  
        ContentSelectionWhileOp::new()       // OK Also implements Op<()>
    ]
}

// Example showing different aggregation strategies with the same ops
batch! {
    FlexiblePipeline<()> = [
        StartTransactionOp,
        ContentSelectionOp::new()
    ]
}

batch! {
    FlexiblePipelineUnit<()> -> unit = [
        StartTransactionOp,
        ContentSelectionOp::new()
    ]
}

// Example of using a repeat op that returns the last result instead of Vec
repeat! {
    ContentSelectionLastResult<()> -> last = {
        counter: "cso",
        limit: "cso_limit", 
        ops: [
            LoadContentOp::new("cso".to_string()),
            BitChoiceOp
        ]
    }
}

// Example using op_wrapper for manual type conversion (rarely needed)
// op_wrapper! {
//     unit ContentSelectionUnitWrapper for Vec<()>
// }

// All these ops are now fully composable and interchangeable
batch! {
    FullyComposablePipeline<()> -> unit = [
        StartTransactionOp,
        ContentSelectionOp::new(),
        ContentSelectionLoopOp::new(),
        ContentSelectionWhileOp::new(),
        ContentSelectionLastResult::new()
    ]
}

#[tokio::main]
async fn main() -> OpResult<()> {
    let mut dry = DryContext::new();
    let mut wet = WetContext::new();
    
    // Set up context variables
    dry.insert("cso_limit", 3_usize);
    dry.insert("has_more_content", true);
    
    println!("=== Content Selection with Sequential Execution ===");
    let content_op = ContentSelectionOp::new();
    let result = content_op.perform(&mut dry, &mut wet).await?;
    println!("Sequential content selection result: {:?}", result);
    
    println!("\n=== Content Selection with Loop ===");
    let loop_op = ContentSelectionLoopOp::new();
    let result = loop_op.perform(&mut dry, &mut wet).await?;
    println!("Loop content selection result: {:?}", result);
    
    println!("\n=== Content Selection with While Loop ===");
    dry.insert("has_more_content", true);
    let while_op = ContentSelectionWhileOp::new();
    let result = while_op.perform(&mut dry, &mut wet).await?;
    println!("While loop content selection result: {:?}", result);
    
    println!("\n=== Full Extraction Pipeline ===");
    let extraction_pipeline = CloseReadOpBatch::new();
    let result = extraction_pipeline.perform(&mut dry, &mut wet).await?;
    println!("Full extraction pipeline result: {:?}", result);
    
    println!("\n=== Fully Composable Pipeline ===");
    let composable_pipeline = FullyComposablePipeline::new();
    let result = composable_pipeline.perform(&mut dry, &mut wet).await?;
    println!("Fully composable pipeline result: {:?}", result);
    
    Ok(())
}
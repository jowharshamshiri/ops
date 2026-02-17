use ops::prelude::*;
use ops::{batch, repeat, repeat_until};

// Example ops for demonstration
#[derive(Debug)]
struct ProcessOp {
    name: String,
}

impl ProcessOp {
    fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl Op<String> for ProcessOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<String> {
        let iteration = dry.get::<usize>("iteration").unwrap_or(0);
        let result = format!("{}-{}", self.name, iteration);
        println!("ProcessOp: {}", result);
        Ok(result)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder(&self.name).build()
    }
}

#[derive(Debug)]
struct CheckOp;

#[async_trait]
impl Op<bool> for CheckOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<bool> {
        let iteration = dry.get::<usize>("iteration").unwrap_or(0);
        let should_continue = iteration < 3; // Stop after 3 iterations
        dry.insert("should_continue", should_continue);
        println!("CheckOp: should_continue = {}", should_continue);
        Ok(should_continue)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("CheckOp").build()
    }
}

// Example 1: Batch operations with different aggregation strategies

// Traditional batch that returns Vec<String>
batch! {
    ProcessingPipeline<String> = [
        ProcessOp::new("step1"),
        ProcessOp::new("step2"),
        ProcessOp::new("step3")
    ]
}

// Batch that returns only the last result
batch! {
    ProcessingPipelineLast<String> -> last = [
        ProcessOp::new("step1"),
        ProcessOp::new("step2"),
        ProcessOp::new("step3")
    ]
}

// Batch that returns only the first result
batch! {
    ProcessingPipelineFirst<String> -> first = [
        ProcessOp::new("step1"),
        ProcessOp::new("step2"),
        ProcessOp::new("step3")
    ]
}

// Batch that ignores results and returns ()
batch! {
    ProcessingPipelineUnit<String> -> unit = [
        ProcessOp::new("step1"),
        ProcessOp::new("step2"),
        ProcessOp::new("step3")
    ]
}

// Example 2: For-loop (repeat) operations with different aggregation strategies

// Traditional repeat that returns Vec<String>
repeat! {
    ProcessingLoop<String> = {
        counter: "iteration",
        limit: "max_iterations",
        ops: [
            ProcessOp::new("loop_step")
        ]
    }
}

// Repeat that returns only the last result
repeat! {
    ProcessingLoopLast<String> -> last = {
        counter: "iteration", 
        limit: "max_iterations",
        ops: [
            ProcessOp::new("loop_step")
        ]
    }
}

// Repeat that ignores results and returns ()
repeat! {
    ProcessingLoopUnit<String> -> unit = {
        counter: "iteration",
        limit: "max_iterations", 
        ops: [
            ProcessOp::new("loop_step")
        ]
    }
}

// Example 3: While-loop operations

// While loop that returns Vec<String> (just ProcessOp)
repeat_until! {
    ProcessUntilDone<String> = {
        counter: "iteration",
        condition: "should_continue",
        max_iterations: 10,
        ops: [
            ProcessOp::new("while_step")
        ]
    }
}

// While loop that returns only the last result
repeat_until! {
    ProcessUntilDoneLast<String> -> last = {
        counter: "iteration",
        condition: "should_continue",
        max_iterations: 10,
        ops: [
            ProcessOp::new("while_step")
        ]
    }
}

// While loop for checking condition (using bool ops)
repeat_until! {
    ConditionChecker<bool> = {
        counter: "iteration",
        condition: "should_continue",
        max_iterations: 10,
        ops: [
            CheckOp
        ]
    }
}

// Example 4: Composable ops - any op can be used within any other

// Batch containing different types of ops
batch! {
    ComposablePipeline<String> -> last = [
        ProcessOp::new("initial"),
        ProcessingPipelineFirst::new(),  // Batch op returning String
        ProcessingLoopLast::new()        // Repeat op returning String
    ]
}

// Example 5: Op wrappers for type conversion

// This would wrap a Vec<String> op to return just String
// op_wrapper! {
//     StringWrapper<Vec<String>> -> String = ProcessingPipeline::new() using last
// }

#[tokio::main]
async fn main() -> OpResult<()> {
    let mut dry = DryContext::new();
    let mut wet = WetContext::new();
    
    // Set up context
    dry.insert("max_iterations", 3_usize);
    dry.insert("should_continue", true);
    
    println!("=== Batch Operations ===");
    
    // Test batch operations
    let pipeline = ProcessingPipeline::new();
    let results = pipeline.perform(&mut dry, &mut wet).await?;
    println!("Batch all results: {:?}", results);
    
    let pipeline_last = ProcessingPipelineLast::new(); 
    let result = pipeline_last.perform(&mut dry, &mut wet).await?;
    println!("Batch last result: {:?}", result);
    
    let pipeline_first = ProcessingPipelineFirst::new();
    let result = pipeline_first.perform(&mut dry, &mut wet).await?;
    println!("Batch first result: {:?}", result);
    
    let pipeline_unit = ProcessingPipelineUnit::new();
    let result = pipeline_unit.perform(&mut dry, &mut wet).await?;
    println!("Batch unit result: {:?}", result);
    
    println!("\n=== Repeat Operations ===");
    
    // Test repeat operations
    let loop_op = ProcessingLoop::new();
    let results = loop_op.perform(&mut dry, &mut wet).await?;
    println!("Repeat all results: {:?}", results);
    
    let loop_last = ProcessingLoopLast::new();
    let result = loop_last.perform(&mut dry, &mut wet).await?;
    println!("Repeat last result: {:?}", result);
    
    let loop_unit = ProcessingLoopUnit::new();
    let result = loop_unit.perform(&mut dry, &mut wet).await?;
    println!("Repeat unit result: {:?}", result);
    
    println!("\n=== While Loop Operations ===");
    
    // Test while loop operations
    dry.insert("should_continue", true);
    let while_op = ProcessUntilDone::new();
    let results = while_op.perform(&mut dry, &mut wet).await?;
    println!("While loop all results: {:?}", results);
    
    dry.insert("should_continue", true);
    let while_last = ProcessUntilDoneLast::new(); 
    let result = while_last.perform(&mut dry, &mut wet).await?;
    println!("While loop last result: {:?}", result);
    
    dry.insert("should_continue", true);
    let condition_checker = ConditionChecker::new();
    let results = condition_checker.perform(&mut dry, &mut wet).await?;
    println!("Condition checker results: {:?}", results);
    
    println!("\n=== Composable Operations ===");
    
    // Test composable pipeline
    let composable = ComposablePipeline::new();
    let result = composable.perform(&mut dry, &mut wet).await?;
    println!("Composable pipeline result: {:?}", result);
    
    Ok(())
}
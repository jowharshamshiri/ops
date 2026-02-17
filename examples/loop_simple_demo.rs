use ops::prelude::*;
use ops::repeat;

// Simple counter op that uses the loop iteration
struct CounterOp;

#[async_trait]
impl Op<String> for CounterOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<String> {
        let iteration = dry.get::<usize>("counter").unwrap_or(0);
        let message = format!("Processing item {}", iteration);
        println!("{}", message);
        Ok(message)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("CounterOp")
            .description("Processes items based on counter")
            .build()
    }
}

// Define a simple processing loop
repeat! {
    SimpleProcessor<String> = {
        counter: "counter",
        limit: 5,
        ops: [
            CounterOp
        ]
    }
}

// Define a more complex loop with multiple ops
struct PrepareOp;

#[async_trait]
impl Op<String> for PrepareOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<String> {
        let iteration = dry.get::<usize>("work_counter").unwrap_or(0);
        println!("  [{}] Preparing work...", iteration);
        dry.insert("prepared", true);
        Ok(format!("Prepared {}", iteration))
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("PrepareOp").build()
    }
}

struct ExecuteOp;

#[async_trait]
impl Op<String> for ExecuteOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<String> {
        let iteration = dry.get::<usize>("work_counter").unwrap_or(0);
        let prepared = dry.get::<bool>("prepared").unwrap_or(false);
        
        if prepared {
            println!("  [{}] Executing work...", iteration);
            Ok(format!("Executed {}", iteration))
        } else {
            Err(OpError::ExecutionFailed("Not prepared".to_string()))
        }
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("ExecuteOp").build()
    }
}

// Complex workflow loop
repeat! {
    WorkflowProcessor<String> = {
        counter: "work_counter",
        limit: 3,
        ops: [
            PrepareOp,
            ExecuteOp
        ]
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Simple Loop Macro Demo ===\n");
    
    // Simple loop example
    println!("1. Simple counter loop:");
    let mut dry = DryContext::new();
    let mut wet = WetContext::new();
    
    let simple_loop = SimpleProcessor::new();
    let results = simple_loop.perform(&mut dry, &mut wet).await?;
    println!("Generated {} messages\n", results.len());
    
    // Complex workflow loop
    println!("2. Complex workflow loop:");
    let mut dry2 = DryContext::new();
    let mut wet2 = WetContext::new();
    
    let workflow_loop = WorkflowProcessor::new();
    let workflow_results = workflow_loop.perform(&mut dry2, &mut wet2).await?;
    println!("Workflow completed {} operations\n", workflow_results.len());
    
    // Show the generated type info
    println!("3. Generated types:");
    println!("  - SimpleProcessor implements Op<Vec<String>>");
    println!("  - WorkflowProcessor implements Op<Vec<String>>");
    println!("  - Both use LoopOp internally with their configured parameters");
    
    Ok(())
}
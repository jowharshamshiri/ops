use ops::prelude::*;
use ops::{repeat_until, break_loop, continue_loop};

// Your ContentSelectionOp with intelligent break/continue logic

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
        println!(" Loading content for iteration {}", iteration);
        
        // Simulate content loading
        dry.insert("content_length", iteration * 1000);
        
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
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        println!(" Inserting data with config: {}", self.config);
        
        // Simulate AI analysis result
        let iteration = dry.get::<usize>("cso").unwrap_or(0);
        let content_quality = match iteration {
            0..=1 => "insufficient", // First 2 iterations are bad
            2 => "adequate",         // 3rd iteration is okay
            _ => "excellent",        // 4th+ iterations are great
        };
        
        dry.insert("content_quality", content_quality.to_string());
        println!("    AI Analysis: Content quality is {}", content_quality);
        
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
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        let content_quality = dry.get::<String>("content_quality")
            .unwrap_or_else(|| "unknown".to_string());
        
        println!(" BitChoice processing quality: {}", content_quality);
        
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
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        let content_quality = dry.get::<String>("content_quality")
            .unwrap_or_else(|| "unknown".to_string());
        let iteration = dry.get::<usize>("cso").unwrap_or(0);
        
        println!(" Reacting to content selection: {}", content_quality);
        
        match content_quality.as_str() {
            "insufficient" => {
                if iteration >= 4 {
                    println!("   ERR Too many failed attempts, giving up");
                    break_loop!(dry); // Stop trying after 5 attempts
                } else {
                    println!("    Content insufficient, will try again");
                    dry.insert("should_continue", true);
                }
            }
            "adequate" => {
                println!("   OK Content is adequate, we can proceed");
                dry.insert("selection_result", "adequate");
                break_loop!(dry); // We got acceptable content, stop
            }
            "excellent" => {
                println!("    Excellent content found!");
                dry.insert("selection_result", "excellent");
                break_loop!(dry); // Perfect content, definitely stop
            }
            _ => {
                println!("   WARN Unknown quality, continuing...");
                dry.insert("should_continue", true);
            }
        }
        
        Ok(())
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("ReactToContentSelectionResponse").build()
    }
}

// Your ContentSelectionOp with intelligent stopping
repeat_until! {
    ContentSelectionOp<()> -> unit = {
        counter: "cso",
        condition: "should_continue",
        max_iterations: 10,
        ops: [
            LoadContentOp::new("cso".to_string()),
            InsertDataOp::with("ai_analysis_config"),
            BitChoiceOp,
            ReactToContentSelectionResponse
        ]
    }
}

// Usage in your extraction pipeline
#[derive(Debug)]
struct StartTransactionOp;

#[async_trait]
impl Op<()> for StartTransactionOp {
    async fn perform(&self, _dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        println!(" Starting transaction");
        Ok(())
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("StartTransactionOp").build()
    }
}

repeat_until! {
    CloseReadOpBatch<()> -> unit = {
        counter: "extraction_step",
        condition: "extraction_active",
        max_iterations: 5,
        ops: [
            StartTransactionOp,
            ContentSelectionOp::new()
        ]
    }
}

#[tokio::main]
async fn main() -> OpResult<()> {
    let mut dry = DryContext::new();
    let mut wet = WetContext::new();
    
    println!(" CONTENT SELECTION WITH INTELLIGENT BREAK/CONTINUE \n");
    
    // Test 1: Content selection that finds adequate content
    println!("=== Test 1: Finding adequate content ===");
    dry.insert("should_continue", true);
    
    let content_op = ContentSelectionOp::new();
    match content_op.perform(&mut dry, &mut wet).await {
        Ok(_) => {
            let result = dry.get::<String>("selection_result");
            println!("OK Content selection completed with result: {:?}", result);
        }
        Err(e) => println!("ERR Error: {}", e),
    }
    
    println!("\n=== Test 2: Full extraction pipeline ===");
    dry.clear_control_flags();
    dry.insert("should_continue", true);
    dry.insert("extraction_active", true);
    
    let extraction_pipeline = CloseReadOpBatch::new();
    match extraction_pipeline.perform(&mut dry, &mut wet).await {
        Ok(_) => {
            let result = dry.get::<String>("selection_result");
            println!("OK Extraction pipeline completed with result: {:?}", result);
        }
        Err(e) => println!("ERR Error: {}", e),
    }
    
    println!("\n SUMMARY:");
    println!("• Your ContentSelectionOp now intelligently uses break_loop! to stop when:");
    println!("  - Content quality is 'adequate' or 'excellent'");
    println!("  - Too many failed attempts (>4)");
    println!("• continue_loop! could be used to skip iterations if needed");
    println!("• The condition variable 'should_continue' controls the loop");
    println!("• max_iterations provides a safety limit");
    
    Ok(())
}
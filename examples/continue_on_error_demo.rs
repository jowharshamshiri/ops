use ops::prelude::*;
use ops::{repeat, DryContext, WetContext, Op, OpResult, OpMetadata, OpError};
use async_trait::async_trait;

#[derive(Debug)]
struct SaveTrackedFileOp {
    counter_key: String,
}

impl SaveTrackedFileOp {
    fn new(counter_key: String) -> Self {
        Self { counter_key }
    }
}

#[async_trait]
impl Op<()> for SaveTrackedFileOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        let counter: usize = dry.get(&self.counter_key).unwrap_or(0);
        println!("SaveTrackedFileOp: processing file {}", counter);
        
        // Simulate failure on iteration 2
        if counter == 2 {
            return Err(OpError::ExecutionFailed("Failed to save file".to_string()));
        }
        
        Ok(())
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("SaveTrackedFileOp").build()
    }
}

#[derive(Debug)]
struct GenerateThumbnailOp {
    counter_key: String,
}

impl GenerateThumbnailOp {
    fn new(counter_key: String) -> Self {
        Self { counter_key }
    }
}

#[async_trait]
impl Op<()> for GenerateThumbnailOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        let counter: usize = dry.get(&self.counter_key).unwrap_or(0);
        println!("GenerateThumbnailOp: generating thumbnail for iteration {}", counter);
        Ok(())
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("GenerateThumbnailOp").build()
    }
}

// Create the macro-generated loop op with continue_on_error enabled
repeat! {
    PostDiscoveryLoopOp<()> = {
        counter: "pdcl",
        limit: "pdcl_limit",
        continue_on_error: true,
        ops: [
            SaveTrackedFileOp::new("pdcl".to_string()),
            GenerateThumbnailOp::new("pdcl".to_string())
        ]
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut dry = DryContext::new()
        .with_value("pdcl_limit", 5_usize);
    let mut wet = WetContext::new();
    
    let loop_op = PostDiscoveryLoopOp::new();
    
    println!("Starting post-discovery loop with continue_on_error=true");
    
    match loop_op.perform(&mut dry, &mut wet).await {
        Ok(_) => {
            println!("Loop completed successfully, even with some iterations failing");
        }
        Err(e) => {
            eprintln!("Loop failed: {}", e);
        }
    }
    
    Ok(())
}
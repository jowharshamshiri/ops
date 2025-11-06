use ops::prelude::*;
use ops::{repeat_until, break_loop, continue_loop, break_loop_scoped};

// Example operations for nested loop testing
#[derive(Debug)]
struct OuterOp {
    id: String,
    action: String,
}

impl OuterOp {
    fn new(id: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            action: action.into(),
        }
    }
}

#[async_trait]
impl Op<String> for OuterOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<String> {
        let outer_iter = dry.get::<usize>("outer_counter").unwrap_or(0);
        let inner_iter = dry.get::<usize>("inner_counter").unwrap_or(0);
        
        println!("🔵 OuterOp[{}] - Outer:{} Inner:{} - Action: {}", 
                self.id, outer_iter, inner_iter, self.action);
        
        match self.action.as_str() {
            "break_inner" if outer_iter == 1 && inner_iter == 1 => {
                println!("   💥 Breaking inner loop!");
                break_loop!(dry); // Breaks the innermost (current) loop
            }
            "break_outer" if outer_iter == 2 && inner_iter == 0 => {
                println!("   🚪 Breaking outer loop!");
                // We need to break the outer loop, but we're in inner context
                // So we need to get the outer loop ID somehow
                if let Some(outer_loop_id) = dry.get::<String>("outer_loop_id") {
                    break_loop_scoped!(dry, outer_loop_id);
                }
            }
            "continue_inner" if outer_iter == 0 && inner_iter == 2 => {
                println!("   ⏭️ Continuing inner loop!");
                continue_loop!(dry); // Continues the innermost (current) loop
            }
            _ => {
                println!("   ✅ Normal processing");
            }
        }
        
        Ok(format!("OuterOp[{}]-{}-{}", self.id, outer_iter, inner_iter))
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder(&format!("OuterOp-{}", self.id)).build()
    }
}

#[derive(Debug)]
struct InnerOp {
    id: String,
}

impl InnerOp {
    fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

#[async_trait]
impl Op<String> for InnerOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<String> {
        let outer_iter = dry.get::<usize>("outer_counter").unwrap_or(0);
        let inner_iter = dry.get::<usize>("inner_counter").unwrap_or(0);
        
        println!("🔴 InnerOp[{}] - Outer:{} Inner:{}", self.id, outer_iter, inner_iter);
        
        Ok(format!("InnerOp[{}]-{}-{}", self.id, outer_iter, inner_iter))
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder(&format!("InnerOp-{}", self.id)).build()
    }
}

#[derive(Debug)]
struct ControlOp {
    name: String,
}

impl ControlOp {
    fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl Op<String> for ControlOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<String> {
        let outer_iter = dry.get::<usize>("outer_counter").unwrap_or(0);
        let inner_iter = dry.get::<usize>("inner_counter").unwrap_or(0);
        
        // Control the loops
        match self.name.as_str() {
            "inner_controller" => {
                // Inner loop runs max 4 times per outer iteration
                let should_continue = inner_iter < 3;
                dry.insert("inner_should_continue", should_continue);
                println!("🎮 InnerController: inner_iter={} -> should_continue={}", inner_iter, should_continue);
            }
            "outer_controller" => {
                // Outer loop runs max 4 times total
                let should_continue = outer_iter < 3;
                dry.insert("outer_should_continue", should_continue);
                println!("🎮 OuterController: outer_iter={} -> should_continue={}", outer_iter, should_continue);
            }
            _ => {}
        }
        
        Ok(format!("ControlOp[{}]-{}-{}", self.name, outer_iter, inner_iter))
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder(&format!("ControlOp-{}", self.name)).build()
    }
}

// Inner loop - processes items within each outer iteration
repeat_until! {
    InnerLoop<String> = {
        counter: "inner_counter",
        condition: "inner_should_continue",
        max_iterations: 10,
        ops: [
            InnerOp::new("A"),
            OuterOp::new("B", "normal"),
            OuterOp::new("C", "break_inner"),      // Will break inner loop at outer:1, inner:1
            OuterOp::new("D", "continue_inner"),   // Will continue inner loop at outer:0, inner:2
            ControlOp::new("inner_controller")
        ]
    }
}

// Wrapper to make InnerLoop return String instead of Vec<String>
#[derive(Debug)]
struct InnerLoopWrapper {
    inner: InnerLoop,
}

impl InnerLoopWrapper {
    fn new() -> Self {
        Self {
            inner: InnerLoop::new(),
        }
    }
}

#[async_trait]
impl Op<String> for InnerLoopWrapper {
    async fn perform(&self, dry: &mut DryContext, wet: &mut WetContext) -> OpResult<String> {
        let results = self.inner.perform(dry, wet).await?;
        Ok(format!("InnerLoop_Results[{}]", results.len()))
    }
    
    fn metadata(&self) -> OpMetadata {
        self.inner.metadata()
    }
}

// Outer loop - processes major iterations
repeat_until! {
    OuterLoop<String> = {
        counter: "outer_counter", 
        condition: "outer_should_continue",
        max_iterations: 10,
        ops: [
            OuterOp::new("1", "normal"),
            OuterOp::new("2", "break_outer"),  // Will try to break outer loop at outer:2
            InnerLoopWrapper::new(),           // This is the nested inner loop wrapper
            ControlOp::new("outer_controller")
        ]
    }
}

// Helper to store outer loop ID for scoped breaking
#[derive(Debug)]
struct OuterLoopWrapper {
    inner: OuterLoop,
}

impl OuterLoopWrapper {
    fn new() -> Self {
        Self {
            inner: OuterLoop::new(),
        }
    }
}

#[async_trait]
impl Op<Vec<String>> for OuterLoopWrapper {
    async fn perform(&self, dry: &mut DryContext, wet: &mut WetContext) -> OpResult<Vec<String>> {
        // Store the outer loop ID for nested ops to reference
        let outer_loop_id = format!("outer_{}", uuid::Uuid::new_v4());
        dry.insert("outer_loop_id", &outer_loop_id);
        
        self.inner.perform(dry, wet).await
    }
    
    fn metadata(&self) -> OpMetadata {
        self.inner.metadata()
    }
}

#[tokio::main]
async fn main() -> OpResult<()> {
    let mut dry = DryContext::new();
    let mut wet = WetContext::new();
    
    println!("🔄 NESTED LOOP SCOPED CONTROL DEMONSTRATION 🔄\n");
    
    // Initialize conditions
    dry.insert("outer_should_continue", true);
    dry.insert("inner_should_continue", true);
    
    println!("=== Starting Nested Loop Execution ===");
    println!("Expected behavior:");
    println!("• Outer loop: 4 iterations (0,1,2,3)");
    println!("• Inner loop: 4 iterations per outer (0,1,2,3)");
    println!("• Special actions:");
    println!("  - Continue inner at outer:0, inner:2");
    println!("  - Break inner at outer:1, inner:1");
    println!("  - Break outer at outer:2, inner:0");
    println!();
    
    let outer_loop = OuterLoopWrapper::new();
    match outer_loop.perform(&mut dry, &mut wet).await {
        Ok(results) => {
            println!("\n✅ Nested loop completed with {} results", results.len());
            println!("Results summary:");
            for (i, result) in results.iter().take(10).enumerate() {
                println!("  {}: {}", i, result);
            }
            if results.len() > 10 {
                println!("  ... and {} more", results.len() - 10);
            }
        }
        Err(e) => println!("❌ Error: {}", e),
    }
    
    println!("\n🎯 SUMMARY:");
    println!("• Each loop has unique UUID-based control variables");
    println!("• break_loop!() targets the current (innermost) loop");
    println!("• break_loop_scoped!() can target specific outer loops");
    println!("• continue_loop!() targets the current loop");
    println!("• No interference between nested loop control flows");
    
    Ok(())
}
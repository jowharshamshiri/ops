use ops::prelude::*;
use ops::{dry_put, dry_require, wet_put_ref, wet_require_ref, perform};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

// Mock database and config services for demonstration
struct DatabaseService {
    connection_info: String,
}

impl DatabaseService {
    fn new() -> Self {
        Self {
            connection_info: "Database connection established".to_string(),
        }
    }
    
    fn save(&self, data: i32) -> String {
        format!("Saved {} to {}", data, self.connection_info)
    }
}

struct ConfigService {
    config_value: i32,
}

impl ConfigService {
    fn new() -> Self {
        Self {
            config_value: 42, // Magic config value
        }
    }
    
    fn get_config(&self) -> i32 {
        self.config_value
    }
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

struct CounterService;

impl CounterService {
    fn get_counter(&self) -> u32 {
        COUNTER.fetch_add(1, Ordering::SeqCst)
    }
}

// Define ops that use both dry and wet contexts
struct DoubleNumberOp;

#[async_trait]
impl Op<i32> for DoubleNumberOp {
    async fn perform(&self, dry: &DryContext, _wet: &WetContext) -> OpResult<i32> {
        let x: i32 = dry_require!(dry, x)?;
        Ok(x * 2)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("DoubleNumberOp")
            .description("Doubles a number")
            .build()
    }
}

struct AddConfigOp;

#[async_trait]
impl Op<i32> for AddConfigOp {
    async fn perform(&self, dry: &DryContext, wet: &WetContext) -> OpResult<i32> {
        let x: i32 = dry_require!(dry, x)?;
        let config_service: Arc<ConfigService> = wet_require_ref!(wet, config_service)?;
        let config = config_service.get_config();
        Ok(x + config)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("AddConfigOp")
            .description("Adds config value to a number")
            .build()
    }
}

struct SaveToDatabaseOp;

#[async_trait]
impl Op<String> for SaveToDatabaseOp {
    async fn perform(&self, dry: &DryContext, wet: &WetContext) -> OpResult<String> {
        let data: i32 = dry_require!(dry, data)?;
        let database: Arc<DatabaseService> = wet_require_ref!(wet, database)?;
        Ok(database.save(data))
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("SaveToDatabaseOp")
            .description("Saves data to database")
            .build()
    }
}

struct IncrementCounterOp;

#[async_trait]
impl Op<i32> for IncrementCounterOp {
    async fn perform(&self, dry: &DryContext, wet: &WetContext) -> OpResult<i32> {
        let base: i32 = dry_require!(dry, base)?;
        let counter_service: Arc<CounterService> = wet_require_ref!(wet, counter_service)?;
        let counter = counter_service.get_counter();
        Ok(base + counter as i32)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("IncrementCounterOp")
            .description("Adds counter value to base number")
            .build()
    }
}

#[tokio::main]
async fn main() -> OpResult<()> {
    tracing_subscriber::fmt::init();
    println!("=== Testing Modern Requirements with Dry/Wet Contexts ===\n");

    // Set up wet context with services
    let mut wet = WetContext::new();
    let database = DatabaseService::new();
    let config_service = ConfigService::new();
    let counter_service = CounterService;
    
    wet_put_ref!(wet, database);
    wet_put_ref!(wet, config_service);
    wet_put_ref!(wet, counter_service);

    // Example 1: Mixed static values and services
    println!("1. Mixed static values and services:");
    let mut dry = DryContext::new();
    let x = 10;
    dry_put!(dry, x);
    
    // Chain operations manually to show data flow
    let double_op = DoubleNumberOp;
    let doubled = double_op.perform(&dry, &wet).await?;
    println!("After doubling {}: {}", x, doubled);
    
    // Update dry context with result for next op
    let x = doubled;
    dry_put!(dry, x);
    
    let add_config_op = AddConfigOp;
    let with_config = add_config_op.perform(&dry, &wet).await?;
    println!("After adding config: {}", with_config);
    
    // Save to database
    let data = with_config;
    dry_put!(dry, data);
    
    let save_op = SaveToDatabaseOp;
    let save_result = save_op.perform(&dry, &wet).await?;
    println!("Save result: {}\n", save_result);

    // Example 2: Only static values (simpler case)
    println!("2. Only static values:");
    let mut dry = DryContext::new();
    let x = 5;
    dry_put!(dry, x);
    
    let result = DoubleNumberOp.perform(&dry, &wet).await?;
    println!("Doubled result: {}\n", result);

    // Example 3: Using services for lazy evaluation
    println!("3. Using services for lazy evaluation:");
    let mut dry = DryContext::new();
    let base = 100;
    dry_put!(dry, base);
    
    // Counter service will increment each time it's called
    let result = IncrementCounterOp.perform(&dry, &wet).await?;
    println!("Base + counter: {}", result);
    
    // Call again to show counter increment
    let result2 = IncrementCounterOp.perform(&dry, &wet).await?;
    println!("Base + counter (incremented): {}\n", result2);

    // Example 4: Using the perform utility for automatic logging
    println!("4. Using perform utility with logging:");
    let mut dry = DryContext::new();
    let x = 7;
    dry_put!(dry, x);
    
    let result = perform(Box::new(DoubleNumberOp), &dry, &wet).await?;
    println!("Logged result: {}\n", result);

    // Example 5: Demonstrate service reuse
    println!("5. Service reuse and shared state:");
    for i in 1..=3 {
        let mut dry = DryContext::new();
        let base = i * 10;
        dry_put!(dry, base);
        
        let result = IncrementCounterOp.perform(&dry, &wet).await?;
        println!("Iteration {}: {} + counter = {}", i, base, result);
    }
    
    // Example 6: Show all op metadata
    println!("\n6. Op Metadata:");
    let ops: Vec<Box<dyn Op<i32>>> = vec![
        Box::new(DoubleNumberOp),
        Box::new(AddConfigOp),
        Box::new(IncrementCounterOp),
    ];
    
    for op in ops {
        let metadata = op.metadata();
        println!("- {}: {}", metadata.name, metadata.description.unwrap_or("No description".to_string()));
    }

    Ok(())
}
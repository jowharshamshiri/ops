# Ops - Rust Ops Framework

Rust ops framework with dry/wet context separation and deferred execution.

## Features

- **Dry/Wet Context separation** - serializable data vs runtime references
- **Op metadata and schema validation** for inputs, references, and outputs
- **Deferred execution** - save op requests for later execution
- **Composable wrappers** - logging and timeout decorators
- **Batch operations** - sequential and parallel execution
- **Ergonomic macros** - context access patterns
- **Memory safety** with zero-cost abstractions

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
ops = { path = "." }
tokio = { version = "1.0", features = ["full"] }
```

### Basic Usage

```rust
use ops::{Op, DryContext, WetContext, OpMetadata, perform};
use async_trait::async_trait;

struct GreetingOp;

#[async_trait]
impl Op<String> for GreetingOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> Result<String, ops::OpError> {
        let name = dry.get_required::<String>("name")?;
        Ok(format!("Hello, {}!", name))
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("GreetingOp")
            .description("Greets a person by name")
            .build()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut dry = DryContext::new().with_value("name", "World");
    let mut wet = WetContext::new();
    
    let op = Box::new(GreetingOp);
    let result = perform(op, &mut dry, &mut wet).await?;
    println!("{}", result);
    
    Ok(())
}
```

### Wrapper Composition

```rust
use ops::{LoggingWrapper, TimeBoundWrapper, DryContext, WetContext};
use std::time::Duration;

let op = Box::new(GreetingOp);
let timeout_op = TimeBoundWrapper::new(op, Duration::from_secs(5));
let logged_op = LoggingWrapper::new(Box::new(timeout_op), "GreetingOp".to_string());

let dry = DryContext::new().with_value("name", "World");
let wet = WetContext::new();

let result = logged_op.perform(&dry, &wet).await?;
```

### Batch Operations

```rust
use ops::{BatchOp, DryContext, WetContext};
use std::sync::Arc;

let ops: Vec<Arc<dyn Op<String>>> = vec![
    Arc::new(GreetingOp),
    Arc::new(GreetingOp),
];

let mut dry = DryContext::new().with_value("name", "Alice");
let mut wet = WetContext::new();

let batch = BatchOp::new(ops);
let results = batch.perform(&mut dry, &mut wet).await?;
```

## Architecture

### Core Components

- **Op Trait**: Async trait with metadata and validation
- **Dry/Wet Contexts**: Separation of data from references
- **Wrapper Pattern**: Composable decorators
- **Batch Operations**: Sequential and parallel execution
- **Deferred Execution**: Save and execute later

### Context System

- **DryContext**: Serializable data (can be persisted)
- **WetContext**: Runtime references (services, connections)
- **Schema Validation**: JSON Schema validation
- **Deferred Execution**: Save op requests for later

## Macros

Ergonomic context access patterns:

```rust
use ops::{dry_put, dry_require, wet_put_ref, wet_require_ref};

// Dry context (serializable data)
dry_put!(dry, user_id);
let name: String = dry_require!(dry, name)?;

// Wet context (runtime references)
wet_put_ref!(wet, database);
let db: Arc<Database> = wet_require_ref!(wet, database)?;
```

Available macros:

- `dry_put!`, `dry_get!`, `dry_require!`, `dry_result!`
- `wet_put_ref!`, `wet_put_arc!`, `wet_get_ref!`, `wet_require_ref!`

## Examples

Run examples to see the framework in action:

```bash
cargo run --example dry_wet_context_demo
cargo run --example macro_usage_demo
```

## Schema Validation

Define input, reference, and output schemas:

```rust
fn metadata(&self) -> OpMetadata {
    OpMetadata::builder("UserOp")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "user_id": {"type": "string"}
            },
            "required": ["user_id"]
        }))
        .reference_schema(json!({
            "type": "object",
            "properties": {
                "database": {"type": "DatabaseService"}
            },
            "required": ["database"]
        }))
        .output_schema(json!({
            "type": "object",
            "properties": {
                "status": {"type": "string"}
            }
        }))
        .build()
}

// Validate contexts before execution
let validation = op.metadata().validate_contexts(&dry, &wet)?;
if validation.is_valid {
    let result = op.perform(&dry, &wet).await?;
}
```

## Testing

```bash
cargo test                          # Unit tests
cargo test --test integration_tests # Integration tests
```

## License

MIT License - see LICENSE file for details.

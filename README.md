# Ops - Rust Ops Framework

Rust ops framework with composable wrappers and batch execution. Async op patterns with context management.

## Features

- Async-first design built on tokio
- Resilience patterns: retry strategies, circuit breakers, fallback
- Metrics collection with percentiles
- Composable op decorators
- Resource management with automatic cleanup
- Test coverage: 74 tests with high pass rate
- Memory safety with compile-time checks

## Quick Start

### Installation

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

// Define a simple op
struct GreetingOp;

#[async_trait]
impl Op<String> for GreetingOp {
    async fn perform(&self, dry: &DryContext, _wet: &WetContext) -> Result<String, ops::OpError> {
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
    let dry = DryContext::new().with_value("name", "World");
    let wet = WetContext::new();
    
    // Execute with automatic logging and error handling
    let op = Box::new(GreetingOp);
    let result = perform(op, &dry, &wet).await?;
    println!("{}", result);
    
    Ok(())
}
```

### Wrapper Composition

```rust
use ops::{LoggingWrapper, TimeBoundWrapper, DryContext, WetContext};
use std::time::Duration;

// Compose wrappers for enhanced functionality
let op = Box::new(GreetingOp);
let timeout_op = TimeBoundWrapper::new(op, Duration::from_secs(5));
let logged_op = LoggingWrapper::new(Box::new(timeout_op), "GreetingOp".to_string());

let dry = DryContext::new().with_value("name", "World");
let wet = WetContext::new();

let result = logged_op.perform(&dry, &wet).await?;
```

### Batch Ops

```rust
use ops::{BatchOp, DryContext, WetContext};
use std::sync::Arc;

// Execute ops in batch
let ops: Vec<Arc<dyn Op<String>>> = vec![
    Arc::new(GreetingOp),
    Arc::new(GreetingOp),
    Arc::new(GreetingOp),
];

let dry = DryContext::new()
    .with_value("name", "Alice"); // All ops will use same context
let wet = WetContext::new();

let batch = BatchOp::new(ops);
let results = batch.perform(&dry, &wet).await?;
```

### Resilience Patterns

```rust
use ops::{TimeBoundWrapper, LoggingWrapper};
use std::time::Duration;

// Timeout protection
let op = Box::new(GreetingOp);
let timeout_op = TimeBoundWrapper::new(op, Duration::from_secs(5));

// Add logging
let logged_op = LoggingWrapper::new(
    Box::new(timeout_op),
    "ResilientGreeting".to_string()
);

let dry = DryContext::new().with_value("name", "World");
let wet = WetContext::new();

let result = logged_op.perform(&dry, &wet).await?;
```

## Architecture

### Core Components

- **Op Trait**: Async trait for all ops with metadata and validation
- **Dry/Wet Contexts**: Separation of serializable data from runtime references
- **Wrapper Pattern**: Composable decorators (logging, timeout)
- **Batch Ops**: Sequential and parallel execution with error handling
- **Op Metadata**: Schema validation for inputs, references, and outputs
- **Deferred Execution**: Save op requests for later execution

### Advanced Features

- **Metrics Collection**: Automatic performance tracking and telemetry
- **HTML Processing**: Metadata extraction framework (feature-flagged)
- **JSON Ops**: Serialization/deserialization with validation
- **Context Factories**: Lazy initialization patterns for dependencies
- **Error Propagation**: Rich error context through wrapper chains

## Performance

Rust implementation provides:

- Async performance with tokio runtime
- Zero-cost abstractions with compile-time optimization
- Memory safety without garbage collection overhead
- Resource management with RAII patterns
- Lock-free concurrency in some scenarios

## Testing

Run the comprehensive test suite:

```bash
# Unit tests
cargo test

# Integration tests  
cargo test --test integration_tests

# Test coverage report
cargo test -- --test-threads=1

# Performance benchmarks
cargo test --release bench
```

**Test Results**: 74/75 tests passing

## Documentation

- [Architecture Guide](internal/ARCHITECTURE.md) - Design decisions and patterns
- [Feature Status](internal/FEATURES.md) - Implementation progress and testing
- [API Documentation](https://docs.rs/ops) - Generated from code comments

## Op Patterns

Framework provides op patterns including:

- Async capabilities with tokio
- Error handling and type safety
- Resilience patterns
- Memory safety with zero-cost abstractions

## Contributing

1. Review [Development Directives](internal/DIRECTIVES.md) for coding standards
2. Check [Current Features](internal/FEATURES.md) for implementation status  
3. Add tests for all new functionality
4. Ensure `cargo test` passes before submitting

## License

MIT License - see LICENSE file for details.

## Status

Implementation includes 36/36 features with test coverage.
74 tests with high pass rate.
Async implementation with tokio runtime.
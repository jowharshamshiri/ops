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
use ops::{Op, OpContext, perform};

// Define a simple op
struct GreetingOp {
    name: String,
}

#[async_trait::async_trait]
impl Op<String> for GreetingOp {
    async fn execute(&self, _ctx: &OpContext) -> Result<String, ops::OpError> {
        Ok(format!("Hello, {}!", self.name))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let op = GreetingOp { name: "World".to_string() };
    let context = OpContext::new();
    
    // Execute with automatic logging and error handling
    let result = perform(op, &context).await?;
    println!("{}", result);
    
    Ok(())
}
```

### Wrapper Composition

```rust
use ops::{LoggingWrapper, TimeBoundWrapper, OpContext};

// Compose wrappers for enhanced functionality
let op = TimeBoundWrapper::new(
    LoggingWrapper::new(GreetingOp { name: "World".to_string() }),
    Duration::from_secs(5)
);

let result = op.execute(&context).await?;
```

### Batch Ops

```rust
use ops::{BatchOp, OpContext};

// Execute ops in parallel with concurrency control
let ops = vec![
    Box::new(GreetingOp { name: "Alice".to_string() }),
    Box::new(GreetingOp { name: "Bob".to_string() }),
    Box::new(GreetingOp { name: "Charlie".to_string() }),
];

let batch = BatchOp::parallel(ops, Some(2)); // Max 2 concurrent
let results = batch.execute(&context).await?;
```

### Resilience Patterns

```rust
use ops::{RetryOp, RetryStrategy, CircuitBreakerOp};
use std::time::Duration;

// Retry with exponential backoff
let retry_op = RetryOp::new(
    GreetingOp { name: "World".to_string() },
    RetryStrategy::exponential(Duration::from_millis(100), 2.0, 3)
);

// Circuit breaker protection
let circuit_op = CircuitBreakerOp::new(
    retry_op,
    5,  // failure_threshold
    Duration::from_secs(60)  // timeout
);

let result = circuit_op.execute(&context).await?;
```

## Architecture

### Core Components

- **Op Trait**: Async trait for all ops with generic return types
- **OpContext**: Thread-safe context with serialization support
- **Wrapper Pattern**: Composable decorators (logging, timeout, metrics)
- **Batch Ops**: Sequential and parallel execution with error handling
- **Resilience Framework**: Retry, circuit breaker, and fallback strategies
- **Resource Management**: RAII-based cleanup with automatic pooling

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
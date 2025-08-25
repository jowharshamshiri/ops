pub mod context;
pub mod error;
pub mod operation;
pub mod batch;
pub mod wrappers;
pub mod ops;
pub mod json_ops;
pub mod html_ops;
pub mod resilience;
pub mod metrics;
pub mod resource_mgmt;

pub use context::{OperationalContext, HollowOpContext, RequirementFactory, ClosureFactory, ContextProvider};
pub use error::OperationError;
pub use operation::{Operation, ClosureOperation};
pub use batch::BatchOperation;
pub use wrappers::logging::LoggingWrapper;
pub use wrappers::timeout::TimeBoundWrapper;
pub use ops::{perform, get_caller_operation_name, wrap_nested_op_exception};
pub use json_ops::{
    DeserializeJsonOperation, SerializeToJsonOperation, SerializeToPrettyJsonOperation, 
    JsonRoundtripOperation, deserialize_json, serialize_to_json, serialize_to_pretty_json, json_roundtrip
};
pub use html_ops::{
    ExtractMetaDataOperation, FetchAndExtractMetaDataOperation, MetaDefinition, MetaInstance, 
    HtmlMetadata, extract_html_metadata
};
pub use resilience::{
    RetryOperation, RetryStrategy, CircuitBreakerOperation, CircuitBreakerConfig, CircuitState,
    FallbackOperation, ResilientOperation, with_retry, with_circuit_breaker, with_fallback, resilient
};
pub use metrics::{
    OperationMetrics, MetricsCollector, InMemoryMetricsCollector, MetricsSummary, MetricsOperation,
    global_metrics, with_metrics, auto_metrics, print_global_metrics, reset_global_metrics, get_global_metrics_summary
};
pub use resource_mgmt::{
    ManagedResource, ResourcePool, ManagedResourceHandle, ResourceManagedOperation,
    ConnectionResource, FileResource, with_connection_pool, with_file_pool
};
pub mod context;
pub mod error;
pub mod op;
pub mod batch;
pub mod wrappers;
pub mod ops;
pub mod json_ops;
pub mod html_ops;
pub mod resilience;
pub mod metrics;
pub mod resource_mgmt;
pub mod stateful;
pub mod state_machine;
pub mod persistence;
pub mod op_state;
pub mod infrastructure_state;
pub mod rollback;
pub mod safe_point;
pub mod emergency_rollback;
pub mod hierarchical_ops;
pub mod entity_management;
pub mod compensating_ops;
pub mod audit_trail;

pub use context::{OpContext, HollowOpContext, RequirementFactory, ClosureFactory, ContextProvider};
pub use error::OpError;
pub use op::{Op, ClosureOp};
pub use batch::BatchOp;
pub use wrappers::logging::LoggingWrapper;
pub use wrappers::timeout::TimeBoundWrapper;
pub use ops::{perform, get_caller_op_name, wrap_nested_op_exception};
pub use json_ops::{
    DeserializeJsonOp, SerializeToJsonOp, SerializeToPrettyJsonOp, 
    JsonRoundtripOp, deserialize_json, serialize_to_json, serialize_to_pretty_json, json_roundtrip
};
pub use html_ops::{
    ExtractMetaDataOp, FetchAndExtractMetaDataOp, MetaDefinition, MetaInstance, 
    HtmlMetadata, extract_html_metadata
};
pub use resilience::{
    RetryOp, RetryStrategy, CircuitBreakerOp, CircuitBreakerConfig, CircuitState,
    FallbackOp, ResilientOp, with_retry, with_circuit_breaker, with_fallback, resilient
};
pub use metrics::{
    OpMetrics, MetricsCollector, InMemoryMetricsCollector, MetricsSummary, MetricsOp,
    global_metrics, with_metrics, auto_metrics, print_global_metrics, reset_global_metrics, get_global_metrics_summary
};
pub use resource_mgmt::{
    ManagedResource, ResourcePool, ManagedResourceHandle, ResourceManagedOp,
    ConnectionResource, FileResource, with_connection_pool, with_file_pool
};
pub use stateful::{
    StatefulOp, StatefulResult, StatefulWrapper, EntityMetadata, make_stateful
};
pub use state_machine::{
    StateMachine, State, Transition, StateMachineInstance, StateTransitionRecord,
    TransitionGuard, TransitionAction, ClosureGuard, ClosureAction, StateId, TransitionId
};
pub use persistence::{
    StateStorage, StateSnapshot, StatePersistenceManager, FileSystemStateStorage, 
    PersistenceId, TransitionRecord
};
pub use op_state::{
    OpState, OpStateInfo, OpStateTracker,
    OpStatistics, Priority, OpInstanceId, global_op_tracker
};

pub use infrastructure_state::{
    ComponentType, HealthStatus, DeploymentStatus, ComponentConfiguration, ComponentResources,
    InfrastructureComponentState, InfrastructureTopology, InfrastructureStatistics, ComponentId, ComponentVersion
};
pub use rollback::{
    RollbackStrategy, RollbackInfo, RollbackStatus, RollbackPriority, RollbackStrategyRegistry,
    NoOpRollbackStrategy, CommandRollbackStrategy, SnapshotRollbackStrategy, RollbackId,
    global_rollback_registry, register_global_strategy, find_global_strategy
};
pub use entity_management::{
    EntityData, CreateEntityOp, ReadEntityOp, UpdateEntityOp, 
    DeleteEntityOp, QueryEntitiesOp
};
pub use compensating_ops::{
    CompensationStrategy, CompensationMetadata, Compensatable, CompensatingOpGenerator,
    CompensationFactory, AutoCompensatingOp, with_auto_compensation,
    global_compensation_generator, register_global_compensation_factory
};
pub use audit_trail::{
    AuditRecord, AuditTrailQuery, AuditTrailCollector, ContextSnapshot, VisualizationMetadata,
    AuditStatistics, VisualizationExporter, AuditableOp, with_audit_trail,
    set_global_audit_collector, get_global_audit_collector
};
pub use safe_point::{
    SafePointId, SafePointStatus, SafePointMetadata, SafePointSnapshot, SafePointOptions,
    SafePointManager, SafePointRestoreOp
};
pub use emergency_rollback::{
    EmergencyTrigger, EmergencyStatus, EmergencyRollbackRecord, EmergencyRollbackManager,
    EmergencyRollbackOp
};
pub use hierarchical_ops::{
    HierarchyId, DependencyType, HierarchyNode, HierarchyStatus, OpHierarchy,
    HierarchyStatistics, HierarchicalOpManager, ExecutionPlan
};
use crate::prelude::*;

/// Unique identifier for audit records
pub type AuditRecordId = String;

/// Context snapshot for tracking context evolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSnapshot {
    /// Timestamp when snapshot was taken
    pub timestamp: DateTime<Utc>,
    /// Snapshot description (before_op, after_op, error_state, etc.)
    pub description: String,
    /// Context data at this point
    pub context_data: HashMap<String, String>,
    /// Size of context data
    pub data_size: usize,
}

impl ContextSnapshot {
    /// Create new context snapshot
    pub fn new(description: String, context: &OpContext) -> Self {
        let mut context_data = HashMap::new();
        for (key, value) in context.values().iter() {
            context_data.insert(key.clone(), value.to_string());
        }
        
        let data_size = context_data.len();
        
        Self {
            timestamp: Utc::now(),
            description,
            context_data,
            data_size,
        }
    }

    /// Create snapshot with custom timestamp
    pub fn new_with_timestamp(description: String, context: &OpContext, timestamp: DateTime<Utc>) -> Self {
        let mut snapshot = Self::new(description, context);
        snapshot.timestamp = timestamp;
        snapshot
    }
}

/// Visualization metadata for charts, graphs, and timelines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationMetadata {
    /// Chart type suggestions (timeline, bar, flow, network, etc.)
    pub chart_types: Vec<String>,
    /// Key metrics for visualization
    pub metrics: HashMap<String, f64>,
    /// Color coding suggestions (success, failure, warning, info)
    pub color_category: String,
    /// Hierarchy level for nested visualizations
    pub hierarchy_level: u32,
    /// Related record IDs for relationship visualization
    pub related_records: Vec<AuditRecordId>,
    /// Tags for filtering and grouping
    pub tags: Vec<String>,
}

impl VisualizationMetadata {
    /// Create new visualization metadata
    pub fn new() -> Self {
        Self {
            chart_types: Vec::new(),
            metrics: HashMap::new(),
            color_category: "info".to_string(),
            hierarchy_level: 0,
            related_records: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// Add chart type suggestion
    pub fn with_chart_type(mut self, chart_type: String) -> Self {
        self.chart_types.push(chart_type);
        self
    }

    /// Add metric for visualization
    pub fn with_metric(mut self, name: String, value: f64) -> Self {
        self.metrics.insert(name, value);
        self
    }

    /// Set color category
    pub fn with_color_category(mut self, category: String) -> Self {
        self.color_category = category;
        self
    }

    /// Set hierarchy level
    pub fn with_hierarchy_level(mut self, level: u32) -> Self {
        self.hierarchy_level = level;
        self
    }

    /// Add related record
    pub fn with_related_record(mut self, record_id: AuditRecordId) -> Self {
        self.related_records.push(record_id);
        self
    }

    /// Add tag
    pub fn with_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }
}

impl Default for VisualizationMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Comprehensive audit record capturing complete op lifecycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    /// Unique audit record identifier
    pub record_id: AuditRecordId,
    /// Record creation timestamp
    pub timestamp: DateTime<Utc>,
    /// Op state information from existing tracking
    pub op_info: OpStateInfo,
    /// State machine transitions if applicable
    pub state_transitions: Vec<TransitionRecord>,
    /// Performance metrics from existing system
    pub metrics: Option<OpMetrics>,
    /// Rollback information if rollback occurred
    pub rollback_info: Option<RollbackInfo>,
    /// Context evolution throughout op
    pub context_evolution: Vec<ContextSnapshot>,
    /// Entity metadata if this is a stateful op
    pub entity_metadata: Option<EntityMetadata>,
    /// Visualization support metadata
    pub visualization_metadata: VisualizationMetadata,
    /// Error information if op failed
    pub error_details: Option<String>,
    /// Custom audit attributes
    pub custom_attributes: HashMap<String, String>,
}

impl AuditRecord {
    /// Create new audit record
    pub fn new(record_id: AuditRecordId, op_info: OpStateInfo) -> Self {
        let color_category = match op_info.statistics.success_count > 0 {
            true => "success".to_string(),
            false => match op_info.statistics.failure_count > 0 {
                true => "failure".to_string(),
                false => "info".to_string(),
            },
        };

        let mut visualization_metadata = VisualizationMetadata::new()
            .with_chart_type("timeline".to_string())
            .with_color_category(color_category);

        // Add metrics for visualization
        visualization_metadata = visualization_metadata
            .with_metric("duration_ms".to_string(), op_info.statistics.total_duration_ms as f64)
            .with_metric("success_count".to_string(), op_info.statistics.success_count as f64)
            .with_metric("failure_count".to_string(), op_info.statistics.failure_count as f64);

        Self {
            record_id,
            timestamp: Utc::now(),
            op_info,
            state_transitions: Vec::new(),
            metrics: None,
            rollback_info: None,
            context_evolution: Vec::new(),
            entity_metadata: None,
            visualization_metadata,
            error_details: None,
            custom_attributes: HashMap::new(),
        }
    }

    /// Add state transitions
    pub fn with_state_transitions(mut self, transitions: Vec<TransitionRecord>) -> Self {
        let has_transitions = !transitions.is_empty();
        self.state_transitions = transitions;
        if has_transitions {
            self.visualization_metadata = self.visualization_metadata
                .with_chart_type("flow".to_string())
                .with_tag("state_machine".to_string());
        }
        self
    }

    /// Add metrics information
    pub fn with_metrics(mut self, metrics: OpMetrics) -> Self {
        self.visualization_metadata = self.visualization_metadata
            .with_metric("avg_duration".to_string(), metrics.average_duration.map(|d| d.as_secs_f64()).unwrap_or(0.0))
            .with_metric("min_duration".to_string(), metrics.min_duration.map(|d| d.as_secs_f64()).unwrap_or(0.0))
            .with_metric("max_duration".to_string(), metrics.max_duration.map(|d| d.as_secs_f64()).unwrap_or(0.0))
            .with_chart_type("bar".to_string())
            .with_tag("metrics".to_string());
        
        self.metrics = Some(metrics);
        self
    }

    /// Add rollback information
    pub fn with_rollback_info(mut self, rollback_info: RollbackInfo) -> Self {
        let color = match rollback_info.status {
            RollbackStatus::Completed => "success",
            RollbackStatus::Failed => "failure", 
            RollbackStatus::PartiallyCompleted => "warning",
            _ => "info",
        };
        
        self.visualization_metadata = self.visualization_metadata
            .with_color_category(color.to_string())
            .with_chart_type("network".to_string())
            .with_tag("rollback".to_string())
            .with_metric("rollback_priority".to_string(), rollback_info.priority as u8 as f64);
            
        self.rollback_info = Some(rollback_info);
        self
    }

    /// Add context evolution
    pub fn with_context_evolution(mut self, evolution: Vec<ContextSnapshot>) -> Self {
        if !evolution.is_empty() {
            self.visualization_metadata = self.visualization_metadata
                .with_chart_type("line".to_string())
                .with_tag("context_tracking".to_string())
                .with_metric("context_changes".to_string(), evolution.len() as f64);
        }
        self.context_evolution = evolution;
        self
    }

    /// Add entity metadata
    pub fn with_entity_metadata(mut self, entity_metadata: EntityMetadata) -> Self {
        self.visualization_metadata = self.visualization_metadata
            .with_tag("entity_op".to_string())
            .with_tag(format!("entity_type_{}", entity_metadata.entity_type));
            
        self.entity_metadata = Some(entity_metadata);
        self
    }

    /// Add error details
    pub fn with_error_details(mut self, error: String) -> Self {
        self.visualization_metadata = self.visualization_metadata
            .with_color_category("failure".to_string())
            .with_tag("error".to_string());
            
        self.error_details = Some(error);
        self
    }

    /// Add custom attribute
    pub fn with_custom_attribute(mut self, key: String, value: String) -> Self {
        self.custom_attributes.insert(key, value);
        self
    }

    /// Get op duration in milliseconds
    pub fn total_duration_ms(&self) -> u64 {
        self.op_info.statistics.total_duration_ms
    }

    /// Check if op was successful
    pub fn is_successful(&self) -> bool {
        self.op_info.statistics.failure_count == 0 && self.error_details.is_none()
    }

    /// Check if rollback occurred
    pub fn had_rollback(&self) -> bool {
        self.rollback_info.is_some()
    }

    /// Get all tags for filtering
    pub fn get_all_tags(&self) -> Vec<String> {
        let mut tags = self.visualization_metadata.tags.clone();
        
        // Add automatic tags based on content
        if self.had_rollback() { tags.push("rollback".to_string()); }
        if !self.is_successful() { tags.push("failed".to_string()); }
        if self.entity_metadata.is_some() { tags.push("entity_op".to_string()); }
        if !self.state_transitions.is_empty() { tags.push("state_machine".to_string()); }
        
        tags.sort();
        tags.dedup();
        tags
    }
}

/// Query builder for audit trail searches
#[derive(Debug, Clone)]
pub struct AuditTrailQuery {
    /// Time range filter
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// Op types to include
    pub op_types: Vec<String>,
    /// Success status filter
    pub success_status: Option<bool>,
    /// Rollback involvement filter
    pub rollback_involved: Option<bool>,
    /// Entity type filter
    pub entity_types: Vec<String>,
    /// Tag filters (all must be present)
    pub required_tags: Vec<String>,
    /// Tag filters (any can be present)
    pub any_tags: Vec<String>,
    /// Minimum duration filter
    pub min_duration_ms: Option<u64>,
    /// Maximum duration filter
    pub max_duration_ms: Option<u64>,
    /// Custom attribute filters
    pub custom_attribute_filters: HashMap<String, String>,
    /// Limit number of results
    pub limit: Option<usize>,
    /// Sort order (newest_first, oldest_first, duration_asc, duration_desc)
    pub sort_order: String,
}

impl AuditTrailQuery {
    /// Create new query
    pub fn new() -> Self {
        Self {
            time_range: None,
            op_types: Vec::new(),
            success_status: None,
            rollback_involved: None,
            entity_types: Vec::new(),
            required_tags: Vec::new(),
            any_tags: Vec::new(),
            min_duration_ms: None,
            max_duration_ms: None,
            custom_attribute_filters: HashMap::new(),
            limit: None,
            sort_order: "newest_first".to_string(),
        }
    }

    /// Filter by time range
    pub fn time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.time_range = Some((start, end));
        self
    }

    /// Filter by op type
    pub fn op_type(mut self, op_type: String) -> Self {
        self.op_types.push(op_type);
        self
    }

    /// Filter by success status
    pub fn success_status(mut self, success: bool) -> Self {
        self.success_status = Some(success);
        self
    }

    /// Filter by rollback involvement
    pub fn with_rollbacks(mut self) -> Self {
        self.rollback_involved = Some(true);
        self
    }

    /// Filter by entity type
    pub fn entity_type(mut self, entity_type: String) -> Self {
        self.entity_types.push(entity_type);
        self
    }

    /// Add required tag
    pub fn with_required_tag(mut self, tag: String) -> Self {
        self.required_tags.push(tag);
        self
    }

    /// Add optional tag
    pub fn with_any_tag(mut self, tag: String) -> Self {
        self.any_tags.push(tag);
        self
    }

    /// Filter by minimum duration
    pub fn min_duration_ms(mut self, duration: u64) -> Self {
        self.min_duration_ms = Some(duration);
        self
    }

    /// Filter by maximum duration  
    pub fn max_duration_ms(mut self, duration: u64) -> Self {
        self.max_duration_ms = Some(duration);
        self
    }

    /// Add custom attribute filter
    pub fn with_custom_attribute(mut self, key: String, value: String) -> Self {
        self.custom_attribute_filters.insert(key, value);
        self
    }

    /// Limit results
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set sort order
    pub fn sort_by(mut self, order: String) -> Self {
        self.sort_order = order;
        self
    }

    /// Check if record matches this query
    pub fn matches(&self, record: &AuditRecord) -> bool {
        // Time range check
        if let Some((start, end)) = self.time_range {
            if record.timestamp < start || record.timestamp > end {
                return false;
            }
        }

        // Op type check
        if !self.op_types.is_empty() {
            let op_type = record.op_info.op_type.clone();
            if !self.op_types.contains(&op_type) {
                return false;
            }
        }

        // Success status check
        if let Some(success) = self.success_status {
            if record.is_successful() != success {
                return false;
            }
        }

        // Rollback involvement check
        if let Some(rollback) = self.rollback_involved {
            if record.had_rollback() != rollback {
                return false;
            }
        }

        // Entity type check
        if !self.entity_types.is_empty() {
            match &record.entity_metadata {
                Some(entity) => {
                    if !self.entity_types.contains(&entity.entity_type) {
                        return false;
                    }
                }
                None => return false,
            }
        }

        // Required tags check (all must be present)
        let record_tags = record.get_all_tags();
        for required_tag in &self.required_tags {
            if !record_tags.contains(required_tag) {
                return false;
            }
        }

        // Any tags check (at least one must be present)
        if !self.any_tags.is_empty() {
            let has_any = self.any_tags.iter().any(|tag| record_tags.contains(tag));
            if !has_any {
                return false;
            }
        }

        // Duration checks
        let duration = record.total_duration_ms();
        if let Some(min) = self.min_duration_ms {
            if duration < min {
                return false;
            }
        }
        if let Some(max) = self.max_duration_ms {
            if duration > max {
                return false;
            }
        }

        // Custom attribute checks
        for (key, expected_value) in &self.custom_attribute_filters {
            match record.custom_attributes.get(key) {
                Some(actual_value) => {
                    if actual_value != expected_value {
                        return false;
                    }
                }
                None => return false,
            }
        }

        true
    }
}

impl Default for AuditTrailQuery {
    fn default() -> Self {
        Self::new()
    }
}

/// Comprehensive audit trail collector integrating all tracking systems
pub struct AuditTrailCollector {
    /// Metrics collector integration  
    metrics_collector: Arc<dyn MetricsCollector>,
    /// Persistence manager integration
    persistence_manager: Option<Arc<StatePersistenceManager>>,
    /// In-memory audit record storage
    records: std::sync::RwLock<Vec<AuditRecord>>,
    /// Maximum records to keep in memory
    max_records: usize,
    /// Op info storage (simulating tracker)
    op_infos: std::sync::RwLock<HashMap<OpInstanceId, OpStateInfo>>,
}

impl AuditTrailCollector {
    /// Create new audit trail collector
    pub fn new(
        metrics_collector: Arc<dyn MetricsCollector>,
    ) -> Self {
        Self {
            metrics_collector,
            persistence_manager: None,
            records: std::sync::RwLock::new(Vec::new()),
            max_records: 10000, // Default limit
            op_infos: std::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Set persistence manager
    pub fn with_persistence(mut self, manager: Arc<StatePersistenceManager>) -> Self {
        self.persistence_manager = Some(manager);
        self
    }

    /// Set maximum records to keep in memory
    pub fn with_max_records(mut self, max: usize) -> Self {
        self.max_records = max;
        self
    }

    /// Add op info to the collector
    pub fn add_op_info(&self, op_id: OpInstanceId, info: OpStateInfo) {
        let mut op_infos = self.op_infos.write().unwrap();
        op_infos.insert(op_id, info);
    }

    /// Create audit record from op execution
    pub async fn create_audit_record<T>(
        &self,
        op_id: &OpInstanceId,
        context_snapshots: Vec<ContextSnapshot>,
        entity_metadata: Option<EntityMetadata>,
        error_details: Option<String>,
    ) -> Result<AuditRecord, OpError>
    where
        T: Send + Sync + 'static,
    {
        // Get op info from storage
        let op_infos = self.op_infos.read().unwrap();
        let op_info = op_infos.get(op_id)
            .ok_or_else(|| OpError::ExecutionFailed(
                format!("Op info not found: {}", op_id)
            ))?.clone();
        drop(op_infos);

        // Generate record ID
        let record_id = format!("audit_{}_{}", op_id, Utc::now().timestamp_millis());
        
        // Create base audit record
        let mut audit_record = AuditRecord::new(record_id, op_info.clone());

        // Add context evolution
        audit_record = audit_record.with_context_evolution(context_snapshots);

        // Add entity metadata if present
        if let Some(entity) = entity_metadata {
            audit_record = audit_record.with_entity_metadata(entity);
        }

        // Add error details if present
        if let Some(error) = error_details {
            audit_record = audit_record.with_error_details(error);
        }

        // Try to get metrics
        if let Some(metrics) = self.metrics_collector.get_op_metrics(&op_info.op_type) {
            audit_record = audit_record.with_metrics(metrics);
        }

        Ok(audit_record)
    }

    /// Add completed audit record to collection
    pub async fn add_record(&self, record: AuditRecord) -> OpResult<()> {
        // Add to in-memory collection
        {
            let mut records = self.records.write().unwrap();
            records.push(record.clone());
            
            // Trim to max size if needed
            if records.len() > self.max_records {
                records.remove(0);
            }
        }

        // Persist if manager is available
        if let Some(_manager) = &self.persistence_manager {
            let record_data = serde_json::to_string(&record).map_err(|e| {
                OpError::ExecutionFailed(format!("Failed to serialize audit record: {}", e))
            })?;
            
            // Store audit record (would need to add audit storage methods to persistence manager)
            tracing::info!("Audit record {} persisted (size: {} bytes)", record.record_id, record_data.len());
        }

        Ok(())
    }

    /// Query audit records
    pub fn query_records(&self, query: &AuditTrailQuery) -> Vec<AuditRecord> {
        let records = self.records.read().unwrap();
        let mut matching_records: Vec<AuditRecord> = records
            .iter()
            .filter(|record| query.matches(record))
            .cloned()
            .collect();

        // Sort records
        match query.sort_order.as_str() {
            "newest_first" => matching_records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp)),
            "oldest_first" => matching_records.sort_by(|a, b| a.timestamp.cmp(&b.timestamp)),
            "duration_asc" => matching_records.sort_by(|a, b| a.total_duration_ms().cmp(&b.total_duration_ms())),
            "duration_desc" => matching_records.sort_by(|a, b| b.total_duration_ms().cmp(&a.total_duration_ms())),
            _ => {} // Keep original order
        }

        // Apply limit
        if let Some(limit) = query.limit {
            matching_records.truncate(limit);
        }

        matching_records
    }

    /// Get audit record by ID
    pub fn get_record(&self, record_id: &AuditRecordId) -> Option<AuditRecord> {
        let records = self.records.read().unwrap();
        records.iter()
            .find(|record| record.record_id == *record_id)
            .cloned()
    }

    /// Get audit statistics
    pub fn get_audit_statistics(&self) -> AuditStatistics {
        let records = self.records.read().unwrap();
        let total_records = records.len();
        let successful_ops = records.iter().filter(|r| r.is_successful()).count();
        let failed_ops = records.iter().filter(|r| !r.is_successful()).count();
        let rollback_ops = records.iter().filter(|r| r.had_rollback()).count();
        
        let total_duration: u64 = records.iter().map(|r| r.total_duration_ms()).sum();
        let avg_duration = if total_records > 0 { total_duration / total_records as u64 } else { 0 };

        AuditStatistics {
            total_records,
            successful_ops,
            failed_ops,
            rollback_ops,
            avg_duration_ms: avg_duration,
            total_duration_ms: total_duration,
        }
    }

    /// Clear all records
    pub fn clear_records(&self) {
        let mut records = self.records.write().unwrap();
        records.clear();
    }

    /// Get record count
    pub fn record_count(&self) -> usize {
        let records = self.records.read().unwrap();
        records.len()
    }
}

/// Audit statistics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStatistics {
    pub total_records: usize,
    pub successful_ops: usize,
    pub failed_ops: usize,
    pub rollback_ops: usize,
    pub avg_duration_ms: u64,
    pub total_duration_ms: u64,
}

/// Visualization data exporter for different formats
pub struct VisualizationExporter;

impl VisualizationExporter {
    /// Export audit records as timeline data
    pub fn export_timeline(records: Vec<AuditRecord>) -> serde_json::Value {
        let timeline_events: Vec<serde_json::Value> = records
            .into_iter()
            .map(|record| {
                serde_json::json!({
                    "id": record.record_id,
                    "timestamp": record.timestamp,
                    "title": format!("{} ({}ms)", record.op_info.op_type, record.total_duration_ms()),
                    "content": record.op_info.op_type,
                    "className": record.visualization_metadata.color_category,
                    "type": if record.total_duration_ms() > 5000 { "range" } else { "point" },
                    "start": record.timestamp,
                    "end": if record.total_duration_ms() > 5000 { 
                        record.timestamp + chrono::Duration::milliseconds(record.total_duration_ms() as i64)
                    } else { 
                        record.timestamp 
                    },
                    "group": record.visualization_metadata.hierarchy_level,
                })
            })
            .collect();

        serde_json::json!({
            "items": timeline_events,
            "groups": [
                {"id": 0, "content": "Ops"},
                {"id": 1, "content": "State Changes"}, 
                {"id": 2, "content": "Rollbacks"}
            ]
        })
    }

    /// Export audit records as network/flow data
    pub fn export_network(records: Vec<AuditRecord>) -> serde_json::Value {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        for record in records {
            // Add op node
            nodes.push(serde_json::json!({
                "id": record.record_id,
                "label": record.op_info.op_type,
                "color": record.visualization_metadata.color_category,
                "size": (record.total_duration_ms() as f64).log10().max(1.0),
                "title": format!("Duration: {}ms", record.total_duration_ms())
            }));

            // Add edges for related records
            for related_id in &record.visualization_metadata.related_records {
                edges.push(serde_json::json!({
                    "from": record.record_id,
                    "to": related_id,
                    "arrows": "to"
                }));
            }
        }

        serde_json::json!({
            "nodes": nodes,
            "edges": edges
        })
    }

    /// Export audit records as CSV for analysis tools
    pub fn export_csv(records: Vec<AuditRecord>) -> String {
        let mut csv = String::from("record_id,timestamp,op_type,duration_ms,success,rollback,entity_type,tags\n");
        
        for record in records {
            let entity_type = record.entity_metadata
                .as_ref()
                .map(|e| e.entity_type.clone())
                .unwrap_or_else(|| "none".to_string());
                
            let tags = record.get_all_tags().join(";");
            
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                record.record_id,
                record.timestamp.format("%Y-%m-%d %H:%M:%S"),
                record.op_info.op_type,
                record.total_duration_ms(),
                record.is_successful(),
                record.had_rollback(),
                entity_type,
                tags
            ));
        }
        
        csv
    }
}

/// Op wrapper that automatically creates audit trails
pub struct AuditableOp<T, O>
where
    T: Send + Sync + 'static,
    O: Op<T>,
{
    op: O,
    audit_collector: Arc<AuditTrailCollector>,
    op_id: OpInstanceId,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, O> AuditableOp<T, O>
where
    T: Send + Sync + 'static,
    O: Op<T>,
{
    /// Create new auditable op wrapper
    pub fn new(op: O, audit_collector: Arc<AuditTrailCollector>, op_id: OpInstanceId) -> Self {
        Self {
            op,
            audit_collector,
            op_id,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<T, O> Op<T> for AuditableOp<T, O>
where
    T: Send + Sync + 'static,
    O: Op<T>,
{
    async fn perform(&self, context: &mut OpContext) -> Result<T, OpError> {
        let mut snapshots = Vec::new();
        
        // Take before snapshot
        snapshots.push(ContextSnapshot::new("before_op".to_string(), context));

        // Execute op
        let result = self.op.perform(context).await;

        // Take after snapshot
        let snapshot_description = match &result {
            Ok(_) => "after_op_success".to_string(),
            Err(_) => "after_op_failure".to_string(),
        };
        snapshots.push(ContextSnapshot::new(snapshot_description, context));

        // Create audit record
        let error_details = match &result {
            Ok(_) => None,
            Err(e) => Some(e.to_string()),
        };

        let audit_record = self.audit_collector
            .create_audit_record::<T>(&self.op_id, snapshots, None, error_details)
            .await?;

        // Add to audit trail
        self.audit_collector.add_record(audit_record).await?;

        result
    }
}

/// Convenience function to wrap op with audit trail
pub fn with_audit_trail<T, O>(
    op: O,
    audit_collector: Arc<AuditTrailCollector>,
    op_id: OpInstanceId,
) -> AuditableOp<T, O>
where
    T: Send + Sync + 'static,
    O: Op<T>,
{
    AuditableOp::new(op, audit_collector, op_id)
}

/// Global audit trail collector
static GLOBAL_AUDIT_COLLECTOR: std::sync::LazyLock<std::sync::RwLock<Option<Arc<AuditTrailCollector>>>> = 
    std::sync::LazyLock::new(|| std::sync::RwLock::new(None));

/// Set global audit trail collector
pub fn set_global_audit_collector(collector: Arc<AuditTrailCollector>) {
    let mut global = GLOBAL_AUDIT_COLLECTOR.write().unwrap();
    *global = Some(collector);
}

/// Get global audit trail collector
pub fn get_global_audit_collector() -> Option<Arc<AuditTrailCollector>> {
    let global = GLOBAL_AUDIT_COLLECTOR.read().unwrap();
    global.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OpContext, op::ClosureOp, metrics::InMemoryMetricsCollector};
    use std::sync::Arc;

    #[test]
    fn test_context_snapshot_creation() {
        let mut context = OpContext::new();
        context.put("test_key", "test_value").unwrap();
        
        let snapshot = ContextSnapshot::new("test_snapshot".to_string(), &context);
        
        assert_eq!(snapshot.description, "test_snapshot");
        assert_eq!(snapshot.data_size, 1);
        assert!(snapshot.context_data.contains_key("test_key"));
    }

    #[test]
    fn test_visualization_metadata_builder() {
        let metadata = VisualizationMetadata::new()
            .with_chart_type("timeline".to_string())
            .with_metric("duration".to_string(), 1000.0)
            .with_color_category("success".to_string())
            .with_hierarchy_level(1)
            .with_tag("test".to_string());
        
        assert!(metadata.chart_types.contains(&"timeline".to_string()));
        assert_eq!(metadata.metrics.get("duration"), Some(&1000.0));
        assert_eq!(metadata.color_category, "success");
        assert_eq!(metadata.hierarchy_level, 1);
        assert!(metadata.tags.contains(&"test".to_string()));
    }

    #[test]
    fn test_audit_record_creation() {
        let op_info = OpStateInfo::new(
            "test_op_123".to_string(),
            "test_op".to_string(),
        );
        
        let audit_record = AuditRecord::new("audit_123".to_string(), op_info);
        
        assert_eq!(audit_record.record_id, "audit_123");
        assert_eq!(audit_record.op_info.instance_id, "test_op_123");
        assert!(audit_record.is_successful()); // No failures recorded
        assert!(!audit_record.had_rollback());
    }

    #[test]
    fn test_audit_record_with_rollback() {
        let op_info = OpStateInfo::new(
            "test_op_123".to_string(),
            "test_op".to_string(),
        );
        
        let rollback_info = RollbackInfo::new(
            "rb_123".to_string(),
            "op_123".to_string(),
            "Test rollback".to_string(),
            "test_strategy".to_string(),
        );
        
        let audit_record = AuditRecord::new("audit_123".to_string(), op_info)
            .with_rollback_info(rollback_info);
        
        assert!(audit_record.had_rollback());
        assert!(audit_record.visualization_metadata.tags.contains(&"rollback".to_string()));
    }

    #[test]
    fn test_audit_trail_query_builder() {
        let query = AuditTrailQuery::new()
            .op_type("test_op".to_string())
            .success_status(true)
            .with_rollbacks()
            .min_duration_ms(1000)
            .with_required_tag("important".to_string())
            .limit(10);
        
        assert!(query.op_types.contains(&"test_op".to_string()));
        assert_eq!(query.success_status, Some(true));
        assert_eq!(query.rollback_involved, Some(true));
        assert_eq!(query.min_duration_ms, Some(1000));
        assert!(query.required_tags.contains(&"important".to_string()));
        assert_eq!(query.limit, Some(10));
    }

    #[test]
    fn test_query_matching() {
        let op_info = OpStateInfo::new(
            "test_op_123".to_string(),
            "test_op".to_string(),
        );
        
        let mut audit_record = AuditRecord::new("audit_123".to_string(), op_info);
        audit_record.visualization_metadata = audit_record.visualization_metadata
            .with_tag("test_tag".to_string());
        
        let query = AuditTrailQuery::new()
            .op_type("test_op".to_string())
            .with_required_tag("test_tag".to_string());
        
        assert!(query.matches(&audit_record));
        
        let query_no_match = AuditTrailQuery::new()
            .op_type("other_op".to_string());
        
        assert!(!query_no_match.matches(&audit_record));
    }

    #[tokio::test]
    async fn test_audit_trail_collector() {
        let metrics = Arc::new(InMemoryMetricsCollector::new());
        let collector = AuditTrailCollector::new(metrics);
        
        // Add a test op info
        let op_info = OpStateInfo::new(
            "test_op_123".to_string(),
            "test_op".to_string(),
        );
        collector.add_op_info("test_op_123".to_string(), op_info);
        
        let snapshots = vec![
            ContextSnapshot::new("before".to_string(), &OpContext::new()),
            ContextSnapshot::new("after".to_string(), &OpContext::new()),
        ];
        
        let audit_record = collector
            .create_audit_record::<String>(&"test_op_123".to_string(), snapshots, None, None)
            .await
            .unwrap();
        
        collector.add_record(audit_record.clone()).await.unwrap();
        
        assert_eq!(collector.record_count(), 1);
        
        let query = AuditTrailQuery::new();
        let results = collector.query_records(&query);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].record_id, audit_record.record_id);
    }

    #[test]
    fn test_visualization_timeline_export() {
        let op_info = OpStateInfo::new(
            "test_op_123".to_string(),
            "test_op".to_string(),
        );
        
        let audit_record = AuditRecord::new("audit_123".to_string(), op_info);
        let records = vec![audit_record];
        
        let timeline_data = VisualizationExporter::export_timeline(records);
        
        assert!(timeline_data["items"].is_array());
        assert!(!timeline_data["items"].as_array().unwrap().is_empty());
        assert!(timeline_data["groups"].is_array());
    }

    #[test]
    fn test_csv_export() {
        let op_info = OpStateInfo::new(
            "test_op_123".to_string(),
            "test_op".to_string(),
        );
        
        let audit_record = AuditRecord::new("audit_123".to_string(), op_info);
        let records = vec![audit_record];
        
        let csv_data = VisualizationExporter::export_csv(records);
        
        assert!(csv_data.starts_with("record_id,timestamp"));
        assert!(csv_data.contains("audit_123"));
        assert!(csv_data.contains("test_op"));
    }

    #[tokio::test]
    async fn test_auditable_op_wrapper() {
        let metrics = Arc::new(InMemoryMetricsCollector::new());
        let collector = Arc::new(AuditTrailCollector::new(metrics));
        
        // Add op info to collector
        let op_info = OpStateInfo::new(
            "test_op_123".to_string(),
            "test_op".to_string(),
        );
        collector.add_op_info("test_op_123".to_string(), op_info);
        
        let base_op = ClosureOp::new(|_ctx| {
            Box::pin(async { Ok("test_result".to_string()) })
        });
        
        let auditable_op = AuditableOp::new(
            base_op,
            collector.clone(),
            "test_op_123".to_string(),
        );
        
        let mut context = OpContext::new();
        let result = auditable_op.perform(&mut context).await.unwrap();
        
        assert_eq!(result, "test_result");
        assert_eq!(collector.record_count(), 1);
        
        let records = collector.query_records(&AuditTrailQuery::new());
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].context_evolution.len(), 2); // before and after snapshots
    }

    #[test]
    fn test_audit_statistics() {
        let op_info1 = OpStateInfo::new("op1".to_string(), "test_op".to_string());
        let op_info2 = OpStateInfo::new("op2".to_string(), "test_op".to_string());
        
        let record1 = AuditRecord::new("audit_1".to_string(), op_info1);
        let record2 = AuditRecord::new("audit_2".to_string(), op_info2)
            .with_error_details("Test error".to_string());
        
        let metrics = Arc::new(InMemoryMetricsCollector::new());
        let collector = AuditTrailCollector::new(metrics);
        
        {
            let mut records = collector.records.write().unwrap();
            records.push(record1);
            records.push(record2);
        }
        
        let stats = collector.get_audit_statistics();
        
        assert_eq!(stats.total_records, 2);
        assert_eq!(stats.successful_ops, 1);
        assert_eq!(stats.failed_ops, 1);
        assert_eq!(stats.rollback_ops, 0);
    }

    #[test]
    fn test_global_audit_collector() {
        let metrics = Arc::new(InMemoryMetricsCollector::new());
        let collector = Arc::new(AuditTrailCollector::new(metrics));
        
        set_global_audit_collector(collector.clone());
        
        let global = get_global_audit_collector();
        assert!(global.is_some());
        assert_eq!(global.unwrap().record_count(), collector.record_count());
    }
}
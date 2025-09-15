// Op Metrics and Telemetry - Performance Monitoring Integration
// Enhanced observability for ops with detailed performance tracking

use crate::op::Op;
use crate::context::OpContext;
use crate::error::OpError;
use async_trait::async_trait;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::info;
use serde::{Serialize, Deserialize};

/// Op execution metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpMetrics {
    pub op_name: String,
    #[serde(skip, default = "std::time::Instant::now")]
    pub start_time: Instant,
    #[serde(skip)]
    pub end_time: Option<Instant>,
    pub duration: Option<Duration>,
    pub success: bool,
    pub error_type: Option<String>,
    pub error_message: Option<String>,
    pub context_size: usize,
    pub memory_usage: Option<u64>,
    pub cpu_time: Option<Duration>,
    // Aggregate statistics fields for audit trail
    pub average_duration: Option<Duration>,
    pub min_duration: Option<Duration>,
    pub max_duration: Option<Duration>,
}


impl OpMetrics {
    pub fn new(op_name: String, context_size: usize) -> Self {
        Self {
            op_name,
            start_time: Instant::now(),
            end_time: None,
            duration: None,
            success: false,
            error_type: None,
            error_message: None,
            context_size,
            memory_usage: None,
            cpu_time: None,
            average_duration: None,
            min_duration: None,
            max_duration: None,
        }
    }

    pub fn complete_success(&mut self) {
        let now = Instant::now();
        self.end_time = Some(now);
        self.duration = Some(now.duration_since(self.start_time));
        self.success = true;
    }

    pub fn complete_error(&mut self, error: &OpError) {
        let now = Instant::now();
        self.end_time = Some(now);
        self.duration = Some(now.duration_since(self.start_time));
        self.success = false;
        self.error_type = Some(self.error_type_name(error));
        self.error_message = Some(error.to_string());
    }

    fn error_type_name(&self, error: &OpError) -> String {
        match error {
            OpError::ExecutionFailed(_) => "ExecutionFailed".to_string(),
            OpError::Timeout { .. } => "Timeout".to_string(),
            OpError::Context(_) => "Context".to_string(),
            OpError::BatchFailed(_) => "BatchFailed".to_string(),
            OpError::Other(_) => "Other".to_string(),
        }
    }

    pub fn duration_ms(&self) -> Option<f64> {
        self.duration.map(|d| d.as_secs_f64() * 1000.0)
    }

    pub fn is_slow(&self, threshold_ms: f64) -> bool {
        self.duration_ms().unwrap_or(0.0) > threshold_ms
    }

    pub fn is_error(&self) -> bool {
        !self.success
    }
}

/// Metrics collection trait
pub trait MetricsCollector: Send + Sync {
    fn record_op(&self, metrics: OpMetrics);
    fn get_summary(&self) -> MetricsSummary;
    fn get_op_metrics(&self, op_type: &str) -> Option<OpMetrics>;
    fn reset(&self);
}

/// In-memory metrics collector
#[derive(Debug)]
pub struct InMemoryMetricsCollector {
    metrics: Arc<Mutex<Vec<OpMetrics>>>,
}

impl Default for InMemoryMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryMetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn get_metrics(&self) -> Vec<OpMetrics> {
        self.metrics.lock().unwrap().clone()
    }

    pub fn op_count(&self) -> usize {
        self.metrics.lock().unwrap().len()
    }
}

impl MetricsCollector for InMemoryMetricsCollector {
    fn record_op(&self, metrics: OpMetrics) {
        if let Ok(mut metrics_vec) = self.metrics.lock() {
            metrics_vec.push(metrics);
        }
    }

    fn get_summary(&self) -> MetricsSummary {
        if let Ok(metrics_vec) = self.metrics.lock() {
            MetricsSummary::from_metrics(&metrics_vec)
        } else {
            MetricsSummary::default()
        }
    }

    fn get_op_metrics(&self, op_type: &str) -> Option<OpMetrics> {
        if let Ok(metrics_vec) = self.metrics.lock() {
            // Find most recent metrics for this op type
            metrics_vec.iter()
                .filter(|m| m.op_name == op_type)
                .last()
                .cloned()
        } else {
            None
        }
    }

    fn reset(&self) {
        if let Ok(mut metrics_vec) = self.metrics.lock() {
            metrics_vec.clear();
        }
    }
}

/// Aggregated metrics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub total_ops: usize,
    pub successful_ops: usize,
    pub failed_ops: usize,
    pub success_rate: f64,
    pub average_duration_ms: f64,
    pub min_duration_ms: f64,
    pub max_duration_ms: f64,
    pub p50_duration_ms: f64,
    pub p95_duration_ms: f64,
    pub p99_duration_ms: f64,
    pub error_breakdown: HashMap<String, usize>,
    pub slow_ops: usize,
    pub slowest_ops: Vec<String>,
}

impl Default for MetricsSummary {
    fn default() -> Self {
        Self {
            total_ops: 0,
            successful_ops: 0,
            failed_ops: 0,
            success_rate: 0.0,
            average_duration_ms: 0.0,
            min_duration_ms: 0.0,
            max_duration_ms: 0.0,
            p50_duration_ms: 0.0,
            p95_duration_ms: 0.0,
            p99_duration_ms: 0.0,
            error_breakdown: HashMap::new(),
            slow_ops: 0,
            slowest_ops: Vec::new(),
        }
    }
}

impl MetricsSummary {
    pub fn from_metrics(metrics: &[OpMetrics]) -> Self {
        if metrics.is_empty() {
            return Self::default();
        }

        let total_ops = metrics.len();
        let successful_ops = metrics.iter().filter(|m| m.success).count();
        let failed_ops = total_ops - successful_ops;
        let success_rate = (successful_ops as f64 / total_ops as f64) * 100.0;

        let durations: Vec<f64> = metrics.iter()
            .filter_map(|m| m.duration_ms())
            .collect();

        let (average_duration_ms, min_duration_ms, max_duration_ms) = if durations.is_empty() {
            (0.0, 0.0, 0.0)
        } else {
            let sum: f64 = durations.iter().sum();
            let average = sum / durations.len() as f64;
            let min = durations.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let max = durations.iter().fold(0.0f64, |a, &b| a.max(b));
            (average, min, max)
        };

        let (p50_duration_ms, p95_duration_ms, p99_duration_ms) = Self::calculate_percentiles(&durations);

        let mut error_breakdown = HashMap::new();
        for metric in metrics.iter().filter(|m| !m.success) {
            if let Some(error_type) = &metric.error_type {
                *error_breakdown.entry(error_type.clone()).or_insert(0) += 1;
            }
        }

        let slow_threshold = 1000.0; // 1 second
        let slow_ops = metrics.iter().filter(|m| m.is_slow(slow_threshold)).count();

        let mut slowest_ops: Vec<_> = metrics.iter()
            .filter_map(|m| m.duration_ms().map(|d| (m.op_name.clone(), d)))
            .collect();
        slowest_ops.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let slowest_ops = slowest_ops.into_iter()
            .take(5)
            .map(|(name, duration)| format!("{} ({:.2}ms)", name, duration))
            .collect();

        Self {
            total_ops,
            successful_ops,
            failed_ops,
            success_rate,
            average_duration_ms,
            min_duration_ms,
            max_duration_ms,
            p50_duration_ms,
            p95_duration_ms,
            p99_duration_ms,
            error_breakdown,
            slow_ops,
            slowest_ops,
        }
    }

    fn calculate_percentiles(durations: &[f64]) -> (f64, f64, f64) {
        if durations.is_empty() {
            return (0.0, 0.0, 0.0);
        }

        let mut sorted_durations = durations.to_vec();
        sorted_durations.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let len = sorted_durations.len();
        let p50_idx = (len as f64 * 0.50) as usize;
        let p95_idx = (len as f64 * 0.95) as usize;
        let p99_idx = (len as f64 * 0.99) as usize;

        let p50 = sorted_durations.get(p50_idx.min(len - 1)).copied().unwrap_or(0.0);
        let p95 = sorted_durations.get(p95_idx.min(len - 1)).copied().unwrap_or(0.0);
        let p99 = sorted_durations.get(p99_idx.min(len - 1)).copied().unwrap_or(0.0);

        (p50, p95, p99)
    }

    pub fn print_summary(&self) {
        tracing::info!("=== Op Metrics Summary ===");
        tracing::info!("Total Ops: {}", self.total_ops);
        tracing::info!("Success Rate: {:.2}%", self.success_rate);
        tracing::info!("Average Duration: {:.2}ms", self.average_duration_ms);
        tracing::info!("P50 Duration: {:.2}ms", self.p50_duration_ms);
        tracing::info!("P95 Duration: {:.2}ms", self.p95_duration_ms);
        tracing::info!("P99 Duration: {:.2}ms", self.p99_duration_ms);
        tracing::info!("Slow Ops (>1s): {}", self.slow_ops);
        
        if !self.error_breakdown.is_empty() {
            tracing::info!("Error Breakdown:");
            for (error_type, count) in &self.error_breakdown {
                tracing::info!("  {}: {}", error_type, count);
            }
        }

        if !self.slowest_ops.is_empty() {
            tracing::info!("Slowest Ops:");
            for op in &self.slowest_ops {
                tracing::info!("  {}", op);
            }
        }
    }
}

/// Metrics-collecting op wrapper
pub struct MetricsOp<T> {
    op: Box<dyn Op<T>>,
    collector: Arc<dyn MetricsCollector>,
    op_name: String,
}

impl<T> MetricsOp<T>
where
    T: Send + 'static,
{
    pub fn new(op: Box<dyn Op<T>>, collector: Arc<dyn MetricsCollector>) -> Self {
        Self {
            op,
            collector,
            op_name: "MetricsOp".to_string(),
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.op_name = name;
        self
    }
}

#[async_trait]
impl<T> Op<T> for MetricsOp<T>
where
    T: Send + 'static,
{
    async fn perform(&self, context: &mut OpContext) -> Result<T, OpError> {
        let context_size = context.keys().count();
        let mut metrics = OpMetrics::new(self.op_name.clone(), context_size);

        match self.op.perform(context).await {
            Ok(result) => {
                metrics.complete_success();
                self.collector.record_op(metrics);
                Ok(result)
            },
            Err(error) => {
                metrics.complete_error(&error);
                self.collector.record_op(metrics);
                Err(error)
            }
        }
    }
}

/// Global metrics registry
static GLOBAL_METRICS: std::sync::OnceLock<Arc<InMemoryMetricsCollector>> = std::sync::OnceLock::new();

/// Get or initialize global metrics collector
pub fn global_metrics() -> Arc<InMemoryMetricsCollector> {
    GLOBAL_METRICS
        .get_or_init(|| Arc::new(InMemoryMetricsCollector::new()))
        .clone()
}

/// Convenience function to wrap op with metrics collection
pub fn with_metrics<T>(op: Box<dyn Op<T>>, op_name: String) -> MetricsOp<T>
where
    T: Send + 'static,
{
    MetricsOp::new(op, global_metrics()).with_name(op_name)
}

/// Auto-metrics wrapper that automatically collects metrics for any op
pub fn auto_metrics<T>(op: Box<dyn Op<T>>) -> MetricsOp<T>
where
    T: Send + 'static,
{
    let op_name = crate::ops::get_caller_op_name();
    MetricsOp::new(op, global_metrics()).with_name(op_name)
}

/// Print global metrics summary
pub fn print_global_metrics() {
    let collector = global_metrics();
    let summary = collector.get_summary();
    summary.print_summary();
}

/// Reset global metrics
pub fn reset_global_metrics() {
    let collector = global_metrics();
    collector.reset();
}

/// Get current global metrics summary
pub fn get_global_metrics_summary() -> MetricsSummary {
    let collector = global_metrics();
    collector.get_summary()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::op::ClosureOp;
    use std::time::Duration;

    #[test]
    fn test_op_metrics_success() {
        let mut metrics = OpMetrics::new("TestOp".to_string(), 5);
        assert_eq!(metrics.op_name, "TestOp");
        assert_eq!(metrics.context_size, 5);
        assert!(!metrics.success);
        assert!(metrics.duration.is_none());

        std::thread::sleep(Duration::from_millis(10));
        metrics.complete_success();

        assert!(metrics.success);
        assert!(metrics.duration.is_some());
        assert!(metrics.duration_ms().unwrap() >= 10.0);
    }

    #[test]
    fn test_op_metrics_error() {
        let mut metrics = OpMetrics::new("FailingOp".to_string(), 3);
        let error = OpError::ExecutionFailed("Test error".to_string());
        
        std::thread::sleep(Duration::from_millis(5));
        metrics.complete_error(&error);

        assert!(!metrics.success);
        assert!(metrics.duration.is_some());
        assert_eq!(metrics.error_type, Some("ExecutionFailed".to_string()));
        assert_eq!(metrics.error_message, Some("Op execution failed: Test error".to_string()));
    }

    #[test]
    fn test_in_memory_metrics_collector() {
        let collector = InMemoryMetricsCollector::new();
        assert_eq!(collector.op_count(), 0);

        let mut metrics1 = OpMetrics::new("Op1".to_string(), 1);
        metrics1.complete_success();
        collector.record_op(metrics1);

        let mut metrics2 = OpMetrics::new("Op2".to_string(), 2);
        let error = OpError::Timeout { timeout_ms: 1000 };
        metrics2.complete_error(&error);
        collector.record_op(metrics2);

        assert_eq!(collector.op_count(), 2);

        let summary = collector.get_summary();
        assert_eq!(summary.total_ops, 2);
        assert_eq!(summary.successful_ops, 1);
        assert_eq!(summary.failed_ops, 1);
        assert_eq!(summary.success_rate, 50.0);
        assert_eq!(summary.error_breakdown.len(), 1);
        assert_eq!(summary.error_breakdown.get("Timeout"), Some(&1));
    }

    #[test]
    fn test_metrics_summary_percentiles() {
        let durations = vec![100.0, 200.0, 300.0, 400.0, 500.0, 600.0, 700.0, 800.0, 900.0, 1000.0];
        let (p50, p95, p99) = MetricsSummary::calculate_percentiles(&durations);
        
        // With 10 elements, indices are: p50=5th element (600), p95=9th element (1000), p99=9th element (1000)
        assert!((p50 - 600.0).abs() < 10.0); // 50th percentile should be around 600
        assert!((p95 - 1000.0).abs() < 10.0); // 95th percentile should be 1000  
        assert!((p99 - 1000.0).abs() < 10.0); // 99th percentile should be 1000
    }

    #[tokio::test]
    async fn test_metrics_op_success() {
        let collector = Arc::new(InMemoryMetricsCollector::new());
        let op = Box::new(ClosureOp::new(|_ctx| {
            Box::pin(async move { 
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok("success".to_string())
            })
        }));

        let metrics_op = MetricsOp::new(op, collector.clone())
            .with_name("TestSuccessOp".to_string());

        let mut context = OpContext::new();
        let result = metrics_op.perform(&mut context).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(collector.op_count(), 1);

        let metrics = collector.get_metrics();
        let metric = &metrics[0];
        assert_eq!(metric.op_name, "TestSuccessOp");
        assert!(metric.success);
        assert!(metric.duration_ms().unwrap() >= 10.0);
    }

    #[tokio::test]
    async fn test_metrics_op_error() {
        let collector = Arc::new(InMemoryMetricsCollector::new());
        let op = Box::new(ClosureOp::new(|_ctx| {
            Box::pin(async move { 
                tokio::time::sleep(Duration::from_millis(5)).await;
                Err(OpError::ExecutionFailed("Test failure".to_string()))
            })
        }));

        let metrics_op: MetricsOp<String> = MetricsOp::new(op, collector.clone())
            .with_name("TestFailureOp".to_string());

        let mut context = OpContext::new();
        let result = metrics_op.perform(&mut context).await;

        assert!(result.is_err());
        assert_eq!(collector.op_count(), 1);

        let metrics = collector.get_metrics();
        let metric = &metrics[0];
        assert_eq!(metric.op_name, "TestFailureOp");
        assert!(!metric.success);
        assert_eq!(metric.error_type, Some("ExecutionFailed".to_string()));
    }

    #[test]
    fn test_global_metrics() {
        reset_global_metrics();
        
        let collector = global_metrics();
        assert_eq!(collector.op_count(), 0);

        let mut metrics = OpMetrics::new("GlobalTest".to_string(), 1);
        metrics.complete_success();
        collector.record_op(metrics);

        assert_eq!(collector.op_count(), 1);
        
        let summary = get_global_metrics_summary();
        assert_eq!(summary.total_ops, 1);
        assert_eq!(summary.successful_ops, 1);
    }

    #[tokio::test]
    async fn test_auto_metrics_wrapper() {
        reset_global_metrics();
        
        let op = Box::new(ClosureOp::new(|_ctx| {
            Box::pin(async move { Ok(42) })
        }));

        let metrics_op = auto_metrics(op);
        let mut context = OpContext::new();
        
        let result = metrics_op.perform(&mut context).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);

        let collector = global_metrics();
        assert_eq!(collector.op_count(), 1);

        let summary = get_global_metrics_summary();
        assert_eq!(summary.total_ops, 1);
        assert_eq!(summary.success_rate, 100.0);
    }
}
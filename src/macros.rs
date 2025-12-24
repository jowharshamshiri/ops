use crate::{DryContext, WetContext, Op, OpResult};
use async_trait::async_trait;

/// Store a variable in dry context using its name as the key
/// dry_put!(dry_context, variable_name)
#[macro_export]
macro_rules! dry_put {
    ($ctx:expr, $var:ident) => {
        $ctx.insert(stringify!($var), $var)
    };
    ($ctx:expr, $var:ident, $value:expr) => {
        $ctx.insert(stringify!($var), $value)
    };
}

/// Store a value in dry context using a custom key name
/// dry_put_key!(dry_context, "custom_key", value)
#[macro_export]
macro_rules! dry_put_key {
    ($ctx:expr, $key:expr, $value:expr) => {
        $ctx.insert($key, $value)
    };
}

/// Retrieve a value from dry context using variable name as key
/// let var: Type = dry_get!(dry_context, var_name);
#[macro_export]
macro_rules! dry_get {
    ($ctx:expr, $var:ident) => {
        $ctx.get::<_>(stringify!($var))
    };
}

/// Retrieve a value from dry context using a custom key name
/// let var: Option<Type> = dry_get_key!(dry_context, "custom_key");
#[macro_export]
macro_rules! dry_get_key {
    ($ctx:expr, $key:expr) => {
        $ctx.get::<_>($key)
    };
}

/// Retrieve a required value from dry context, return error if missing
/// let var: Type = dry_require!(dry_context, var_name)?;
#[macro_export]
macro_rules! dry_require {
    ($ctx:expr, $var:ident) => {
        $ctx.get_required::<_>(stringify!($var))
    };
}

/// Retrieve a required value from dry context using a custom key name
/// let var: Type = dry_require_key!(dry_context, "custom_key")?;
#[macro_export]
macro_rules! dry_require_key {
    ($ctx:expr, $key:expr) => {
        $ctx.get_required::<_>($key)
    };
}

/// Store result in dry context under both the op name and "result" key
/// dry_result!(dry_context, "OpName", result_value);
#[macro_export]
macro_rules! dry_result {
    ($ctx:expr, $trigger_name:expr, $result:expr) => {
        {
            $ctx.insert($trigger_name, $result.clone());
            $ctx.insert("result", $result);
        }
    };
}

/// Store a reference in wet context without serialization
/// wet_put_ref!(wet_context, var_name, value);
#[macro_export]
macro_rules! wet_put_ref {
    ($ctx:expr, $var:ident, $value:expr) => {
        $ctx.insert_ref(stringify!($var), $value)
    };
    ($ctx:expr, $var:ident) => {
        $ctx.insert_ref(stringify!($var), $var)
    };
}

/// Store a reference in wet context using a custom key name
/// wet_put_ref_key!(wet_context, "custom_key", value);
#[macro_export]
macro_rules! wet_put_ref_key {
    ($ctx:expr, $key:expr, $value:expr) => {
        $ctx.insert_ref($key, $value)
    };
}

/// Store an Arc in wet context without additional wrapping
/// wet_put_arc!(wet_context, var_name, arc_value);
#[macro_export]
macro_rules! wet_put_arc {
    ($ctx:expr, $var:ident, $value:expr) => {
        $ctx.insert_arc(stringify!($var), $value)
    };
    ($ctx:expr, $var:ident) => {
        $ctx.insert_arc(stringify!($var), $var)
    };
}

/// Store an Arc in wet context using a custom key name
/// wet_put_arc_key!(wet_context, "custom_key", arc_value);
#[macro_export]
macro_rules! wet_put_arc_key {
    ($ctx:expr, $key:expr, $value:expr) => {
        $ctx.insert_arc($key, $value)
    };
}

/// Get a reference from wet context
/// let var: Option<Arc<Type>> = wet_get_ref!(wet_context, var_name);
#[macro_export]
macro_rules! wet_get_ref {
    ($ctx:expr, $var:ident) => {
        $ctx.get_ref::<_>(stringify!($var))
    };
}

/// Get a reference from wet context using a custom key name
/// let var: Option<Arc<Type>> = wet_get_ref_key!(wet_context, "custom_key");
#[macro_export]
macro_rules! wet_get_ref_key {
    ($ctx:expr, $key:expr) => {
        $ctx.get_ref::<_>($key)
    };
}

/// Require a reference from wet context with error handling
/// let var: Arc<Type> = wet_require_ref!(wet_context, var_name)?;
#[macro_export]
macro_rules! wet_require_ref {
    ($ctx:expr, $var:ident) => {
        $ctx.get_required::<_>(stringify!($var))
    };
}

/// Require a reference from wet context using a custom key name
/// let var: Arc<Type> = wet_require_ref_key!(wet_context, "custom_key")?;
#[macro_export]
macro_rules! wet_require_ref_key {
    ($ctx:expr, $key:expr) => {
        $ctx.get_required::<_>($key)
    };
}

/// Aggregation strategies for composable ops
#[derive(Debug, Clone)]
pub enum AggregationStrategy {
    /// Return all results as Vec<T>
    All,
    /// Return only the last result
    Last,
    /// Return only the first result
    First,
    /// Return the merged result (for types that support merging)
    Merge,
    /// Return unit () - ignore all results
    Unit,
}

/// Create a named batch op type with flexible return types and aggregation
/// Usage:
/// ```rust
/// // Returns Vec<T> (default behavior)
/// batch! {
///     ProcessingPipeline<String> = [
///         ValidationOp::new(),
///         TransformOp::new()
///     ]
/// }
/// 
/// // Returns T using last result
/// batch! {
///     ProcessingPipeline<String> -> last = [
///         ValidationOp::new(),
///         TransformOp::new()
///     ]
/// }
/// 
/// // Returns () ignoring all results
/// batch! {
///     ProcessingPipeline<()> -> unit = [
///         SideEffectOp::new(),
///         LoggingOp::new()
///     ]
/// }
/// ```
#[macro_export]
macro_rules! batch {
    // Default behavior: returns Vec<T>
    ($name:ident<$T:ty> = [$($op:expr),+ $(,)?]) => {
        batch!($name<$T> -> all = [$($op),+]);
    };
    
    // Flexible aggregation strategy
    ($name:ident<$T:ty> -> $strategy:ident = [$($op:expr),+ $(,)?]) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            batch: $crate::BatchOp<$T>,
            strategy: $crate::macros::AggregationStrategy,
        }
        
        impl $name {
            pub fn new() -> Self {
                let ops: Vec<std::sync::Arc<dyn $crate::Op<$T>>> = vec![
                    $(std::sync::Arc::new($op)),+
                ];
                Self {
                    batch: $crate::BatchOp::new(ops),
                    strategy: batch!(@strategy $strategy),
                }
            }
            
            pub fn with_continue_on_error(mut self, continue_on_error: bool) -> Self {
                self.batch = self.batch.with_continue_on_error(continue_on_error);
                self
            }
        }
        
        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
        
        batch!(@impl $name<$T> -> $strategy);
    };
    
    // Strategy selection helper
    (@strategy all) => { $crate::macros::AggregationStrategy::All };
    (@strategy last) => { $crate::macros::AggregationStrategy::Last };
    (@strategy first) => { $crate::macros::AggregationStrategy::First };
    (@strategy merge) => { $crate::macros::AggregationStrategy::Merge };
    (@strategy unit) => { $crate::macros::AggregationStrategy::Unit };
    
    // Implementation for Vec<T> return (all strategy)
    (@impl $name:ident<$T:ty> -> all) => {
        #[async_trait::async_trait]
        impl $crate::Op<Vec<$T>> for $name {
            async fn perform(&self, dry: &mut $crate::DryContext, wet: &mut $crate::WetContext) -> $crate::OpResult<Vec<$T>> {
                self.batch.perform(dry, wet).await
            }
            
            fn metadata(&self) -> $crate::OpMetadata {
                self.batch.metadata()
            }
        }
    };
    
    // Implementation for T return (last strategy)
    (@impl $name:ident<$T:ty> -> last) => {
        #[async_trait::async_trait]
        impl $crate::Op<$T> for $name {
            async fn perform(&self, dry: &mut $crate::DryContext, wet: &mut $crate::WetContext) -> $crate::OpResult<$T> {
                let results = self.batch.perform(dry, wet).await?;
                results.into_iter().last()
                    .ok_or_else(|| $crate::OpError::ExecutionFailed("No results from batch operation".to_string()))
            }
            
            fn metadata(&self) -> $crate::OpMetadata {
                self.batch.metadata()
            }
        }
    };
    
    // Implementation for T return (first strategy)
    (@impl $name:ident<$T:ty> -> first) => {
        #[async_trait::async_trait]
        impl $crate::Op<$T> for $name {
            async fn perform(&self, dry: &mut $crate::DryContext, wet: &mut $crate::WetContext) -> $crate::OpResult<$T> {
                let results = self.batch.perform(dry, wet).await?;
                results.into_iter().next()
                    .ok_or_else(|| $crate::OpError::ExecutionFailed("No results from batch operation".to_string()))
            }
            
            fn metadata(&self) -> $crate::OpMetadata {
                self.batch.metadata()
            }
        }
    };
    
    // Implementation for () return (unit strategy)
    (@impl $name:ident<$T:ty> -> unit) => {
        #[async_trait::async_trait]
        impl $crate::Op<()> for $name {
            async fn perform(&self, dry: &mut $crate::DryContext, wet: &mut $crate::WetContext) -> $crate::OpResult<()> {
                let _results = self.batch.perform(dry, wet).await?;
                Ok(())
            }
            
            fn metadata(&self) -> $crate::OpMetadata {
                self.batch.metadata()
            }
        }
    };
    
    // Implementation for merge strategy (requires Default + Clone)
    (@impl $name:ident<$T:ty> -> merge) => {
        #[async_trait::async_trait]
        impl $crate::Op<$T> for $name 
        where 
            $T: Default + Clone
        {
            async fn perform(&self, dry: &mut $crate::DryContext, wet: &mut $crate::WetContext) -> $crate::OpResult<$T> {
                let results = self.batch.perform(dry, wet).await?;
                // Simple merge strategy - for more complex merging, implement custom logic
                Ok(results.into_iter().last().unwrap_or_default())
            }
            
            fn metadata(&self) -> $crate::OpMetadata {
                self.batch.metadata()
            }
        }
    };
}

/// Create a named for-loop op type with flexible return types
/// The limit is loaded from a variable in the dry context
/// Usage:
/// ```rust
/// // Returns Vec<T> (default)
/// repeat! {
///     ProcessLoop<String> = {
///         counter: "iteration",
///         limit: "max_iterations",
///         ops: [StepOp::new(), LogOp::new()]
///     }
/// }
/// 
/// // Returns T using last result
/// repeat! {
///     ProcessLoop<String> -> last = {
///         counter: "iteration",
///         limit: "max_iterations",
///         ops: [StepOp::new(), LogOp::new()]
///     }
/// }
/// ```
#[macro_export]
macro_rules! repeat {
    // Default behavior: returns Vec<T>
    ($name:ident<$T:ty> = {
        counter: $counter:expr,
        limit: $limit_var:expr,
        ops: [$($op:expr),+ $(,)?]
    }) => {
        repeat!($name<$T> -> all = {
            counter: $counter,
            limit: $limit_var,
            ops: [$($op),+]
        });
    };
    
    // With continue_on_error option
    ($name:ident<$T:ty> = {
        counter: $counter:expr,
        limit: $limit_var:expr,
        continue_on_error: $continue_on_error:expr,
        ops: [$($op:expr),+ $(,)?]
    }) => {
        repeat!($name<$T> -> all = {
            counter: $counter,
            limit: $limit_var,
            continue_on_error: $continue_on_error,
            ops: [$($op),+]
        });
    };
    
    // Flexible aggregation strategy
    ($name:ident<$T:ty> -> $strategy:ident = {
        counter: $counter:expr,
        limit: $limit_var:expr,
        ops: [$($op:expr),+ $(,)?]
    }) => {
        repeat!($name<$T> -> $strategy = {
            counter: $counter,
            limit: $limit_var,
            continue_on_error: false,
            ops: [$($op),+]
        });
    };
    
    // Flexible aggregation strategy with continue_on_error
    ($name:ident<$T:ty> -> $strategy:ident = {
        counter: $counter:expr,
        limit: $limit_var:expr,
        continue_on_error: $continue_on_error:expr,
        ops: [$($op:expr),+ $(,)?]
    }) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            counter_var: String,
            limit_var: String,
            continue_on_error: bool,
            strategy: $crate::macros::AggregationStrategy,
        }
        
        impl $name {
            pub fn new() -> Self {
                Self {
                    counter_var: $counter.to_string(),
                    limit_var: $limit_var.to_string(),
                    continue_on_error: $continue_on_error,
                    strategy: repeat!(@strategy $strategy),
                }
            }
            
            fn create_ops() -> Vec<std::sync::Arc<dyn $crate::Op<$T>>> {
                vec![
                    $(std::sync::Arc::new($op)),+
                ]
            }
        }
        
        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
        
        repeat!(@impl $name<$T> -> $strategy);
        
        impl $name {
            fn build_metadata(&self) -> $crate::OpMetadata {
                let limit_var = self.limit_var.clone();
                let mut properties = serde_json::Map::new();
                properties.insert(
                    limit_var.clone(),
                    serde_json::json!({ "type": "integer", "minimum": 0 })
                );
                
                $crate::OpMetadata::builder(stringify!($name))
                    .description(format!("For-loop with dynamic limit from '{}'", limit_var))
                    .input_schema(serde_json::json!({
                        "type": "object",
                        "properties": properties,
                        "required": [limit_var]
                    }))
                    .build()
            }
        }
    };
    
    // Strategy selection helper
    (@strategy all) => { $crate::macros::AggregationStrategy::All };
    (@strategy last) => { $crate::macros::AggregationStrategy::Last };
    (@strategy first) => { $crate::macros::AggregationStrategy::First };
    (@strategy merge) => { $crate::macros::AggregationStrategy::Merge };
    (@strategy unit) => { $crate::macros::AggregationStrategy::Unit };
    
    // Implementation for Vec<T> return (all strategy)
    (@impl $name:ident<$T:ty> -> all) => {
        #[async_trait::async_trait]
        impl $crate::Op<Vec<$T>> for $name {
            async fn perform(&self, dry: &mut $crate::DryContext, wet: &mut $crate::WetContext) -> $crate::OpResult<Vec<$T>> {
                let limit = dry.get_required::<usize>(&self.limit_var)?;
                let ops = Self::create_ops();
                let loop_op = $crate::loop_op::LoopOp::new(
                    self.counter_var.clone(),
                    limit,
                    ops
                ).with_continue_on_error(self.continue_on_error);
                loop_op.perform(dry, wet).await
            }
            
            fn metadata(&self) -> $crate::OpMetadata {
                self.build_metadata()
            }
        }
    };
    
    // Implementation for T return (last strategy)
    (@impl $name:ident<$T:ty> -> last) => {
        #[async_trait::async_trait]
        impl $crate::Op<$T> for $name {
            async fn perform(&self, dry: &mut $crate::DryContext, wet: &mut $crate::WetContext) -> $crate::OpResult<$T> {
                let limit = dry.get_required::<usize>(&self.limit_var)?;
                let ops = Self::create_ops();
                let loop_op = $crate::loop_op::LoopOp::new(
                    self.counter_var.clone(),
                    limit,
                    ops
                ).with_continue_on_error(self.continue_on_error);
                let results = loop_op.perform(dry, wet).await?;
                results.into_iter().last()
                    .ok_or_else(|| $crate::OpError::ExecutionFailed("No results from repeat operation".to_string()))
            }
            
            fn metadata(&self) -> $crate::OpMetadata {
                self.build_metadata()
            }
        }
    };
    
    // Implementation for T return (first strategy)
    (@impl $name:ident<$T:ty> -> first) => {
        #[async_trait::async_trait]
        impl $crate::Op<$T> for $name {
            async fn perform(&self, dry: &mut $crate::DryContext, wet: &mut $crate::WetContext) -> $crate::OpResult<$T> {
                let limit = dry.get_required::<usize>(&self.limit_var)?;
                let ops = Self::create_ops();
                let loop_op = $crate::loop_op::LoopOp::new(
                    self.counter_var.clone(),
                    limit,
                    ops
                ).with_continue_on_error(self.continue_on_error);
                let results = loop_op.perform(dry, wet).await?;
                results.into_iter().next()
                    .ok_or_else(|| $crate::OpError::ExecutionFailed("No results from repeat operation".to_string()))
            }
            
            fn metadata(&self) -> $crate::OpMetadata {
                self.build_metadata()
            }
        }
    };
    
    // Implementation for () return (unit strategy)
    (@impl $name:ident<$T:ty> -> unit) => {
        #[async_trait::async_trait]
        impl $crate::Op<()> for $name {
            async fn perform(&self, dry: &mut $crate::DryContext, wet: &mut $crate::WetContext) -> $crate::OpResult<()> {
                let limit = dry.get_required::<usize>(&self.limit_var)?;
                let ops = Self::create_ops();
                let loop_op = $crate::loop_op::LoopOp::new(
                    self.counter_var.clone(),
                    limit,
                    ops
                ).with_continue_on_error(self.continue_on_error);
                let _results = loop_op.perform(dry, wet).await?;
                Ok(())
            }
            
            fn metadata(&self) -> $crate::OpMetadata {
                self.build_metadata()
            }
        }
    };
    
    // Implementation for merge strategy
    (@impl $name:ident<$T:ty> -> merge) => {
        #[async_trait::async_trait]
        impl $crate::Op<$T> for $name 
        where 
            $T: Default + Clone
        {
            async fn perform(&self, dry: &mut $crate::DryContext, wet: &mut $crate::WetContext) -> $crate::OpResult<$T> {
                let limit = dry.get_required::<usize>(&self.limit_var)?;
                let ops = Self::create_ops();
                let loop_op = $crate::loop_op::LoopOp::new(
                    self.counter_var.clone(),
                    limit,
                    ops
                ).with_continue_on_error(self.continue_on_error);
                let results = loop_op.perform(dry, wet).await?;
                Ok(results.into_iter().last().unwrap_or_default())
            }
            
            fn metadata(&self) -> $crate::OpMetadata {
                self.build_metadata()
            }
        }
    };
}

/// Create a named while-loop op type with condition-based iteration
/// The condition is checked from a boolean variable in the dry context
/// Internal control flags use unique UUIDs to prevent conflicts in nested loops
/// Usage:
/// ```rust
/// // Returns Vec<T> (default)
/// repeat_until! {
///     ProcessUntilDone<String> = {
///         counter: "iteration",
///         condition: "should_continue",  // Boolean variable in dry context
///         max_iterations: 100,           // Safety limit
///         ops: [ProcessOp::new(), CheckOp::new()]
///     }
/// }
/// 
/// // Returns T using last result
/// repeat_until! {
///     ProcessUntilDone<String> -> last = {
///         counter: "iteration",
///         condition: "should_continue",
///         max_iterations: 100,
///         ops: [ProcessOp::new(), CheckOp::new()]
///     }
/// }
/// ```
#[macro_export]
macro_rules! repeat_until {
    // Default behavior: returns Vec<T>
    ($name:ident<$T:ty> = {
        counter: $counter:expr,
        condition: $condition_var:expr,
        max_iterations: $max:expr,
        ops: [$($op:expr),+ $(,)?]
    }) => {
        repeat_until!($name<$T> -> all = {
            counter: $counter,
            condition: $condition_var,
            max_iterations: $max,
            ops: [$($op),+]
        });
    };
    
    // Flexible aggregation strategy
    ($name:ident<$T:ty> -> $strategy:ident = {
        counter: $counter:expr,
        condition: $condition_var:expr,
        max_iterations: $max:expr,
        ops: [$($op:expr),+ $(,)?]
    }) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            counter_var: String,
            condition_var: String,
            max_iterations: usize,
            strategy: $crate::macros::AggregationStrategy,
            // Unique control variables to prevent conflicts in nested loops
            continue_var: String,
            break_var: String,
            loop_id: String,
        }
        
        impl $name {
            pub fn new() -> Self {
                // Generate a unique loop ID for scoped control flow variables
				// Keep the preceding double underscores to minimize risk of collisions
				let loop_id = ::uuid::Uuid::new_v4().to_string();
                Self {
                    counter_var: $counter.to_string(),
                    condition_var: $condition_var.to_string(),
                    max_iterations: $max,
                    strategy: repeat_until!(@strategy $strategy),
                    continue_var: format!("__continue_loop_{}", loop_id),
                    break_var: format!("__break_loop_{}", loop_id),
                    loop_id,
                }
            }
            
            fn create_ops() -> Vec<std::sync::Arc<dyn $crate::Op<$T>>> {
                vec![
                    $(std::sync::Arc::new($op)),+
                ]
            }
            
            async fn perform_repeat_until(&self, dry: &mut $crate::DryContext, wet: &mut $crate::WetContext) -> $crate::OpResult<Vec<$T>> {
                let mut results = Vec::new();
                let mut iteration = 0;
                
                // Set loop context for nested control flow
                dry.insert("__current_loop_id", &self.loop_id);
                
                while iteration < self.max_iterations {
                    // Check condition from context
                    let should_continue = dry.get::<bool>(&self.condition_var)
                        .unwrap_or(false);
                    
                    if !should_continue {
                        break;
                    }
                    
                    // Set counter in context
                    dry.insert(&self.counter_var, iteration);
                    
                    // Clear scoped control flags for this iteration
                    dry.insert(&self.continue_var, false);
                    dry.insert(&self.break_var, false);
                    
                    // Execute ops
                    let ops = Self::create_ops();
                    for op in ops {
                        let result = op.perform(dry, wet).await?;
                        results.push(result);
                        
                        // Check for early termination signals
                        if dry.is_aborted() {
                            return Err($crate::OpError::Aborted(
                                dry.abort_reason().cloned().unwrap_or_else(|| "While loop aborted".to_string())
                            ));
                        }
                        
                        // Check scoped continue flag
                        if dry.get::<bool>(&self.continue_var).unwrap_or(false) {
                            dry.insert(&self.continue_var, false); // Clear flag
                            break; // Continue to next iteration
                        }
                        
                        // Check scoped break flag
                        if dry.get::<bool>(&self.break_var).unwrap_or(false) {
                            dry.insert(&self.break_var, false); // Clear flag
                            return Ok(results); // Break out of entire loop
                        }
                    }
                    
                    iteration += 1;
                }
                
                Ok(results)
            }
        }
        
        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
        
        repeat_until!(@impl $name<$T> -> $strategy);
        
        impl $name {
            fn build_metadata(&self) -> $crate::OpMetadata {
                let condition_var = self.condition_var.clone();
                let mut properties = serde_json::Map::new();
                properties.insert(
                    condition_var.clone(),
                    serde_json::json!({ "type": "boolean" })
                );
                
                $crate::OpMetadata::builder(stringify!($name))
                    .description(format!("While-loop with condition from '{}' (max {} iterations)", condition_var, self.max_iterations))
                    .input_schema(serde_json::json!({
                        "type": "object",
                        "properties": properties,
                        "required": [condition_var]
                    }))
                    .build()
            }
        }
    };
    
    // Strategy selection helper
    (@strategy all) => { $crate::macros::AggregationStrategy::All };
    (@strategy last) => { $crate::macros::AggregationStrategy::Last };
    (@strategy first) => { $crate::macros::AggregationStrategy::First };
    (@strategy merge) => { $crate::macros::AggregationStrategy::Merge };
    (@strategy unit) => { $crate::macros::AggregationStrategy::Unit };
    
    // Implementation for Vec<T> return (all strategy)
    (@impl $name:ident<$T:ty> -> all) => {
        #[async_trait::async_trait]
        impl $crate::Op<Vec<$T>> for $name {
            async fn perform(&self, dry: &mut $crate::DryContext, wet: &mut $crate::WetContext) -> $crate::OpResult<Vec<$T>> {
                self.perform_repeat_until(dry, wet).await
            }
            
            fn metadata(&self) -> $crate::OpMetadata {
                self.build_metadata()
            }
        }
    };
    
    // Implementation for T return (last strategy)
    (@impl $name:ident<$T:ty> -> last) => {
        #[async_trait::async_trait]
        impl $crate::Op<$T> for $name {
            async fn perform(&self, dry: &mut $crate::DryContext, wet: &mut $crate::WetContext) -> $crate::OpResult<$T> {
                let results = self.perform_repeat_until(dry, wet).await?;
                results.into_iter().last()
                    .ok_or_else(|| $crate::OpError::ExecutionFailed("No results from while loop operation".to_string()))
            }
            
            fn metadata(&self) -> $crate::OpMetadata {
                self.build_metadata()
            }
        }
    };
    
    // Implementation for T return (first strategy)
    (@impl $name:ident<$T:ty> -> first) => {
        #[async_trait::async_trait]
        impl $crate::Op<$T> for $name {
            async fn perform(&self, dry: &mut $crate::DryContext, wet: &mut $crate::WetContext) -> $crate::OpResult<$T> {
                let results = self.perform_repeat_until(dry, wet).await?;
                results.into_iter().next()
                    .ok_or_else(|| $crate::OpError::ExecutionFailed("No results from while loop operation".to_string()))
            }
            
            fn metadata(&self) -> $crate::OpMetadata {
                self.build_metadata()
            }
        }
    };
    
    // Implementation for () return (unit strategy)
    (@impl $name:ident<$T:ty> -> unit) => {
        #[async_trait::async_trait]
        impl $crate::Op<()> for $name {
            async fn perform(&self, dry: &mut $crate::DryContext, wet: &mut $crate::WetContext) -> $crate::OpResult<()> {
                let _results = self.perform_repeat_until(dry, wet).await?;
                Ok(())
            }
            
            fn metadata(&self) -> $crate::OpMetadata {
                self.build_metadata()
            }
        }
    };
    
    // Implementation for merge strategy
    (@impl $name:ident<$T:ty> -> merge) => {
        #[async_trait::async_trait]
        impl $crate::Op<$T> for $name 
        where 
            $T: Default + Clone
        {
            async fn perform(&self, dry: &mut $crate::DryContext, wet: &mut $crate::WetContext) -> $crate::OpResult<$T> {
                let results = self.perform_repeat_until(dry, wet).await?;
                Ok(results.into_iter().last().unwrap_or_default())
            }
            
            fn metadata(&self) -> $crate::OpMetadata {
                self.build_metadata()
            }
        }
    };
}

/// Create a wrapper that converts any Op<T> to Op<U> using a conversion strategy
/// For advanced composability between different op types
/// Usage is typically not needed with the new flexible batch!/repeat!/repeat_until! macros
#[macro_export] 
macro_rules! op_wrapper {
    // Convert any Op<T> to Op<()> by ignoring the result
    (unit $name:ident for $from:ty) => {
        pub struct $name<T> 
        where 
            T: $crate::Op<$from> + Send + Sync,
        {
            inner: T,
        }
        
        impl<T> $name<T>
        where 
            T: $crate::Op<$from> + Send + Sync,
        {
            pub fn new(op: T) -> Self {
                Self { inner: op }
            }
        }
        
        #[async_trait::async_trait]
        impl<T> $crate::Op<()> for $name<T>
        where 
            T: $crate::Op<$from> + Send + Sync,
        {
            async fn perform(&self, dry: &mut $crate::DryContext, wet: &mut $crate::WetContext) -> $crate::OpResult<()> {
                let _result = self.inner.perform(dry, wet).await?;
                Ok(())
            }
            
            fn metadata(&self) -> $crate::OpMetadata {
                self.inner.metadata()
            }
        }
    };
    
    // Convert Op<Vec<T>> to Op<T> using last element
    (last $name:ident for $elem:ty) => {
        pub struct $name<T> 
        where 
            T: $crate::Op<Vec<$elem>> + Send + Sync,
        {
            inner: T,
            _phantom: std::marker::PhantomData<$elem>,
        }
        
        impl<T> $name<T>
        where 
            T: $crate::Op<Vec<$elem>> + Send + Sync,
        {
            pub fn new(op: T) -> Self {
                Self { 
                    inner: op,
                    _phantom: std::marker::PhantomData,
                }
            }
        }
        
        #[async_trait::async_trait]
        impl<T> $crate::Op<$elem> for $name<T>
        where 
            T: $crate::Op<Vec<$elem>> + Send + Sync,
        {
            async fn perform(&self, dry: &mut $crate::DryContext, wet: &mut $crate::WetContext) -> $crate::OpResult<$elem> {
                let results = self.inner.perform(dry, wet).await?;
                results.into_iter().last()
                    .ok_or_else(|| $crate::OpError::ExecutionFailed("No results from wrapped operation".to_string()))
            }
            
            fn metadata(&self) -> $crate::OpMetadata {
                self.inner.metadata()
            }
        }
    };
}

// Control flow macros

/// Abort macro that sets the abort flag in DryContext
/// 
/// This macro should be used when an operation determines that continuing
/// execution would be futile and should not trigger retries.
/// 
/// # Usage
/// ```
/// use ops::{abort, DryContext, OpError};
/// 
/// fn example_op(dry: &mut DryContext) -> Result<(), OpError> {
///     abort!(dry);
/// }
/// ```
#[macro_export]
macro_rules! abort {
    ($dry:expr) => {
        {
            $dry.set_abort(None);
            return Err($crate::OpError::Aborted("Operation aborted".to_string()));
        }
    };
    ($dry:expr, $reason:expr) => {
        {
            let reason_str = $reason.to_string();
            $dry.set_abort(Some(reason_str.clone()));
            return Err($crate::OpError::Aborted(reason_str));
        }
    };
}

/// Continue loop macro that sets the continue flag in DryContext
/// 
/// This macro should be used within loop operations to skip the rest
/// of the current iteration, similar to 'continue' in a for loop.
/// 
/// Automatically targets the current loop context.
/// 
/// # Usage
/// ```
/// use ops::{continue_loop, DryContext, OpError};
/// 
/// fn example_op(dry: &mut DryContext) -> Result<i32, OpError> {
///     continue_loop!(dry);
/// }
/// ```
#[macro_export]
macro_rules! continue_loop {
    ($dry:expr) => {
        {
            // Get current loop ID and set scoped continue flag
            if let Some(loop_id) = $dry.get::<String>("__current_loop_id") {
                let continue_var = format!("__continue_loop_{}", loop_id);
                $dry.insert(&continue_var, true);
            }
            return Ok(Default::default()); // Return default value for the op type
        }
    };
}

/// Break loop macro that sets the break flag in DryContext
/// 
/// This macro should be used within loop operations to exit the entire
/// loop immediately, similar to 'break' in a for loop.
/// 
/// Automatically targets the current loop context.
/// 
/// # Usage
/// ```
/// use ops::{break_loop, DryContext, OpError};
/// 
/// fn example_op(dry: &mut DryContext) -> Result<i32, OpError> {
///     break_loop!(dry);
/// }
/// ```
#[macro_export]
macro_rules! break_loop {
    ($dry:expr) => {
        {
            // Get current loop ID and set scoped break flag
            if let Some(loop_id) = $dry.get::<String>("__current_loop_id") {
                let break_var = format!("__break_loop_{}", loop_id);
                $dry.insert(&break_var, true);
            }
            return Ok(Default::default()); // Return default value for the op type
        }
    };
}

/// Break a specific loop by targeting its loop ID
/// 
/// # Usage
/// ```
/// use ops::{break_loop_scoped, DryContext, OpError};
/// 
/// fn example_op(dry: &mut DryContext) -> Result<i32, OpError> {
///     break_loop_scoped!(dry, "my_loop_id");
/// }
/// ```
#[macro_export]
macro_rules! break_loop_scoped {
    ($dry:expr, $loop_id:expr) => {
        {
            let break_var = format!("__break_loop_{}", $loop_id);
            $dry.insert(&break_var, true);
            return Ok(Default::default()); // Return default value for the op type
        }
    };
}

/// Utility macro to check if operation should be aborted
/// 
/// Returns early with Aborted error if abort flag is set
/// 
/// # Usage
/// ```
/// use ops::{check_abort, DryContext, OpError};
/// 
/// fn example_op(dry: &mut DryContext) -> Result<i32, OpError> {
///     check_abort!(dry);
///     Ok(42)
/// }
/// ```
#[macro_export]
macro_rules! check_abort {
    ($dry:expr) => {
        if $dry.is_aborted() {
            let reason = $dry.abort_reason()
                .cloned()
                .unwrap_or_else(|| "Operation aborted".to_string());
            return Err($crate::OpError::Aborted(reason));
        }
    };
}

/// Macro to create a void wrapper type that discards the result
#[macro_export]
macro_rules! void_op {
    ($wrapper_name:ident, $op_type:ty) => {
        #[derive(Clone)]
        pub struct $wrapper_name {
            wrapped: $op_type,
        }

        impl $wrapper_name {
            pub fn new() -> Self {
                Self { wrapped: <$op_type>::new() }
            }
        }

        #[async_trait::async_trait]
        impl $crate::Op<()> for $wrapper_name {
            async fn perform(&self, dry: &mut DryContext, wet: &mut WetContext) -> OpResult<()> {
                self.wrapped.perform(dry, wet).await.map(|_| ())
            }
            
            fn metadata(&self) -> OpMetadata {
                self.wrapped.metadata()
            }
            
            async fn rollback(&self, dry: &mut DryContext, wet: &mut WetContext) -> OpResult<()> {
                self.wrapped.rollback(dry, wet).await
            }
        }
    };
}


/// Macro to create a Trigger for any operation type
///
/// This macro generates all the boilerplate code needed to set a trigger for any Op type
/// and make it compatible with the TriggerRegistry system.
///
/// Usage:
/// ```rust
/// // Basic usage - creates MyTrigger struct
/// wire_trigger!(MyTrigger, MyOp, "MyOperation");
///
/// // Real examples
/// wire_trigger!(FolderScanTrigger, crate::ops::FolderScanOp, "FolderScanOp");
/// wire_trigger!(CustomAnalysisTrigger, MyCustomAnalysisOp, "CustomAnalysis");
///
/// // Then register in your registry
/// registry.register("my_task_type", || {
///     Box::new(MyTrigger::new())
/// });
/// ```
#[macro_export]
macro_rules! wire_trigger {
    ($wrapper_name:ident, $op_type:ty, $name:expr) => {
        paste::paste! {
            // Create a void wrapper for the op type with unique name
            $crate::void_op!([<$wrapper_name VoidOp>], $op_type);
            
            pub struct $wrapper_name {
                action: [<$wrapper_name VoidOp>],
            }

            impl $wrapper_name {
                pub fn new() -> Self {
                    Self {
                        action: [<$wrapper_name VoidOp>]::new(),
                    }
                }
            }
        }

        impl Default for $wrapper_name {
            fn default() -> Self {
                Self::new()
            }
        }

        #[async_trait::async_trait]
        impl $crate::Trigger for $wrapper_name {
			fn actions(&self) -> Vec<Arc<dyn $crate::Op<()>>> {
				vec![Arc::new(self.action.clone())]
			}

			fn predicate(&self) -> Arc<dyn $crate::Op<bool>> {
				Arc::new($crate::TrivialPredicate::default())
			}
        }
    };
    
    ($wrapper_name:ident, $op_type:ty, $name:expr, $predicate:expr) => {
        paste::paste! {
            // Create a void wrapper for the op type with unique name
            $crate::void_op!([<$wrapper_name VoidOp>], $op_type);
            
            pub struct $wrapper_name {
                action: [<$wrapper_name VoidOp>],
                predicate: Arc<dyn $crate::Op<bool>>,
            }

            impl $wrapper_name {
                pub fn new() -> Self {
                    Self {
                        action: [<$wrapper_name VoidOp>]::new(),
                        predicate: Arc::new($predicate),
                    }
                }
                
                pub fn with_predicate(predicate: Arc<dyn $crate::Op<bool>>) -> Self {
                    Self {
                        action: [<$wrapper_name VoidOp>]::new(),
                        predicate,
                    }
                }
            }
        }

        impl Default for $wrapper_name {
            fn default() -> Self {
                Self::new()
            }
        }

		#[async_trait::async_trait]
		impl $crate::Trigger for $wrapper_name {
			fn predicate(&self) -> Arc<dyn $crate::Op<bool>> {
				Arc::clone(&self.predicate)
			}
			fn actions(&self) -> Vec<Arc<dyn $crate::Op<()>>> {
				vec![Arc::new(self.action.clone())]
			}
		}
    };
}

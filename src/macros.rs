/// Store a variable in dry context using its name as the key
/// dry_put!(dry_context, variable_name)
#[macro_export]
macro_rules! dry_put {
    ($ctx:expr, $var:ident) => {
        $ctx.insert(stringify!($var), $var)
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

/// Retrieve a required value from dry context, return error if missing
/// let var: Type = dry_require!(dry_context, var_name)?;
#[macro_export]
macro_rules! dry_require {
    ($ctx:expr, $var:ident) => {
        $ctx.get_required::<_>(stringify!($var))
    };
}

/// Store result in dry context under both the op name and "result" key
/// dry_result!(dry_context, "OpName", result_value);
#[macro_export]
macro_rules! dry_result {
    ($ctx:expr, $op_name:expr, $result:expr) => {
        {
            $ctx.insert($op_name, $result.clone());
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

/// Require a reference from wet context with error handling
/// let var: Arc<Type> = wet_require_ref!(wet_context, var_name)?;
#[macro_export]
macro_rules! wet_require_ref {
    ($ctx:expr, $var:ident) => {
        $ctx.get_required::<_>(stringify!($var))
    };
}

/// Create a named batch op type with a fixed list of child ops
/// Usage:
/// ```rust
/// batch! {
///     ProcessingPipeline<String> = [
///         ValidationOp::new(),
///         TransformOp::new(),
///         PersistOp::new()
///     ]
/// }
/// ```
#[macro_export]
macro_rules! batch {
    ($name:ident<$T:ty> = [$($op:expr),+ $(,)?]) => {
        pub struct $name {
            batch: $crate::BatchOp<$T>,
        }
        
        impl $name {
            pub fn new() -> Self {
                let ops: Vec<std::sync::Arc<dyn $crate::Op<$T>>> = vec![
                    $(std::sync::Arc::new($op)),+
                ];
                Self {
                    batch: $crate::BatchOp::new(ops),
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
}

/// Create a named loop op type with a fixed list of child ops
/// The limit is loaded from a variable in the dry context
/// Usage:
/// ```rust
/// repeat! {
///     ProcessLoop<String> = {
///         counter: "iteration",
///         limit: "max_iterations",  // Variable name in dry context
///         ops: [
///             StepOp::new(),
///             LogOp::new()
///         ]
///     }
/// }
/// ```
#[macro_export]
macro_rules! repeat {
    ($name:ident<$T:ty> = {
        counter: $counter:expr,
        limit: $limit_var:expr,
        ops: [$($op:expr),+ $(,)?]
    }) => {
        pub struct $name {
            counter_var: String,
            limit_var: String,
        }
        
        impl $name {
            pub fn new() -> Self {
                Self {
                    counter_var: $counter.to_string(),
                    limit_var: $limit_var.to_string(),
                }
            }
            
            fn create_ops() -> Vec<Box<dyn $crate::Op<$T>>> {
                vec![
                    $(Box::new($op)),+
                ]
            }
        }
        
        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
        
        #[async_trait::async_trait]
        impl $crate::Op<Vec<$T>> for $name {
            async fn perform(&self, dry: &mut $crate::DryContext, wet: &mut $crate::WetContext) -> $crate::OpResult<Vec<$T>> {
                // Get limit from context
                let limit = dry.get_required::<usize>(&self.limit_var)?;
                
                let ops = Self::create_ops();
                let loop_op = $crate::loop_op::LoopOp::new(
                    self.counter_var.clone(),
                    limit,
                    ops
                );
                loop_op.perform(dry, wet).await
            }
            
            fn metadata(&self) -> $crate::OpMetadata {
                let limit_var = self.limit_var.clone();
                let mut properties = serde_json::Map::new();
                properties.insert(
                    limit_var.clone(),
                    serde_json::json!({ "type": "integer", "minimum": 0 })
                );
                
                $crate::OpMetadata::builder(stringify!($name))
                    .description(format!("Loop with dynamic limit from '{}'", limit_var))
                    .input_schema(serde_json::json!({
                        "type": "object",
                        "properties": properties,
                        "required": [limit_var]
                    }))
                    .build()
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
            $dry.set_continue_loop();
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
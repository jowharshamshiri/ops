/// Store a variable in dry context using its name as the key
/// dry_put!(dry_context, variable_name)
#[macro_export]
macro_rules! dry_put {
    ($ctx:expr, $var:ident) => {
        $ctx.insert(stringify!($var), $var)
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
/// Usage:
/// ```rust
/// repeat! {
///     ProcessLoop<String> = {
///         counter: "iteration",
///         limit: 10,
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
        limit: $limit:expr,
        ops: [$($op:expr),+ $(,)?]
    }) => {
        pub struct $name {
            loop_op: $crate::loop_op::LoopOp<$T>,
        }
        
        impl $name {
            pub fn new() -> Self {
                let ops: Vec<Box<dyn $crate::Op<$T>>> = vec![
                    $(Box::new($op)),+
                ];
                Self {
                    loop_op: $crate::loop_op::LoopOp::new($counter.to_string(), $limit, ops),
                }
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
                self.loop_op.perform(dry, wet).await
            }
            
            fn metadata(&self) -> $crate::OpMetadata {
                self.loop_op.metadata()
            }
        }
    };
}
/// Ergonomic macro to create Op implementations from function-like syntax
/// 
/// The op reads its inputs from the OpContext and writes its output back to the OpContext.
/// Special parameter `context: OpContext` will be automatically passed without input extraction.
/// 
/// Usage:
/// ```rust
/// op!(hello_operation(name: String) -> String {
///     Ok(format!("Hello, {}!", name))
/// });
/// 
/// op!(context_aware_op(context: OpContext, base: i32) -> i32 {
///     context.set("debug".to_string(), serde_json::json!("executed"));
///     Ok(base + 10)
/// });
/// ```
/// 
/// This generates a struct that implements `Op<()>` and:
/// - Reads inputs from OpContext using parameter names as keys
/// - Passes `context: OpContext` parameter directly without input extraction
/// - Executes the body with parameters bound
/// - Writes the result back to OpContext using the op name and "result"
#[macro_export]
macro_rules! op {
    // Special case: single parameter that is ctx: OpContext
    ($fn_name:ident(ctx: OpContext) -> $output_type:ty $body:block) => {
        paste::paste! {
            pub struct [<$fn_name:camel>];
            
            impl [<$fn_name:camel>] {
                pub fn new() -> Self {
                    Self
                }
            }
            
            impl Default for [<$fn_name:camel>] {
                fn default() -> Self {
                    Self::new()
                }
            }
            
            #[async_trait::async_trait]
            impl $crate::op::Op<()> for [<$fn_name:camel>] {
                async fn perform(&self, context: &mut $crate::context::OpContext) -> std::result::Result<(), $crate::error::OpError> {
                    tracing::debug!("Executing {}", stringify!([<$fn_name:camel>]));
                    
                    // Execute the operation body with context directly as mutable reference
                    let result: std::result::Result<$output_type, $crate::error::OpError> = {
                        let mut ctx = context;
                        $body
                    };
                    
                    // Store result in context under the op name
                    match result {
                        Ok(output) => {
                            match serde_json::to_value(&output) {
                                Ok(json_value) => {
                                    let op_name = stringify!([<$fn_name:camel>]);
                                    context.set(op_name.to_string(), json_value.clone());
                                    context.set("result".to_string(), json_value);
                                    tracing::debug!("Successfully stored result for {} under key '{}'", op_name, op_name);
                                    Ok(())
                                },
                                Err(e) => {
                                    Err($crate::error::OpError::ExecutionFailed(
                                        format!("Failed to serialize output: {}", e)
                                    ))
                                }
                            }
                        },
                        Err(e) => {
                            tracing::error!("Operation {} failed: {}", stringify!([<$fn_name:camel>]), e);
                            Err(e)
                        }
                    }
                }
            }
        }
    };

    // Special case: single parameter that is ctx: mut OpContext  
    ($fn_name:ident(ctx: mut OpContext) -> $output_type:ty $body:block) => {
        paste::paste! {
            pub struct [<$fn_name:camel>];
            
            impl [<$fn_name:camel>] {
                pub fn new() -> Self {
                    Self
                }
            }
            
            impl Default for [<$fn_name:camel>] {
                fn default() -> Self {
                    Self::new()
                }
            }
            
            #[async_trait::async_trait]
            impl $crate::op::Op<()> for [<$fn_name:camel>] {
                async fn perform(&self, context: &mut $crate::context::OpContext) -> std::result::Result<(), $crate::error::OpError> {
                    tracing::debug!("Executing {}", stringify!([<$fn_name:camel>]));
                    
                    // Execute the operation body with context directly as mutable reference
                    let result: std::result::Result<$output_type, $crate::error::OpError> = {
                        let mut ctx = context;
                        $body
                    };
                    
                    // Store result in context under the op name
                    match result {
                        Ok(output) => {
                            match serde_json::to_value(&output) {
                                Ok(json_value) => {
                                    let op_name = stringify!([<$fn_name:camel>]);
                                    context.set(op_name.to_string(), json_value.clone());
                                    context.set("result".to_string(), json_value);
                                    tracing::debug!("Successfully stored result for {} under key '{}'", op_name, op_name);
                                    Ok(())
                                },
                                Err(e) => {
                                    Err($crate::error::OpError::ExecutionFailed(
                                        format!("Failed to serialize output: {}", e)
                                    ))
                                }
                            }
                        },
                        Err(e) => {
                            tracing::error!("Operation {} failed: {}", stringify!([<$fn_name:camel>]), e);
                            Err(e)
                        }
                    }
                }
            }
        }
    };

    // Single parameter case (not context)
    ($fn_name:ident($input_name:ident: $input_type:ty) -> $output_type:ty $body:block) => {
        paste::paste! {
            pub struct [<$fn_name:camel>];
            
            impl [<$fn_name:camel>] {
                pub fn new() -> Self {
                    Self
                }
            }
            
            impl Default for [<$fn_name:camel>] {
                fn default() -> Self {
                    Self::new()
                }
            }
            
            #[async_trait::async_trait]
            impl $crate::op::Op<()> for [<$fn_name:camel>] {
                async fn perform(&self, context: &mut $crate::context::OpContext) -> std::result::Result<(), $crate::error::OpError> {
                    tracing::debug!("Executing {}", stringify!([<$fn_name:camel>]));
                    
                    // Use requirement system for input with fallback to "result" for daisy chaining
                    let input_key = stringify!($input_name);
                    let $input_name: $input_type = match context.get::<$input_type>(input_key) {
                        Some(value) => {
                            tracing::debug!("Found input '{}' via requirement system", input_key);
                            value
                        },
                        None => {
                            // Fallback: try "result" for daisy chaining
                            match context.get::<$input_type>("result") {
                                Some(value) => {
                                    tracing::debug!("Using 'result' for input '{}' via requirement fallback", input_key);
                                    value
                                },
                                None => {
                                    return Err($crate::error::OpError::ExecutionFailed(
                                        format!("Missing required input: '{}' (checked both '{}' and 'result')", input_key, input_key)
                                    ));
                                }
                            }
                        }
                    };
                    
                    // Execute the operation body
                    let result: std::result::Result<$output_type, $crate::error::OpError> = $body;
                    
                    // Store result in context under the op name
                    match result {
                        Ok(output) => {
                            // Convert output to serde_json::Value for storage
                            match serde_json::to_value(&output) {
                                Ok(json_value) => {
                                    let op_name = stringify!([<$fn_name:camel>]);
                                    context.set(op_name.to_string(), json_value.clone());
                                    // Also store as "result" for convenience
                                    context.set("result".to_string(), json_value);
                                    tracing::debug!("Successfully stored result for {} under key '{}'", op_name, op_name);
                                    Ok(())
                                },
                                Err(e) => {
                                    Err($crate::error::OpError::ExecutionFailed(
                                        format!("Failed to serialize output: {}", e)
                                    ))
                                }
                            }
                        },
                        Err(e) => {
                            tracing::error!("Operation {} failed: {}", stringify!([<$fn_name:camel>]), e);
                            Err(e)
                        }
                    }
                }
            }
        }
    };
    
    // Multiple parameters with ctx: OpContext as first parameter
    ($fn_name:ident(ctx: OpContext, $($input_name:ident: $input_type:ty),+ $(,)?) -> $output_type:ty $body:block) => {
        paste::paste! {
            pub struct [<$fn_name:camel>];
            
            impl [<$fn_name:camel>] {
                pub fn new() -> Self {
                    Self
                }
            }
            
            impl Default for [<$fn_name:camel>] {
                fn default() -> Self {
                    Self::new()
                }
            }
            
            #[async_trait::async_trait]
            impl $crate::op::Op<()> for [<$fn_name:camel>] {
                async fn perform(&self, context: &mut $crate::context::OpContext) -> std::result::Result<(), $crate::error::OpError> {
                    tracing::debug!("Executing {}", stringify!([<$fn_name:camel>]));
                    
                    // Extract regular parameters first
                    $(
                        let input_key = stringify!($input_name);
                        let $input_name: $input_type = match context.get::<$input_type>(input_key) {
                            Some(value) => {
                                tracing::debug!("Found input '{}' via requirement system", input_key);
                                value
                            },
                            None => {
                                return Err($crate::error::OpError::ExecutionFailed(
                                    format!("Missing required input: '{}'", input_key)
                                ));
                            }
                        };
                    )+
                    
                    // Execute the operation body with ctx available
                    let result: std::result::Result<$output_type, $crate::error::OpError> = {
                        let mut ctx = context;
                        $body
                    };
                    
                    // Store result in context under the op name
                    match result {
                        Ok(output) => {
                            match serde_json::to_value(&output) {
                                Ok(json_value) => {
                                    let op_name = stringify!([<$fn_name:camel>]);
                                    context.set(op_name.to_string(), json_value.clone());
                                    context.set("result".to_string(), json_value);
                                    tracing::debug!("Successfully stored result for {} under key '{}'", op_name, op_name);
                                    Ok(())
                                },
                                Err(e) => {
                                    Err($crate::error::OpError::ExecutionFailed(
                                        format!("Failed to serialize output: {}", e)
                                    ))
                                }
                            }
                        },
                        Err(e) => {
                            tracing::error!("Operation {} failed: {}", stringify!([<$fn_name:camel>]), e);
                            Err(e)
                        }
                    }
                }
            }
        }
    };

    // Multiple parameters with ctx: mut OpContext as first parameter
    ($fn_name:ident(ctx: mut OpContext, $($input_name:ident: $input_type:ty),+ $(,)?) -> $output_type:ty $body:block) => {
        paste::paste! {
            pub struct [<$fn_name:camel>];
            
            impl [<$fn_name:camel>] {
                pub fn new() -> Self {
                    Self
                }
            }
            
            impl Default for [<$fn_name:camel>] {
                fn default() -> Self {
                    Self::new()
                }
            }
            
            #[async_trait::async_trait]
            impl $crate::op::Op<()> for [<$fn_name:camel>] {
                async fn perform(&self, context: &mut $crate::context::OpContext) -> std::result::Result<(), $crate::error::OpError> {
                    tracing::debug!("Executing {}", stringify!([<$fn_name:camel>]));
                    
                    // Extract regular parameters first
                    $(
                        let input_key = stringify!($input_name);
                        let $input_name: $input_type = match context.get::<$input_type>(input_key) {
                            Some(value) => {
                                tracing::debug!("Found input '{}' via requirement system", input_key);
                                value
                            },
                            None => {
                                return Err($crate::error::OpError::ExecutionFailed(
                                    format!("Missing required input: '{}'", input_key)
                                ));
                            }
                        };
                    )+
                    
                    // Execute the operation body with ctx available
                    let result: std::result::Result<$output_type, $crate::error::OpError> = {
                        let mut ctx = context;
                        $body
                    };
                    
                    // Store result in context under the op name
                    match result {
                        Ok(output) => {
                            match serde_json::to_value(&output) {
                                Ok(json_value) => {
                                    let op_name = stringify!([<$fn_name:camel>]);
                                    context.set(op_name.to_string(), json_value.clone());
                                    context.set("result".to_string(), json_value);
                                    tracing::debug!("Successfully stored result for {} under key '{}'", op_name, op_name);
                                    Ok(())
                                },
                                Err(e) => {
                                    Err($crate::error::OpError::ExecutionFailed(
                                        format!("Failed to serialize output: {}", e)
                                    ))
                                }
                            }
                        },
                        Err(e) => {
                            tracing::error!("Operation {} failed: {}", stringify!([<$fn_name:camel>]), e);
                            Err(e)
                        }
                    }
                }
            }
        }
    };

    // Zero parameters case
    ($fn_name:ident() -> $output_type:ty $body:block) => {
        paste::paste! {
            pub struct [<$fn_name:camel>];
            
            impl [<$fn_name:camel>] {
                pub fn new() -> Self {
                    Self
                }
            }
            
            impl Default for [<$fn_name:camel>] {
                fn default() -> Self {
                    Self::new()
                }
            }
            
            #[async_trait::async_trait]
            impl $crate::op::Op<()> for [<$fn_name:camel>] {
                async fn perform(&self, context: &mut $crate::context::OpContext) -> std::result::Result<(), $crate::error::OpError> {
                    tracing::debug!("Executing {}", stringify!([<$fn_name:camel>]));
                    
                    // Execute the operation body
                    let result: std::result::Result<$output_type, $crate::error::OpError> = $body;
                    
                    // Store result in context under the op name
                    match result {
                        Ok(output) => {
                            match serde_json::to_value(&output) {
                                Ok(json_value) => {
                                    let op_name = stringify!([<$fn_name:camel>]);
                                    context.set(op_name.to_string(), json_value.clone());
                                    context.set("result".to_string(), json_value);
                                    tracing::debug!("Successfully stored result for {} under key '{}'", op_name, op_name);
                                    Ok(())
                                },
                                Err(e) => {
                                    Err($crate::error::OpError::ExecutionFailed(
                                        format!("Failed to serialize output: {}", e)
                                    ))
                                }
                            }
                        },
                        Err(e) => {
                            tracing::error!("Operation {} failed: {}", stringify!([<$fn_name:camel>]), e);
                            Err(e)
                        }
                    }
                }
            }
        }
    };

    // Multiple parameters - regular case (no ctx parameter)
    ($fn_name:ident($($input_name:ident: $input_type:ty),+ $(,)?) -> $output_type:ty $body:block) => {
        $crate::_op_multi_param_impl!($fn_name ($($input_name: $input_type),+) -> $output_type $body);
    };
}

/// Helper macro to handle multiple parameters with potential context
#[macro_export]
macro_rules! _op_multi_param_impl {
    ($fn_name:ident ($($input_name:ident: $input_type:ty),+ $(,)?) -> $output_type:ty $body:block) => {
        paste::paste! {
            pub struct [<$fn_name:camel>];
            
            impl [<$fn_name:camel>] {
                pub fn new() -> Self {
                    Self
                }
            }
            
            impl Default for [<$fn_name:camel>] {
                fn default() -> Self {
                    Self::new()
                }
            }
            
            #[async_trait::async_trait]
            impl $crate::op::Op<()> for [<$fn_name:camel>] {
                async fn perform(&self, ctx: &mut $crate::context::OpContext) -> std::result::Result<(), $crate::error::OpError> {
                    tracing::debug!("Executing {}", stringify!([<$fn_name:camel>]));
                    
                    // Handle each parameter - context gets special treatment
                    $(
                        $crate::_op_param_extract!(ctx, $input_name: $input_type);
                    )+
                    
                    // Execute the operation body
                    let result: std::result::Result<$output_type, $crate::error::OpError> = $body;
                    
                    // Store result in context under the op name
                    match result {
                        Ok(output) => {
                            match serde_json::to_value(&output) {
                                Ok(json_value) => {
                                    let op_name = stringify!([<$fn_name:camel>]);
                                    ctx.set(op_name.to_string(), json_value.clone());
                                    // Also store as "result" for convenience
                                    ctx.set("result".to_string(), json_value);
                                    tracing::debug!("Successfully stored result for {} under key '{}'", op_name, op_name);
                                    Ok(())
                                },
                                Err(e) => {
                                    Err($crate::error::OpError::ExecutionFailed(
                                        format!("Failed to serialize output: {}", e)
                                    ))
                                }
                            }
                        },
                        Err(e) => {
                            tracing::error!("Operation {} failed: {}", stringify!([<$fn_name:camel>]), e);
                            Err(e)
                        }
                    }
                }
            }
        }
    };
}

/// Helper macro to extract parameters, treating context specially
#[macro_export] 
macro_rules! _op_param_extract {
    // Special case: ctx parameter gets passed directly as mutable reference
    ($context_var:ident, ctx: OpContext) => {
        let mut ctx = $context_var;
    };
    
    // Special case: ctx parameter with explicit mut gets passed directly as mutable reference
    ($context_var:ident, ctx: mut OpContext) => {
        let mut ctx = $context_var;
    };
    
    // Regular parameter extraction from context
    ($context_var:ident, $input_name:ident: $input_type:ty) => {
        let input_key = stringify!($input_name);
        let $input_name: $input_type = match $context_var.get::<$input_type>(input_key) {
            Some(value) => {
                tracing::debug!("Found input '{}' via requirement system", input_key);
                value
            },
            None => {
                return Err($crate::error::OpError::ExecutionFailed(
                    format!("Missing required input: '{}'", input_key)
                ));
            }
        };
    };
}

/// Macro to run a daisy chain of ops with unified requirement syntax
/// 
/// Supports mixed static values and factory functions:
/// ```rust
/// // Create new context
/// let context = perform!(
///     [("x", 5),                           // Static value
///      ("database", || connect_to_db()),   // Factory function  
///      ("config", || load_config())],      // Factory function
///     HelloOperation,
///     MathOperation, 
///     ComplexOperation
/// )?;
/// 
/// // Use existing context
/// let context = perform!(
///     with_context existing_context,       // Use existing OpContext
///     [("additional", 10)],                // Additional inputs
///     AnotherOperation
/// )?;
/// ```
#[macro_export]
macro_rules! perform {
    // Use existing context with additional inputs (must use `with_context` keyword to disambiguate)
    (with_context $existing_context:expr, [$($input:tt),* $(,)?], $($op_name:ident),+ $(,)?) => {{
        async {
            let mut context = $existing_context;
            
            // Set up additional inputs using helper macro
            $(
                $crate::_setup_input!(context, $input)?;
            )*
            
            // Run each op in sequence
            $(
                {
                    let op = $op_name::new();
                    if let Err(e) = $crate::op::Op::perform(&op, &mut context).await {
                        tracing::error!("Op {} failed: {}", stringify!($op_name), e);
                        return Err(e);
                    }
                    tracing::debug!("Op {} completed successfully", stringify!($op_name));
                }
            )+
            
            Ok::<$crate::context::OpContext, $crate::error::OpError>(context)
        }
    }};

    // Use existing context with no additional inputs (must use `with_context` keyword to disambiguate)
    (with_context $existing_context:expr, $($op_name:ident),+ $(,)?) => {{
        async {
            let mut context = $existing_context;
            
            // Run each op in sequence
            $(
                {
                    let op = $op_name::new();
                    if let Err(e) = $crate::op::Op::perform(&op, &mut context).await {
                        tracing::error!("Op {} failed: {}", stringify!($op_name), e);
                        return Err(e);
                    }
                    tracing::debug!("Op {} completed successfully", stringify!($op_name));
                }
            )+
            
            Ok::<$crate::context::OpContext, $crate::error::OpError>(context)
        }
    }};
    
    // Parse the input list with helper to distinguish types (create new context)
    ([$($input:tt),* $(,)?], $($op_name:ident),+ $(,)?) => {{
        async {
            let mut context = $crate::context::OpContext::new();
            
            // Set up all inputs using helper macro
            $(
                $crate::_setup_input!(context, $input)?;
            )*
            
            // Run each op in sequence
            $(
                {
                    let op = $op_name::new();
                    if let Err(e) = $crate::op::Op::perform(&op, &mut context).await {
                        tracing::error!("Op {} failed: {}", stringify!($op_name), e);
                        return Err(e);
                    }
                    tracing::debug!("Op {} completed successfully", stringify!($op_name));
                }
            )+
            
            Ok::<$crate::context::OpContext, $crate::error::OpError>(context)
        }
    }};
}

/// Helper macro to set up different types of inputs
#[macro_export]
macro_rules! _setup_input {
    // Factory function: ("key", || expr)
    ($context:expr, ($key:expr, || $factory:expr)) => {{
        let factory = $crate::context::ClosureFactory::new(|| $factory);
        $context.require($key, Box::new(factory))
    }};
    
    // Factory function with move: ("key", move || expr)  
    ($context:expr, ($key:expr, move || $factory:expr)) => {{
        let factory = $crate::context::ClosureFactory::new(move || $factory);
        $context.require($key, Box::new(factory))
    }};
    
    // Static value: ("key", value)
    ($context:expr, ($key:expr, $value:expr)) => {{
        $context.put($key, $value)
    }};
}


/// Simplified macro for running ops and getting the final result
/// 
/// Usage:
/// ```rust
/// let result: String = get_result!(
///     perform!(
///         [("name", json!("World"))],
///         HelloOperation
///     ).await?
/// );
/// ```
#[macro_export]
macro_rules! get_result {
    ($context_expr:expr) => {{
        let context = $context_expr;
        context.get_raw("result").cloned()
    }};
}

/// Ultra-simple macro that runs ops and extracts the result in one go
/// 
/// Usage:
/// ```rust
/// let result = execute_ops!(
///     [("name", json!("World")), ("x", json!(5))],
///     HelloOperation,
///     MathOperation
/// ).await?;
/// ```
#[macro_export]  
macro_rules! execute_ops {
    ([$($input:tt),* $(,)?], $($op_name:ident),+ $(,)?) => {{
        async {
            let context = $crate::perform!([$($input),*], $($op_name),+).await?;
            Ok::<Option<serde_json::Value>, $crate::error::OpError>(
                context.get_raw("result").cloned()
            )
        } 
    }};
}
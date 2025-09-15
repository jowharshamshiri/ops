/// Ergonomic macro to create Op implementations from function-like syntax
/// 
/// The op reads its inputs from the OpContext and writes its output back to the OpContext.
/// 
/// Usage:
/// ```rust
/// op!(hello_operation(name: String) -> String {
///     Ok(format!("Hello, {}!", name))
/// });
/// ```
/// 
/// This generates a struct `HelloOperation` that implements `Op<()>` and:
/// - Reads `name` from OpContext using the key "name"
/// - Executes the body with the input bound to the parameter name
/// - Writes the result back to OpContext using the key "result"
#[macro_export]
macro_rules! op {
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
    
    // Support multiple input parameters
    ($fn_name:ident($($input_name:ident: $input_type:ty),+ $(,)?) -> $output_type:ty $body:block) => {
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
                    
                    // Use requirement system for all inputs 
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
                    
                    // Execute the operation body
                    let result: std::result::Result<$output_type, $crate::error::OpError> = $body;
                    
                    // Store result in context under the op name
                    match result {
                        Ok(output) => {
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
}

/// Macro to run a daisy chain of ops with unified requirement syntax
/// 
/// Supports mixed static values and factory functions:
/// ```rust
/// let context = perform!(
///     [("x", 5),                           // Static value
///      ("database", || connect_to_db()),   // Factory function  
///      ("config", || load_config())],      // Factory function
///     HelloOperation,
///     MathOperation, 
///     ComplexOperation
/// )?;
/// ```
#[macro_export]
macro_rules! perform {
    // Parse the input list with helper to distinguish types
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
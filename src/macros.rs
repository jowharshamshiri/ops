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
                    log::debug!("Executing {}", stringify!([<$fn_name:camel>]));
                    
                    // Extract input from context with type validation
                    // First try the named parameter, then try "result" for daisy chaining
                    let input_key = stringify!($input_name);
                    let $input_name: $input_type = match context.get_raw(input_key) {
                        Some(value) => {
                            // Try to deserialize from serde_json::Value to the expected type 
                            match serde_json::from_value(value.clone()) {
                                Ok(typed_value) => typed_value,
                                Err(e) => {
                                    return Err($crate::error::OpError::ExecutionFailed(
                                        format!("Invalid type for input '{}': expected {}, error: {}", 
                                            input_key, stringify!($input_type), e)
                                    ));
                                }
                            }
                        },
                        None => {
                            // Try "result" for daisy chaining (most recent op result)
                            match context.get_raw("result") {
                                Some(value) => {
                                    match serde_json::from_value(value.clone()) {
                                        Ok(typed_value) => {
                                            log::debug!("Using 'result' for input '{}'", input_key);
                                            typed_value
                                        },
                                        Err(e) => {
                                            return Err($crate::error::OpError::ExecutionFailed(
                                                format!("Invalid type for input '{}' from result: expected {}, error: {}", 
                                                    input_key, stringify!($input_type), e)
                                            ));
                                        }
                                    }
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
                                    log::debug!("Successfully stored result for {} under key '{}'", op_name, op_name);
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
                            log::error!("Operation {} failed: {}", stringify!([<$fn_name:camel>]), e);
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
                    log::debug!("Executing {}", stringify!([<$fn_name:camel>]));
                    
                    // Extract all inputs from context with type validation
                    $(
                        let input_key = stringify!($input_name);
                        let $input_name: $input_type = match context.get_raw(input_key) {
                            Some(value) => {
                                match serde_json::from_value(value.clone()) {
                                    Ok(typed_value) => typed_value,
                                    Err(e) => {
                                        return Err($crate::error::OpError::ExecutionFailed(
                                            format!("Invalid type for input '{}': expected {}, error: {}", 
                                                input_key, stringify!($input_type), e)
                                        ));
                                    }
                                }
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
                                    log::debug!("Successfully stored result for {} under key '{}'", op_name, op_name);
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
                            log::error!("Operation {} failed: {}", stringify!([<$fn_name:camel>]), e);
                            Err(e)
                        }
                    }
                }
            }
        }
    };
}

/// Macro to run a daisy chain of ops with simple syntax
/// 
/// Usage:
/// ```rust
/// let context = run_ops!(
///     [("name", json!("World")), ("x", json!(5))],
///     HelloOperation,
///     MathOperation,
///     ComplexOperation
/// )?;
/// ```
/// 
/// This will:
/// 1. Put all inputs into a fresh OpContext
/// 2. Run each op in sequence
/// 3. Return the final OpContext with all results
#[macro_export]
macro_rules! run_ops {
    ([$(($input_key:expr, $input_value:expr)),* $(,)?], $($op_name:ident),+ $(,)?) => {{
        async {
            let mut context = $crate::context::OpContext::new();
            
            // Set all inputs in context
            $(
                context.set($input_key.to_string(), $input_value);
            )*
            
            // Run each op in sequence
            $(
                {
                    let op = $op_name::new();
                    if let Err(e) = $crate::op::Op::perform(&op, &mut context).await {
                        log::error!("Op {} failed: {}", stringify!($op_name), e);
                        return Err(e);
                    }
                    log::debug!("Op {} completed successfully", stringify!($op_name));
                }
            )+
            
            Ok::<$crate::context::OpContext, $crate::error::OpError>(context)
        }
    }};
}

/// Simplified macro for running ops and getting the final result
/// 
/// Usage:
/// ```rust
/// let result: String = get_result!(
///     run_ops!(
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
    ([$(($input_key:expr, $input_value:expr)),* $(,)?], $($op_name:ident),+ $(,)?) => {{
        async {
            let context = $crate::run_ops!([$(($input_key, $input_value)),*], $($op_name),+).await?;
            Ok::<Option<serde_json::Value>, $crate::error::OpError>(
                context.get_raw("result").cloned()
            )
        }
    }};
}
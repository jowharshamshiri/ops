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
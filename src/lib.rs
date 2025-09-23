pub mod contexts;
pub mod error;
pub mod op;
pub mod op_metadata;
pub mod batch;
pub mod batch_metadata;
pub mod wrappers;
pub mod ops;
pub mod macros;
pub mod loop_op;

pub use contexts::{DryContext, WetContext};
pub use error::OpError;
pub use op::Op;
pub use op_metadata::{OpMetadata, OpRequest, ValidationReport};
pub use batch::BatchOp;
pub use loop_op::LoopOp;
pub use wrappers::logging::LoggingWrapper;
pub use wrappers::timeout::TimeBoundWrapper;
pub use ops::{perform, get_caller_op_name, wrap_nested_op_exception};

pub type OpResult<T> = std::result::Result<T, OpError>;

pub mod prelude {
	pub use std::any::Any;
	pub use std::sync::Arc;
	pub use serde_json;
	pub use serde::{Serialize, Deserialize};
	pub use tracing::{debug, info, warn, error, trace};
	
	pub use crate::{
		OpResult,
		Op, OpError, DryContext, WetContext, OpMetadata
	};
	pub use async_trait::async_trait;
	pub use std::collections::{HashMap};
	pub use chrono::{DateTime, Utc};
}
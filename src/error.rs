use crate::prelude::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OpError {
    #[error("Op execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Op timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
    
    #[error("Context error: {0}")]
    Context(String),
    
    #[error("Batch op failed: {0}")]
    BatchFailed(String),
    
    #[error("Op aborted: {0}")]
    Aborted(String),
    
    #[error("Trigger error: {0}")]
    Trigger(String),
    
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl Clone for OpError {
    fn clone(&self) -> Self {
        match self {
            Self::ExecutionFailed(msg) => Self::ExecutionFailed(msg.clone()),
            Self::Timeout { timeout_ms } => Self::Timeout { timeout_ms: *timeout_ms },
            Self::Context(msg) => Self::Context(msg.clone()),
            Self::BatchFailed(msg) => Self::BatchFailed(msg.clone()),
            Self::Aborted(msg) => Self::Aborted(msg.clone()),
            Self::Trigger(msg) => Self::Trigger(msg.clone()),
            Self::Other(boxed_error) => Self::ExecutionFailed(format!("{}", boxed_error)),
        }
    }
}

impl From<serde_json::Error> for OpError {
    fn from(e: serde_json::Error) -> Self {
        OpError::Other(Box::new(e))
    }
}
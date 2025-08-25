use thiserror::Error;

#[derive(Error, Debug)]
pub enum OperationError {
    #[error("Operation execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Operation timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
    
    #[error("Context error: {0}")]
    Context(String),
    
    #[error("Batch operation failed: {0}")]
    BatchFailed(String),
    
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl Clone for OperationError {
    fn clone(&self) -> Self {
        match self {
            Self::ExecutionFailed(msg) => Self::ExecutionFailed(msg.clone()),
            Self::Timeout { timeout_ms } => Self::Timeout { timeout_ms: *timeout_ms },
            Self::Context(msg) => Self::Context(msg.clone()),
            Self::BatchFailed(msg) => Self::BatchFailed(msg.clone()),
            Self::Other(boxed_error) => Self::ExecutionFailed(format!("{}", boxed_error)),
        }
    }
}
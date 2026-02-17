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

#[cfg(test)]
mod tests {
    use super::*;

    // TEST104: Verify OpError::ExecutionFailed displays with the correct message format
    #[test]
    fn test_104_op_error_display_execution_failed() {
        let err = OpError::ExecutionFailed("something broke".to_string());
        assert_eq!(err.to_string(), "Op execution failed: something broke");
    }

    // TEST105: Verify OpError::Timeout displays with the correct timeout_ms value
    #[test]
    fn test_105_op_error_display_timeout() {
        let err = OpError::Timeout { timeout_ms: 250 };
        assert_eq!(err.to_string(), "Op timeout after 250ms");
    }

    // TEST106: Verify OpError::Context displays with the correct message format
    #[test]
    fn test_106_op_error_display_context() {
        let err = OpError::Context("missing key".to_string());
        assert_eq!(err.to_string(), "Context error: missing key");
    }

    // TEST107: Verify OpError::Aborted displays with the correct message format
    #[test]
    fn test_107_op_error_display_aborted() {
        let err = OpError::Aborted("user cancelled".to_string());
        assert_eq!(err.to_string(), "Op aborted: user cancelled");
    }

    // TEST108: Clone an OpError::ExecutionFailed and verify the clone is identical
    #[test]
    fn test_108_op_error_clone_execution_failed() {
        let err = OpError::ExecutionFailed("fail msg".to_string());
        let cloned = err.clone();
        assert_eq!(err.to_string(), cloned.to_string());
        match cloned {
            OpError::ExecutionFailed(msg) => assert_eq!(msg, "fail msg"),
            _ => panic!("wrong variant"),
        }
    }

    // TEST109: Clone OpError::Timeout and verify timeout_ms is preserved
    #[test]
    fn test_109_op_error_clone_timeout() {
        let err = OpError::Timeout { timeout_ms: 500 };
        let cloned = err.clone();
        match cloned {
            OpError::Timeout { timeout_ms } => assert_eq!(timeout_ms, 500),
            _ => panic!("wrong variant"),
        }
    }

    // TEST110: Clone OpError::Other and verify it becomes ExecutionFailed with the error message preserved
    #[test]
    fn test_110_op_error_clone_other_converts_to_execution_failed() {
        use std::io;
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file missing");
        let err = OpError::Other(Box::new(io_err));
        let cloned = err.clone();
        // Other cannot be cloned directly — it converts to ExecutionFailed preserving the message
        match cloned {
            OpError::ExecutionFailed(msg) => assert!(msg.contains("file missing")),
            _ => panic!("expected ExecutionFailed from cloned Other"),
        }
    }

    // TEST111: Convert a serde_json::Error into OpError via From impl
    #[test]
    fn test_111_op_error_from_serde_json_error() {
        let json_err = serde_json::from_str::<i32>("not_a_number").unwrap_err();
        let op_err: OpError = json_err.into();
        // Must be the Other variant wrapping the serde error
        match op_err {
            OpError::Other(_) => {}
            _ => panic!("expected Other variant from serde_json::Error conversion"),
        }
    }
}
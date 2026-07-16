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

    /// A CLASSIFIED failure inside wrapping context (a batch child, a
    /// trigger-wrapped op, …) — the wrapper preserves the origin's failure
    /// identity instead of flattening it into prose. `chain` is the wrapping
    /// text (which op, which index) for humans; class/code/reason are the
    /// origin's, verbatim.
    #[error("{chain}")]
    WrappedClassified {
        chain: String,
        code: String,
        class: crate::failure::FailureClass,
        reason: String,
    },
    
    #[error("Op aborted: {0}")]
    Aborted(String),
    
    #[error("Trigger error: {0}")]
    Trigger(String),

    /// A failure carrying its FULL identity from the emit source: the
    /// machine-readable `code` the origin error declares (`error_code()`),
    /// the failure CLASS it declares (`failure_class()` — whose problem it
    /// is), and the leaf human message. Wrapping layers construct this from
    /// classified origins instead of folding everything into prose; the
    /// engine's run record and retry policy read it structurally.
    #[error("{code}: {message}")]
    Classified {
        code: String,
        class: crate::failure::FailureClass,
        message: String,
    },
    
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl OpError {
    /// The failure class this error DECLARES. Classified variants carry
    /// their origin's declaration; everything else is `Internal` —
    /// unclassified means "ours", never a guess (docs/failure-taxonomy.md).
    pub fn failure_class(&self) -> crate::failure::FailureClass {
        match self {
            Self::Classified { class, .. } => *class,
            Self::WrappedClassified { class, .. } => *class,
            _ => crate::failure::FailureClass::Internal,
        }
    }

    /// The machine-readable code declared at the emit source, when the
    /// failure carried one.
    pub fn failure_code(&self) -> Option<&str> {
        match self {
            Self::Classified { code, .. } => Some(code),
            Self::WrappedClassified { code, .. } => Some(code),
            _ => None,
        }
    }

    /// The LEAF human reason — the origin's own message for classified
    /// failures, the Display chain otherwise.
    pub fn failure_reason(&self) -> String {
        match self {
            Self::Classified { message, .. } => message.clone(),
            Self::WrappedClassified { reason, .. } => reason.clone(),
            other => other.to_string(),
        }
    }
}

impl Clone for OpError {
    fn clone(&self) -> Self {
        match self {
            Self::ExecutionFailed(msg) => Self::ExecutionFailed(msg.clone()),
            Self::Timeout { timeout_ms } => Self::Timeout { timeout_ms: *timeout_ms },
            Self::Context(msg) => Self::Context(msg.clone()),
            Self::BatchFailed(msg) => Self::BatchFailed(msg.clone()),
            Self::WrappedClassified { chain, code, class, reason } => {
                Self::WrappedClassified {
                    chain: chain.clone(),
                    code: code.clone(),
                    class: *class,
                    reason: reason.clone(),
                }
            }
            Self::Aborted(msg) => Self::Aborted(msg.clone()),
            Self::Trigger(msg) => Self::Trigger(msg.clone()),
            Self::Classified { code, class, message } => Self::Classified {
                code: code.clone(),
                class: *class,
                message: message.clone(),
            },
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

    // TEST0104: Verify OpError::ExecutionFailed displays with the correct message format
    #[test]
    fn test0104_op_error_display_execution_failed() {
        let err = OpError::ExecutionFailed("something broke".to_string());
        assert_eq!(err.to_string(), "Op execution failed: something broke");
    }

    // TEST0105: Verify OpError::Timeout displays with the correct timeout_ms value
    #[test]
    fn test0105_op_error_display_timeout() {
        let err = OpError::Timeout { timeout_ms: 250 };
        assert_eq!(err.to_string(), "Op timeout after 250ms");
    }

    // TEST0106: Verify OpError::Context displays with the correct message format
    #[test]
    fn test0106_op_error_display_context() {
        let err = OpError::Context("missing key".to_string());
        assert_eq!(err.to_string(), "Context error: missing key");
    }

    // TEST0107: Verify OpError::Aborted displays with the correct message format
    #[test]
    fn test0107_op_error_display_aborted() {
        let err = OpError::Aborted("user cancelled".to_string());
        assert_eq!(err.to_string(), "Op aborted: user cancelled");
    }

    // TEST0108: Clone an OpError::ExecutionFailed and verify the clone is identical
    #[test]
    fn test0108_op_error_clone_execution_failed() {
        let err = OpError::ExecutionFailed("fail msg".to_string());
        let cloned = err.clone();
        assert_eq!(err.to_string(), cloned.to_string());
        match cloned {
            OpError::ExecutionFailed(msg) => assert_eq!(msg, "fail msg"),
            _ => panic!("wrong variant"),
        }
    }

    // TEST0109: Clone OpError::Timeout and verify timeout_ms is preserved
    #[test]
    fn test0109_op_error_clone_timeout() {
        let err = OpError::Timeout { timeout_ms: 500 };
        let cloned = err.clone();
        match cloned {
            OpError::Timeout { timeout_ms } => assert_eq!(timeout_ms, 500),
            _ => panic!("wrong variant"),
        }
    }

    // TEST0110: Clone OpError::Other and verify it becomes ExecutionFailed with the error message preserved
    #[test]
    fn test0110_op_error_clone_other_converts_to_execution_failed() {
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

    // TEST1901: classified variants carry the emit source's identity through
    // the accessors; unclassified variants are Internal with no code — the
    // taxonomy's own rule (docs/failure-taxonomy.md).
    #[test]
    fn test1901_classified_accessors() {
        use crate::failure::FailureClass;

        let classified = OpError::Classified {
            code: "CONTEXT_OVERFLOW".to_string(),
            class: FailureClass::Input,
            message: "prompt too large".to_string(),
        };
        assert_eq!(classified.failure_class(), FailureClass::Input);
        assert_eq!(classified.failure_code(), Some("CONTEXT_OVERFLOW"));
        assert_eq!(classified.failure_reason(), "prompt too large");
        assert_eq!(classified.to_string(), "CONTEXT_OVERFLOW: prompt too large");

        let wrapped = OpError::WrappedClassified {
            chain: "Op 3-generate failed: CONTEXT_OVERFLOW: prompt too large".to_string(),
            code: "CONTEXT_OVERFLOW".to_string(),
            class: FailureClass::Input,
            reason: "prompt too large".to_string(),
        };
        assert_eq!(wrapped.failure_class(), FailureClass::Input);
        assert_eq!(wrapped.failure_code(), Some("CONTEXT_OVERFLOW"));
        assert_eq!(
            wrapped.failure_reason(),
            "prompt too large",
            "the reason is the LEAF message, not the wrap chain"
        );
        assert_eq!(
            wrapped.to_string(),
            "Op 3-generate failed: CONTEXT_OVERFLOW: prompt too large",
            "Display keeps the human chain"
        );

        let plain = OpError::ExecutionFailed("boom".to_string());
        assert_eq!(plain.failure_class(), FailureClass::Internal);
        assert_eq!(plain.failure_code(), None);
    }

    // TEST1902: cloning a classified error preserves its full identity —
    // the run-record path clones the terminal error before persisting.
    #[test]
    fn test1902_clone_preserves_classification() {
        use crate::failure::FailureClass;

        let original = OpError::WrappedClassified {
            chain: "Op 'x' failed: GPU_OUT_OF_MEMORY: no VRAM".to_string(),
            code: "GPU_OUT_OF_MEMORY".to_string(),
            class: FailureClass::Resource,
            reason: "no VRAM".to_string(),
        };
        let cloned = original.clone();
        assert_eq!(cloned.failure_class(), FailureClass::Resource);
        assert_eq!(cloned.failure_code(), Some("GPU_OUT_OF_MEMORY"));
        assert_eq!(cloned.failure_reason(), "no VRAM");
    }

    // TEST0111: Convert a serde_json::Error into OpError via From impl
    #[test]
    fn test0111_op_error_from_serde_json_error() {
        let json_err = serde_json::from_str::<i32>("not_a_number").unwrap_err();
        let op_err: OpError = json_err.into();
        // Must be the Other variant wrapping the serde error
        match op_err {
            OpError::Other(_) => {}
            _ => panic!("expected Other variant from serde_json::Error conversion"),
        }
    }
}

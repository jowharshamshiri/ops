//! The failure taxonomy — WHOSE problem a failure is.
//!
//! Defined ONCE here in `ops` (the leaf crate of the error path — capdag
//! depends on ops, so this is the deepest shared home) and re-exported by
//! capdag as the cartridge-contract surface (`capdag::FailureClass`).
//! Cartridge error enums declare a `failure_class()` per variant beside
//! `error_code()`; the bifaci ERR frame carries the class over the wire (all
//! four language runtimes mirror it); the orchestrator and the engine carry
//! it structurally to the run record. No layer ever infers another layer's
//! class from message text — an error that reaches a boundary without a
//! declared class is `Internal` (unclassified means "ours", never a guess).
//! See `docs/failure-taxonomy.md` (repo root) for the full architecture and
//! `capdag/docs/17.2-error-handling.md` for the protocol contract.

use serde::{Deserialize, Serialize};

/// Whose problem a failure is. Declared at the error's DEFINITION site,
/// carried structurally through every hop.
///
/// `Default` is `Internal` — the taxonomy's own rule made mechanical: an
/// error deserialized without a declared class is unclassified, and
/// unclassified means "ours".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FailureClass {
    /// Deterministic on the INPUT (context overflow, invalid request,
    /// unsupported format). The user's to fix; retrying can never succeed —
    /// tasks failing with this class are marked permanently failed.
    Input,
    /// A compute resource was exhausted (GPU VRAM, host memory). Often
    /// transient (another process holding memory) — retryable.
    Resource,
    /// The environment failed (network, registry, model download/integrity,
    /// cartridge process death). Transient by nature — retryable.
    Environment,
    /// Everything else: a defect in the engine or a cartridge. Ours, said
    /// plainly. Retryable (races un-race), but never blamed on the user.
    #[default]
    Internal,
}

impl FailureClass {
    /// The wire token — used in the ERR frame meta, the machine_runs
    /// columns, the gRPC proto, and the loom. One vocabulary everywhere.
    pub fn as_str(&self) -> &'static str {
        match self {
            FailureClass::Input => "input",
            FailureClass::Resource => "resource",
            FailureClass::Environment => "environment",
            FailureClass::Internal => "internal",
        }
    }

    /// Parse a wire token. Unknown tokens are a PROTOCOL error, not a
    /// fallback case — the caller decides whether to fail hard (engine
    /// receipt of a malformed frame) or treat the frame as unclassified.
    pub fn from_wire(token: &str) -> Option<FailureClass> {
        match token {
            "input" => Some(FailureClass::Input),
            "resource" => Some(FailureClass::Resource),
            "environment" => Some(FailureClass::Environment),
            "internal" => Some(FailureClass::Internal),
            _ => None,
        }
    }

    /// Whether retrying can NEVER succeed: the failure is a deterministic
    /// function of the input. Resource/environment/internal stay retryable
    /// (memory frees up, networks recover, races un-race).
    pub fn is_permanent(&self) -> bool {
        matches!(self, FailureClass::Input)
    }
}

impl std::fmt::Display for FailureClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TEST1730: the wire vocabulary round-trips exactly and rejects unknowns.
    #[test]
    fn test1730_wire_tokens_round_trip() {
        for class in [
            FailureClass::Input,
            FailureClass::Resource,
            FailureClass::Environment,
            FailureClass::Internal,
        ] {
            assert_eq!(FailureClass::from_wire(class.as_str()), Some(class));
        }
        assert_eq!(FailureClass::from_wire("user-error"), None);
        assert_eq!(FailureClass::from_wire(""), None);
    }

    // TEST1731: only Input is permanent — the retry machinery keys on this.
    #[test]
    fn test1731_only_input_is_permanent() {
        assert!(FailureClass::Input.is_permanent());
        assert!(!FailureClass::Resource.is_permanent());
        assert!(!FailureClass::Environment.is_permanent());
        assert!(!FailureClass::Internal.is_permanent());
    }
}

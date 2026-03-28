// =============================================================================
// vil_types::signals — Control Plane Signals
// =============================================================================
// Out-of-band control signals for the reactive session fabric.
// Used by vil_rt::session and all reactive adapters.
//
// Signals are sent via the Control Lane, separate from the Data Lane,
// so that session termination is never blocked by payload congestion.
// =============================================================================

use serde::{Deserialize, Serialize};

/// Out-of-band control signal for the reactive session fabric.
///
/// Sent via the Control Lane and consumed by the session registry
/// to manage session lifecycle deterministically.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ControlSignal {
    /// Session completed normally. All data has been sent.
    Done { session_id: u64 },

    /// Session failed with an error. Includes code and reason.
    Error {
        session_id: u64,
        code: u16,
        reason: String,
    },

    /// Session forcibly aborted (disconnect, timeout, etc.).
    Abort { session_id: u64 },
}

// SAFETY: ControlSignal is a repr(u8) enum with defined variants — no pointers or address-dependent data.
unsafe impl crate::markers::Vasi for ControlSignal {}
// PodLike is NOT implemented because of String in Error variant

impl crate::markers::MessageContract for ControlSignal {
    const META: crate::specs::MessageMeta = crate::specs::MessageMeta {
        name: "ControlSignal",
        layout: crate::enums::LayoutProfile::Relative,
        transfer_caps: &[
            crate::enums::TransferMode::LoanWrite,
            crate::enums::TransferMode::LoanRead,
        ],
        is_stable: false, // Contains String in Error variant
        semantic_kind: crate::enums::SemanticKind::Event,
        memory_class: crate::enums::MemoryClass::ControlHeap,
    };
}

impl ControlSignal {
    /// Create a Done signal for the given session.
    pub fn done(session_id: u64) -> Self {
        Self::Done { session_id }
    }

    /// Create an Error signal for the given session.
    pub fn error(session_id: u64, code: u16, reason: impl Into<String>) -> Self {
        Self::Error {
            session_id,
            code,
            reason: reason.into(),
        }
    }

    /// Create an Abort signal for the given session.
    pub fn abort(session_id: u64) -> Self {
        Self::Abort { session_id }
    }

    /// Extract the session_id from any signal variant.
    pub fn session_id(&self) -> u64 {
        match self {
            Self::Done { session_id } => *session_id,
            Self::Error { session_id, .. } => *session_id,
            Self::Abort { session_id } => *session_id,
        }
    }

    /// Whether this signal indicates end of session (Done or Abort).
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Done { .. } | Self::Abort { .. })
    }
}

impl std::fmt::Display for ControlSignal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Done { session_id } => write!(f, "DONE(session={})", session_id),
            Self::Error {
                session_id,
                code,
                reason,
            } => {
                write!(
                    f,
                    "ERROR(session={}, code={}, reason={})",
                    session_id, code, reason
                )
            }
            Self::Abort { session_id } => write!(f, "ABORT(session={})", session_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_signal_done() {
        let sig = ControlSignal::done(42);
        assert_eq!(sig.session_id(), 42);
        assert!(sig.is_terminal());
    }

    #[test]
    fn test_control_signal_error() {
        let sig = ControlSignal::error(7, 500, "upstream failed");
        assert_eq!(sig.session_id(), 7);
        assert!(!sig.is_terminal());
    }

    #[test]
    fn test_control_signal_abort() {
        let sig = ControlSignal::abort(99);
        assert_eq!(sig.session_id(), 99);
        assert!(sig.is_terminal());
    }

    #[test]
    fn test_control_signal_display() {
        let sig = ControlSignal::done(1);
        assert_eq!(format!("{}", sig), "DONE(session=1)");
    }
}

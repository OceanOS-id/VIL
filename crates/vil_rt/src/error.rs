// =============================================================================
// vil_rt::error — Runtime Error Types
// =============================================================================
// Error types for the VIL runtime.
// Each error variant is clear and actionable — developers should not need
// to guess what went wrong.
//
// TASK LIST:
// [x] RtError enum
// [x] Display + Error impls
// =============================================================================

use std::fmt;

use vil_types::{PortId, SampleId};

/// Errors that can occur in the VIL runtime.
#[derive(Debug)]
pub enum RtError {
    /// Unknown port ID.
    UnknownPort(PortId),
    /// Port name not found on ProcessHandle.
    UnknownPortName(String),
    /// Port has no route (not yet connected).
    PortHasNoRoute(PortId),
    /// Loan was not initialized before publish.
    LoanWasNeverInitialized(SampleId),
    /// Sample not found in shared store.
    MissingSample(SampleId),
    /// Sample type mismatch on downcast.
    TypeMismatch(&'static str),
    /// Queue empty on recv.
    QueueEmpty(PortId),
    /// Heap full during data plane allocation.
    HeapFull(u64),
}

impl fmt::Display for RtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPort(id) => write!(f, "unknown port {}", id),
            Self::UnknownPortName(name) => write!(f, "unknown port name '{}'", name),
            Self::PortHasNoRoute(id) => write!(f, "port {} has no route (not connected)", id),
            Self::LoanWasNeverInitialized(id) => {
                write!(f, "loan {} was never initialized before publish", id)
            }
            Self::MissingSample(id) => write!(f, "sample {} not found in shared store", id),
            Self::TypeMismatch(expected) => write!(f, "sample type mismatch, expected {}", expected),
            Self::QueueEmpty(id) => write!(f, "queue for port {} is empty", id),
            Self::HeapFull(id) => write!(f, "shared memory heap {} is full", id),
        }
    }
}

impl std::error::Error for RtError {}

// =============================================================================
// vil_types::ids — Identity Types
// =============================================================================
// Identity types for the entire VIL domain. All IDs are Copy, Clone,
// Hash, Eq, Ord — suitable as keys in HashMap/BTreeMap and for
// runtime comparisons.
//
// TASK LIST:
// [x] ProcessId — semantic process identity
// [x] PortId — communication port identity
// [x] InterfaceId — interface contract identity
// [x] SampleId — sample/loan identity on shared heap
// [x] RegionId — allocation region identity on exchange heap
// [x] Epoch — process generation version (for crash detection)
// [x] HostId — unique machine identity in cluster
// =============================================================================

use core::fmt;
use serde::{Serialize, Deserialize};

/// Unique process identity within the VIL runtime.
/// A process is the unit of execution and semantic failure domain.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProcessId(pub u64);

/// Unique communication port identity.
/// A port is the entry/exit point for messages on a process.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PortId(pub u64);

/// Unique interface contract identity.
/// An interface defines a set of ports forming a communication contract.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct InterfaceId(pub u64);

/// Unique sample identity on the shared exchange heap.
/// Each loan/publish produces one SampleId.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SampleId(pub u64);

/// Allocation region identity on the exchange heap.
/// Regions enable allocation isolation per message-class or per use-case.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RegionId(pub u64);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Epoch(pub u64);

/// Unique machine identity within the VIL cluster.
/// Used for remote DMA identification and cross-host routing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct HostId(pub u32);

// --- Display implementations ---

impl fmt::Display for ProcessId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Process({})", self.0)
    }
}

impl fmt::Display for PortId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Port({})", self.0)
    }
}

impl fmt::Display for SampleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sample({})", self.0)
    }
}

impl fmt::Display for RegionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Region({})", self.0)
    }
}

impl fmt::Display for Epoch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Epoch({})", self.0)
    }
}

impl fmt::Display for InterfaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Interface({})", self.0)
    }
}

impl fmt::Display for HostId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Host({})", self.0)
    }
}

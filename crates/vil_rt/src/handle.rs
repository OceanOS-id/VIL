// =============================================================================
// vil_rt::handle — Process Handle & Registered Port
// =============================================================================
// ProcessHandle is the "key" held by every registered process in the runtime.
// Through this handle, a process can:
//   - look up its port IDs
//   - access the RuntimeWorld for loan/publish/recv
//
// TASK LIST:
// [x] RegisteredPort — port ID + spec
// [x] ProcessHandle — per-process runtime handle
// =============================================================================

use std::collections::HashMap;

use vil_types::{CleanupPolicy, PortId, PortSpec, ProcessId, ProcessSpec};

use crate::error::RtError;
use crate::world::VastarRuntimeWorld;

/// Port registered in the runtime, with an assigned ID.
#[derive(Clone, Copy, Debug)]
pub struct RegisteredPort {
    /// Runtime-assigned ID for this port.
    pub id: PortId,
    /// Original port specification.
    pub spec: PortSpec,
}

/// Handle held by a process after registration.
///
/// Through this handle, a process can access its ports and
/// the RuntimeWorld for loan/publish/recv operations.
#[derive(Clone)]
pub struct ProcessHandle {
    pub(crate) process_id: ProcessId,
    pub(crate) spec: ProcessSpec,
    pub(crate) ports: HashMap<String, RegisteredPort>,
    pub(crate) world: VastarRuntimeWorld,
}

impl ProcessHandle {
    /// Process ID assigned by the runtime.
    pub fn id(&self) -> ProcessId {
        self.process_id
    }

    /// Cleanup policy for this process.
    pub fn cleanup_policy(&self) -> CleanupPolicy {
        self.spec.cleanup
    }

    /// Get port ID by name.
    pub fn port_id(&self, name: &str) -> Result<PortId, RtError> {
        self.ports
            .get(name)
            .map(|registered| registered.id)
            .ok_or_else(|| RtError::UnknownPortName(name.to_string()))
    }

    /// Access the RuntimeWorld (for loan/publish/recv).
    pub fn world(&self) -> VastarRuntimeWorld {
        self.world.clone()
    }

    /// List all registered ports.
    pub fn registered_ports(&self) -> &HashMap<String, RegisteredPort> {
        &self.ports
    }
}

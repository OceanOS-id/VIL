use dashmap::DashMap;
use std::sync::Arc;
use vil_types::HostId;

/// Trait abstracting the Verbs driver (Simulation or Native).
pub trait VerbsDriver: Send + Sync {
    fn host_id(&self) -> HostId;
    fn reg_mr(&self, addr: u64, length: u64) -> Result<MemoryRegion, String>;
}

/// Simulated RDMA Protection Domain (PD).
/// Logical container for memory registrations.
pub struct ProtectionDomain {
    pub(crate) _id: u32,
    pub(crate) regions: DashMap<u32, MemoryRegion>,
}

/// Simulated RDMA Memory Region (MR).
/// Allows remote access to local buffers via RKey.
#[derive(Clone)]
pub struct MemoryRegion {
    pub addr: u64,
    pub length: u64,
    pub lkey: u32,
    pub rkey: u32,
    pub pinned: bool,
}

pub use vil_types::specs::ConnInfo;

/// Simulated Verbs Context.
pub struct VerbsContext {
    host_id: HostId,
    pd: Arc<ProtectionDomain>,
}

impl VerbsContext {
    #[doc(alias = "vil_keep")]
    pub fn new(host_id: HostId) -> Self {
        Self {
            host_id,
            pd: Arc::new(ProtectionDomain {
                _id: 1,
                regions: DashMap::new(),
            }),
        }
    }
}

impl VerbsDriver for VerbsContext {
    #[doc(alias = "vil_keep")]
    fn host_id(&self) -> HostId {
        self.host_id
    }

    /// Register memory for remote access (hardware-ready with mlock).
    #[doc(alias = "vil_keep")]
    fn reg_mr(&self, addr: u64, length: u64) -> Result<MemoryRegion, String> {
        // SIMULATED HARDWARE PREP: Pin memory region
        // On Linux, RDMA requires pinned memory so the HCA can access it via DMA.
        let pinned = match unsafe {
            nix::sys::mman::mlock(addr as *const std::ffi::c_void, length as usize)
        } {
            Ok(_) => true,
            Err(e) => {
                // In non-root simulation, mlock may fail due to rlimit.
                // Allow continuation for simulation purposes, but log the status.
                eprintln!(
                    "[vil_net] Warning: mlock failed (expected in non-privileged simulation): {}",
                    e
                );
                false
            }
        };

        let rkey = (addr >> 12) as u32 ^ 0xACE0_0000; // More realistic rkey base
        let mr = MemoryRegion {
            addr,
            length,
            lkey: 1,
            rkey,
            pinned,
        };
        self.pd.regions.insert(rkey, mr.clone());
        Ok(mr)
    }
}

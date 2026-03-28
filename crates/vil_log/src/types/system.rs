// =============================================================================
// vil_log::types::system — SystemPayload
// =============================================================================
//
// OS / system resource event payload (CPU, memory, I/O, signals).
// =============================================================================

/// System resource event payload. Fits in 192 bytes.
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct SystemPayload {
    /// CPU usage percentage * 100 (e.g. 7550 = 75.50%).
    pub cpu_pct_x100: u16,
    /// Memory usage in kilobytes.
    pub mem_kb: u32,
    /// Available memory in kilobytes.
    pub mem_avail_kb: u32,
    /// File descriptor count.
    pub fd_count: u32,
    /// Thread count.
    pub thread_count: u32,
    /// Number of open sockets.
    pub socket_count: u32,
    /// Event type: 0=metrics 1=signal 2=oom 3=panic 4=startup 5=shutdown
    pub event_type: u8,
    /// Signal number (if event_type == signal).
    pub signal_num: u8,
    /// Exit code (if event_type == shutdown).
    pub exit_code: u8,
    /// Padding.
    pub _pad: u8,
    /// Disk read bytes (this interval).
    pub disk_read_bytes: u64,
    /// Disk write bytes (this interval).
    pub disk_write_bytes: u64,
    /// Network receive bytes (this interval).
    pub net_rx_bytes: u64,
    /// Network transmit bytes (this interval).
    pub net_tx_bytes: u64,
    /// Inline extended data (msgpack).
    pub meta_bytes: [u8; 128],
}

impl Default for SystemPayload {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

const _: () = {
    assert!(
        std::mem::size_of::<SystemPayload>() <= 192,
        "SystemPayload must fit within 192 bytes"
    );
};

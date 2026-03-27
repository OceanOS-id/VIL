// =============================================================================
// vil_trigger_evm::error — EvmFault
// =============================================================================
//
// VIL-compliant plain enum fault for EVM trigger operations.
// No thiserror, no String fields — COMPLIANCE.md §4.
// =============================================================================

/// Fault type for all EVM blockchain trigger operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvmFault {
    /// Failed to connect to the JSON-RPC endpoint.
    ConnectionFailed {
        /// FxHash of the RPC URL.
        url_hash: u32,
        /// Transport error kind code.
        reason_code: u32,
    },
    /// The eth_subscribe call was rejected by the node.
    SubscribeFailed {
        /// FxHash of the event signature.
        sig_hash: u32,
        /// RPC error code.
        rpc_code: i32,
    },
    /// The subscription stream closed unexpectedly.
    StreamClosed {
        /// FxHash of the RPC URL.
        url_hash: u32,
    },
    /// A log entry could not be decoded.
    DecodeFailed {
        /// Block number where the failure occurred.
        block_number: u64,
        /// Log index in the block.
        log_index: u32,
    },
    /// The contract address is malformed.
    InvalidAddress {
        /// FxHash of the raw address string.
        addr_hash: u32,
    },
}

impl EvmFault {
    /// Return a stable numeric code for log fields.
    pub fn as_error_code(&self) -> u32 {
        match self {
            EvmFault::ConnectionFailed { .. } => 1,
            EvmFault::SubscribeFailed { .. }  => 2,
            EvmFault::StreamClosed { .. }     => 3,
            EvmFault::DecodeFailed { .. }     => 4,
            EvmFault::InvalidAddress { .. }   => 5,
        }
    }
}

// =============================================================================
// vil_trigger_evm::events — connector events emitted on Data Lane
// =============================================================================

use vil_connector_macros::connector_event;

/// Emitted when an EVM log event is received from the blockchain.
#[connector_event]
pub struct LogEmitted {
    /// FxHash of the contract address string.
    pub contract_hash: u32,
    /// FxHash of the event signature string.
    pub sig_hash: u32,
    /// Block number where the log was emitted (truncated to u32 for compliance).
    pub block_number_lo: u32,
    /// Log index within the block.
    pub log_index: u32,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}

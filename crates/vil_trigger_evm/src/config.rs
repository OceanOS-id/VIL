// =============================================================================
// vil_trigger_evm::config — EvmConfig
// =============================================================================
//
// Configuration for the Ethereum EVM log subscription trigger.
// =============================================================================

/// Configuration for the VIL EVM blockchain event trigger.
///
/// # Example YAML
/// ```yaml
/// evm:
///   rpc_url: "wss://mainnet.infura.io/ws/v3/YOUR_KEY"
///   contract_address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
///   event_signature: "Transfer(address,address,uint256)"
/// ```
#[derive(Debug, Clone)]
pub struct EvmConfig {
    /// Ethereum JSON-RPC WebSocket URL.
    pub rpc_url: String,
    /// Contract address to watch (hex, with 0x prefix).
    pub contract_address: String,
    /// Event signature string for topic0 filter (e.g. "Transfer(address,address,uint256)").
    pub event_signature: String,
}

impl EvmConfig {
    /// Construct a new `EvmConfig`.
    pub fn new(
        rpc_url: impl Into<String>,
        contract_address: impl Into<String>,
        event_signature: impl Into<String>,
    ) -> Self {
        Self {
            rpc_url: rpc_url.into(),
            contract_address: contract_address.into(),
            event_signature: event_signature.into(),
        }
    }
}

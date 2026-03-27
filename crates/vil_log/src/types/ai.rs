// =============================================================================
// vil_log::types::ai — AiPayload
// =============================================================================
//
// AI/LLM inference log payload. Tracks model invocation metadata.
// =============================================================================

/// AI/LLM inference event payload. Fits in 192 bytes.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AiPayload {
    /// FxHash of the model name (e.g. "gpt-4o").
    pub model_hash: u32,
    /// FxHash of the provider name (e.g. "openai").
    pub provider_hash: u32,
    /// Number of input tokens consumed.
    pub input_tokens: u32,
    /// Number of output tokens generated.
    pub output_tokens: u32,
    /// Inference latency in microseconds.
    pub latency_us: u32,
    /// Total cost in micro-USD (e.g. 1000 = $0.001).
    pub cost_micro_usd: u32,
    /// HTTP status from provider (0 = no HTTP call).
    pub provider_status: u16,
    /// Operation type: 0=chat 1=completion 2=embed 3=rerank 4=image
    pub op_type: u8,
    /// Whether streaming was used.
    pub streaming: u8,
    /// Number of retry attempts.
    pub retries: u8,
    /// Cache hit: 0=miss 1=exact 2=semantic
    pub cache_hit: u8,
    /// Padding.
    pub _pad: [u8; 2],
    /// Inline request/response metadata (msgpack).
    pub meta_bytes: [u8; 160],
}

impl Default for AiPayload {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

const _: () = {
    assert!(
        std::mem::size_of::<AiPayload>() <= 192,
        "AiPayload must fit within 192 bytes"
    );
};

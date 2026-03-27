// 1 byte enum — stack-allocated.

/// Transaction scope hint. Zero-cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TxScope {
    ReadOnly,
    ReadWrite,
    RequiresNew,
    JoinIfPresent,
    None,
}

impl Default for TxScope { fn default() -> Self { Self::None } }

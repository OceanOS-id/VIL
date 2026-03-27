// 1 byte enum — compile-time annotation, zero runtime cost.

/// Portability tier for DB operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PortabilityTier {
    /// P0: Portable core — works on all SQL providers.
    P0,
    /// P1: Capability-gated — needs specific provider feature.
    P1,
    /// P2: Provider-specific — may need changes on switch.
    P2,
}

impl std::fmt::Display for PortabilityTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::P0 => write!(f, "P0 (Portable)"),
            Self::P1 => write!(f, "P1 (Capability-Gated)"),
            Self::P2 => write!(f, "P2 (Provider-Specific)"),
        }
    }
}

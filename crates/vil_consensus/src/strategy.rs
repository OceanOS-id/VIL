/// Combination strategy for selecting the best response from multiple providers.
#[derive(Debug, Clone)]
pub enum ConsensusStrategy {
    /// Return the longest response (assumes more detail = better).
    Longest,
    /// Return the response most similar to others (majority agreement).
    MajorityAgreement,
    /// Score each response and pick highest.
    BestOfN,
    /// Weighted by provider confidence/reputation.
    Weighted(Vec<f32>),
    /// Custom scoring function (uses BestOfN internally; override scores externally).
    Custom,
}

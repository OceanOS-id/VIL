use std::sync::Arc;
use std::time::Instant;

use serde::Serialize;
use tokio::task::JoinSet;
use vil_llm::{ChatMessage, LlmProvider};
use vil_log::app_log;
use vil_macros::VilAiEvent;

use crate::config::ConsensusConfig;
use crate::scorer::{score_response, text_similarity};
use crate::strategy::ConsensusStrategy;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors returned by the consensus engine.
#[derive(Debug)]
pub enum ConsensusError {
    /// All providers failed.
    AllProvidersFailed(Vec<String>),
    /// Not enough successful responses (below min_responses).
    InsufficientResponses { got: usize, need: usize },
}

impl std::fmt::Display for ConsensusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AllProvidersFailed(errs) => {
                write!(f, "all providers failed: {}", errs.join("; "))
            }
            Self::InsufficientResponses { got, need } => {
                write!(f, "got {} responses, need at least {}", got, need)
            }
        }
    }
}

impl std::error::Error for ConsensusError {}

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// Response from a single provider.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderResponse {
    pub provider: String,
    pub model: String,
    pub content: String,
    pub latency_ms: u64,
    pub score: f32,
    pub error: Option<String>,
}

/// Combined consensus result.
#[derive(Debug, Clone, Serialize, VilAiEvent)]
pub struct ConsensusResult {
    pub answer: String,
    pub model: String,
    pub all_responses: Vec<ProviderResponse>,
    pub strategy_used: String,
    pub total_ms: u64,
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// Parallel multi-model inference engine with consensus-based result combination.
pub struct ConsensusEngine {
    providers: Vec<Arc<dyn LlmProvider>>,
    strategy: ConsensusStrategy,
    timeout_ms: u64,
}

impl ConsensusEngine {
    /// Create a new consensus engine with the given providers and strategy.
    pub fn new(providers: Vec<Arc<dyn LlmProvider>>, strategy: ConsensusStrategy) -> Self {
        Self {
            providers,
            strategy,
            timeout_ms: 30_000,
        }
    }

    /// Create from a full config.
    pub fn from_config(providers: Vec<Arc<dyn LlmProvider>>, config: ConsensusConfig) -> Self {
        Self {
            providers,
            strategy: config.strategy,
            timeout_ms: config.timeout_ms,
        }
    }

    /// Number of registered providers.
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    /// Name of the configured consensus strategy.
    pub fn strategy_name(&self) -> String {
        match &self.strategy {
            ConsensusStrategy::Longest => "longest".into(),
            ConsensusStrategy::MajorityAgreement => "majority_agreement".into(),
            ConsensusStrategy::BestOfN => "best_of_n".into(),
            ConsensusStrategy::Weighted(_) => "weighted".into(),
            ConsensusStrategy::Custom => "custom".into(),
        }
    }

    /// Set the per-provider timeout.
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Run parallel inference across all providers and combine results.
    ///
    /// Latency = max(individual latencies), not sum.
    pub async fn query(&self, messages: &[ChatMessage]) -> Result<ConsensusResult, ConsensusError> {
        let start = Instant::now();

        // 1. Spawn all provider calls concurrently.
        let mut join_set = JoinSet::new();

        for provider in &self.providers {
            let provider = Arc::clone(provider);
            let msgs: Vec<ChatMessage> = messages.to_vec();
            let timeout = self.timeout_ms;

            join_set.spawn(async move {
                let t0 = Instant::now();
                let result = tokio::time::timeout(
                    std::time::Duration::from_millis(timeout),
                    provider.chat(&msgs),
                )
                .await;

                let latency_ms = t0.elapsed().as_millis() as u64;
                let pname = provider.provider_name().to_string();
                let mname = provider.model().to_string();

                match result {
                    Ok(Ok(resp)) => ProviderResponse {
                        provider: pname,
                        model: mname,
                        content: resp.content,
                        latency_ms,
                        score: 0.0,
                        error: None,
                    },
                    Ok(Err(e)) => ProviderResponse {
                        provider: pname,
                        model: mname,
                        content: String::new(),
                        latency_ms,
                        score: 0.0,
                        error: Some(format!("{}", e)),
                    },
                    Err(_) => ProviderResponse {
                        provider: pname,
                        model: mname,
                        content: String::new(),
                        latency_ms,
                        score: 0.0,
                        error: Some("timeout".to_string()),
                    },
                }
            });
        }

        // 2. Collect results.
        let mut all_responses: Vec<ProviderResponse> = Vec::new();
        while let Some(res) = join_set.join_next().await {
            match res {
                Ok(pr) => all_responses.push(pr),
                Err(e) => {
                    app_log!(Warn, "consensus_provider_panic", { error: e.to_string() });
                }
            }
        }

        // 3. Separate successes and failures.
        let mut successes: Vec<&mut ProviderResponse> = all_responses
            .iter_mut()
            .filter(|r| r.error.is_none())
            .collect();

        if successes.is_empty() {
            let errors: Vec<String> = all_responses
                .iter()
                .filter_map(|r| r.error.clone())
                .collect();
            return Err(ConsensusError::AllProvidersFailed(errors));
        }

        // 4. Score each successful response.
        for resp in successes.iter_mut() {
            let s = score_response(&resp.content);
            resp.score = s.total;
        }

        // 5. Apply strategy.
        let (winner_idx, strategy_name) = self.apply_strategy(&all_responses);

        let total_ms = start.elapsed().as_millis() as u64;
        let winner = &all_responses[winner_idx];

        app_log!(Info, "consensus_reached", { strategy: strategy_name.to_string(), model: winner.model.clone(), total_ms: total_ms });

        Ok(ConsensusResult {
            answer: winner.content.clone(),
            model: winner.model.clone(),
            all_responses,
            strategy_used: strategy_name,
            total_ms,
        })
    }

    /// Apply the configured strategy to pick a winner. Returns (index, strategy_name).
    fn apply_strategy(&self, responses: &[ProviderResponse]) -> (usize, String) {
        let successful: Vec<(usize, &ProviderResponse)> = responses
            .iter()
            .enumerate()
            .filter(|(_, r)| r.error.is_none())
            .collect();

        if successful.is_empty() {
            return (0, "none".to_string());
        }

        match &self.strategy {
            ConsensusStrategy::Longest => {
                let (idx, _) = successful
                    .iter()
                    .max_by_key(|(_, r)| r.content.len())
                    .unwrap();
                (*idx, "longest".to_string())
            }

            ConsensusStrategy::BestOfN | ConsensusStrategy::Custom => {
                let (idx, _) = successful
                    .iter()
                    .max_by(|(_, a), (_, b)| a.score.partial_cmp(&b.score).unwrap())
                    .unwrap();
                (*idx, "best_of_n".to_string())
            }

            ConsensusStrategy::MajorityAgreement => {
                // Pick the response with highest average similarity to all others.
                let mut best_idx = successful[0].0;
                let mut best_avg = f32::NEG_INFINITY;

                for &(i, ri) in &successful {
                    let avg: f32 = successful
                        .iter()
                        .filter(|&&(j, _)| j != i)
                        .map(|&(_, rj)| text_similarity(&ri.content, &rj.content))
                        .sum::<f32>()
                        / (successful.len() as f32 - 1.0).max(1.0);

                    if avg > best_avg {
                        best_avg = avg;
                        best_idx = i;
                    }
                }

                (best_idx, "majority_agreement".to_string())
            }

            ConsensusStrategy::Weighted(weights) => {
                let (idx, _) = successful
                    .iter()
                    .max_by(|(i_a, a), (i_b, b)| {
                        let wa = weights.get(*i_a).copied().unwrap_or(1.0);
                        let wb = weights.get(*i_b).copied().unwrap_or(1.0);
                        let score_a = a.score * wa;
                        let score_b = b.score * wb;
                        score_a.partial_cmp(&score_b).unwrap()
                    })
                    .unwrap();
                (*idx, "weighted".to_string())
            }
        }
    }
}

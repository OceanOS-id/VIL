//! Semantic types for web crawling operations.
//!
//! These types follow VIL's process-oriented semantic model:
//! - Events: immutable audit records (Data Lane)
//! - Faults: error signals (Control Lane)
//! - State: mutable tracked state (Data Lane)

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

// ── Events (Data Lane, immutable audit) ─────────────────────────────

/// Emitted after every page crawl completes.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct CrawlEvent {
    pub url: String,
    pub status: u16,
    pub latency_ms: u64,
    pub content_length: usize,
    pub depth: usize,
}

// ── Faults (Control Lane, error signals) ────────────────────────────

/// Classification of crawl failure modes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CrawlFaultType {
    FetchError,
    Timeout,
    RobotsDisallowed,
    DomainBlocked,
    TooManyErrors,
}

/// Emitted when a crawl operation fails.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct CrawlFault {
    pub url: String,
    pub error_type: CrawlFaultType,
    pub message: String,
    pub retry_possible: bool,
}

impl CrawlFault {
    pub fn fetch_error(url: &str, msg: &str) -> Self {
        Self {
            url: url.into(),
            error_type: CrawlFaultType::FetchError,
            message: msg.into(),
            retry_possible: true,
        }
    }

    pub fn timeout(url: &str) -> Self {
        Self {
            url: url.into(),
            error_type: CrawlFaultType::Timeout,
            message: "request timed out".into(),
            retry_possible: true,
        }
    }
}

// ── State (Data Lane, mutable tracked) ──────────────────────────────

/// Tracks cumulative crawler statistics.
#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct CrawlerState {
    pub pages_crawled: u64,
    pub errors: u64,
    pub total_bytes: u64,
    pub avg_latency_ms: f64,
}

impl CrawlerState {
    pub fn record(&mut self, event: &CrawlEvent) {
        self.pages_crawled += 1;
        self.total_bytes += event.content_length as u64;
        let n = self.pages_crawled as f64;
        self.avg_latency_ms =
            self.avg_latency_ms * (n - 1.0) / n + event.latency_ms as f64 / n;
    }

    pub fn record_error(&mut self) {
        self.errors += 1;
    }
}

// =============================================================================
// Auto-Reconnect — Exponential backoff reconnection to sidecars
// =============================================================================
//
// When a sidecar connection drops, this module handles reconnection with
// configurable exponential backoff, jitter, and re-handshake.

use crate::transport::{SidecarConnection, socket_path};
use crate::protocol::{Message, Handshake};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Policy controlling reconnection behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconnectPolicy {
    /// Maximum number of reconnect attempts. Default: 10.
    pub max_retries: u32,
    /// Base backoff in milliseconds. Default: 100.
    pub base_backoff_ms: u64,
    /// Maximum backoff in milliseconds. Default: 30000 (30s).
    pub max_backoff_ms: u64,
    /// Whether to add jitter to backoff. Default: true.
    pub jitter: bool,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            max_retries: 10,
            base_backoff_ms: 100,
            max_backoff_ms: 30000,
            jitter: true,
        }
    }
}

impl ReconnectPolicy {
    /// Calculate backoff duration for attempt N (0-indexed).
    pub fn backoff_duration(&self, attempt: u32) -> Duration {
        let base = self.base_backoff_ms.saturating_mul(1u64 << attempt.min(12));
        let capped = base.min(self.max_backoff_ms);
        let jittered = if self.jitter {
            // Simple deterministic jitter: ±25%
            let jitter_range = capped / 4;
            if jitter_range == 0 {
                capped
            } else {
                capped - jitter_range / 2 + (attempt as u64 * 7 % (jitter_range + 1))
            }
        } else {
            capped
        };
        Duration::from_millis(jittered)
    }
}

/// Attempt to reconnect to a sidecar with exponential backoff.
/// Returns the new connection on success.
pub async fn reconnect_with_backoff(
    name: &str,
    handshake: &Handshake,
    policy: &ReconnectPolicy,
) -> Result<SidecarConnection, ReconnectError> {
    let sock = socket_path(name);

    for attempt in 0..policy.max_retries {
        let delay = policy.backoff_duration(attempt);
        if attempt > 0 {
            tracing::info!(
                sidecar = %name,
                attempt = attempt + 1,
                max = policy.max_retries,
                delay_ms = %delay.as_millis(),
                "reconnecting to sidecar"
            );
            tokio::time::sleep(delay).await;
        }

        match SidecarConnection::connect(&sock).await {
            Ok(mut conn) => {
                // Re-handshake
                if let Err(e) = conn.send(&Message::Handshake(handshake.clone())).await {
                    tracing::warn!(sidecar = %name, error = %e, "handshake send failed");
                    continue;
                }
                match conn.recv().await {
                    Ok(Message::HandshakeAck(ack)) if ack.accepted => {
                        tracing::info!(sidecar = %name, attempt = attempt + 1, "reconnected successfully");
                        return Ok(conn);
                    }
                    Ok(Message::HandshakeAck(ack)) => {
                        tracing::warn!(sidecar = %name, reason = ?ack.reject_reason, "handshake rejected");
                        continue;
                    }
                    Ok(_) => {
                        tracing::warn!(sidecar = %name, "unexpected handshake response");
                        continue;
                    }
                    Err(e) => {
                        tracing::warn!(sidecar = %name, error = %e, "handshake recv failed");
                        continue;
                    }
                }
            }
            Err(e) => {
                tracing::debug!(sidecar = %name, attempt = attempt + 1, error = %e, "connect failed");
                continue;
            }
        }
    }

    Err(ReconnectError::MaxRetriesExhausted {
        name: name.to_string(),
        attempts: policy.max_retries,
    })
}

/// Errors from reconnection attempts.
#[derive(Debug)]
pub enum ReconnectError {
    /// All retry attempts exhausted.
    MaxRetriesExhausted { name: String, attempts: u32 },
}

impl std::fmt::Display for ReconnectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MaxRetriesExhausted { name, attempts } =>
                write!(f, "reconnect to sidecar '{}' failed after {} attempts", name, attempts),
        }
    }
}

impl std::error::Error for ReconnectError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reconnect_policy_defaults() {
        let policy = ReconnectPolicy::default();
        assert_eq!(policy.max_retries, 10);
        assert_eq!(policy.base_backoff_ms, 100);
        assert_eq!(policy.max_backoff_ms, 30000);
        assert!(policy.jitter);
    }

    #[test]
    fn test_backoff_exponential_growth() {
        let policy = ReconnectPolicy {
            max_retries: 10,
            base_backoff_ms: 100,
            max_backoff_ms: 30000,
            jitter: false,
        };

        // Without jitter, backoff should be exact exponential
        assert_eq!(policy.backoff_duration(0), Duration::from_millis(100));   // 100 * 2^0
        assert_eq!(policy.backoff_duration(1), Duration::from_millis(200));   // 100 * 2^1
        assert_eq!(policy.backoff_duration(2), Duration::from_millis(400));   // 100 * 2^2
        assert_eq!(policy.backoff_duration(3), Duration::from_millis(800));   // 100 * 2^3
        assert_eq!(policy.backoff_duration(4), Duration::from_millis(1600));  // 100 * 2^4
    }

    #[test]
    fn test_backoff_capped_at_max() {
        let policy = ReconnectPolicy {
            max_retries: 20,
            base_backoff_ms: 100,
            max_backoff_ms: 5000,
            jitter: false,
        };

        // 100 * 2^6 = 6400, capped to 5000
        assert_eq!(policy.backoff_duration(6), Duration::from_millis(5000));
        // Higher attempts stay capped
        assert_eq!(policy.backoff_duration(10), Duration::from_millis(5000));
        assert_eq!(policy.backoff_duration(15), Duration::from_millis(5000));
    }

    #[test]
    fn test_backoff_with_jitter() {
        let policy = ReconnectPolicy {
            max_retries: 10,
            base_backoff_ms: 100,
            max_backoff_ms: 30000,
            jitter: true,
        };

        // With jitter, values should differ from exact exponential but stay in range
        for attempt in 0..10 {
            let dur = policy.backoff_duration(attempt);
            let base = (100u64 * (1u64 << attempt.min(12))).min(30000);
            let jitter_range = base / 4;
            let lower = base.saturating_sub(jitter_range / 2);
            let upper = base + jitter_range / 2 + 1;
            assert!(
                dur.as_millis() >= lower as u128 && dur.as_millis() <= upper as u128,
                "attempt {}: {:?} not in range [{}, {}]",
                attempt, dur, lower, upper
            );
        }
    }

    #[test]
    fn test_backoff_deterministic_with_jitter() {
        let policy = ReconnectPolicy::default();

        // Same attempt should produce the same duration (deterministic jitter)
        let d1 = policy.backoff_duration(3);
        let d2 = policy.backoff_duration(3);
        assert_eq!(d1, d2);
    }

    #[test]
    fn test_reconnect_error_display() {
        let err = ReconnectError::MaxRetriesExhausted {
            name: "fraud".to_string(),
            attempts: 10,
        };
        assert_eq!(
            format!("{}", err),
            "reconnect to sidecar 'fraud' failed after 10 attempts"
        );
    }
}

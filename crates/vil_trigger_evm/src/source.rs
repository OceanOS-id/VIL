// =============================================================================
// vil_trigger_evm::source — EvmTrigger
// =============================================================================
//
// Ethereum JSON-RPC log subscription trigger using alloy.
//
// On every matching contract log:
//   1. Times the subscription poll.
//   2. Emits mq_log! with timing, data size, and contract hash.
//   3. Calls on_event callback with a TriggerEvent.
//
// Requires a WebSocket RPC endpoint (wss://).
// No println!, tracing, or log crate — COMPLIANCE.md §8.
// =============================================================================

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use futures_util::StreamExt;

use alloy::providers::{ProviderBuilder, Provider};
use alloy::rpc::types::Filter;
use alloy::primitives::Address;
use alloy::network::Ethereum;
use alloy::transports::ws::WsConnect;

use vil_log::{mq_log, types::MqPayload};
use vil_log::dict::register_str;

use vil_trigger_core::traits::{TriggerSource, EventCallback};
use vil_trigger_core::types::{TriggerEvent, TriggerFault};

use crate::config::EvmConfig;
use crate::error::EvmFault;

/// Ethereum EVM log subscription trigger.
pub struct EvmTrigger {
    config:        EvmConfig,
    paused:        Arc<AtomicBool>,
    sequence:      Arc<AtomicU64>,
    url_hash:      u32,
    contract_hash: u32,
    sig_hash:      u32,
    kind_hash:     u32,
}

impl EvmTrigger {
    /// Create a new `EvmTrigger` from config.
    pub fn new(config: EvmConfig) -> Self {
        let url_hash      = register_str(&config.rpc_url);
        let contract_hash = register_str(&config.contract_address);
        let sig_hash      = register_str(&config.event_signature);
        let kind_hash     = register_str("evm");
        Self {
            config,
            paused: Arc::new(AtomicBool::new(false)),
            sequence: Arc::new(AtomicU64::new(0)),
            url_hash,
            contract_hash,
            sig_hash,
            kind_hash,
        }
    }

    fn map_fault(f: EvmFault, kind_hash: u32) -> TriggerFault {
        TriggerFault::SourceUnavailable {
            kind_hash,
            reason_code: f.as_error_code(),
        }
    }

    async fn subscribe_logs_loop(&self, on_event: &EventCallback) -> Result<(), EvmFault> {
        let url_hash      = self.url_hash;
        let contract_hash = self.contract_hash;
        let sig_hash      = self.sig_hash;
        let kind_hash     = self.kind_hash;

        // Parse contract address.
        let addr: Address = self
            .config
            .contract_address
            .parse()
            .map_err(|_| EvmFault::InvalidAddress { addr_hash: contract_hash })?;

        // Connect via WebSocket using the pubsub-capable RootProvider.
        let ws = WsConnect::new(self.config.rpc_url.clone());
        let provider = ProviderBuilder::new()
            .network::<Ethereum>()
            .connect_ws(ws)
            .await
            .map_err(|_| EvmFault::ConnectionFailed {
                url_hash,
                reason_code: 1,
            })?;

        // Build log filter for the contract address.
        let filter = Filter::new().address(addr);

        // Subscribe to matching logs (GetSubscription implements IntoFuture).
        let sub = provider
            .subscribe_logs(&filter)
            .await
            .map_err(|_| EvmFault::SubscribeFailed {
                sig_hash,
                rpc_code: -1,
            })?;

        let mut log_stream = sub.into_stream();

        loop {
            if self.paused.load(Ordering::Relaxed) {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                continue;
            }

            let start = std::time::Instant::now();

            match log_stream.next().await {
                None => {
                    return Err(EvmFault::StreamClosed { url_hash });
                }
                Some(log) => {
                    let elapsed  = start.elapsed();
                    let seq      = self.sequence.fetch_add(1, Ordering::Relaxed);
                    let data_len = log.data().data.len() as u32;

                    // Emit mq_log! on every matching EVM log.
                    mq_log!(Info, MqPayload {
                        broker_hash:    url_hash,
                        topic_hash:     contract_hash,
                        group_hash:     sig_hash,
                        offset:         seq,
                        message_bytes:  data_len,
                        e2e_latency_us: elapsed.as_micros() as u32,
                        op_type:        1, // consume
                        partition:      0,
                        retries:        0,
                        compression:    0,
                        ..MqPayload::default()
                    });

                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos() as u64;

                    on_event(TriggerEvent {
                        kind_hash,
                        source_hash:   contract_hash,
                        sequence:      seq,
                        timestamp_ns:  ts,
                        payload_bytes: data_len,
                        op: 0,
                        _pad: [0; 3],
                    });
                }
            }
        }
    }
}

#[async_trait]
impl TriggerSource for EvmTrigger {
    fn kind(&self) -> &'static str {
        "evm"
    }

    async fn start(&self, on_event: EventCallback) -> Result<(), TriggerFault> {
        self.subscribe_logs_loop(&on_event)
            .await
            .map_err(|e| Self::map_fault(e, self.kind_hash))
    }

    async fn pause(&self) -> Result<(), TriggerFault> {
        self.paused.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn resume(&self) -> Result<(), TriggerFault> {
        self.paused.store(false, Ordering::Relaxed);
        Ok(())
    }

    async fn stop(&self) -> Result<(), TriggerFault> {
        self.paused.store(true, Ordering::Relaxed);
        Ok(())
    }
}

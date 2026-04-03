// =============================================================================
// vil_trigger_webhook::source — WebhookTrigger
// =============================================================================
//
// HTTP webhook receiver trigger using axum.
//
// On every valid webhook POST:
//   1. Reads body + verifies HMAC signature.
//   2. Times the verification + dispatch.
//   3. Emits mq_log! with timing, body size, and path hash.
//   4. Calls on_event callback with a TriggerEvent.
//
// No println!, tracing, or log crate — COMPLIANCE.md §8.
// =============================================================================

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use axum::{body::to_bytes, extract::State, http::StatusCode, routing::post, Router};
use tokio::sync::mpsc;

use vil_log::dict::register_str;
use vil_log::{mq_log, types::MqPayload};

use vil_trigger_core::traits::{EventCallback, TriggerSource};
use vil_trigger_core::types::{TriggerEvent, TriggerFault};

use crate::config::WebhookConfig;
use crate::error::WebhookFault;
use crate::verify::verify_hmac;

/// Shared state passed into the axum handler.
#[derive(Clone)]
struct HandlerState {
    secret: Vec<u8>,
    tx: mpsc::UnboundedSender<(u64, u32)>,
    host_hash: u32,
    path_hash: u32,
    kind_hash: u32,
    sequence: Arc<AtomicU64>,
}

/// axum handler: reads body, verifies HMAC, emits mq_log!, sends to channel.
async fn webhook_handler(
    State(state): State<HandlerState>,
    req: axum::extract::Request,
) -> StatusCode {
    let start = std::time::Instant::now();

    // Extract signature header before consuming the request.
    let sig_header = req
        .headers()
        .get("x-hub-signature-256")
        .or_else(|| req.headers().get("x-signature-256"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_owned());

    let body = match to_bytes(req.into_body(), 4 * 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => return StatusCode::BAD_REQUEST,
    };

    let body_len = body.len() as u32;

    // Verify HMAC if a secret is configured.
    if !state.secret.is_empty() {
        match sig_header {
            None => return StatusCode::UNAUTHORIZED,
            Some(sig) => {
                if !verify_hmac(&state.secret, &body, &sig) {
                    return StatusCode::UNAUTHORIZED;
                }
            }
        }
    }

    let elapsed = start.elapsed();
    let seq = state.sequence.fetch_add(1, Ordering::Relaxed);

    // Emit mq_log! on every successful webhook delivery.
    mq_log!(
        Info,
        MqPayload {
            broker_hash: state.host_hash,
            topic_hash: state.path_hash,
            group_hash: state.kind_hash,
            offset: seq,
            message_bytes: body_len,
            e2e_latency_ns: elapsed.as_nanos() as u64,
            op_type: 1, // consume
            partition: 0,
            retries: 0,
            compression: 0,
            ..MqPayload::default()
        }
    );

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;

    let _ = state.tx.send((ts, body_len));

    StatusCode::OK
}

/// HTTP webhook receiver trigger.
pub struct WebhookTrigger {
    config: WebhookConfig,
    paused: Arc<AtomicBool>,
    sequence: Arc<AtomicU64>,
    addr_hash: u32,
    path_hash: u32,
    kind_hash: u32,
}

impl WebhookTrigger {
    /// Create a new `WebhookTrigger` from config.
    pub fn new(config: WebhookConfig) -> Self {
        let addr_hash = register_str(&config.listen_addr);
        let path_hash = register_str(&config.path);
        let kind_hash = register_str("webhook");
        Self {
            config,
            paused: Arc::new(AtomicBool::new(false)),
            sequence: Arc::new(AtomicU64::new(0)),
            addr_hash,
            path_hash,
            kind_hash,
        }
    }

    fn map_fault(f: WebhookFault, kind_hash: u32) -> TriggerFault {
        TriggerFault::SourceUnavailable {
            kind_hash,
            reason_code: f.as_error_code(),
        }
    }

    async fn serve(&self, on_event: EventCallback) -> Result<(), WebhookFault> {
        let addr_hash = self.addr_hash;
        let path_hash = self.path_hash;
        let kind_hash = self.kind_hash;
        let paused = self.paused.clone();
        let sequence = self.sequence.clone();

        let (tx, mut rx) = mpsc::unbounded_channel::<(u64, u32)>();

        let state = HandlerState {
            secret: self.config.secret.as_bytes().to_vec(),
            tx,
            host_hash: addr_hash,
            path_hash,
            kind_hash,
            sequence: sequence.clone(),
        };

        let path = self.config.path.clone();
        let app = Router::new()
            .route(&path, post(webhook_handler))
            .with_state(state);

        let listen_addr: SocketAddr =
            self.config
                .listen_addr
                .parse()
                .map_err(|_| WebhookFault::BindFailed {
                    addr_hash,
                    os_code: 22,
                })?;

        let listener = tokio::net::TcpListener::bind(listen_addr)
            .await
            .map_err(|e| WebhookFault::BindFailed {
                addr_hash,
                os_code: e.raw_os_error().unwrap_or(0) as u32,
            })?;

        // Spawn the HTTP server.
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        // Dispatch events from the channel.
        loop {
            if paused.load(Ordering::Relaxed) {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                continue;
            }

            if let Some((ts, body_len)) = rx.recv().await {
                let seq = sequence.load(Ordering::Relaxed);
                on_event(TriggerEvent {
                    kind_hash,
                    source_hash: path_hash,
                    sequence: seq,
                    timestamp_ns: ts,
                    payload_bytes: body_len,
                    op: 0,
                    _pad: [0; 3],
                });
            } else {
                return Err(WebhookFault::ServerShutdown { addr_hash });
            }
        }
    }
}

#[async_trait]
impl TriggerSource for WebhookTrigger {
    fn kind(&self) -> &'static str {
        "webhook"
    }

    async fn start(&self, on_event: EventCallback) -> Result<(), TriggerFault> {
        self.serve(on_event)
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

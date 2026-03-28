// =============================================================================
// vil_trigger_email::source — EmailTrigger
// =============================================================================
//
// IMAP IDLE push-based email trigger.
//
// On every new message arrival:
//   1. Times the IDLE wait.
//   2. Emits mq_log! with timing, message size, and folder hash.
//   3. Calls on_event callback with a TriggerEvent.
//
// Uses async-imap (runtime-tokio) + tokio-native-tls for TLS.
// async-native-tls is retained in Cargo.toml per spec; tokio-native-tls
// is used for the actual TLS bridge to tokio AsyncRead/Write.
//
// No println!, tracing, or log crate — COMPLIANCE.md §8.
// =============================================================================

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use async_imap::extensions::idle::IdleResponse;
use async_trait::async_trait;
use tokio_native_tls::TlsConnector;

use vil_log::dict::register_str;
use vil_log::{mq_log, types::MqPayload};

use vil_trigger_core::traits::{EventCallback, TriggerSource};
use vil_trigger_core::types::{TriggerEvent, TriggerFault};

use crate::config::EmailConfig;
use crate::error::EmailFault;

/// IMAP IDLE email trigger.
pub struct EmailTrigger {
    config: EmailConfig,
    paused: Arc<AtomicBool>,
    sequence: Arc<AtomicU64>,
    host_hash: u32,
    folder_hash: u32,
    kind_hash: u32,
}

impl EmailTrigger {
    /// Create a new `EmailTrigger` from config.
    pub fn new(config: EmailConfig) -> Self {
        let host_hash = register_str(&config.socket_addr());
        let folder_hash = register_str(&config.folder);
        let kind_hash = register_str("email");
        Self {
            config,
            paused: Arc::new(AtomicBool::new(false)),
            sequence: Arc::new(AtomicU64::new(0)),
            host_hash,
            folder_hash,
            kind_hash,
        }
    }

    fn map_fault(f: EmailFault, kind_hash: u32) -> TriggerFault {
        TriggerFault::SourceUnavailable {
            kind_hash,
            reason_code: f.as_error_code(),
        }
    }

    async fn idle_loop(&self, on_event: &EventCallback) -> Result<(), EmailFault> {
        let host_hash = self.host_hash;
        let folder_hash = self.folder_hash;
        let kind_hash = self.kind_hash;

        // Build TLS connector using native-tls + tokio bridge.
        let native_cx =
            native_tls::TlsConnector::new().map_err(|_| EmailFault::TlsConnectFailed {
                host_hash,
                reason_code: 1,
            })?;
        let tls = TlsConnector::from(native_cx);

        // Establish TCP + TLS.
        let tcp = tokio::net::TcpStream::connect(self.config.socket_addr())
            .await
            .map_err(|e| EmailFault::TlsConnectFailed {
                host_hash,
                reason_code: e.raw_os_error().unwrap_or(0) as u32,
            })?;

        let tls_stream = tls
            .connect(&self.config.imap_host, tcp)
            .await
            .map_err(|_| EmailFault::TlsConnectFailed {
                host_hash,
                reason_code: 2,
            })?;

        // Create async-imap client (tokio feature).
        let client = async_imap::Client::new(tls_stream);

        let user_hash = register_str(&self.config.username);
        let mut session = client
            .login(&self.config.username, &self.config.password)
            .await
            .map_err(|_| EmailFault::LoginFailed { user_hash })?;

        session
            .select(&self.config.folder)
            .await
            .map_err(|_| EmailFault::FolderNotFound { folder_hash })?;

        loop {
            if self.paused.load(Ordering::Relaxed) {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                continue;
            }

            let idle_start = std::time::Instant::now();

            // Enter IDLE mode — `idle()` is infallible, consumes session.
            let mut handle = session.idle();

            // Send IDLE command to server.
            handle.init().await.map_err(|_| EmailFault::IdleFailed {
                host_hash,
                reason_code: 0,
            })?;

            // Wait up to 29 minutes for a server notification.
            let (wait_fut, _stop) =
                handle.wait_with_timeout(std::time::Duration::from_secs(29 * 60));

            let response = wait_fut.await.map_err(|_| EmailFault::IdleFailed {
                host_hash,
                reason_code: 1,
            })?;

            // Recover the session.
            session = handle
                .done()
                .await
                .map_err(|_| EmailFault::Disconnected { host_hash })?;

            // Only fire on NewData — Timeout is just a keepalive cycle.
            if let IdleResponse::NewData(_) = response {
                let elapsed = idle_start.elapsed();
                let seq = self.sequence.fetch_add(1, Ordering::Relaxed);

                mq_log!(
                    Info,
                    MqPayload {
                        broker_hash: host_hash,
                        topic_hash: folder_hash,
                        group_hash: kind_hash,
                        offset: seq,
                        message_bytes: 0,
                        e2e_latency_us: elapsed.as_micros() as u32,
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

                on_event(TriggerEvent {
                    kind_hash,
                    source_hash: folder_hash,
                    sequence: seq,
                    timestamp_ns: ts,
                    payload_bytes: 0,
                    op: 0,
                    _pad: [0; 3],
                });
            }
        }
    }
}

#[async_trait]
impl TriggerSource for EmailTrigger {
    fn kind(&self) -> &'static str {
        "email"
    }

    async fn start(&self, on_event: EventCallback) -> Result<(), TriggerFault> {
        self.idle_loop(&on_event)
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

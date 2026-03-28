// =============================================================================
// Sidecar Dispatcher — Route invocations to the correct sidecar
// =============================================================================
//
// Given a target sidecar name and method, the dispatcher:
//   1. Serializes request data to SHM
//   2. Sends Invoke descriptor over UDS
//   3. Awaits Result descriptor
//   4. Reads response data from SHM
//
// This is the core "call a sidecar" path used by VxKernel.

use crate::protocol::*;
use crate::registry::{SidecarHealth, SidecarRegistry};
use std::sync::atomic::AtomicU64;
use std::time::Instant;

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Result of a sidecar invocation.
#[derive(Debug)]
pub struct InvokeResponse {
    /// Raw response bytes read from SHM.
    pub data: Vec<u8>,
    /// Latency of the invocation in microseconds.
    pub latency_us: u64,
}

/// Invoke a method on a sidecar with the given request data.
///
/// This is the primary API for calling sidecar handlers from vil-server.
///
/// Flow:
///   1. Write `request_data` to the sidecar's SHM region
///   2. Send Invoke message (descriptor) over UDS
///   3. Wait for Result message (descriptor) over UDS
///   4. Read response data from SHM
pub async fn invoke(
    registry: &SidecarRegistry,
    target: &str,
    method: &str,
    request_data: &[u8],
) -> Result<InvokeResponse, DispatchError> {
    let start = Instant::now();

    // Get connection, SHM, config, and metrics
    let (conn, shm, timeout_ms, metrics) = {
        let entry = registry
            .get(target)
            .ok_or_else(|| DispatchError::NotRegistered(target.to_string()))?;

        if entry.health != SidecarHealth::Healthy {
            return Err(DispatchError::Unavailable {
                name: target.to_string(),
                health: entry.health.to_string(),
            });
        }

        let conn = entry
            .connection
            .clone()
            .ok_or_else(|| DispatchError::NotConnected(target.to_string()))?;
        let shm = entry
            .shm
            .clone()
            .ok_or_else(|| DispatchError::NoShm(target.to_string()))?;

        (conn, shm, entry.config.timeout_ms, entry.metrics.clone())
    };

    metrics.invoke_start();

    // Step 1: Write request data to SHM
    let (offset, len) = shm
        .write(request_data)
        .map_err(|e| DispatchError::ShmWrite(e.to_string()))?;

    // Step 2: Build and send Invoke
    let request_id = REQUEST_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let descriptor = ShmDescriptor::new(request_id, 0, offset, len)
        .with_method(method)
        .with_timeout(timeout_ms);

    let invoke_msg = Message::Invoke(Invoke {
        descriptor,
        method: method.to_string(),
    });

    // Step 3: Send invoke and wait for result (with timeout)
    let timeout = std::time::Duration::from_millis(timeout_ms);

    let result = tokio::time::timeout(timeout, async {
        let mut conn = conn.lock().await;
        conn.send(&invoke_msg).await?;
        conn.recv().await
    })
    .await;

    match result {
        Ok(Ok(Message::Result(r))) => {
            if r.request_id != request_id {
                metrics.invoke_error();
                return Err(DispatchError::RequestIdMismatch {
                    expected: request_id,
                    got: r.request_id,
                });
            }

            match r.status {
                InvokeStatus::Ok => {
                    // Step 4: Read response from SHM
                    let resp_data = if let Some(desc) = r.descriptor {
                        shm.read(desc.offset, desc.len)
                            .map(|d| d.to_vec())
                            .map_err(|e| DispatchError::ShmRead(e.to_string()))?
                    } else {
                        Vec::new()
                    };

                    let latency_us = start.elapsed().as_micros() as u64;
                    metrics.invoke_ok(latency_us);

                    Ok(InvokeResponse {
                        data: resp_data,
                        latency_us,
                    })
                }
                InvokeStatus::Error => {
                    metrics.invoke_error();
                    Err(DispatchError::SidecarError(
                        r.error.unwrap_or_else(|| "unknown error".into()),
                    ))
                }
                InvokeStatus::Timeout => {
                    metrics.invoke_timeout();
                    Err(DispatchError::Timeout(target.to_string()))
                }
                InvokeStatus::MethodNotFound => {
                    metrics.invoke_error();
                    Err(DispatchError::MethodNotFound {
                        sidecar: target.to_string(),
                        method: method.to_string(),
                    })
                }
            }
        }
        Ok(Ok(other)) => {
            metrics.invoke_error();
            Err(DispatchError::UnexpectedMessage(format!(
                "expected Result, got {:?}",
                std::mem::discriminant(&other)
            )))
        }
        Ok(Err(e)) => {
            metrics.invoke_error();
            Err(DispatchError::Transport(e.to_string()))
        }
        Err(_) => {
            metrics.invoke_timeout();
            Err(DispatchError::Timeout(target.to_string()))
        }
    }
}

/// Invoke with retry (uses config.retry count).
pub async fn invoke_with_retry(
    registry: &SidecarRegistry,
    target: &str,
    method: &str,
    request_data: &[u8],
) -> Result<InvokeResponse, DispatchError> {
    let max_retries = registry
        .get(target)
        .map(|e| e.config.retry)
        .unwrap_or(0);

    let mut last_err = None;
    for attempt in 0..=max_retries {
        if attempt > 0 {
            // Exponential backoff: 100ms, 200ms, 400ms, ...
            let delay = std::time::Duration::from_millis(100 * (1 << (attempt - 1)));
            tokio::time::sleep(delay).await;
            {
                use vil_log::app_log;
                app_log!(Debug, "sidecar.dispatch.retry", { sidecar: vil_log::dict::register_str(target) as u64, method: vil_log::dict::register_str(method) as u64, attempt: (attempt + 1) as u64, max: (max_retries + 1) as u64 });
            }
        }

        match invoke(registry, target, method, request_data).await {
            Ok(resp) => return Ok(resp),
            Err(e) => {
                // Only retry on transient errors
                if matches!(
                    e,
                    DispatchError::Timeout(_)
                        | DispatchError::Transport(_)
                        | DispatchError::ShmWrite(_)
                ) {
                    last_err = Some(e);
                    continue;
                }
                return Err(e);
            }
        }
    }

    Err(last_err.unwrap_or_else(|| DispatchError::NotRegistered(target.to_string())))
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum DispatchError {
    NotRegistered(String),
    NotConnected(String),
    NoShm(String),
    Unavailable { name: String, health: String },
    ShmWrite(String),
    ShmRead(String),
    Transport(String),
    Timeout(String),
    SidecarError(String),
    MethodNotFound { sidecar: String, method: String },
    RequestIdMismatch { expected: u64, got: u64 },
    UnexpectedMessage(String),
}

impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotRegistered(n) => write!(f, "sidecar '{}' not registered", n),
            Self::NotConnected(n) => write!(f, "sidecar '{}' not connected", n),
            Self::NoShm(n) => write!(f, "no SHM region for sidecar '{}'", n),
            Self::Unavailable { name, health } => {
                write!(f, "sidecar '{}' unavailable (health: {})", name, health)
            }
            Self::ShmWrite(e) => write!(f, "SHM write: {}", e),
            Self::ShmRead(e) => write!(f, "SHM read: {}", e),
            Self::Transport(e) => write!(f, "transport: {}", e),
            Self::Timeout(n) => write!(f, "timeout invoking sidecar '{}'", n),
            Self::SidecarError(e) => write!(f, "sidecar error: {}", e),
            Self::MethodNotFound { sidecar, method } => {
                write!(f, "method '{}' not found on sidecar '{}'", method, sidecar)
            }
            Self::RequestIdMismatch { expected, got } => {
                write!(f, "request ID mismatch: expected {}, got {}", expected, got)
            }
            Self::UnexpectedMessage(m) => write!(f, "unexpected message: {}", m),
        }
    }
}

impl std::error::Error for DispatchError {}

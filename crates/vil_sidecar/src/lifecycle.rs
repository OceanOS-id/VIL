// =============================================================================
// Sidecar Lifecycle — Spawn, connect, health check, drain, shutdown
// =============================================================================
//
// Manages the full lifecycle of a sidecar process:
//   1. Spawn (optional auto-spawn from command)
//   2. Connect (UDS) + Handshake
//   3. Health check loop (periodic, marks unhealthy after N failures)
//   4. Drain (stop new work, wait for in-flight)
//   5. Shutdown (terminate process)
//   6. Auto-reconnect on crash (exponential backoff)

use crate::protocol::*;
use crate::registry::{SidecarHealth, SidecarRegistry};
use crate::shm_bridge::ShmRegion;
use crate::transport::{SidecarListener, TransportError};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Connect to a sidecar (or accept its connection) and perform handshake.
///
/// After successful handshake, the sidecar entry in the registry is updated
/// with the connection, SHM region, methods, and health = Healthy.
pub async fn connect_sidecar(
    registry: &SidecarRegistry,
    name: &str,
) -> Result<(), LifecycleError> {
    let config = {
        let entry = registry
            .get(name)
            .ok_or_else(|| LifecycleError::NotRegistered(name.to_string()))?;
        entry.config.clone()
    };

    let socket_path = config.socket_path();
    let shm_path = config.shm_path();

    // Create SHM region for this sidecar
    let shm = ShmRegion::create(&shm_path, config.shm_size)
        .map_err(|e| LifecycleError::ShmError(e.to_string()))?;
    let shm = Arc::new(shm);

    // Listen for incoming sidecar connection
    let listener = SidecarListener::bind(&socket_path)
        .await
        .map_err(|e| LifecycleError::Transport(e))?;

    // If auto-spawn is configured, spawn the sidecar process
    let pid = if let Some(ref cmd) = config.command {
        Some(spawn_sidecar_process(cmd)?)
    } else {
        {
            use vil_log::app_log;
            app_log!(Info, "sidecar.waiting.connect", { sidecar: name, socket: socket_path.as_str() });
        }
        None
    };

    // Accept connection with timeout
    let conn = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        listener.accept(),
    )
    .await
    .map_err(|_| LifecycleError::HandshakeTimeout(name.to_string()))?
    .map_err(LifecycleError::Transport)?;

    let mut conn = conn;

    // Wait for Handshake message from sidecar
    let handshake = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        conn.recv(),
    )
    .await
    .map_err(|_| LifecycleError::HandshakeTimeout(name.to_string()))?
    .map_err(LifecycleError::Transport)?;

    let methods = match handshake {
        Message::Handshake(h) => {
            // Validate sidecar name matches
            if h.name != name {
                conn.send(&Message::HandshakeAck(HandshakeAck {
                    accepted: false,
                    shm_path: String::new(),
                    shm_size: 0,
                    reject_reason: Some(format!(
                        "name mismatch: expected '{}', got '{}'",
                        name, h.name
                    )),
                }))
                .await
                .ok();
                return Err(LifecycleError::NameMismatch {
                    expected: name.to_string(),
                    got: h.name,
                });
            }

            // Validate auth token if configured
            if let Some(ref expected_token) = config.auth_token {
                if h.auth_token.as_deref() != Some(expected_token.as_str()) {
                    conn.send(&Message::HandshakeAck(HandshakeAck {
                        accepted: false,
                        shm_path: String::new(),
                        shm_size: 0,
                        reject_reason: Some("authentication failed".into()),
                    }))
                    .await
                    .ok();
                    return Err(LifecycleError::AuthFailed(name.to_string()));
                }
            }

            {
                use vil_log::app_log;
                app_log!(Info, "sidecar.handshake.accepted", { sidecar: name, version: h.version.as_str() });
            }
            h.methods
        }
        other => {
            return Err(LifecycleError::UnexpectedMessage(format!(
                "expected Handshake, got {:?}",
                std::mem::discriminant(&other)
            )));
        }
    };

    // Send HandshakeAck
    conn.send(&Message::HandshakeAck(HandshakeAck {
        accepted: true,
        shm_path: shm_path.clone(),
        shm_size: config.shm_size,
        reject_reason: None,
    }))
    .await
    .map_err(LifecycleError::Transport)?;

    // Update registry entry
    {
        let mut entry = registry
            .get_mut(name)
            .ok_or_else(|| LifecycleError::NotRegistered(name.to_string()))?;
        entry.connection = Some(Arc::new(Mutex::new(conn)));
        entry.shm = Some(shm);
        entry.health = SidecarHealth::Healthy;
        entry.methods = methods;
        entry.pid = pid;
    }

    Ok(())
}

/// Spawn a sidecar process from a shell command.
fn spawn_sidecar_process(command: &str) -> Result<u32, LifecycleError> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err(LifecycleError::SpawnFailed("empty command".into()));
    }

    let child = std::process::Command::new(parts[0])
        .args(&parts[1..])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| LifecycleError::SpawnFailed(format!("{}: {}", command, e)))?;

    let pid = child.id();
    {
        use vil_log::{system_log, types::SystemPayload};
        system_log!(Info, SystemPayload { event_type: 4, ..Default::default() });
        {
            use vil_log::app_log;
            app_log!(Info, "sidecar.process.spawned", { command: command, pid: pid as u64 });
        }
    }
    Ok(pid)
}

/// Send a health check to a sidecar and update its status.
pub async fn health_check(
    registry: &SidecarRegistry,
    name: &str,
    max_failures: u64,
) -> Result<HealthOk, LifecycleError> {
    let conn = {
        let entry = registry
            .get(name)
            .ok_or_else(|| LifecycleError::NotRegistered(name.to_string()))?;
        entry
            .connection
            .clone()
            .ok_or_else(|| LifecycleError::NotConnected(name.to_string()))?
    };

    let mut conn = conn.lock().await;

    // Send Health
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        conn.send(&Message::Health).await?;
        conn.recv().await
    })
    .await;

    match result {
        Ok(Ok(Message::HealthOk(health))) => {
            // Update metrics: health ok
            if let Some(entry) = registry.get(name) {
                entry.metrics.health_ok();
            }
            if let Some(mut entry) = registry.get_mut(name) {
                entry.health = SidecarHealth::Healthy;
            }
            Ok(health)
        }
        Ok(Ok(other)) => {
            mark_unhealthy(registry, name, max_failures);
            Err(LifecycleError::UnexpectedMessage(format!(
                "expected HealthOk, got {:?}",
                std::mem::discriminant(&other)
            )))
        }
        Ok(Err(e)) => {
            mark_unhealthy(registry, name, max_failures);
            Err(LifecycleError::Transport(e))
        }
        Err(_) => {
            mark_unhealthy(registry, name, max_failures);
            Err(LifecycleError::HealthTimeout(name.to_string()))
        }
    }
}

/// Mark a sidecar as unhealthy and potentially disconnected.
fn mark_unhealthy(registry: &SidecarRegistry, name: &str, max_failures: u64) {
    if let Some(entry) = registry.get(name) {
        entry.metrics.health_failure();
        let failures = entry.metrics.health_failures.load(std::sync::atomic::Ordering::Relaxed);

        if failures >= max_failures {
            drop(entry);
            if let Some(mut entry) = registry.get_mut(name) {
                entry.health = SidecarHealth::Disconnected;
                entry.connection = None;
                use vil_log::app_log;
                app_log!(Warn, "sidecar.disconnected", { sidecar: name, failures: failures });
            }
        } else {
            drop(entry);
            if let Some(mut entry) = registry.get_mut(name) {
                entry.health = SidecarHealth::Unhealthy;
                use vil_log::app_log;
                app_log!(Warn, "sidecar.health.failed", { sidecar: name, failures: failures, max: max_failures });
            }
        }
    }
}

/// Gracefully drain a sidecar: send DRAIN, wait for DRAINED, then SHUTDOWN.
pub async fn drain_sidecar(
    registry: &SidecarRegistry,
    name: &str,
) -> Result<(), LifecycleError> {
    let conn = {
        let mut entry = registry
            .get_mut(name)
            .ok_or_else(|| LifecycleError::NotRegistered(name.to_string()))?;
        entry.health = SidecarHealth::Draining;
        entry
            .connection
            .clone()
            .ok_or_else(|| LifecycleError::NotConnected(name.to_string()))?
    };

    let mut conn = conn.lock().await;

    // Send Drain
    conn.send(&Message::Drain)
        .await
        .map_err(LifecycleError::Transport)?;

    {
        use vil_log::app_log;
        app_log!(Info, "sidecar.draining", { sidecar: name });
    }

    // Wait for Drained (with timeout)
    let result = tokio::time::timeout(std::time::Duration::from_secs(60), conn.recv()).await;

    match result {
        Ok(Ok(Message::Drained)) => {
            use vil_log::app_log;
            app_log!(Info, "sidecar.drained", { sidecar: name });
        }
        Ok(Ok(_)) => {
            use vil_log::app_log;
            app_log!(Warn, "sidecar.drain.unexpected", { sidecar: name });
        }
        Ok(Err(e)) => {
            use vil_log::app_log;
            app_log!(Warn, "sidecar.drain.error", { sidecar: name, error: e.to_string() });
        }
        Err(_) => {
            use vil_log::app_log;
            app_log!(Warn, "sidecar.drain.timeout", { sidecar: name });
        }
    }

    // Send Shutdown
    conn.send(&Message::Shutdown).await.ok();

    // Update registry
    if let Some(mut entry) = registry.get_mut(name) {
        entry.health = SidecarHealth::Stopped;
        entry.connection = None;
    }

    // Cleanup SHM
    let shm_path = crate::transport::shm_path(name);
    crate::shm_bridge::remove_shm_region(&shm_path);

    Ok(())
}

/// Spawn a background health check loop for a sidecar.
///
/// Returns a JoinHandle that can be aborted to stop the loop.
pub fn spawn_health_loop(
    registry: Arc<SidecarRegistry>,
    name: String,
    interval_ms: u64,
    max_failures: u64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let interval = std::time::Duration::from_millis(interval_ms);
        loop {
            tokio::time::sleep(interval).await;

            // Check if sidecar is still registered and connected
            let is_connected = registry
                .get(&name)
                .map(|e| e.connection.is_some() && e.health != SidecarHealth::Stopped)
                .unwrap_or(false);

            if !is_connected {
                {
                    use vil_log::app_log;
                    app_log!(Debug, "sidecar.health.loop.stop", { sidecar: vil_log::dict::register_str(&name) as u64 });
                }
                break;
            }

            if let Err(e) = health_check(&registry, &name, max_failures).await {
                {
                    use vil_log::app_log;
                    app_log!(Debug, "sidecar.health.check.failed", { sidecar: vil_log::dict::register_str(&name) as u64, error: e.to_string() });
                }
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum LifecycleError {
    NotRegistered(String),
    NotConnected(String),
    Transport(TransportError),
    ShmError(String),
    SpawnFailed(String),
    HandshakeTimeout(String),
    HealthTimeout(String),
    NameMismatch { expected: String, got: String },
    AuthFailed(String),
    UnexpectedMessage(String),
}

impl std::fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotRegistered(n) => write!(f, "sidecar '{}' not registered", n),
            Self::NotConnected(n) => write!(f, "sidecar '{}' not connected", n),
            Self::Transport(e) => write!(f, "transport: {}", e),
            Self::ShmError(e) => write!(f, "SHM: {}", e),
            Self::SpawnFailed(e) => write!(f, "spawn failed: {}", e),
            Self::HandshakeTimeout(n) => write!(f, "handshake timeout for '{}'", n),
            Self::HealthTimeout(n) => write!(f, "health check timeout for '{}'", n),
            Self::NameMismatch { expected, got } => {
                write!(f, "name mismatch: expected '{}', got '{}'", expected, got)
            }
            Self::AuthFailed(n) => write!(f, "authentication failed for '{}'", n),
            Self::UnexpectedMessage(m) => write!(f, "unexpected message: {}", m),
        }
    }
}

impl std::error::Error for LifecycleError {}

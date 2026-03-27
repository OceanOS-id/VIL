// =============================================================================
// Sidecar Transport — Async Unix Domain Socket communication
// =============================================================================
//
// Length-prefixed framing over UDS:
//   [4 bytes: payload length (LE u32)] [N bytes: JSON payload]
//
// The transport handles framing, sending, and receiving of protocol Messages.
// Actual data payloads live in SHM — only descriptors flow over the socket.

use crate::protocol::{self, Message};
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};

/// Error type for transport operations.
#[derive(Debug)]
pub enum TransportError {
    Io(std::io::Error),
    Encode(serde_json::Error),
    Decode(serde_json::Error),
    ConnectionClosed,
    FrameTooLarge(u32),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "transport I/O error: {}", e),
            Self::Encode(e) => write!(f, "encode error: {}", e),
            Self::Decode(e) => write!(f, "decode error: {}", e),
            Self::ConnectionClosed => write!(f, "connection closed"),
            Self::FrameTooLarge(size) => write!(f, "frame too large: {} bytes", size),
        }
    }
}

impl std::error::Error for TransportError {}

impl From<std::io::Error> for TransportError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Maximum frame size (16 MB). Protects against malformed length prefixes.
const MAX_FRAME_SIZE: u32 = 16 * 1024 * 1024;

// ---------------------------------------------------------------------------
// SidecarConnection — a single UDS connection with framed messaging
// ---------------------------------------------------------------------------

/// A framed connection to a sidecar (or from a sidecar to the host).
pub struct SidecarConnection {
    stream: UnixStream,
    read_buf: Vec<u8>,
}

impl SidecarConnection {
    /// Wrap an existing UnixStream.
    pub fn new(stream: UnixStream) -> Self {
        Self {
            stream,
            read_buf: Vec::with_capacity(4096),
        }
    }

    /// Connect to a sidecar at the given socket path.
    pub async fn connect(path: impl AsRef<Path>) -> Result<Self, TransportError> {
        let stream = UnixStream::connect(path).await?;
        Ok(Self::new(stream))
    }

    /// Send a protocol message.
    pub async fn send(&mut self, msg: &Message) -> Result<(), TransportError> {
        let json = serde_json::to_vec(msg).map_err(TransportError::Encode)?;
        let len = json.len() as u32;
        self.stream.write_all(&len.to_le_bytes()).await?;
        self.stream.write_all(&json).await?;
        self.stream.flush().await?;
        Ok(())
    }

    /// Receive a protocol message. Returns None if the connection is closed.
    pub async fn recv(&mut self) -> Result<Message, TransportError> {
        // Read 4-byte length prefix
        let mut len_buf = [0u8; 4];
        match self.stream.read_exact(&mut len_buf).await {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(TransportError::ConnectionClosed);
            }
            Err(e) => return Err(TransportError::Io(e)),
        }

        let len = u32::from_le_bytes(len_buf);
        if len > MAX_FRAME_SIZE {
            return Err(TransportError::FrameTooLarge(len));
        }

        // Read payload
        self.read_buf.resize(len as usize, 0);
        match self.stream.read_exact(&mut self.read_buf).await {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(TransportError::ConnectionClosed);
            }
            Err(e) => return Err(TransportError::Io(e)),
        }

        protocol::decode_message(&self.read_buf).map_err(TransportError::Decode)
    }

    /// Get a reference to the underlying stream (for shutdown, etc.).
    pub fn stream(&self) -> &UnixStream {
        &self.stream
    }
}

// ---------------------------------------------------------------------------
// SidecarListener — accepts incoming sidecar connections
// ---------------------------------------------------------------------------

/// Listens for incoming sidecar connections on a Unix domain socket.
pub struct SidecarListener {
    listener: UnixListener,
    path: String,
}

impl SidecarListener {
    /// Bind to the given socket path. Removes stale socket file if it exists.
    pub async fn bind(path: impl Into<String>) -> Result<Self, TransportError> {
        let path = path.into();
        // Remove stale socket file
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path)?;
        tracing::info!(path = %path, "sidecar listener bound");
        Ok(Self { listener, path })
    }

    /// Accept a new sidecar connection.
    pub async fn accept(&self) -> Result<SidecarConnection, TransportError> {
        let (stream, _addr) = self.listener.accept().await?;
        tracing::debug!(path = %self.path, "sidecar connection accepted");
        Ok(SidecarConnection::new(stream))
    }

    /// Get the socket path.
    pub fn path(&self) -> &str {
        &self.path
    }
}

impl Drop for SidecarListener {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

// ---------------------------------------------------------------------------
// Convenience: socket path generation
// ---------------------------------------------------------------------------

/// Generate a standard socket path for a sidecar.
///
/// Pattern: `/tmp/vil_sidecar_{name}.sock`
pub fn socket_path(name: &str) -> String {
    format!("/tmp/vil_sidecar_{}.sock", name)
}

/// Generate a standard SHM path for a sidecar.
///
/// Pattern: `/dev/shm/vil_sc_{name}`
pub fn shm_path(name: &str) -> String {
    format!("/dev/shm/vil_sc_{}", name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_send_recv_roundtrip() {
        let dir = tempdir().unwrap();
        let sock = dir.path().join("test.sock");
        let sock_str = sock.to_str().unwrap().to_string();

        let listener = SidecarListener::bind(&sock_str).await.unwrap();

        let send_handle = tokio::spawn({
            let sock_str = sock_str.clone();
            async move {
                let mut conn = SidecarConnection::connect(&sock_str).await.unwrap();
                conn.send(&Message::Handshake(Handshake {
                    name: "test-sidecar".into(),
                    version: "1.0".into(),
                    methods: vec!["ping".into()],
                    capabilities: vec![],
                    auth_token: None,
                }))
                .await
                .unwrap();

                // Wait for ack
                let ack = conn.recv().await.unwrap();
                assert!(matches!(ack, Message::HandshakeAck(_)));
            }
        });

        // Accept and respond
        let mut conn = listener.accept().await.unwrap();
        let msg = conn.recv().await.unwrap();

        match msg {
            Message::Handshake(h) => {
                assert_eq!(h.name, "test-sidecar");
                assert_eq!(h.methods, vec!["ping"]);
            }
            _ => panic!("expected Handshake"),
        }

        conn.send(&Message::HandshakeAck(HandshakeAck {
            accepted: true,
            shm_path: "/dev/shm/vil_sc_test".into(),
            shm_size: 64 * 1024 * 1024,
            reject_reason: None,
        }))
        .await
        .unwrap();

        send_handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_invoke_result_flow() {
        let dir = tempdir().unwrap();
        let sock = dir.path().join("invoke.sock");
        let sock_str = sock.to_str().unwrap().to_string();

        let listener = SidecarListener::bind(&sock_str).await.unwrap();

        let client = tokio::spawn({
            let sock_str = sock_str.clone();
            async move {
                let mut conn = SidecarConnection::connect(&sock_str).await.unwrap();

                // Send invoke
                conn.send(&Message::Invoke(Invoke {
                    descriptor: ShmDescriptor::new(1, 0, 0, 128).with_method("score"),
                    method: "score".into(),
                }))
                .await
                .unwrap();

                // Wait for result
                let result = conn.recv().await.unwrap();
                match result {
                    Message::Result(r) => {
                        assert_eq!(r.request_id, 1);
                        assert_eq!(r.status, InvokeStatus::Ok);
                    }
                    _ => panic!("expected Result"),
                }
            }
        });

        let mut conn = listener.accept().await.unwrap();
        let msg = conn.recv().await.unwrap();

        match msg {
            Message::Invoke(inv) => {
                assert_eq!(inv.method, "score");
                // Send result back
                conn.send(&Message::Result(InvokeResult {
                    request_id: inv.descriptor.request_id,
                    status: InvokeStatus::Ok,
                    descriptor: Some(ShmDescriptor::new(1, 0, 1024, 64)),
                    error: None,
                }))
                .await
                .unwrap();
            }
            _ => panic!("expected Invoke"),
        }

        client.await.unwrap();
    }

    #[tokio::test]
    async fn test_health_drain_shutdown() {
        let dir = tempdir().unwrap();
        let sock = dir.path().join("lifecycle.sock");
        let sock_str = sock.to_str().unwrap().to_string();

        let listener = SidecarListener::bind(&sock_str).await.unwrap();

        let client = tokio::spawn({
            let sock_str = sock_str.clone();
            async move {
                let mut conn = SidecarConnection::connect(&sock_str).await.unwrap();

                // Health
                conn.send(&Message::Health).await.unwrap();
                let resp = conn.recv().await.unwrap();
                assert!(matches!(resp, Message::HealthOk(_)));

                // Drain
                conn.send(&Message::Drain).await.unwrap();
                let resp = conn.recv().await.unwrap();
                assert!(matches!(resp, Message::Drained));

                // Shutdown
                conn.send(&Message::Shutdown).await.unwrap();
            }
        });

        let mut conn = listener.accept().await.unwrap();

        // Health
        let msg = conn.recv().await.unwrap();
        assert!(matches!(msg, Message::Health));
        conn.send(&Message::HealthOk(HealthOk {
            in_flight: 0,
            total_processed: 100,
            total_errors: 2,
            uptime_secs: 3600,
        }))
        .await
        .unwrap();

        // Drain
        let msg = conn.recv().await.unwrap();
        assert!(matches!(msg, Message::Drain));
        conn.send(&Message::Drained).await.unwrap();

        // Shutdown
        let msg = conn.recv().await.unwrap();
        assert!(matches!(msg, Message::Shutdown));

        client.await.unwrap();
    }

    #[test]
    fn test_socket_path() {
        assert_eq!(socket_path("fraud"), "/tmp/vil_sidecar_fraud.sock");
    }

    #[test]
    fn test_shm_path() {
        assert_eq!(shm_path("fraud"), "/dev/shm/vil_sc_fraud");
    }
}

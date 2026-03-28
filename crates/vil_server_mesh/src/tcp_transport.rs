// =============================================================================
// VIL Server Mesh — TCP Tri-Lane Transport
// =============================================================================
//
// Production-grade TCP transport for cross-host Tri-Lane communication.
// Used when services are on different hosts and SHM is unavailable.
//
// Wire Protocol (length-prefixed binary framing):
//   [4 bytes: total frame length (LE u32)]
//   [1 byte:  lane (0=Trigger, 1=Data, 2=Control)]
//   [2 bytes: from_len (LE u16)]
//   [from_len bytes: from service name]
//   [2 bytes: to_len (LE u16)]
//   [to_len bytes: to service name]
//   [4 bytes: payload_len (LE u32)]
//   [payload_len bytes: actual data]

use std::sync::Arc;

use dashmap::DashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use crate::Lane;

// =============================================================================
// Constants
// =============================================================================

/// Maximum frame size: 16 MB. Frames larger than this are rejected.
const MAX_FRAME_SIZE: u32 = 16 * 1024 * 1024;

/// Channel buffer size for lane receivers.
const LANE_CHANNEL_BUFFER: usize = 4096;

// =============================================================================
// Error Type
// =============================================================================

#[derive(Debug)]
pub enum TcpLaneError {
    ConnectionFailed(String),
    SendFailed(String),
    ReceiveFailed(String),
    ListenFailed(String),
    FrameTooLarge(u32),
    InvalidFrame(String),
}

impl std::fmt::Display for TcpLaneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TcpLaneError::ConnectionFailed(e) => write!(f, "TCP connection failed: {}", e),
            TcpLaneError::SendFailed(e) => write!(f, "TCP send failed: {}", e),
            TcpLaneError::ReceiveFailed(e) => write!(f, "TCP receive failed: {}", e),
            TcpLaneError::ListenFailed(e) => write!(f, "TCP listen failed: {}", e),
            TcpLaneError::FrameTooLarge(sz) => {
                write!(
                    f,
                    "TCP frame too large: {} bytes (max {})",
                    sz, MAX_FRAME_SIZE
                )
            }
            TcpLaneError::InvalidFrame(e) => write!(f, "Invalid TCP frame: {}", e),
        }
    }
}

impl std::error::Error for TcpLaneError {}

// =============================================================================
// Lane Message
// =============================================================================

/// A message received on a Tri-Lane TCP channel.
#[derive(Debug, Clone)]
pub struct LaneMessage {
    pub from: String,
    pub to: String,
    /// 0 = Trigger, 1 = Data, 2 = Control
    pub lane: u8,
    pub data: Vec<u8>,
}

// =============================================================================
// Tri-Lane Receivers (mpsc-based, mirrors SHM design)
// =============================================================================

/// Receiver channels for each of the three lanes.
pub struct TcpTriLaneReceivers {
    pub trigger: tokio::sync::mpsc::Receiver<LaneMessage>,
    pub data: tokio::sync::mpsc::Receiver<LaneMessage>,
    pub control: tokio::sync::mpsc::Receiver<LaneMessage>,
}

// =============================================================================
// Wire Protocol — encode / decode
// =============================================================================

/// Encode a Tri-Lane message into the wire format.
///
/// Returns the full frame including the 4-byte length prefix.
pub fn encode_frame(from: &str, to: &str, lane: Lane, payload: &[u8]) -> Vec<u8> {
    // Body = lane(1) + from_len(2) + from + to_len(2) + to + payload_len(4) + payload
    let body_len = 1 + 2 + from.len() + 2 + to.len() + 4 + payload.len();
    let mut frame = Vec::with_capacity(4 + body_len);

    // Total frame length (excluding this 4-byte header)
    frame.extend_from_slice(&(body_len as u32).to_le_bytes());

    // Lane byte
    frame.push(lane_to_byte(lane));

    // From service name
    frame.extend_from_slice(&(from.len() as u16).to_le_bytes());
    frame.extend_from_slice(from.as_bytes());

    // To service name
    frame.extend_from_slice(&(to.len() as u16).to_le_bytes());
    frame.extend_from_slice(to.as_bytes());

    // Payload
    frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    frame.extend_from_slice(payload);

    frame
}

/// Decode a Tri-Lane message from a body buffer (after the 4-byte length prefix
/// has already been consumed and `body_len` bytes read).
pub fn decode_frame(body: &[u8]) -> Result<LaneMessage, TcpLaneError> {
    if body.len() < 1 + 2 {
        return Err(TcpLaneError::InvalidFrame("frame too short".into()));
    }

    let mut pos = 0;

    // Lane
    let lane_byte = body[pos];
    if lane_byte > 2 {
        return Err(TcpLaneError::InvalidFrame(format!(
            "invalid lane byte: {}",
            lane_byte
        )));
    }
    pos += 1;

    // From
    if pos + 2 > body.len() {
        return Err(TcpLaneError::InvalidFrame("truncated from_len".into()));
    }
    let from_len = u16::from_le_bytes([body[pos], body[pos + 1]]) as usize;
    pos += 2;
    if pos + from_len > body.len() {
        return Err(TcpLaneError::InvalidFrame("truncated from field".into()));
    }
    let from = std::str::from_utf8(&body[pos..pos + from_len])
        .map_err(|e| TcpLaneError::InvalidFrame(format!("invalid from UTF-8: {}", e)))?
        .to_owned();
    pos += from_len;

    // To
    if pos + 2 > body.len() {
        return Err(TcpLaneError::InvalidFrame("truncated to_len".into()));
    }
    let to_len = u16::from_le_bytes([body[pos], body[pos + 1]]) as usize;
    pos += 2;
    if pos + to_len > body.len() {
        return Err(TcpLaneError::InvalidFrame("truncated to field".into()));
    }
    let to = std::str::from_utf8(&body[pos..pos + to_len])
        .map_err(|e| TcpLaneError::InvalidFrame(format!("invalid to UTF-8: {}", e)))?
        .to_owned();
    pos += to_len;

    // Payload
    if pos + 4 > body.len() {
        return Err(TcpLaneError::InvalidFrame("truncated payload_len".into()));
    }
    let payload_len =
        u32::from_le_bytes([body[pos], body[pos + 1], body[pos + 2], body[pos + 3]]) as usize;
    pos += 4;
    if pos + payload_len > body.len() {
        return Err(TcpLaneError::InvalidFrame("truncated payload".into()));
    }
    let data = body[pos..pos + payload_len].to_vec();

    Ok(LaneMessage {
        from,
        to,
        lane: lane_byte,
        data,
    })
}

fn lane_to_byte(lane: Lane) -> u8 {
    match lane {
        Lane::Trigger => 0,
        Lane::Data => 1,
        Lane::Control => 2,
    }
}

/// Convert a wire byte back to a `Lane` enum.
#[allow(dead_code)]
pub fn byte_to_lane(b: u8) -> Result<Lane, TcpLaneError> {
    match b {
        0 => Ok(Lane::Trigger),
        1 => Ok(Lane::Data),
        2 => Ok(Lane::Control),
        _ => Err(TcpLaneError::InvalidFrame(format!(
            "invalid lane byte: {}",
            b
        ))),
    }
}

// =============================================================================
// TcpTriLaneSender — Persistent connection with reconnect
// =============================================================================

/// Sends Tri-Lane messages over a persistent TCP connection.
///
/// Automatically reconnects if the connection drops. Thread-safe via
/// internal `Mutex` on the socket.
pub struct TcpTriLaneSender {
    addr: String,
    conn: Mutex<Option<TcpStream>>,
}

impl TcpTriLaneSender {
    pub fn new(addr: impl Into<String>) -> Self {
        Self {
            addr: addr.into(),
            conn: Mutex::new(None),
        }
    }

    /// Send a framed Tri-Lane message. Reconnects automatically on failure.
    pub async fn send(
        &self,
        from: &str,
        to: &str,
        lane: Lane,
        data: &[u8],
    ) -> Result<usize, TcpLaneError> {
        let frame = encode_frame(from, to, lane, data);

        let mut guard = self.conn.lock().await;

        // Attempt send on existing connection; reconnect once on failure.
        for attempt in 0..2u8 {
            if guard.is_none() {
                let stream = TcpStream::connect(&self.addr)
                    .await
                    .map_err(|e| TcpLaneError::ConnectionFailed(format!("{}: {}", self.addr, e)))?;
                stream
                    .set_nodelay(true)
                    .map_err(|e| TcpLaneError::ConnectionFailed(format!("set_nodelay: {}", e)))?;
                *guard = Some(stream);
            }

            let stream = guard.as_mut().unwrap();
            match stream.write_all(&frame).await {
                Ok(()) => return Ok(data.len()),
                Err(e) => {
                    // Drop broken connection; retry once.
                    *guard = None;
                    if attempt == 1 {
                        return Err(TcpLaneError::SendFailed(format!(
                            "send to {} failed after reconnect: {}",
                            self.addr, e
                        )));
                    }
                    {
                        use vil_log::app_log;
                        app_log!(Warn, "mesh.tcp.send.failed", { addr: self.addr.as_str(), error: e.to_string() });
                    }
                }
            }
        }

        Err(TcpLaneError::SendFailed("unreachable".into()))
    }

    pub fn addr(&self) -> &str {
        &self.addr
    }
}

// =============================================================================
// TcpTriLaneListener — Accept connections, dispatch to lane channels
// =============================================================================

/// Listens for incoming TCP Tri-Lane connections and dispatches messages
/// to per-lane mpsc channels.
pub struct TcpTriLaneListener {
    addr: String,
}

impl TcpTriLaneListener {
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into() }
    }

    /// Start listening. Returns receivers for the three lanes.
    ///
    /// Spawns a background tokio task that accepts TCP connections and
    /// reads framed messages, routing each to the correct lane channel.
    pub async fn start(&self) -> Result<(TcpTriLaneReceivers, std::net::SocketAddr), TcpLaneError> {
        let listener = TcpListener::bind(&self.addr)
            .await
            .map_err(|e| TcpLaneError::ListenFailed(format!("{}: {}", self.addr, e)))?;

        let local_addr = listener
            .local_addr()
            .map_err(|e| TcpLaneError::ListenFailed(format!("local_addr: {}", e)))?;

        let (trigger_tx, trigger_rx) = tokio::sync::mpsc::channel(LANE_CHANNEL_BUFFER);
        let (data_tx, data_rx) = tokio::sync::mpsc::channel(LANE_CHANNEL_BUFFER);
        let (control_tx, control_rx) = tokio::sync::mpsc::channel(LANE_CHANNEL_BUFFER);

        let senders = Arc::new([trigger_tx, data_tx, control_tx]);

        // Spawn the accept loop.
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, peer)) => {
                        let senders = senders.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(stream, &senders).await {
                                {
                                    use vil_log::app_log;
                                    app_log!(Debug, "mesh.tcp.conn.ended", { peer: vil_log::dict::register_str(&peer.to_string()) as u64, error: e.to_string() });
                                }
                            }
                        });
                    }
                    Err(e) => {
                        {
                            use vil_log::app_log;
                            app_log!(Error, "mesh.tcp.accept.error", { error: e.to_string() });
                        }
                        // Brief pause to avoid busy-loop on persistent errors.
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    }
                }
            }
        });

        {
            use vil_log::app_log;
            app_log!(Info, "mesh.tcp.listener.started", { addr: local_addr.to_string() });
        }

        Ok((
            TcpTriLaneReceivers {
                trigger: trigger_rx,
                data: data_rx,
                control: control_rx,
            },
            local_addr,
        ))
    }
}

/// Read framed messages from a single TCP connection and dispatch them.
async fn handle_connection(
    mut stream: TcpStream,
    senders: &[tokio::sync::mpsc::Sender<LaneMessage>; 3],
) -> Result<(), TcpLaneError> {
    let _ = stream.set_nodelay(true);

    loop {
        // Read 4-byte frame length.
        let mut len_buf = [0u8; 4];
        match stream.read_exact(&mut len_buf).await {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                // Clean disconnect.
                return Ok(());
            }
            Err(e) => {
                return Err(TcpLaneError::ReceiveFailed(format!(
                    "read frame length: {}",
                    e
                )));
            }
        }

        let frame_len = u32::from_le_bytes(len_buf);
        if frame_len > MAX_FRAME_SIZE {
            return Err(TcpLaneError::FrameTooLarge(frame_len));
        }

        // Read the body.
        let mut body = vec![0u8; frame_len as usize];
        stream
            .read_exact(&mut body)
            .await
            .map_err(|e| TcpLaneError::ReceiveFailed(format!("read frame body: {}", e)))?;

        let msg = decode_frame(&body)?;
        let lane_idx = msg.lane as usize;
        if lane_idx >= 3 {
            return Err(TcpLaneError::InvalidFrame(format!(
                "lane index out of range: {}",
                lane_idx
            )));
        }

        // Send to the appropriate lane channel. If the receiver is dropped,
        // we just silently discard (the service may have shut down).
        let _ = senders[lane_idx].send(msg).await;
    }
}

// =============================================================================
// TcpTriLaneRouter — Manages TCP connections for remote services
// =============================================================================

/// Central router for TCP Tri-Lane communication.
///
/// Manages peer addresses, persistent sender connections, and the
/// listener for incoming messages.
pub struct TcpTriLaneRouter {
    /// Remote service addresses: service_name -> "host:port"
    remote_peers: DashMap<String, String>,
    /// Persistent sender connections: addr -> TcpTriLaneSender
    senders: DashMap<String, Arc<TcpTriLaneSender>>,
    /// Listen address for incoming connections.
    listen_addr: String,
}

impl TcpTriLaneRouter {
    pub fn new(listen_addr: impl Into<String>) -> Self {
        Self {
            remote_peers: DashMap::new(),
            senders: DashMap::new(),
            listen_addr: listen_addr.into(),
        }
    }

    /// Register a remote peer (service name -> "host:port").
    pub fn add_peer(&self, service_name: &str, addr: &str) {
        self.remote_peers
            .insert(service_name.to_owned(), addr.to_owned());
        {
            use vil_log::app_log;
            app_log!(Info, "mesh.tcp.peer.registered", { service: service_name, addr: addr });
        }
    }

    /// Check whether a service has a registered remote peer address.
    pub fn has_peer(&self, service_name: &str) -> bool {
        self.remote_peers.contains_key(service_name)
    }

    /// Send data to a remote service via TCP.
    ///
    /// Lazily creates a persistent `TcpTriLaneSender` for each target address
    /// and reuses it across calls.
    pub async fn send(
        &self,
        from: &str,
        to: &str,
        lane: Lane,
        data: &[u8],
    ) -> Result<usize, TcpLaneError> {
        let addr = self
            .remote_peers
            .get(to)
            .map(|v| v.value().clone())
            .ok_or_else(|| {
                TcpLaneError::ConnectionFailed(format!("no registered peer for service '{}'", to))
            })?;

        // Get or create the sender for this address.
        let sender = self
            .senders
            .entry(addr.clone())
            .or_insert_with(|| Arc::new(TcpTriLaneSender::new(addr)))
            .value()
            .clone();

        sender.send(from, to, lane, data).await
    }

    /// Start the TCP listener. Returns receivers for incoming messages
    /// dispatched to the correct lane.
    pub async fn start_listener(&self) -> Result<TcpTriLaneReceivers, TcpLaneError> {
        let listener = TcpTriLaneListener::new(&self.listen_addr);
        let (receivers, local_addr) = listener.start().await?;
        {
            use vil_log::app_log;
            app_log!(Info, "mesh.tcp.router.active", { addr: local_addr.to_string() });
        }
        Ok(receivers)
    }

    /// Number of registered remote peers.
    pub fn peer_count(&self) -> usize {
        self.remote_peers.len()
    }

    /// The configured listen address.
    pub fn listen_addr(&self) -> &str {
        &self.listen_addr
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Lane;

    #[test]
    fn test_frame_encode_decode() {
        let from = "auth-service";
        let to = "gateway";
        let lane = Lane::Data;
        let payload = b"hello world";

        let frame = encode_frame(from, to, lane, payload);

        // Verify length prefix.
        let body_len = u32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]) as usize;
        assert_eq!(body_len, frame.len() - 4);

        // Decode the body (skip the 4-byte length prefix).
        let msg = decode_frame(&frame[4..]).expect("decode should succeed");
        assert_eq!(msg.from, from);
        assert_eq!(msg.to, to);
        assert_eq!(msg.lane, 1); // Data = 1
        assert_eq!(msg.data, payload);
    }

    #[test]
    fn test_frame_encode_decode_all_lanes() {
        for (lane, expected_byte) in [
            (Lane::Trigger, 0u8),
            (Lane::Data, 1u8),
            (Lane::Control, 2u8),
        ] {
            let frame = encode_frame("a", "b", lane, b"x");
            let msg = decode_frame(&frame[4..]).unwrap();
            assert_eq!(msg.lane, expected_byte);
            assert_eq!(msg.from, "a");
            assert_eq!(msg.to, "b");
            assert_eq!(msg.data, b"x");
        }
    }

    #[test]
    fn test_decode_invalid_frame() {
        // Too short.
        assert!(decode_frame(&[]).is_err());
        assert!(decode_frame(&[0]).is_err());

        // Invalid lane byte.
        let mut frame = encode_frame("a", "b", Lane::Trigger, b"x");
        frame[4] = 99; // corrupt lane byte in the body
        assert!(decode_frame(&frame[4..]).is_err());
    }

    #[tokio::test]
    async fn test_send_recv_loopback() {
        // Start listener on ephemeral port.
        let listener = TcpTriLaneListener::new("127.0.0.1:0");
        let (mut receivers, local_addr) = listener.start().await.expect("listen should succeed");

        // Send a message via TcpTriLaneSender.
        let sender = TcpTriLaneSender::new(local_addr.to_string());
        let bytes_sent = sender
            .send("svc-a", "svc-b", Lane::Data, b"payload-42")
            .await
            .expect("send should succeed");
        assert_eq!(bytes_sent, 10);

        // Receive it on the Data lane.
        let msg = tokio::time::timeout(std::time::Duration::from_secs(2), receivers.data.recv())
            .await
            .expect("should receive within timeout")
            .expect("channel should not be closed");

        assert_eq!(msg.from, "svc-a");
        assert_eq!(msg.to, "svc-b");
        assert_eq!(msg.lane, 1);
        assert_eq!(msg.data, b"payload-42");
    }

    #[tokio::test]
    async fn test_lane_routing() {
        // Start listener.
        let listener = TcpTriLaneListener::new("127.0.0.1:0");
        let (mut receivers, local_addr) = listener.start().await.expect("listen");

        let sender = TcpTriLaneSender::new(local_addr.to_string());

        // Send one message on each lane.
        sender
            .send("a", "b", Lane::Trigger, b"trigger-msg")
            .await
            .unwrap();
        sender
            .send("a", "b", Lane::Data, b"data-msg")
            .await
            .unwrap();
        sender
            .send("a", "b", Lane::Control, b"control-msg")
            .await
            .unwrap();

        let timeout = std::time::Duration::from_secs(2);

        // Each message should arrive on the correct lane channel.
        let trigger_msg = tokio::time::timeout(timeout, receivers.trigger.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(trigger_msg.data, b"trigger-msg");
        assert_eq!(trigger_msg.lane, 0);

        let data_msg = tokio::time::timeout(timeout, receivers.data.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(data_msg.data, b"data-msg");
        assert_eq!(data_msg.lane, 1);

        let control_msg = tokio::time::timeout(timeout, receivers.control.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(control_msg.data, b"control-msg");
        assert_eq!(control_msg.lane, 2);
    }

    #[tokio::test]
    async fn test_router_send_recv() {
        // Set up a router with a listener.
        let router = TcpTriLaneRouter::new("127.0.0.1:0");

        // We need to start the listener first to get the actual bound address.
        // Since the router binds to :0, we use TcpTriLaneListener directly
        // and then create a sender pointing to the actual port.
        let listener = TcpTriLaneListener::new("127.0.0.1:0");
        let (mut receivers, local_addr) = listener.start().await.unwrap();

        // Register the peer at the actual listen address.
        router.add_peer("svc-target", &local_addr.to_string());
        assert_eq!(router.peer_count(), 1);
        assert!(router.has_peer("svc-target"));
        assert!(!router.has_peer("unknown-svc"));

        // Send through the router.
        let n = router
            .send("svc-origin", "svc-target", Lane::Trigger, b"hello-router")
            .await
            .unwrap();
        assert_eq!(n, b"hello-router".len());

        let msg = tokio::time::timeout(std::time::Duration::from_secs(2), receivers.trigger.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(msg.from, "svc-origin");
        assert_eq!(msg.to, "svc-target");
        assert_eq!(msg.data, b"hello-router");
    }

    #[tokio::test]
    async fn test_router_unknown_peer() {
        let router = TcpTriLaneRouter::new("127.0.0.1:0");
        let result = router.send("a", "nonexistent", Lane::Data, b"fail").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            TcpLaneError::ConnectionFailed(msg) => {
                assert!(msg.contains("nonexistent"));
            }
            other => panic!("expected ConnectionFailed, got {:?}", other),
        }
    }
}

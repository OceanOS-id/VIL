// =============================================================================
// VIL Server Mesh Channel — SHM and TCP channels
// =============================================================================
//
// SHM channels for co-located services (1-5µs latency).
// TCP channels for remote services (fallback, ~500µs).

use bytes::Bytes;
use tokio::sync::mpsc;

/// A message sent through the mesh channel.
#[derive(Debug, Clone)]
pub struct MeshMessage {
    /// Source service
    pub from: String,
    /// Target service
    pub to: String,
    /// Lane type
    pub lane: super::Lane,
    /// Payload (Bytes for zero-copy sharing within process)
    pub payload: Bytes,
}

/// Channel handle for sending messages between services.
#[derive(Clone)]
pub struct MeshSender {
    tx: mpsc::Sender<MeshMessage>,
}

/// Channel handle for receiving messages from other services.
pub struct MeshReceiver {
    rx: mpsc::Receiver<MeshMessage>,
}

/// Create a new mesh channel pair.
pub fn mesh_channel(buffer_size: usize) -> (MeshSender, MeshReceiver) {
    let (tx, rx) = mpsc::channel(buffer_size);
    (MeshSender { tx }, MeshReceiver { rx })
}

impl MeshSender {
    /// Send a message through the channel.
    pub async fn send(&self, msg: MeshMessage) -> Result<(), MeshSendError> {
        self.tx
            .send(msg)
            .await
            .map_err(|_| MeshSendError::ChannelClosed)
    }
}

impl MeshReceiver {
    /// Receive the next message from the channel.
    pub async fn recv(&mut self) -> Option<MeshMessage> {
        self.rx.recv().await
    }
}

/// Error when sending a mesh message.
#[derive(Debug)]
pub enum MeshSendError {
    ChannelClosed,
}

impl std::fmt::Display for MeshSendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeshSendError::ChannelClosed => write!(f, "Mesh channel closed"),
        }
    }
}

impl std::error::Error for MeshSendError {}

//! Typed inter-agent message channel.

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// A message passed between agents in the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// Name of the sending agent.
    pub from: String,
    /// Name of the receiving agent.
    pub to: String,
    /// Textual content (typically the agent's output).
    pub content: String,
    /// Arbitrary metadata attached to the message.
    pub metadata: serde_json::Value,
}

impl AgentMessage {
    /// Create a new message.
    pub fn new(from: impl Into<String>, to: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            content: content.into(),
            metadata: serde_json::Value::Null,
        }
    }

    /// Attach metadata to the message.
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// A typed channel between two agents.
///
/// Wraps a tokio MPSC channel so that one agent can send `AgentMessage`s
/// that another agent will receive.
pub struct AgentChannel {
    pub tx: mpsc::Sender<AgentMessage>,
    pub rx: mpsc::Receiver<AgentMessage>,
}

impl AgentChannel {
    /// Create a new channel pair with the given buffer capacity.
    pub fn new(buffer: usize) -> Self {
        let (tx, rx) = mpsc::channel(buffer);
        Self { tx, rx }
    }

    /// Split into sender / receiver halves (consumes self).
    pub fn split(self) -> (mpsc::Sender<AgentMessage>, mpsc::Receiver<AgentMessage>) {
        (self.tx, self.rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_channel_send_recv() {
        let ch = AgentChannel::new(8);
        let (tx, mut rx) = ch.split();

        let msg = AgentMessage::new("a", "b", "hello");
        tx.send(msg).await.unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received.from, "a");
        assert_eq!(received.to, "b");
        assert_eq!(received.content, "hello");
    }

    #[tokio::test]
    async fn test_message_with_metadata() {
        let msg =
            AgentMessage::new("x", "y", "data").with_metadata(serde_json::json!({"key": "value"}));
        assert_eq!(msg.metadata["key"], "value");
    }
}

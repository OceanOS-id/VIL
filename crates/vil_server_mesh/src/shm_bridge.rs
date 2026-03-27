// =============================================================================
// VIL Server Mesh SHM Bridge — Zero-copy inter-service communication
// =============================================================================
//
// Bridges vil_shm::ExchangeHeap into the mesh channel system.
// Co-located services use SHM regions for zero-copy message passing.
// Remote services fall back to TCP (via the standard MeshChannel).
//
// Architecture:
//   Producer → write payload to SHM region → publish offset + metadata via mpsc
//   Consumer → read offset from mpsc → resolve pointer in SHM → zero-copy access

use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::mpsc;
use vil_shm::{ExchangeHeap, Offset, RegionStats};

use crate::Lane;

/// Region identifier for SHM channels (re-exported from vil_types).
pub type ShmRegionId = vil_types::RegionId;

/// A message descriptor passed through the SHM channel.
/// Instead of carrying the payload, it carries an offset into the SHM region.
#[derive(Debug, Clone)]
pub struct ShmDescriptor {
    /// Source service name
    pub from: String,
    /// Target service name
    pub to: String,
    /// Lane type
    pub lane: Lane,
    /// SHM region where the data lives
    pub region_id: ShmRegionId,
    /// Offset into the SHM region
    pub offset: Offset,
    /// Length of the payload in bytes
    pub len: usize,
}

/// SHM-backed mesh channel for zero-copy inter-service communication.
///
/// Instead of copying payload bytes through a tokio mpsc channel,
/// the producer writes data into a shared ExchangeHeap region and
/// sends only a small descriptor (offset + length) through the channel.
/// The consumer reads the data directly from SHM using the offset.
pub struct ShmMeshChannel {
    heap: Arc<ExchangeHeap>,
    region_id: ShmRegionId,
    tx: mpsc::Sender<ShmDescriptor>,
    region_name: String,
}

/// Receiver side of an SHM mesh channel.
pub struct ShmMeshReceiver {
    heap: Arc<ExchangeHeap>,
    rx: mpsc::Receiver<ShmDescriptor>,
}

impl ShmMeshChannel {
    /// Create a new SHM mesh channel pair for a service.
    ///
    /// Allocates a dedicated SHM region for the channel.
    /// Default region size: 16MB (suitable for most workloads).
    pub fn new(
        heap: Arc<ExchangeHeap>,
        service_name: &str,
        buffer_size: usize,
        region_size: usize,
    ) -> (Self, ShmMeshReceiver) {
        let region_name = format!("vil_mesh_{}", service_name);
        let region_id = heap.create_region(&region_name, region_size);

        let (tx, rx) = mpsc::channel(buffer_size);

        let channel = ShmMeshChannel {
            heap: heap.clone(),
            region_id,
            tx,
            region_name,
        };

        let receiver = ShmMeshReceiver {
            heap,
            rx,
        };

        (channel, receiver)
    }

    /// Write data into SHM and send a descriptor through the channel.
    /// Returns the number of bytes written, or an error.
    pub async fn send(
        &self,
        from: &str,
        to: &str,
        lane: Lane,
        data: &[u8],
    ) -> Result<usize, ShmChannelError> {
        // Allocate space in the SHM region
        let offset = self.heap
            .alloc_bytes(self.region_id, data.len(), 8)
            .ok_or(ShmChannelError::RegionFull)?;

        // Write data into SHM (single copy: caller buffer → SHM)
        if !self.heap.write_bytes(self.region_id, offset, data) {
            return Err(ShmChannelError::WriteFailed);
        }

        // Send only the descriptor (not the data) through the channel
        let desc = ShmDescriptor {
            from: from.to_string(),
            to: to.to_string(),
            lane,
            region_id: self.region_id,
            offset,
            len: data.len(),
        };

        self.tx.send(desc).await.map_err(|_| ShmChannelError::ChannelClosed)?;

        Ok(data.len())
    }

    /// Get the SHM region name
    pub fn region_name(&self) -> &str {
        &self.region_name
    }

    /// Get region statistics
    pub fn stats(&self) -> Option<RegionStats> {
        self.heap.region_stats(self.region_id)
    }
}

impl ShmMeshReceiver {
    /// Receive the next descriptor and read the data from SHM.
    /// This performs zero-copy — the returned Bytes points directly
    /// to a copy of the SHM data (one allocation for the Bytes container).
    pub async fn recv(&mut self) -> Option<ShmMessage> {
        let desc = self.rx.recv().await?;

        // Read data from SHM region
        let data = self.heap.read_bytes(desc.region_id, desc.offset, desc.len)?;

        Some(ShmMessage {
            from: desc.from,
            to: desc.to,
            lane: desc.lane,
            payload: Bytes::from(data),
        })
    }

    /// Receive just the descriptor (for advanced zero-copy scenarios
    /// where the consumer wants to read directly from SHM).
    pub async fn recv_descriptor(&mut self) -> Option<ShmDescriptor> {
        self.rx.recv().await
    }

    /// Read bytes directly from SHM using a descriptor.
    /// Returns None if the region or offset is invalid.
    pub fn read_from_descriptor(&self, desc: &ShmDescriptor) -> Option<Vec<u8>> {
        self.heap.read_bytes(desc.region_id, desc.offset, desc.len)
    }
}

/// A fully materialized message from the SHM channel.
#[derive(Debug, Clone)]
pub struct ShmMessage {
    pub from: String,
    pub to: String,
    pub lane: Lane,
    pub payload: Bytes,
}

/// Errors from SHM channel operations.
#[derive(Debug)]
pub enum ShmChannelError {
    /// SHM region is full — needs compaction or larger region
    RegionFull,
    /// Failed to write data to SHM
    WriteFailed,
    /// Channel receiver has been dropped
    ChannelClosed,
}

impl std::fmt::Display for ShmChannelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShmChannelError::RegionFull => write!(f, "SHM region full"),
            ShmChannelError::WriteFailed => write!(f, "SHM write failed"),
            ShmChannelError::ChannelClosed => write!(f, "SHM channel closed"),
        }
    }
}

impl std::error::Error for ShmChannelError {}

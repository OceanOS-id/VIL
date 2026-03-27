// =============================================================================
// VIL Inference — Dynamic Batch Scheduler
// =============================================================================
// Collects individual inference requests and batches them for efficiency.
// Flushes when max_batch_size is reached or after max_wait_ms timeout.
// =============================================================================

use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};
use tokio::time::Duration;
use tracing;

use crate::backend::{InferError, InferInput, InferOutput, ModelBackend};

/// A pending inference request waiting to be batched.
struct PendingRequest {
    input: InferInput,
    response_tx: oneshot::Sender<Result<InferOutput, InferError>>,
}

/// Dynamic batch scheduler that transparently batches individual requests.
pub struct DynamicBatcher {
    backend: Arc<dyn ModelBackend>,
    max_batch_size: usize,
    max_wait_ms: u64,
    pending: Mutex<Vec<PendingRequest>>,
}

impl DynamicBatcher {
    /// Create a new batcher. Spawns a background flush timer.
    pub fn new(
        backend: Arc<dyn ModelBackend>,
        max_batch_size: usize,
        max_wait_ms: u64,
    ) -> Arc<Self> {
        let batcher = Arc::new(Self {
            backend,
            max_batch_size,
            max_wait_ms,
            pending: Mutex::new(Vec::new()),
        });

        // Spawn background timer for partial-batch flushing
        let batcher_clone = Arc::clone(&batcher);
        tokio::spawn(async move {
            let interval = Duration::from_millis(batcher_clone.max_wait_ms);
            loop {
                tokio::time::sleep(interval).await;
                batcher_clone.flush().await;
            }
        });

        batcher
    }

    /// Submit a single inference request. Batching happens transparently.
    pub async fn infer(&self, input: InferInput) -> Result<InferOutput, InferError> {
        let (tx, rx) = oneshot::channel();
        let should_flush;

        {
            let mut pending = self.pending.lock().await;
            pending.push(PendingRequest {
                input,
                response_tx: tx,
            });
            should_flush = pending.len() >= self.max_batch_size;
        }

        if should_flush {
            self.flush().await;
        }

        // Wait for the result from the batch execution
        rx.await.unwrap_or(Err(InferError::ExecutionFailed(
            "batcher channel closed".into(),
        )))
    }

    /// Flush all pending requests as a single batch.
    async fn flush(&self) {
        let requests: Vec<PendingRequest> = {
            let mut pending = self.pending.lock().await;
            if pending.is_empty() {
                return;
            }
            std::mem::take(&mut *pending)
        };

        let count = requests.len();
        tracing::debug!(count, "flushing batch");

        let inputs: Vec<InferInput> = requests.iter().map(|r| r.input.clone()).collect();
        let results = self.backend.infer_batch(&inputs).await;

        match results {
            Ok(outputs) => {
                for (req, output) in requests.into_iter().zip(outputs.into_iter()) {
                    let _ = req.response_tx.send(Ok(output));
                }
            }
            Err(e) => {
                for req in requests {
                    let _ = req.response_tx.send(Err(e.clone()));
                }
            }
        }
    }
}

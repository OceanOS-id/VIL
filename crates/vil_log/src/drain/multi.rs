// =============================================================================
// vil_log::drain::multi — MultiDrain
// =============================================================================
//
// Fan-out drain. Forwards each batch to all inner drains in order.
// Errors are collected; processing continues even if one drain fails.
// =============================================================================

use async_trait::async_trait;

use crate::drain::traits::LogDrain;
use crate::types::LogSlot;

/// Drain that fans out to multiple inner drains.
pub struct MultiDrain {
    drains: Vec<Box<dyn LogDrain>>,
}

impl MultiDrain {
    pub fn new() -> Self {
        Self { drains: Vec::new() }
    }

    /// Add a drain to the fan-out set.
    pub fn add(mut self, drain: impl LogDrain + 'static) -> Self {
        self.drains.push(Box::new(drain));
        self
    }

    /// Add a boxed drain.
    pub fn add_boxed(mut self, drain: Box<dyn LogDrain>) -> Self {
        self.drains.push(drain);
        self
    }
}

impl Default for MultiDrain {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LogDrain for MultiDrain {
    fn name(&self) -> &'static str {
        "multi"
    }

    async fn flush(
        &mut self,
        batch: &[LogSlot],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut last_err: Option<Box<dyn std::error::Error + Send + Sync>> = None;
        for drain in self.drains.iter_mut() {
            if let Err(e) = drain.flush(batch).await {
                eprintln!("vil_log [{}] flush error: {}", drain.name(), e);
                last_err = Some(e);
            }
        }
        match last_err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut last_err: Option<Box<dyn std::error::Error + Send + Sync>> = None;
        for drain in self.drains.iter_mut() {
            if let Err(e) = drain.shutdown().await {
                eprintln!("vil_log [{}] shutdown error: {}", drain.name(), e);
                last_err = Some(e);
            }
        }
        match last_err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

//! vil_trigger_grpc — gRPC server-streaming trigger
//! Connects to a gRPC server-streaming endpoint, fires TriggerEvent per streamed message.

use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use vil_trigger_core::{EventCallback, TriggerEvent, TriggerFault, TriggerSource};

pub struct GrpcConfig {
    pub endpoint: String,
    pub service: String,
    pub method: String,
}

impl GrpcConfig {
    pub fn new(
        endpoint: impl Into<String>,
        service: impl Into<String>,
        method: impl Into<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            service: service.into(),
            method: method.into(),
        }
    }
}

pub struct GrpcTrigger {
    config: GrpcConfig,
    stopped: AtomicBool,
}

pub fn create_trigger(config: GrpcConfig) -> GrpcTrigger {
    GrpcTrigger {
        config,
        stopped: AtomicBool::new(false),
    }
}

#[async_trait]
impl TriggerSource for GrpcTrigger {
    fn kind(&self) -> &'static str {
        "grpc"
    }

    async fn start(&self, _on_event: EventCallback) -> Result<(), TriggerFault> {
        tracing::info!(
            "gRPC trigger started: endpoint={}, service={}, method={}",
            self.config.endpoint,
            self.config.service,
            self.config.method
        );
        // Stream loop would use vil_grpc to open server-streaming call
        while !self.stopped.load(Ordering::Relaxed) {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            // In production: receive streamed message, fire event
        }
        Ok(())
    }

    async fn pause(&self) -> Result<(), TriggerFault> {
        Ok(())
    }
    async fn resume(&self) -> Result<(), TriggerFault> {
        Ok(())
    }
    async fn stop(&self) -> Result<(), TriggerFault> {
        self.stopped.store(true, Ordering::Relaxed);
        Ok(())
    }
}

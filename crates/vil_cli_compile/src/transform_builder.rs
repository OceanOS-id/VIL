//! TransformBuilder — runtime builder for transform nodes.
//!
//! A transform node receives data, applies a function, and publishes the result.
//! Follows the HttpSinkBuilder/HttpSourceBuilder pattern.

use std::sync::Arc;
use vil_rt::VastarRuntimeWorld;
use vil_types::{
    BackpressurePolicy, BoundaryKind, CleanupPolicy, DeliveryGuarantee, ExecClass, GenericToken,
    ObservabilitySpec, PortDirection, PortSpec, Priority, ProcessSpec, QueueKind, TransferMode,
};

/// Builder for configuring a transform node.
pub struct TransformBuilder {
    pub name: String,
    pub in_port_name: String,
    pub out_port_name: String,
    pub ctrl_out_port_name: Option<String>,
    pub capacity: usize,
}

#[allow(dead_code)]
impl TransformBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            in_port_name: "in".into(),
            out_port_name: "out".into(),
            ctrl_out_port_name: None,
            capacity: 1024,
        }
    }

    pub fn in_port(mut self, name: impl Into<String>) -> Self {
        self.in_port_name = name.into();
        self
    }

    pub fn out_port(mut self, name: impl Into<String>) -> Self {
        self.out_port_name = name.into();
        self
    }

    pub fn ctrl_out_port(mut self, name: impl Into<String>) -> Self {
        self.ctrl_out_port_name = Some(name.into());
        self
    }

    pub fn build_spec(&self) -> ProcessSpec {
        let mut ports = vec![
            PortSpec {
                name: Box::leak(self.in_port_name.clone().into_boxed_str()),
                direction: PortDirection::In,
                queue: QueueKind::Mpmc,
                capacity: self.capacity,
                backpressure: BackpressurePolicy::Block,
                transfer_mode: TransferMode::LoanRead,
                boundary: BoundaryKind::InterThreadLocal,
                timeout_ms: None,
                priority: Priority::Normal,
                delivery: DeliveryGuarantee::BestEffort,
                observability: ObservabilitySpec::default(),
            },
            PortSpec {
                name: Box::leak(self.out_port_name.clone().into_boxed_str()),
                direction: PortDirection::Out,
                queue: QueueKind::Mpmc,
                capacity: self.capacity,
                backpressure: BackpressurePolicy::Block,
                transfer_mode: TransferMode::LoanWrite,
                boundary: BoundaryKind::InterThreadLocal,
                timeout_ms: None,
                priority: Priority::Normal,
                delivery: DeliveryGuarantee::BestEffort,
                observability: ObservabilitySpec::default(),
            },
        ];

        if let Some(ref ctrl) = self.ctrl_out_port_name {
            ports.push(PortSpec {
                name: Box::leak(ctrl.clone().into_boxed_str()),
                direction: PortDirection::Out,
                queue: QueueKind::Mpmc,
                capacity: 256,
                backpressure: BackpressurePolicy::Block,
                transfer_mode: TransferMode::Copy,
                boundary: BoundaryKind::InterThreadLocal,
                timeout_ms: None,
                priority: Priority::Normal,
                delivery: DeliveryGuarantee::BestEffort,
                observability: ObservabilitySpec::default(),
            });
        }

        ProcessSpec {
            id: Box::leak(self.name.to_lowercase().into_boxed_str()),
            name: Box::leak(self.name.clone().into_boxed_str()),
            exec: ExecClass::Thread,
            cleanup: CleanupPolicy::ReclaimOrphans,
            ports: Box::leak(ports.into_boxed_slice()),
            observability: ObservabilitySpec::default(),
        }
    }
}

/// Runtime transform node.
pub struct TransformNode {
    builder: TransformBuilder,
}

impl TransformNode {
    pub fn from_builder(builder: TransformBuilder) -> Self {
        Self { builder }
    }

    /// Run the transform worker with a passthrough function.
    /// The transform_fn processes raw bytes: input → output.
    pub fn run_worker<F>(
        self,
        world: Arc<VastarRuntimeWorld>,
        handle: vil_rt::ProcessHandle,
        transform_fn: F,
    ) -> std::thread::JoinHandle<()>
    where
        F: Fn(&[u8]) -> Vec<u8> + Send + 'static,
    {
        let in_port = self.builder.in_port_name.clone();
        let out_port = self.builder.out_port_name.clone();
        let name = self.builder.name.clone();

        std::thread::spawn(move || {
            let in_pid = match handle.port_id(&in_port) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!(
                        "[transform:{}] in_port '{}' not found: {}",
                        name, in_port, e
                    );
                    return;
                }
            };
            let out_pid = match handle.port_id(&out_port) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!(
                        "[transform:{}] out_port '{}' not found: {}",
                        name, out_port, e
                    );
                    return;
                }
            };

            let process_id = handle.id();

            loop {
                match world.recv::<GenericToken>(in_pid) {
                    Ok(guard) => {
                        let input_data = guard.data.as_slice();
                        let output_data = transform_fn(input_data);

                        // Publish transformed data to out_port
                        match world.loan_uninit::<GenericToken>(out_pid) {
                            Ok(loan) => {
                                let token = GenericToken {
                                    session_id: guard.session_id,
                                    is_done: guard.is_done,
                                    data: vil_types::VSlice::from_vec(output_data),
                                };
                                let written = loan.write(token);
                                let _ = world.publish(process_id, out_pid, written);
                            }
                            Err(_) => {
                                // Could not loan — skip this message
                            }
                        }

                        if guard.is_done {
                            break;
                        }
                    }
                    Err(_) => {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                }
            }
        })
    }
}

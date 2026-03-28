// =============================================================================
// crates/vil_capsule/src/runner.rs — CapsuleRunner
// =============================================================================
// Bridges VIL workflows with WASM capsules.
// CapsuleRunner receives messages from VIL lanes, forwards them to a capsule,
// and processes the results back into VIL lanes.
// =============================================================================

use crate::{CapsuleError, CapsuleHost, CapsuleInput, CapsuleOutput};

/// Runner that integrates CapsuleHost into the VIL runtime world.
/// Attached to a ProcessIR with trust_zone = WasmCapsule.
pub struct CapsuleRunner {
    host: CapsuleHost,
    pub process_name: String,
}

impl CapsuleRunner {
    pub fn new(process_name: impl Into<String>, host: CapsuleHost) -> Self {
        Self {
            host,
            process_name: process_name.into(),
        }
    }

    /// Send a payload to the capsule and process the result.
    /// In production, `payload` is serialized data from the input port.
    pub fn dispatch(
        &self,
        function_name: impl Into<String>,
        payload: Vec<u8>,
    ) -> Result<CapsuleOutput, CapsuleError> {
        let input = CapsuleInput {
            function_name: function_name.into(),
            payload,
        };

        self.host.call(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CapsuleConfig, CapsuleHost};

    #[test]
    fn test_runner_no_wasm_feature() {
        let config = CapsuleConfig::new("TestPlugin", vec![0x00, 0x61, 0x73, 0x6d]); // WAA magic
        let host = CapsuleHost::new(config);
        let runner = CapsuleRunner::new("WasmPlugin", host);

        let result = runner.dispatch("process", vec![42u8]);

        #[cfg(not(feature = "wasm"))]
        {
            match result {
                Err(CapsuleError::WasmFeatureNotEnabled) => {} // Expected!
                _ => panic!("Expected WasmFeatureNotEnabled error"),
            }
        }
    }
}

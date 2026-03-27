// =============================================================================
// vil_edge_deploy::error — EdgeFault
// =============================================================================

use vil_connector_macros::connector_fault;
use vil_log::dict::register_str;

/// Faults that can occur in edge deployment operations.
#[connector_fault]
pub enum EdgeFault {
    /// The YAML config file could not be read.
    ConfigReadFailed,
    /// The YAML config file could not be parsed.
    ConfigParseFailed,
    /// The config serialization to YAML failed.
    SerializeFailed,
    /// The requested target is not supported.
    UnsupportedTarget,
    /// The requested profile is invalid.
    InvalidProfile,
    /// A required field was missing from config.
    MissingField,
}

impl EdgeFault {
    /// Returns the registered hash for this fault variant's name.
    pub fn code_hash(&self) -> u32 {
        match self {
            EdgeFault::ConfigReadFailed  => register_str("edge_deploy.fault.config_read_failed"),
            EdgeFault::ConfigParseFailed => register_str("edge_deploy.fault.config_parse_failed"),
            EdgeFault::SerializeFailed   => register_str("edge_deploy.fault.serialize_failed"),
            EdgeFault::UnsupportedTarget => register_str("edge_deploy.fault.unsupported_target"),
            EdgeFault::InvalidProfile    => register_str("edge_deploy.fault.invalid_profile"),
            EdgeFault::MissingField      => register_str("edge_deploy.fault.missing_field"),
        }
    }
}

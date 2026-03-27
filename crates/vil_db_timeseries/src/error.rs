// =============================================================================
// vil_db_timeseries::error — TimeseriesFault
// =============================================================================
//
// VIL-compliant fault enum for time-series operations.
// No String fields, no thiserror — only u32/u64 numeric codes.
// =============================================================================

use vil_connector_macros::connector_fault;

/// Fault type for all time-series operations.
///
/// All string-derived context is stored as u32 FxHash values registered via
/// `vil_log::dict::register_str()`.
#[connector_fault]
pub enum TimeseriesFault {
    /// Failed to connect to the backend.
    ConnectionFailed {
        /// FxHash of the host/URL string.
        host_hash: u32,
        /// Numeric reason code.
        reason_code: u32,
    },
    /// `write_points` operation failed.
    WriteFailed {
        /// FxHash of the bucket/measurement name.
        bucket_hash: u32,
        /// Numeric reason code.
        reason_code: u32,
    },
    /// `query_flux` operation failed.
    QueryFailed {
        /// FxHash of the Flux query string.
        query_hash: u32,
        /// Numeric reason code.
        reason_code: u32,
    },
    /// Feature not enabled for the selected backend.
    FeatureNotEnabled {
        /// FxHash of the feature name.
        feature_hash: u32,
    },
}

// =============================================================================
// vil_db_timeseries::error — TimeseriesFault
// =============================================================================
//
// VIL-compliant fault enum for time-series operations.
// No String fields, no thiserror — only u32/u64 numeric codes.
// =============================================================================

/// Fault type for all time-series operations.
///
/// All string-derived context is stored as u32 FxHash values registered via
/// `vil_log::dict::register_str()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl TimeseriesFault {
    /// Return a stable numeric error code for log `error_code` fields.
    pub fn as_error_code(&self) -> u32 {
        match self {
            TimeseriesFault::ConnectionFailed { .. } => 1,
            TimeseriesFault::WriteFailed { .. } => 2,
            TimeseriesFault::QueryFailed { .. } => 3,
            TimeseriesFault::FeatureNotEnabled { .. } => 4,
        }
    }
}

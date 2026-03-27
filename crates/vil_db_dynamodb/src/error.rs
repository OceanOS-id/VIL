// =============================================================================
// vil_db_dynamodb::error — DynamoFault
// =============================================================================
//
// VIL-compliant fault enum for DynamoDB operations.
// No String fields, no thiserror — only u32/u64 numeric codes.
// =============================================================================

/// Fault type for all DynamoDB operations.
///
/// All string-derived context is stored as u32 FxHash values registered via
/// `vil_log::dict::register_str()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamoFault {
    /// Failed to build or load AWS config/credentials.
    ConfigFailed {
        /// Numeric reason code.
        reason_code: u32,
    },
    /// `GetItem` operation failed.
    GetFailed {
        /// FxHash of the table name.
        table_hash: u32,
        /// Numeric reason code.
        reason_code: u32,
    },
    /// `PutItem` operation failed.
    PutFailed {
        /// FxHash of the table name.
        table_hash: u32,
        /// Numeric reason code.
        reason_code: u32,
    },
    /// `DeleteItem` operation failed.
    DeleteFailed {
        /// FxHash of the table name.
        table_hash: u32,
        /// Numeric reason code.
        reason_code: u32,
    },
    /// `Query` operation failed.
    QueryFailed {
        /// FxHash of the table name.
        table_hash: u32,
        /// Numeric reason code.
        reason_code: u32,
    },
    /// `Scan` operation failed.
    ScanFailed {
        /// FxHash of the table name.
        table_hash: u32,
        /// Numeric reason code.
        reason_code: u32,
    },
    /// `UpdateItem` operation failed.
    UpdateFailed {
        /// FxHash of the table name.
        table_hash: u32,
        /// Numeric reason code.
        reason_code: u32,
    },
}

impl DynamoFault {
    /// Return a stable numeric error code for log `error_code` fields.
    pub fn as_error_code(&self) -> u32 {
        match self {
            DynamoFault::ConfigFailed { .. } => 1,
            DynamoFault::GetFailed { .. } => 2,
            DynamoFault::PutFailed { .. } => 3,
            DynamoFault::DeleteFailed { .. } => 4,
            DynamoFault::QueryFailed { .. } => 5,
            DynamoFault::ScanFailed { .. } => 6,
            DynamoFault::UpdateFailed { .. } => 7,
        }
    }
}

// =============================================================================
// vil_db_timeseries::types — Result type alias
// =============================================================================

use crate::error::TimeseriesFault;

/// Convenience Result type for all time-series operations.
pub type TimeseriesResult<T> = Result<T, TimeseriesFault>;

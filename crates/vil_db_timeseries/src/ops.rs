// =============================================================================
// vil_db_timeseries::ops — Time-series operations on TimeseriesClient
// =============================================================================
//
// All operations:
//   1. Record `Instant::now()` before the driver call.
//   2. Execute the driver call.
//   3. Emit `db_log!` via `emit_db_log` with timing, op_type, and error_code.
//   4. Return Result<T, TimeseriesFault>.
//
// op_type constants:
//   0 = QUERY (query_flux)
//   1 = WRITE (write_points)
//
// No println!, tracing::info!, or any non-VIL log call.
//
// influxdb2 0.5 API notes:
//   - client.write(bucket, stream) — takes 2 args (no org arg at call site;
//     org is embedded in the Client at construction time).
//   - client.query_raw(query) — returns Vec<FluxRecord>, not String.
//
// TimescaleDB note:
//   When using the `timescale` feature, callers should use `vil_db_sqlx`
//   directly. The methods here are gated on the `influxdb` feature.
// =============================================================================

use std::time::Instant;

use vil_log::dict::register_str;

use crate::client::{emit_db_log, fault_code_from_err, TimeseriesClient};
use crate::error::TimeseriesFault;
use crate::types::TimeseriesResult;

// op_type codes
const OP_QUERY: u8 = 0;
const OP_WRITE: u8 = 1;

impl TimeseriesClient {
    // =========================================================================
    // write_points  [influxdb feature only]
    // =========================================================================

    /// Write a batch of data points to InfluxDB.
    ///
    /// `points` is a `Vec` of `influxdb2::models::DataPoint`.
    /// Emits `db_log!` with `op_type = 1` (WRITE).
    ///
    /// For TimescaleDB, use `vil_db_sqlx` with hypertable INSERT statements.
    #[cfg(feature = "influxdb")]
    pub async fn write_points(
        &self,
        points: Vec<influxdb2::models::DataPoint>,
    ) -> TimeseriesResult<()> {
        let bucket = &self.config().bucket;
        let bucket_hash = register_str(bucket);
        let count = points.len() as u32;

        let start = Instant::now();
        let result = self
            .raw_influx()
            .write(bucket.as_str(), futures::stream::iter(points))
            .await;
        let elapsed_us = start.elapsed().as_micros() as u32;

        match result {
            Ok(_) => {
                emit_db_log(self.db_hash(), bucket, OP_WRITE, elapsed_us, count, 0, self.pool_id());
                Ok(())
            }
            Err(e) => {
                emit_db_log(self.db_hash(), bucket, OP_WRITE, elapsed_us, 0, 1, self.pool_id());
                Err(TimeseriesFault::WriteFailed {
                    bucket_hash,
                    reason_code: fault_code_from_err(&e),
                })
            }
        }
    }

    // =========================================================================
    // query_flux  [influxdb feature only]
    // =========================================================================

    /// Run a Flux query against InfluxDB and return the raw `Vec<FluxRecord>`.
    ///
    /// Emits `db_log!` with `op_type = 0` (QUERY).
    ///
    /// For TimescaleDB, use `vil_db_sqlx` with SQL queries.
    #[cfg(feature = "influxdb")]
    pub async fn query_flux(
        &self,
        flux: &str,
    ) -> TimeseriesResult<Vec<influxdb2::api::query::FluxRecord>> {
        let query_hash = register_str(flux);

        let start = Instant::now();
        let result = self
            .raw_influx()
            .query_raw(Some(influxdb2::models::Query::new(flux.to_string())))
            .await;
        let elapsed_us = start.elapsed().as_micros() as u32;

        match result {
            Ok(records) => {
                let rows = records.len() as u32;
                emit_db_log(self.db_hash(), flux, OP_QUERY, elapsed_us, rows, 0, self.pool_id());
                Ok(records)
            }
            Err(e) => {
                emit_db_log(self.db_hash(), flux, OP_QUERY, elapsed_us, 0, 1, self.pool_id());
                Err(TimeseriesFault::QueryFailed {
                    query_hash,
                    reason_code: fault_code_from_err(&e),
                })
            }
        }
    }

    // =========================================================================
    // timescale note (timescale feature)
    // =========================================================================

    /// Returns Ok(()) as a compile-time architectural marker.
    ///
    /// TimescaleDB is accessed via `vil_db_sqlx`. Use that crate directly with
    /// hypertable-compatible SQL (e.g. `INSERT INTO metrics ...`).
    #[cfg(feature = "timescale")]
    pub fn timescale_note(&self) -> TimeseriesResult<()> {
        Ok(())
    }
}

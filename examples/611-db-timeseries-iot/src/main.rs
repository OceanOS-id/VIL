// ╔════════════════════════════════════════════════════════════╗
// ║  611 — Smart Building: Energy Monitoring (Time-Series)     ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Smart Building — Energy Monitoring               ║
// ║  Pattern:  VX_APP                                           ║
// ║  Features: ServiceCtx, ShmSlice, VilResponse, in-memory    ║
// ║            time-series store with aggregation                ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Write sensor readings from building energy       ║
// ║  meters (HVAC, lighting, EV chargers), query time ranges,   ║
// ║  and compute hourly/daily aggregates for dashboards.         ║
// ║  In production, swap for vil_db_timeseries::TimeseriesClient ║
// ║  backed by InfluxDB or TimescaleDB.                         ║
// ╚════════════════════════════════════════════════════════════╝
//
// Production pattern with real InfluxDB:
//
//   use vil_db_timeseries::{TimeseriesClient, TimeseriesConfig};
//   let config = TimeseriesConfig::new(
//       "http://localhost:8086", "building-org", "my-token", "energy-metrics",
//   );
//   let client = TimeseriesClient::new(config).await.expect("connect");
//   // Write: client.write_line_protocol("sensor,id=hvac-01 value=23.5 1712300000")
//   // Query: client.query_flux(r#"from(bucket:"energy-metrics") |> range(start: -1h)"#)
//
// Run:   cargo run -p vil-db-timeseries-iot
// Test:
//   curl -X POST http://localhost:8080/api/sensors/write \
//     -H 'Content-Type: application/json' \
//     -d '{"sensor_id":"hvac-floor-3","value":23.5,"unit":"kWh","timestamp":1712300000}'
//
//   curl 'http://localhost:8080/api/sensors/query?sensor_id=hvac-floor-3&start=1712290000&end=1712310000'
//
//   curl 'http://localhost:8080/api/sensors/aggregate?sensor_id=hvac-floor-3&interval=hourly'

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use vil_server::prelude::*;

// ── Data Models ──────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct SensorReading {
    sensor_id: String,
    value: f64,
    unit: String,
    timestamp: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct WriteResponse {
    status: String,
    sensor_id: String,
    timestamp: u64,
    total_readings: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct QueryResponse {
    sensor_id: String,
    readings: Vec<SensorReading>,
    count: usize,
    time_range: TimeRange,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct TimeRange {
    start: u64,
    end: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct AggregateResponse {
    sensor_id: String,
    interval: String,
    buckets: Vec<AggregateBucket>,
    overall: OverallStats,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct AggregateBucket {
    bucket_start: u64,
    bucket_end: u64,
    count: usize,
    min: f64,
    max: f64,
    avg: f64,
    sum: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct OverallStats {
    total_readings: usize,
    min: f64,
    max: f64,
    avg: f64,
}

// ── In-Memory Time-Series Store ──────────────────────────────────────────
// Sorted Vec per sensor_id. In production, use vil_db_timeseries with
// InfluxDB (Flux queries, line protocol writes) or TimescaleDB (SQL).

struct TimeseriesStore {
    /// sensor_id -> sorted Vec of readings (sorted by timestamp).
    data: RwLock<HashMap<String, Vec<SensorReading>>>,
}

impl TimeseriesStore {
    fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }

    async fn write(&self, reading: SensorReading) -> usize {
        let mut guard = self.data.write().await;
        let series = guard.entry(reading.sensor_id.clone()).or_default();
        // Insert in sorted order by timestamp
        let pos = series.partition_point(|r| r.timestamp <= reading.timestamp);
        series.insert(pos, reading);
        series.len()
    }

    async fn query(&self, sensor_id: &str, start: u64, end: u64) -> Vec<SensorReading> {
        let guard = self.data.read().await;
        match guard.get(sensor_id) {
            Some(series) => {
                series.iter()
                    .filter(|r| r.timestamp >= start && r.timestamp <= end)
                    .cloned()
                    .collect()
            }
            None => Vec::new(),
        }
    }

    async fn all_readings(&self, sensor_id: &str) -> Vec<SensorReading> {
        let guard = self.data.read().await;
        guard.get(sensor_id).cloned().unwrap_or_default()
    }

    #[allow(dead_code)]
    async fn total_count(&self) -> usize {
        let guard = self.data.read().await;
        guard.values().map(|v| v.len()).sum()
    }
}

struct AppState {
    store: TimeseriesStore,
}

// ── Handlers ─────────────────────────────────────────────────────────────

/// POST /api/sensors/write — write a sensor reading.
async fn sensor_write(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<WriteResponse>> {
    let reading: SensorReading = body.json()
        .map_err(|_| VilError::bad_request(
            "invalid JSON — expected {\"sensor_id\", \"value\", \"unit\", \"timestamp\"}"
        ))?;

    let state = ctx.state::<Arc<AppState>>()
        .map_err(|_| VilError::internal("state not found"))?;

    let sensor_id = reading.sensor_id.clone();
    let timestamp = reading.timestamp;
    let count = state.store.write(reading).await;

    Ok(VilResponse::ok(WriteResponse {
        status: "written".into(),
        sensor_id,
        timestamp,
        total_readings: count,
    }))
}

/// Query params for time-range queries.
#[derive(Debug, Deserialize)]
struct QueryParams {
    sensor_id: String,
    #[serde(default)]
    start: Option<u64>,
    #[serde(default)]
    end: Option<u64>,
}

/// GET /api/sensors/query?sensor_id=X&start=T1&end=T2 — query time range.
async fn sensor_query(
    ctx: ServiceCtx,
    Query(params): Query<QueryParams>,
) -> HandlerResult<VilResponse<QueryResponse>> {
    let sensor_id = params.sensor_id;
    let start: u64 = params.start.unwrap_or(0);
    let end: u64 = params.end.unwrap_or(u64::MAX);

    let state = ctx.state::<Arc<AppState>>()
        .map_err(|_| VilError::internal("state not found"))?;

    let readings = state.store.query(&sensor_id, start, end).await;
    let count = readings.len();

    Ok(VilResponse::ok(QueryResponse {
        sensor_id,
        readings,
        count,
        time_range: TimeRange { start, end },
    }))
}

/// Query params for aggregate endpoint.
#[derive(Debug, Deserialize)]
struct AggregateParams {
    sensor_id: String,
    #[serde(default = "default_interval")]
    interval: String,
}

fn default_interval() -> String {
    "hourly".into()
}

/// GET /api/sensors/aggregate?sensor_id=X&interval=hourly|daily — aggregated stats.
async fn sensor_aggregate(
    ctx: ServiceCtx,
    Query(params): Query<AggregateParams>,
) -> HandlerResult<VilResponse<AggregateResponse>> {
    let sensor_id = params.sensor_id;
    let interval = params.interval;

    let bucket_seconds: u64 = match interval.as_str() {
        "hourly" => 3600,
        "daily" => 86400,
        _ => return Err(VilError::bad_request("interval must be 'hourly' or 'daily'")),
    };

    let state = ctx.state::<Arc<AppState>>()
        .map_err(|_| VilError::internal("state not found"))?;

    let readings = state.store.all_readings(&sensor_id).await;

    if readings.is_empty() {
        return Ok(VilResponse::ok(AggregateResponse {
            sensor_id,
            interval,
            buckets: Vec::new(),
            overall: OverallStats {
                total_readings: 0,
                min: 0.0,
                max: 0.0,
                avg: 0.0,
            },
        }));
    }

    // Build time buckets
    let min_ts = readings.first().map(|r| r.timestamp).unwrap_or(0);
    let bucket_start_base = (min_ts / bucket_seconds) * bucket_seconds;

    let mut bucket_map: HashMap<u64, Vec<f64>> = HashMap::new();
    for r in &readings {
        let bucket_key = ((r.timestamp - bucket_start_base) / bucket_seconds) * bucket_seconds
            + bucket_start_base;
        bucket_map.entry(bucket_key).or_default().push(r.value);
    }

    let mut buckets: Vec<AggregateBucket> = bucket_map.into_iter()
        .map(|(start, values)| {
            let count = values.len();
            let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let sum: f64 = values.iter().sum();
            let avg = sum / count as f64;
            AggregateBucket {
                bucket_start: start,
                bucket_end: start + bucket_seconds,
                count,
                min,
                max,
                avg,
                sum,
            }
        })
        .collect();

    buckets.sort_by_key(|b| b.bucket_start);

    // Overall stats
    let all_values: Vec<f64> = readings.iter().map(|r| r.value).collect();
    let overall_min = all_values.iter().cloned().fold(f64::INFINITY, f64::min);
    let overall_max = all_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let overall_sum: f64 = all_values.iter().sum();
    let overall_avg = overall_sum / all_values.len() as f64;

    Ok(VilResponse::ok(AggregateResponse {
        sensor_id,
        interval,
        buckets,
        overall: OverallStats {
            total_readings: readings.len(),
            min: overall_min,
            max: overall_max,
            avg: overall_avg,
        },
    }))
}

// ── Main ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let store = TimeseriesStore::new();

    // Seed demo data: 24 hours of HVAC readings (one per hour)
    for hour in 0..24u64 {
        let ts = 1712300000 + hour * 3600;
        // Simulate energy usage pattern: higher during business hours
        let value = if (8..18).contains(&hour) { 45.0 + (hour as f64) * 1.2 } else { 12.0 + (hour as f64) * 0.3 };
        let reading = SensorReading {
            sensor_id: "hvac-floor-3".into(),
            value,
            unit: "kWh".into(),
            timestamp: ts,
        };
        store.write(reading).await;
    }

    // Seed: lighting sensor
    for hour in 0..24u64 {
        let ts = 1712300000 + hour * 3600;
        let value = if (7..20).contains(&hour) { 8.5 + (hour as f64) * 0.4 } else { 1.2 };
        let reading = SensorReading {
            sensor_id: "lighting-lobby".into(),
            value,
            unit: "kWh".into(),
            timestamp: ts,
        };
        store.write(reading).await;
    }

    // Seed: EV charger
    for hour in 0..24u64 {
        let ts = 1712300000 + hour * 3600;
        let value = if (9..17).contains(&hour) { 22.0 + (hour as f64) * 2.0 } else { 0.5 };
        let reading = SensorReading {
            sensor_id: "ev-charger-b1".into(),
            value,
            unit: "kWh".into(),
            timestamp: ts,
        };
        store.write(reading).await;
    }

    let state = Arc::new(AppState { store });

    let svc = ServiceProcess::new("sensors")
        .prefix("/api")
        .endpoint(Method::POST, "/sensors/write", post(sensor_write))
        .endpoint(Method::GET, "/sensors/query", get(sensor_query))
        .endpoint(Method::GET, "/sensors/aggregate", get(sensor_aggregate))
        .state(state);

    VilApp::new("timeseries-iot-energy")
        .port(8080)
        .observer(true)
        .service(svc)
        .run()
        .await;
}

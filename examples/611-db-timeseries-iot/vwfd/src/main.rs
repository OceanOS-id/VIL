// 611 — TimeSeries IoT Sensors (VWFD)
// Business logic identical to standard:
//   POST /sensors/write → WriteResponse { status, sensor_id, timestamp, total_readings }
//   GET /sensors/query → QueryResponse { sensor_id, readings[], count, time_range }
//   GET /sensors/aggregate → AggregateResponse { sensor_id, interval, buckets[], overall }
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

static READING_COUNT: AtomicU64 = AtomicU64::new(0);

fn sensor_write(input: &Value) -> Result<Value, String> {
    let body = &input["body"];
    let total = READING_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    Ok(json!({
        "status": "ok",
        "sensor_id": body["sensor_id"].as_str().unwrap_or("sensor-001"),
        "timestamp": "2024-01-15T10:30:00Z",
        "total_readings": total
    }))
}

fn sensor_query(input: &Value) -> Result<Value, String> {
    let query = &input["query"];
    let sensor_id = query["sensor_id"].as_str().unwrap_or("sensor-001");
    Ok(json!({
        "sensor_id": sensor_id,
        "readings": [
            {"sensor_id": sensor_id, "value": 23.5, "unit": "celsius", "timestamp": "2024-01-15T10:00:00Z"},
            {"sensor_id": sensor_id, "value": 24.1, "unit": "celsius", "timestamp": "2024-01-15T10:05:00Z"},
            {"sensor_id": sensor_id, "value": 23.8, "unit": "celsius", "timestamp": "2024-01-15T10:10:00Z"},
            {"sensor_id": sensor_id, "value": 24.3, "unit": "celsius", "timestamp": "2024-01-15T10:15:00Z"},
            {"sensor_id": sensor_id, "value": 23.9, "unit": "celsius", "timestamp": "2024-01-15T10:20:00Z"}
        ],
        "count": 5,
        "time_range": {"start": "2024-01-15T10:00:00Z", "end": "2024-01-15T10:20:00Z"}
    }))
}

fn sensor_aggregate(input: &Value) -> Result<Value, String> {
    let query = &input["query"];
    let sensor_id = query["sensor_id"].as_str().unwrap_or("sensor-001");
    let interval = query["interval"].as_str().unwrap_or("5m");
    Ok(json!({
        "sensor_id": sensor_id,
        "interval": interval,
        "buckets": [
            {"start": "2024-01-15T10:00:00Z", "end": "2024-01-15T10:05:00Z", "min": 23.5, "max": 24.1, "avg": 23.8, "sum": 47.6, "count": 2},
            {"start": "2024-01-15T10:05:00Z", "end": "2024-01-15T10:10:00Z", "min": 23.8, "max": 23.8, "avg": 23.8, "sum": 23.8, "count": 1},
            {"start": "2024-01-15T10:10:00Z", "end": "2024-01-15T10:15:00Z", "min": 24.3, "max": 24.3, "avg": 24.3, "sum": 24.3, "count": 1},
            {"start": "2024-01-15T10:15:00Z", "end": "2024-01-15T10:20:00Z", "min": 23.9, "max": 23.9, "avg": 23.9, "sum": 23.9, "count": 1}
        ],
        "overall": {"min": 23.5, "max": 24.3, "avg": 23.92, "total_readings": 5}
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/611-db-timeseries-iot/vwfd/workflows", 8080)
        .native("sensor_write", sensor_write)
        .native("sensor_query", sensor_query)
        .native("sensor_aggregate", sensor_aggregate)
        .run().await;
}

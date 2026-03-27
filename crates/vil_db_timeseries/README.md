# vil_db_timeseries

VIL Database Plugin for time-series databases.

Supports two backends selected via Cargo features:

| Feature      | Backend      | Driver        |
|--------------|--------------|---------------|
| `influxdb`   | InfluxDB v2  | `influxdb2`   |
| `timescale`  | TimescaleDB  | `vil_db_sqlx` |

## Operations

| Method          | op_type | Feature    | Description                  |
|-----------------|---------|------------|------------------------------|
| `write_points`  | 1       | influxdb   | Write DataPoint batch        |
| `query_flux`    | 0       | influxdb   | Run a Flux query             |
| `timescale_note`| —       | timescale  | Marker — use vil_db_sqlx     |

## Compliance

- `vil_log` for all logging — no `println!`, `tracing`, or `eprintln!`
- All string fields hashed via `register_str()` → `u32`
- Error type: `TimeseriesFault` — plain enum, no `String` fields, no `thiserror`

## TimescaleDB

TimescaleDB is a PostgreSQL extension. Use `vil_db_sqlx` directly with
hypertable-compatible SQL (e.g. `INSERT INTO metrics (time, value) VALUES ...`).
The `timescale` feature is a compile-time marker only.

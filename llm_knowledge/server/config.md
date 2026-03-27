# Configuration — Profiles, YAML, ENV

VIL uses a 3-layer configuration with semantic profiles.

## Precedence

```
Code Default → YAML (vil-server.yaml) → Profile (dev/staging/prod) → ENV (VIL_*)
```

Environment variables always win.

## Profiles

```rust
VilApp::new("my-app")
    .profile("prod")    // Apply production tuning
    .port(8080)
    .run().await;
```

| Profile | SHM | Logging | DB Pool | Admin | Security |
|---------|-----|---------|---------|-------|----------|
| `dev` | 8MB, check/64 | debug, text | 5 conn | all on | off |
| `staging` | 64MB, check/256 | info, json | 20 conn | selective | rate limit |
| `prod` | 256MB, check/1024 | warn, json | 50 conn | all off | hardened |

## YAML Config

```yaml
profile: prod
server:
  port: 8080                          # VIL_SERVER_PORT
  workers: 0                          # VIL_WORKERS (0 = num_cpus)
shm:
  pool_size: "256MB"                  # VIL_SHM_POOL_SIZE
  reset_threshold_pct: 90             # VIL_SHM_RESET_PCT
  check_interval: 1024               # VIL_SHM_CHECK_INTERVAL
pipeline:
  queue_capacity: 4096                # VIL_PIPELINE_QUEUE_CAPACITY
database:
  postgres:
    url: "postgres://vil:vil@db:5432/vil"  # VIL_DATABASE_URL
    max_connections: 50               # VIL_DATABASE_MAX_CONNECTIONS
  redis:
    url: "redis://redis:6380"         # VIL_REDIS_URL
mq:
  nats:
    url: "nats://nats:4222"           # VIL_NATS_URL
  kafka:
    brokers: "kafka:9092"             # VIL_KAFKA_BROKERS
```

## Loading

```rust
use vil_server_config::FullServerConfig;

// YAML + profile + env overrides
let config = FullServerConfig::from_file_with_env("vil-server.yaml".as_ref())?;

// Env only
let config = ServerConfig::from_env();
```

## Key ENV Vars

| Variable | Description |
|----------|-------------|
| `VIL_PROFILE` | Active profile (dev/staging/prod) |
| `VIL_SERVER_PORT` | HTTP listen port |
| `VIL_SHM_POOL_SIZE` | SHM pool size (e.g., "256MB") |
| `VIL_SHM_CHECK_INTERVAL` | Amortized reset check (P99 tuning) |
| `VIL_DATABASE_URL` | PostgreSQL connection string |
| `VIL_REDIS_URL` | Redis connection string |
| `VIL_NATS_URL` | NATS connection string |
| `VIL_KAFKA_BROKERS` | Kafka broker list |

> Full reference: vil-server.reference.yaml

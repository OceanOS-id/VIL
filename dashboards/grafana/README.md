# VIL Grafana Dashboard Templates

Pre-built Grafana dashboards for monitoring VIL services via OpenTelemetry + Prometheus.

## Dashboards

| Dashboard | File | Description |
|-----------|------|-------------|
| Pipeline Overview | `vil-pipeline-overview.json` | Request rate, latency P50/P95/P99, error rate, throughput |
| SHM Utilization | `vil-shm-utilization.json` | Exchange heap usage, slot allocation, compaction rate |
| Tri-Lane Latency | `vil-tri-lane-latency.json` | Per-lane latency (Trigger/Data/Control), queue depth |
| Database Pool | `vil-database-pool.json` | Connection pool active/idle/waiting, query latency |
| MQ Consumer Lag | `vil-mq-consumer-lag.json` | Per-topic consumer lag, publish/consume rate, ack ratio |
| AI Gateway | `vil-ai-gateway.json` | LLM latency, token usage, cost, cache hit rate |

## Alert Rules

| Alert | File | Condition |
|-------|------|-----------|
| SHM Exhaustion | `alerts/shm-exhaustion.yaml` | SHM usage > 90% for 5 minutes |
| Pipeline Stall | `alerts/pipeline-stall.yaml` | Zero events processed for 2 minutes |
| Error Rate | `alerts/error-rate.yaml` | Error rate > 5% for 3 minutes |

## Setup

1. Import dashboards via Grafana UI: Dashboards → Import → Upload JSON
2. Configure Prometheus data source pointing to your OTel collector
3. (Optional) Import alert rules via Grafana provisioning

## Data Source

These dashboards expect metrics exported via `vil_otel` → OpenTelemetry Collector → Prometheus.

Required VIL metric prefixes:
- `vil_pipeline_*` — request rate, latency, errors
- `vil_shm_*` — heap usage, allocation
- `vil_trilane_*` — lane latency, queue depth
- `vil_db_*` — query latency, pool stats
- `vil_mq_*` — consumer lag, throughput
- `vil_ai_*` — LLM latency, tokens, cost

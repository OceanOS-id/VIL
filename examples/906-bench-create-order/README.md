# 906 — Benchmark: Create Order

Head-to-head benchmark workflow for comparing VIL vs other workflow engines.

## Business Logic

```
POST /api/orders {"email": "...", "prices": [100, 200, 300]}
  → uuid_v4()           → order_id
  → mean(prices)        → total (200.0)
  → sha256(email)       → customer_hash
  → JSON response
```

All compute via built-in expression functions. Zero custom handler code.

## Run

```bash
cargo run --release -p vil-vwfd-bench-create-order
curl -X POST http://localhost:8080/api/orders \
  -H 'Content-Type: application/json' \
  -d '{"email":"alice@test.com","prices":[100,200,300]}'
```

## Benchmark

```bash
# Warmup
hey -m POST -H 'Content-Type: application/json' \
  -d '{"email":"bench@test.com","prices":[100,200,300]}' \
  -c 10 -n 500 http://localhost:8080/api/orders > /dev/null

# Bench
hey -m POST -H 'Content-Type: application/json' \
  -d '{"email":"bench@test.com","prices":[100,200,300]}' \
  -c 10 -n 5000 http://localhost:8080/api/orders
```

## Results (i9-11900F, 32GB, Ubuntu 22.04)

| Mode | Throughput | P50 | P99 |
|------|-----------|-----|-----|
| Native binary | 43K req/s | 0.1ms | 1.2ms |
| Docker container | 37K req/s | 0.2ms | 1.4ms |

## Kestra Comparison

See `docs-dev-jangan-ditrack/vil/marketing/head-to-head/COMPARISON.md` for
head-to-head benchmark against Kestra v1.3.9 (Docker, same machine).

VIL: **37K req/s** vs Kestra: **804 req/s** = **47x faster** (both Docker).

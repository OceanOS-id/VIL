# 501 — VIL Log: Stdout Dev Mode

Demonstrates VIL's semantic log system with stdout drain in pretty format.

## Run

```bash
cargo run -p example-501-villog-stdout-dev
```

## What it shows

- `app_log!` — structured business events (order, payment, inventory)
- `access_log!` — HTTP request/response logs with status codes and latency
- `ai_log!` — LLM operation logs (provider, model, tokens, cost)
- Pretty colored stdout output — ideal for local development

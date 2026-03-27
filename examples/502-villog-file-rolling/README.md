# 502 — VIL Log: File Rolling Drain

Demonstrates VIL's file drain with daily rotation and file retention.

## Run

```bash
cargo run -p example-502-villog-file-rolling
```

## What it shows

- `FileDrain` with `RotationStrategy::Daily`
- Max 7 retained rotated files
- JSON Lines output format (one event per line)
- 100 events across `app_log!`, `access_log!`, `db_log!`

## Output

Logs are written to `./logs/app.log` (relative to where you run the binary).
Inspect with:

```bash
cat ./logs/app.log | jq .
```

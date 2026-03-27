# 801-trigger-cron-basic

Cron trigger firing every 5 seconds, printing events.

## What it shows

- `create_cron_trigger()` with a 6-field cron expression (`*/5 * * * * *`)
- `TriggerSource::start()` with an `EventCallback`
- Receiving `TriggerEvent` descriptors from the mpsc channel
- `mq_log!` auto-emitted by `vil_trigger_cron` on every fire
- `StdoutDrain::resolved()` output format

## No external services required

The trigger uses the system clock only. No network or Docker needed.

## Run

```bash
cargo run -p example-801-trigger-cron-basic
```

The example fires 3 times (every 5 seconds) and then exits.
Total runtime: ~15 seconds.

# 802-trigger-fs-watcher

Filesystem watcher trigger on a local directory.

## What it shows

- `create_fs_trigger()` watching `/tmp/vil-802-watch` for `*.log` files
- `TriggerSource::start()` with an `EventCallback`
- Receiving `TriggerEvent` descriptors as files are created
- `mq_log!` auto-emitted by `vil_trigger_fs` on every filesystem event
- `StdoutDrain::resolved()` output format

## No external services required

Uses the OS filesystem notification API (inotify on Linux, FSEvents on macOS).
No network or Docker needed.

## Run

```bash
cargo run -p example-802-trigger-fs-watcher
```

The example creates 5 `.log` files in `/tmp/vil-802-watch`, waits for events,
then cleans up and exits.

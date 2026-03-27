# NDJSON Pipeline

Newline-delimited JSON streaming with line-by-line processing via `HttpFormat::NDJSON`.

## Basic NDJSON Pipeline

```rust
use vil_sdk::prelude::*;
use vil_sdk::http::{HttpSinkBuilder, HttpSourceBuilder, HttpFormat};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sink = HttpSinkBuilder::new()
        .port(3080)
        .path("/trigger")
        .build();

    let source = HttpSourceBuilder::new()
        .url("http://upstream/api/v1/credits/ndjson")
        .format(HttpFormat::NDJSON)
        .build();

    let (_ir, handles) = vil_workflow! {
        name: "NdjsonGateway",
        token: ShmToken,
        instances: [ sink, source ],
        routes: [
            sink.out -> source.in (LoanWrite),
            source.data -> sink.in (LoanWrite),
        ]
    };

    for h in handles { h.join().unwrap(); }
    Ok(())
}
```

## NDJSON Format

Each line is a self-contained JSON object, separated by `\n`:

```
{"account_id":"A001","amount":50000,"kolektabilitas":1}\n
{"account_id":"A002","amount":75000,"kolektabilitas":3}\n
{"account_id":"A003","amount":120000,"kolektabilitas":5}\n
```

## Filter with .transform()

Process each line independently -- return `None` to drop:

```rust
let source = HttpSourceBuilder::new()
    .url("http://upstream/credits/ndjson")
    .format(HttpFormat::NDJSON)
    .transform(|line: &[u8]| -> Option<Vec<u8>> {
        let record: CreditRecord = serde_json::from_slice(line).ok()?;
        if record.kolektabilitas >= 3 {
            Some(line.to_vec())
        } else {
            None
        }
    })
    .build();
```

## Enrich Each Line

```rust
.transform(|line: &[u8]| -> Option<Vec<u8>> {
    let mut v: serde_json::Value = serde_json::from_slice(line).ok()?;
    v["risk_category"] = match v["kolektabilitas"].as_u64()? {
        1..=2 => serde_json::json!("performing"),
        3..=4 => serde_json::json!("watch"),
        _ => serde_json::json!("loss"),
    };
    Some(serde_json::to_vec(&v).unwrap())
})
```

## NDJSON vs SSE

| Feature | NDJSON | SSE |
|---------|--------|-----|
| Format | `{json}\n` per line | `data: {json}\n\n` |
| Direction | Batch/streaming | Server-push only |
| Browser support | Fetch API | EventSource API |
| Use case | Data pipelines, ETL | Real-time UI updates, AI chat |
| VIL format | `HttpFormat::NDJSON` | `HttpFormat::SSE` |

## Trigger from Client

```bash
# Trigger NDJSON pipeline
curl -X POST http://localhost:3080/trigger \
  -H "Content-Type: application/json" \
  -d '{"filter":"npl_only"}'

# Response streams back as NDJSON
{"account_id":"A002","amount":75000,"kolektabilitas":3}
{"account_id":"A003","amount":120000,"kolektabilitas":5}
```

> Reference: docs/vil/004-VIL-Developer_Guide-Pipeline-Streaming.md

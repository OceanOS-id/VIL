# Pipeline .transform()

The `.transform()` method on `HttpSourceBuilder` applies inline processing to each streamed record without a separate processor node.

## Signature

```rust
.transform(|line: &[u8]| -> Option<Vec<u8>> {
    // Return Some(bytes) to emit, None to drop
})
```

## Filter (Drop Records)

Return `None` to drop records that fail a condition:

```rust
let source = HttpSourceBuilder::new()
    .url("http://upstream/data")
    .format(HttpFormat::NDJSON)
    .transform(|line: &[u8]| -> Option<Vec<u8>> {
        let record: Credit = serde_json::from_slice(line).ok()?;
        if record.kolektabilitas >= 3 {
            Some(line.to_vec())  // Keep NPL records
        } else {
            None  // Drop healthy records
        }
    })
    .build();
```

## Enrich (Add Fields)

Parse, add fields, re-serialize:

```rust
.transform(|line: &[u8]| -> Option<Vec<u8>> {
    let mut record: serde_json::Value = serde_json::from_slice(line).ok()?;
    record["processed_at"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
    record["risk_score"] = serde_json::json!(compute_risk(&record));
    Some(serde_json::to_vec(&record).unwrap())
})
```

## Map (Restructure)

Transform the shape of each record:

```rust
.transform(|line: &[u8]| -> Option<Vec<u8>> {
    let input: RawCredit = serde_json::from_slice(line).ok()?;
    let output = CreditSummary {
        id: input.account_id,
        status: if input.kolektabilitas >= 3 { "NPL" } else { "HEALTHY" },
        amount: input.outstanding_balance,
    };
    Some(serde_json::to_vec(&output).unwrap())
})
```

## Validate (Check + Pass or Drop)

Validate structure, drop malformed records:

```rust
.transform(|line: &[u8]| -> Option<Vec<u8>> {
    let record: serde_json::Value = serde_json::from_slice(line).ok()?;
    // Must have required fields
    if record.get("account_id").is_none() || record.get("amount").is_none() {
        return None;  // Drop invalid
    }
    // Amount must be positive
    if record["amount"].as_f64().unwrap_or(0.0) <= 0.0 {
        return None;
    }
    Some(line.to_vec())  // Pass valid records through
})
```

## Chaining Transforms

Multiple transforms execute in order:

```rust
let source = HttpSourceBuilder::new()
    .url("http://upstream/data")
    .format(HttpFormat::NDJSON)
    .transform(|line: &[u8]| -> Option<Vec<u8>> {
        // Step 1: Filter
        let r: Credit = serde_json::from_slice(line).ok()?;
        if r.kolektabilitas >= 3 { Some(line.to_vec()) } else { None }
    })
    .transform(|line: &[u8]| -> Option<Vec<u8>> {
        // Step 2: Enrich
        let mut v: serde_json::Value = serde_json::from_slice(line).ok()?;
        v["flagged"] = serde_json::json!(true);
        Some(serde_json::to_vec(&v).unwrap())
    })
    .build();
```

## SSE Transform

Works the same for SSE streams -- each SSE event data is passed as `line`:

```rust
let source = HttpSourceBuilder::new()
    .url("http://upstream/stream")
    .format(HttpFormat::SSE)
    .dialect(SseSourceDialect::OpenAi)
    .json_tap("choices[0].delta.content")
    .transform(|chunk: &[u8]| -> Option<Vec<u8>> {
        let text = std::str::from_utf8(chunk).ok()?;
        if text.contains("REDACTED") { None } else { Some(chunk.to_vec()) }
    })
    .build();
```

> Reference: docs/vil/004-VIL-Developer_Guide-Pipeline-Streaming.md

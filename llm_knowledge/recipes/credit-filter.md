# Recipe: Credit NPL Filter

NDJSON pipeline filtering non-performing loans (kolektabilitas >= 3) with .transform().

## Full Example

```rust
use vil_sdk::prelude::*;
use vil_sdk::http::{HttpSinkBuilder, HttpSourceBuilder, HttpFormat};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct CreditRecord {
    account_id: String,
    debtor_name: String,
    outstanding_balance: f64,
    kolektabilitas: u8,  // 1=Current, 2=Special, 3=Sub, 4=Doubtful, 5=Loss
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sink = HttpSinkBuilder::new()
        .port(3080)
        .path("/trigger")
        .build();

    let source = HttpSourceBuilder::new()
        .url("http://localhost:18081/api/v1/credits/ndjson")
        .format(HttpFormat::NDJSON)
        .transform(|line: &[u8]| -> Option<Vec<u8>> {
            let record: CreditRecord = serde_json::from_slice(line).ok()?;
            if record.kolektabilitas >= 3 {
                Some(line.to_vec())  // Keep NPL records
            } else {
                None  // Drop healthy loans
            }
        })
        .build();

    let (_ir, handles) = vil_workflow! {
        name: "CreditNplFilter",
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

## With Risk Enrichment

Add risk category and timestamp to each record:

```rust
.transform(|line: &[u8]| -> Option<Vec<u8>> {
    let record: CreditRecord = serde_json::from_slice(line).ok()?;
    if record.kolektabilitas < 3 { return None; }

    let mut v = serde_json::to_value(&record).ok()?;
    v["risk_category"] = match record.kolektabilitas {
        3 => serde_json::json!("substandard"),
        4 => serde_json::json!("doubtful"),
        5 => serde_json::json!("loss"),
        _ => serde_json::json!("unknown"),
    };
    v["npl_ratio"] = serde_json::json!(record.outstanding_balance / 1_000_000.0);
    Some(serde_json::to_vec(&v).unwrap())
})
```

## Kolektabilitas Reference

| Level | Category | NPL? |
|-------|----------|------|
| 1 | Current (Lancar) | No |
| 2 | Special Mention (DPK) | No |
| 3 | Substandard (Kurang Lancar) | Yes |
| 4 | Doubtful (Diragukan) | Yes |
| 5 | Loss (Macet) | Yes |

## Test

```bash
# Trigger the filter
curl -X POST http://localhost:3080/trigger \
  -H "Content-Type: application/json" \
  -d '{"filter":"npl"}'

# Output: only kolektabilitas >= 3 records
{"account_id":"A002","debtor_name":"PT ABC","outstanding_balance":75000000,"kolektabilitas":3}
{"account_id":"A005","debtor_name":"CV XYZ","outstanding_balance":120000000,"kolektabilitas":5}
```

> Reference: docs/vil/004-VIL-Developer_Guide-Pipeline-Streaming.md

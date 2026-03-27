# vil_soap — VIL SOAP/WSDL Client

SOAP 1.1 over HTTP client for VIL Phase 2 protocol crates.

## Features

- Full SOAP envelope build/parse via `quick-xml`
- Automatic `db_log!` emit on every `call_action()` — `op_type=4 CALL`
- VIL-compliant `SoapFault` plain enum (no thiserror, no String fields)
- `reqwest` + `rustls-tls` for safe TLS without OpenSSL dependency
- Timing captured via `Instant::now()` — `duration_us` in every log slot

## Boundary Classification

| Path | Mode | Notes |
|------|------|-------|
| SOAP HTTP request/response | Copy | Network boundary — serialization required |
| Internal result passing | Copy | Control-weight, not hot-path |

## Tri-Lane Mapping

| Lane | Direction | Content |
|------|-----------|---------|
| Trigger | Inbound → VIL | SOAP call request descriptor |
| Data | Outbound ← VIL | Parsed response body XML |
| Control | Bidirectional | Timeout / HTTP error / Fault signals |

## Usage

```rust
use vil_soap::{SoapClient, SoapConfig};

let config = SoapConfig::new(
    "http://service.example.com/api?wsdl",
    "http://service.example.com/api",
);
let client = SoapClient::new(config)?;

let response = client.call_action(
    "GetUser",
    "http://service.example.com/",
    "<tns:GetUser><tns:userId>42</tns:userId></tns:GetUser>",
).await?;

println!("Body: {}", response.body_xml);
```

## Thread Hint

`SoapClient` is `Send + Sync`. `reqwest` manages an internal connection pool.
No extra log threads spawned — add 0 to `LogConfig.threads`.

## Compliance

- COMPLIANCE.md §8 — `vil_log` dependency, `db_log!` auto-emit, `register_str()` hashes
- COMPLIANCE.md §4 — `SoapFault` plain enum, no `thiserror`, no `String` fields

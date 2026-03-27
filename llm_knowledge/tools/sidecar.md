# Sidecar SDK

`vil_sidecar` enables Python and Go processes to participate in VIL pipelines via Unix Domain Socket (UDS) transport.

## Architecture

```
VIL Server (Rust) <--UDS--> Sidecar Process (Python/Go)
     |                           |
  SHM Bridge               SDK Client
     |                           |
  ExchangeHeap            Local Processing
```

## Python SDK

```python
from vil_sdk import VilSidecar, ShmBridge

sidecar = VilSidecar(
    name="ml-inference",
    socket="/tmp/vil-sidecar.sock",
)

@sidecar.handler("predict")
async def predict(request):
    model = load_model()
    result = model.predict(request.data)
    return {"prediction": result.tolist()}

sidecar.run()
```

## Go SDK

```go
package main

import "github.com/prdmid/vil-sdk-go"

func main() {
    sidecar := vil.NewSidecar("data-processor", "/tmp/vil-sidecar.sock")

    sidecar.Handle("transform", func(req vil.Request) (vil.Response, error) {
        data := req.JSON()
        result := processData(data)
        return vil.OK(result), nil
    })

    sidecar.Run()
}
```

## Server-Side Registration

```rust
use vil_sidecar::prelude::*;

let sidecar = SidecarPool::new()
    .socket("/tmp/vil-sidecar.sock")
    .pool_size(4)
    .reconnect(ReconnectPolicy::exponential(100, 5000))
    .build()
    .await?;

let service = ServiceProcess::new("ml")
    .extension(sidecar.clone())
    .endpoint(Method::POST, "/predict", post(predict_handler));
```

## Handler with Sidecar

```rust
#[vil_handler(shm)]
async fn predict_handler(ctx: ServiceCtx, slice: ShmSlice) -> VilResponse<Prediction> {
    let sidecar = ctx.state::<SidecarPool>();
    let response = sidecar.call("predict", slice.as_bytes()).await?;
    let prediction: Prediction = serde_json::from_slice(&response)?;
    VilResponse::ok(prediction)
}
```

## Connection Pool

```rust
let pool = SidecarPool::new()
    .socket("/tmp/vil-sidecar.sock")
    .pool_size(8)              // 8 concurrent connections
    .timeout(Duration::from_secs(30))
    .health_interval(Duration::from_secs(10))
    .build()
    .await?;
```

## SHM Bridge

For large payloads, sidecar can read/write directly from ExchangeHeap:

```python
@sidecar.handler("process_large")
async def process_large(request):
    # Read from SHM (zero-copy on Linux)
    data = request.shm_read()
    result = heavy_computation(data)
    # Write result back to SHM
    return request.shm_write(result)
```

## Failover

```rust
let pool = SidecarPool::new()
    .socket("/tmp/vil-sidecar.sock")
    .failover(FailoverPolicy::restart(max_retries: 3))
    .build()
    .await?;
```

> Reference: docs/vil/006-VIL-Developer_Guide-CLI-Deployment.md

# VIL SDK Languages

VIL supports 9 SDK languages via the transpile system. Rust is the native implementation; the other 8 are generated from VIL IR via `vil compile --sdk <lang>`.

## Quick Reference

| Language | SDK Type | Transport |
|----------|----------|-----------|
| Rust | Native | SHM + HTTP |
| Python | Transpile | UDS / HTTP |
| Go | Transpile | UDS / HTTP |
| Java | Transpile | HTTP |
| TypeScript | Transpile | HTTP / WebSocket |
| C# | Transpile | HTTP |
| Kotlin | Transpile | HTTP |
| Swift | Transpile | HTTP |
| Zig | Transpile | UDS / HTTP |

## Generating an SDK

```bash
# Generate SDK for all languages
vil compile pipeline.vil.yaml --sdk all

# Single language
vil compile pipeline.vil.yaml --sdk python
vil compile pipeline.vil.yaml --sdk go

# Output to directory
vil compile pipeline.vil.yaml --sdk typescript --out ./sdk/ts
```

## vil init with SDK Templates

```bash
# Rust (native)
vil init my-service --lang rust

# Python sidecar SDK
vil init my-service --lang python

# Go sidecar SDK
vil init my-service --lang go

# TypeScript (Node.js)
vil init my-service --lang typescript

# Java (Maven)
vil init my-service --lang java

# C# (.NET)
vil init my-service --lang csharp

# Kotlin
vil init my-service --lang kotlin

# Swift
vil init my-service --lang swift

# Zig
vil init my-service --lang zig
```

## Python SDK

```python
from vil_sdk import VilClient, SseStream

client = VilClient(base_url="http://localhost:8080")

# Simple request
response = client.post("/api/orders", json={"item": "widget", "qty": 10})

# SSE streaming
with client.stream_post("/ai/chat", json={"prompt": "Hello"}) as stream:
    for chunk in SseStream(stream, dialect="openai"):
        print(chunk.content, end="", flush=True)
```

Generated sidecar (UDS transport):
```python
# Auto-generated from vil compile --sdk python
from vil_sdk.sidecar import SidecarClient

client = SidecarClient("/tmp/vil-my-service.sock")
result = client.call("process_order", {"order_id": 123})
```

## Go SDK

```go
import "github.com/vil-project/vil-sdk-go"

client := vil.NewClient("http://localhost:8080")

// JSON request
var order Order
err := client.Post("/api/orders", orderReq, &order)

// SSE streaming
stream, err := client.StreamPost("/ai/chat", chatReq)
for chunk := range stream.Chunks() {
    fmt.Print(chunk.Content)
}
```

## TypeScript SDK

```typescript
import { VilClient, SseStream } from '@vil-project/vil-sdk';

const client = new VilClient({ baseUrl: 'http://localhost:8080' });

// JSON request
const order = await client.post<Order>('/api/orders', { item: 'widget', qty: 10 });

// SSE streaming
const stream = client.streamPost('/ai/chat', { prompt: 'Hello' });
for await (const chunk of SseStream(stream, { dialect: 'openai' })) {
    process.stdout.write(chunk.content);
}

// WebSocket
const ws = client.websocket('/events');
ws.on('message', (msg) => console.log(msg));
ws.send({ type: 'subscribe', topic: 'orders' });
```

## Java SDK

```java
import io.vilproject.sdk.VilClient;
import io.vilproject.sdk.SseStream;

VilClient client = VilClient.builder()
    .baseUrl("http://localhost:8080")
    .build();

// JSON request
Order order = client.post("/api/orders", orderReq, Order.class);

// SSE streaming
try (SseStream stream = client.streamPost("/ai/chat", chatReq)) {
    stream.chunks().forEach(chunk -> System.out.print(chunk.getContent()));
}
```

## C# SDK

```csharp
using VilProject.Sdk;

var client = new VilClient("http://localhost:8080");

// JSON request
var order = await client.PostAsync<Order>("/api/orders", orderReq);

// SSE streaming
await foreach (var chunk in client.StreamPostAsync("/ai/chat", chatReq))
{
    Console.Write(chunk.Content);
}
```

## Kotlin SDK

```kotlin
import io.vilproject.sdk.VilClient

val client = VilClient("http://localhost:8080")

// JSON request
val order: Order = client.post("/api/orders", orderReq)

// SSE streaming (coroutines)
client.streamPost("/ai/chat", chatReq).collect { chunk ->
    print(chunk.content)
}
```

## Swift SDK

```swift
import VilSDK

let client = VilClient(baseURL: URL(string: "http://localhost:8080")!)

// JSON request
let order: Order = try await client.post("/api/orders", body: orderReq)

// SSE streaming (AsyncSequence)
for try await chunk in client.streamPost("/ai/chat", body: chatReq) {
    print(chunk.content, terminator: "")
}
```

## Zig SDK

```zig
const vil = @import("vil-sdk");

var client = vil.Client.init(allocator, "http://localhost:8080");
defer client.deinit();

// JSON request
const order = try client.post(Order, "/api/orders", order_req);

// UDS sidecar
var sidecar = vil.Sidecar.connect(allocator, "/tmp/vil-service.sock");
const result = try sidecar.call("process_order", .{ .order_id = 123 });
```

## SDK vs Sidecar

| Mode | Langs | Transport | Use When |
|------|-------|-----------|----------|
| HTTP SDK | All 9 | HTTP/JSON | General-purpose, simple integration |
| Sidecar UDS | Python, Go, Zig | Unix Domain Socket | Low-latency co-located processes |
| Transpile | All 9 | Generated client | Pipeline-specific typed interface |

See [tools/sidecar.md](sidecar.md) for sidecar pool configuration.
See [tools/custom-code.md](custom-code.md) for 3 execution modes.

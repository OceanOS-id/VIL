# VIL SDK Integration Guide: Zero-Copy Cross-Language Integration

VIL provides a *process-oriented zero-copy infrastructure* that enables multiple programming languages to communicate through Shared Memory (SHM) with minimal latency. This guide covers integration patterns, best practices, and performance optimization techniques for each supported language.

---

## Executive Summary

VIL's cross-language capability is built on:
- **Transpile SDK**: Write in Python/Go/Java/TypeScript, compile to native Rust binary via `vil compile`.
- **Sidecar SDK**: External process integration (Python/Go) via Unix Domain Sockets + SHM zero-copy IPC.
- **Zero-Copy Memory**: Direct access to SHM buffers without serialization overhead.
- **Lane-Aware Messaging**: Automatic routing through Trigger, Data, and Control lanes.

> **Note (2026-03-26):** The FFI runtime (`vil_ffi`) and language-specific FFI bindings (ctypes/cgo/JNI/ffi-napi) have been **removed**. VIL now uses a transpile-only SDK model. The FFI SDK sections below (Sections 1-7) are retained as historical reference but the FFI crate no longer ships.

---

## Transpile SDK — Write in Your Language, Compile to Native Binary

VIL's Transpile SDK enables developers to write VIL pipelines in **Python, Go, Java, or TypeScript** using a high-level DSL, then compile them to **native Rust binaries** with `vil compile`. The compiled output runs at full Rust-native performance — no FFI overhead, no runtime dependencies.

### How It Works

```
┌─────────────────┐     vil compile      ┌──────────────────────┐
│ Python/Go/Java/ │  ─────────────────────→ │ Native Rust Binary   │
│ TypeScript DSL  │   --from python         │ (~3,855 req/s SSE)   │
│                 │   --input gateway.py    │ Single static binary │
│ VilPipeline + │   --output gateway      │ Zero FFI overhead    │
│ VilServer     │   --release             │ All 9 VIL         │
│                 │                         │ semantics compiled   │
└─────────────────┘                         └──────────────────────┘
```

### Two Modes of Operation

| Mode | Use Case | How | Performance |
|------|----------|-----|-------------|
| **Transpile (Compile)** | All environments (dev + prod) | `vil compile --from <lang> --input <file> --output <name> --release` | Rust-native (~3,855 req/s) |
| **Sidecar** | External process integration | Python/Go sidecar over UDS + SHM zero-copy | ~78K ops/s |

Use `vil compile` to transpile your DSL code into a native binary. For external process integration (ML models, etc.), use the Sidecar SDK.

### VilPipeline DSL — All 9 Semantics

The Transpile DSL exposes all 9 VIL semantic primitives through `VilPipeline` and `VilServer`:

| DSL Method | VIL Semantic |
|------------|----------------|
| `pipeline.semantic_type(...)` | `#[vil_state]`, `#[vil_event]`, `#[vil_decision]` |
| `pipeline.error(...)` | `#[vil_fault]` |
| `pipeline.state(...)` | State machine transitions |
| `pipeline.mesh(...)` | Tri-Lane mesh routing |
| `pipeline.failover(...)` | HA failover strategy |
| `pipeline.sse_event(...)` | `#[derive(VilSseEvent)]` |
| `server.upstream(...)` | HTTP source configuration |
| `server.sse(True)` | SSE streaming mode |
| `server.run()` | `VilApp` entry point |

### DSL Examples by Language

#### Python

```python
from vil import VilPipeline, VilServer

pipeline = VilPipeline("ai-gateway")
pipeline.semantic_type("InferenceState", fields={"session_id": "u64", "tokens": "u32"})
pipeline.error("InferenceFault", variants=["Timeout", "ModelUnavailable"])
pipeline.state("SessionState", transitions=["init -> processing -> complete"])
pipeline.mesh(route=("ingress", "inference"), lane="Data")
pipeline.failover(primary="inference", backup="inference_backup", on="HostDown", strategy="Immediate")
pipeline.sse_event("StreamChunk", topic="inference_stream", fields={"content": "String"})

server = VilServer(pipeline, port=3080)
server.upstream("http://localhost:4545/v1/chat/completions")
server.sse(True)

if __name__ == "__main__":
    server.run()
```

#### Go

```go
package main

import vil "github.com/oceanos-id/vil-sdk-go"

func main() {
    pipeline := vil.NewPipeline("ai-gateway")
    pipeline.SemanticType("InferenceState", vil.Fields{"session_id": "u64", "tokens": "u32"})
    pipeline.Error("InferenceFault", []string{"Timeout", "ModelUnavailable"})
    pipeline.State("SessionState", []string{"init -> processing -> complete"})
    pipeline.Mesh(vil.Route{"ingress", "inference"}, vil.LaneData)
    pipeline.Failover("inference", "inference_backup", vil.OnHostDown, vil.Immediate)
    pipeline.SseEvent("StreamChunk", "inference_stream", vil.Fields{"content": "String"})

    server := vil.NewServer(pipeline, 3080)
    server.Upstream("http://localhost:4545/v1/chat/completions")
    server.Sse(true)
    server.Run()
}
```

#### Java

```java
import dev.vil.transpile.*;

public class Gateway {
    public static void main(String[] args) {
        var pipeline = new VilPipeline("ai-gateway");
        pipeline.semanticType("InferenceState", Map.of("session_id", "u64", "tokens", "u32"));
        pipeline.error("InferenceFault", List.of("Timeout", "ModelUnavailable"));
        pipeline.state("SessionState", List.of("init -> processing -> complete"));
        pipeline.mesh(new Route("ingress", "inference"), Lane.DATA);
        pipeline.failover("inference", "inference_backup", Trigger.HOST_DOWN, Strategy.IMMEDIATE);
        pipeline.sseEvent("StreamChunk", "inference_stream", Map.of("content", "String"));

        var server = new VilServer(pipeline, 3080);
        server.upstream("http://localhost:4545/v1/chat/completions");
        server.sse(true);
        server.run();
    }
}
```

#### TypeScript

```typescript
import { VilPipeline, VilServer } from "@vil/sdk";

const pipeline = new VilPipeline("ai-gateway");
pipeline.semanticType("InferenceState", { session_id: "u64", tokens: "u32" });
pipeline.error("InferenceFault", ["Timeout", "ModelUnavailable"]);
pipeline.state("SessionState", ["init -> processing -> complete"]);
pipeline.mesh({ from: "ingress", to: "inference" }, "Data");
pipeline.failover("inference", "inference_backup", "HostDown", "Immediate");
pipeline.sseEvent("StreamChunk", "inference_stream", { content: "String" });

const server = new VilServer(pipeline, 3080);
server.upstream("http://localhost:4545/v1/chat/completions");
server.sse(true);
server.run();
```

### Compiling to Native Binary

```bash
# Python → native binary
vil compile --from python --input gateway.py --output gateway --release

# Go → native binary
vil compile --from go --input gateway.go --output gateway --release

# Java → native binary
vil compile --from java --input Gateway.java --output gateway --release

# TypeScript → native binary
vil compile --from typescript --input pipeline.ts --output pipeline --release

# With manifest (saves .vil.yaml next to source)
vil compile --from python --input gateway.py --output gateway --release --save-manifest

# Target VIL Binary format
vil compile --from python --input gateway.py --output gateway --release --target vlb
```

### Benchmark Results

Compiled binaries achieve Rust-native performance — there is no difference between hand-written Rust and a transpiled Python/Go/Java/TypeScript DSL:

| Metric | Hand-Written Rust | Transpiled DSL (any language) |
|--------|-------------------|-------------------------------|
| SSE Pipeline throughput | ~3,855 req/s | ~3,855 req/s |
| Latency (P99) | Rust-native | Rust-native (identical binary) |
| Binary size | Static binary | Static binary (same output) |
| Runtime dependencies | None | None |

**See:** [`examples-sdk/`](../../examples-sdk/) for complete runnable examples in all languages.

---

## Sidecar SDK — External Process Integration via SHM IPC

VIL's Sidecar SDK enables external processes (Python, Go, etc.) to participate as VIL Process activities via zero-copy SHM IPC.

### How It Works

```
┌──────────────┐     UDS (descriptors)     ┌──────────────┐
│  VilApp    │ ◄────────────────────────► │  Sidecar     │
│  (Rust host) │   /dev/shm (zero-copy)    │  (Python/Go) │
└──────────────┘                            └──────────────┘
```

- **Transport**: Unix Domain Socket (descriptors only, ~48 bytes)
- **Data Plane**: Shared memory (`/dev/shm/vil_sc_{name}`) — zero-copy via mmap
- **Protocol**: Length-prefixed JSON (Handshake → Invoke → Result → Health → Drain → Shutdown)

### Performance

| Metric | Result |
|--------|--------|
| SHM write (1KB) | 257ns / 3,804 MB/s |
| UDS roundtrip | 12.7µs (~78K ops/s) |
| Full invoke flow | 0.5µs (1.8M ops/s) |
| WASM Pool dispatch | 19ns overhead |
| CircuitBreaker check | 2ns |

### Sidecar SDK Packages

| Language | Package | Location |
|----------|---------|----------|
| Python | `vil_sidecar` | `sdk/sidecar/python/` |
| Go | `vil_sidecar` | `sdk/sidecar/go/` |

### Python SDK Usage

The Python sidecar SDK (`vil_sidecar.py`) uses the `@method` decorator pattern for handler registration:

```python
from vil_sidecar import VilSidecar

app = VilSidecar("fraud-checker")

@app.method("fraud_check")
def fraud_check(request: dict) -> dict:
    """Called by VilApp when the 'fraud_check' activity is dispatched."""
    score = ml_model.predict(request["features"])
    return {"score": float(score), "is_fraud": score > 0.8}

@app.method("enrich_profile")
def enrich_profile(request: dict) -> dict:
    return {"name": lookup(request["user_id"]), "tier": "gold"}

app.run()  # Connects to VilApp host, enters event loop
```

### Go SDK Usage

The Go sidecar SDK uses `NewSidecar` + `Method()` registration + `Run()`:

```go
package main

import vil "github.com/oceanos-id/vil-sidecar-go"

func main() {
    app := vil.NewSidecar("ml-engine")

    app.Method("predict", func(req vil.Request) vil.Response {
        result := model.Predict(req.JSON())
        return vil.OK(result)
    })

    app.Method("classify", func(req vil.Request) vil.Response {
        label := classifier.Run(req.JSON())
        return vil.OK(map[string]string{"label": label})
    })

    app.Run()  // Connects to VilApp host, enters event loop
}
```

### Protocol Details

The sidecar protocol operates over **Unix Domain Sockets (UDS)** with a **length-prefixed JSON** wire format:

| Phase | Direction | Description |
|-------|-----------|-------------|
| **Handshake** | Sidecar → Host | `{"type":"handshake","name":"fraud-checker","methods":["fraud_check"]}` |
| **Invoke** | Host → Sidecar | `{"type":"invoke","method":"fraud_check","payload":{...}}` |
| **Result** | Sidecar → Host | `{"type":"result","data":{...}}` or `{"type":"error","message":"..."}` |
| **Health** | Host → Sidecar | `{"type":"health"}` → `{"type":"health_ok"}` |
| **Drain** | Host → Sidecar | `{"type":"drain"}` — stop accepting new invocations |
| **Shutdown** | Host → Sidecar | `{"type":"shutdown"}` — graceful termination |

- **Data plane**: Shared memory (`/dev/shm/vil_sc_{name}`) for zero-copy payload transfer; UDS carries only descriptors (~48 bytes).
- **ConnectionPool**: Built-in connection pooling with configurable backpressure limits and automatic reconnect on socket failure.

See `examples-sdk/sidecar/` and `examples/021-basic-usage-sidecar-python/` for complete runnable examples.

---

## FFI SDK — DEPRECATED (Historical Reference Only)

> **Deprecated (2026-03-26):** The FFI runtime (`vil_ffi`) has been removed. The sections below are retained as historical reference. For cross-language integration, use the **Transpile SDK** (`vil compile`) or the **Sidecar SDK** (UDS + SHM).

---

## 1. Rust (Native SDK) — First-Class Citizen

Rust is the primary implementation language and offers maximum control over VIL semantics.

### 1.1 Setup and Initialization

```toml
[dependencies]
vil_sdk = { path = "../crates/vil_sdk" }
vil_macros = { path = "../crates/vil_macros" }
vil_types = { path = "../crates/vil_types" }
```

### 1.2 Process Declaration and Lifecycle

#### Pipeline Pattern (vil_workflow macro)

```rust
use vil_sdk::prelude::*;

#[vil_process]
#[trace_hop]
struct DataProcessor {
    session_id: SessionId,
}

#[vil_main]
async fn main() {
    // Initialize shared memory runtime
    let world = VastarRuntimeWorld::new_shared().unwrap();

    // Create or join a workflow
    let (_ir, handles) = vil_workflow! {
        name: "RustPipeline",
        instances: [ DataProcessor ],
        routes: [
            // Define message routes here
        ]
    };

    // Wait for pipeline to finish
    for h in handles {
        h.join().unwrap();
    }
}
```

#### Process-Oriented Pattern (VilApp + ServiceProcess + vil_endpoint)

```rust
use vil_sdk::prelude::*;
use vil_server_core::VilApp;

#[vil_process]
struct MyService;

#[vil_endpoint(method = "POST", path = "/ingest")]
async fn ingest(req: VilRequest) -> VilResponse {
    let body = req.body();
    // Process incoming data via SHM
    VilResponse::ok(b"accepted")
}

fn main() {
    let app = VilApp::builder()
        .process::<MyService>("ingest-worker")
        .endpoint(ingest)
        .build()
        .unwrap();
    app.run();
}
```

### 1.3 Semantic Message Types

Always use semantic macros instead of generic `#[message]`:

```rust
#[vil_state]
pub struct ProcessingState {
    pub record_count: u64,
    pub last_timestamp: u64,
}

#[vil_event]
pub struct AuditLog {
    pub operation: String,
    pub timestamp: u64,
}

#[vil_fault]
pub enum ProcessingError {
    InvalidInput { reason: String },
    ResourceExhausted,
    TimeoutExpired { elapsed_ms: u64 },
}

#[vil_decision]
pub struct RoutingDecision {
    pub target_node: u32,
    pub priority: u8,
}
```

### 1.4 Zero-Copy Transfer Patterns via VAPI FFI

In cross-language scenarios, data transfer uses the C-ABI VAPI functions:

```rust
// These are the actual FFI functions exposed by vil_ffi (see crates/vil_ffi/src/lib.rs)

// Publish raw bytes to a port (zero-copy into SHM ring buffer)
// extern "C" fn vapi_publish_bytes(process, port_id, data, len) -> i32

// Receive bytes from a port (non-blocking, returns -2 if no data)
// extern "C" fn vapi_recv_bytes(process, port_id, &out_data, &out_len) -> i32

// Release buffer returned by vapi_recv_bytes
// extern "C" fn vapi_release_buffer(process, data, len)

// Connect two ports in the world (wire output -> input)
// extern "C" fn vapi_world_connect(world, from_port, to_port)
```

Within pure Rust, the runtime uses `world.publish_value()` and `world.recv()` directly on `ProcessHandle`, avoiding the FFI boundary entirely.

### 1.5 Observability and Instrumentation

```rust
#[vil_process]
#[trace_hop]                          // Automatic latency tracking
#[latency_marker("processing")]       // Dashboard label
#[counters(messages, errors, latency)]
struct ObservableProcessor;

// Access metrics at runtime
let snapshot = world.counters_snapshot();
println!("Messages Published: {}", snapshot.messages_published());
println!("P99 Latency: {} µs", snapshot.p99_latency_micros());
println!("Error Count: {}", snapshot.error_count());
```

### 1.6 Distributed Topology with Macro DSL

```rust
let (_ir, handles) = vil_workflow! {
    name: "DistributedProcessor",
    hosts: [
        edge: Host("10.0.0.1:9000"),
        core: Host("10.0.0.2:9000"),
    ],
    instances: [
        ingress @ edge,
        processor @ core,
    ],
    routes: [
        ingress.trigger -> processor.trigger (LoanWrite, transport: RDMA),
        ingress.data -> processor.data (LoanWrite, transport: RDMA),
    ],
    failover: [
        processor => backup_processor (on: HostDown, strategy: Immediate),
    ]
};
```

### 1.7 Error Handling and Resilience

```rust
use vil_sdk::fault::*;

// Structured error handling via Control Lane
pub async fn resilient_operation(processor: &mut DataProcessor) -> Result<()> {
    match processor.execute_critical_task().await {
        Ok(result) => {
            processor.signal_success().await?;
            Ok(result)
        }
        Err(e) => {
            let fault = ProcessingError::from(e);
            processor.signal_error(fault).await?;
            processor.control_abort(processor.session_id()).await?;
            Err(e)
        }
    }
}
```

---

## 2. C/C++ (VAPI C-ABI Binding)

C/C++ interoperability is achieved through the stable VAPI (VIL API) C-ABI interface.

### 2.1 Setup and Linking

```cmake
# CMakeLists.txt
cmake_minimum_required(VERSION 3.16)
project(vil_cpp_client)

find_library(VIL_FFI vil_ffi PATHS ${SDK_DIST}/lib)
find_path(VIL_INCLUDE vapi.h PATHS ${SDK_DIST}/include)

add_executable(cpp_app main.cpp)
target_include_directories(cpp_app PRIVATE ${VIL_INCLUDE})
target_link_libraries(cpp_app ${VIL_FFI})
```

### 2.2 VAPI C-ABI Function Reference

The complete VAPI surface consists of 14 functions, 2 structs:

```c
// vapi.h — VIL C-ABI (stable interface)

// Utility
const char* vil_version();
int         vil_health_check();       // Returns 0 if healthy
const char* vil_last_error();

// World lifecycle
void* vapi_world_new_shared();          // Returns VapiWorld*
void  vapi_world_free(void* world);

// Process management
void* vapi_process_register(void* world, const char* name); // Returns VapiProcess*
void  vapi_process_free(void* process);

// Data plane
void    vapi_world_connect(void* world, uint64_t from_port, uint64_t to_port);
int32_t vapi_publish_bytes(void* process, uint64_t port_id, const uint8_t* data, size_t len);
int32_t vapi_recv_bytes(void* process, uint64_t port_id, uint8_t** out_data, size_t* out_len);
void    vapi_release_buffer(void* process, uint8_t* data, size_t len);

// Control plane
typedef struct {
    uint32_t    fault_type;
    uint32_t    error_code;
    const char* message;
    size_t      message_length;
} VapiFaultSignal;

int32_t vapi_send_fault(void* process, const VapiFaultSignal* fault);
int32_t vapi_control_abort(void* process, uint64_t session_id);

// Metrics
typedef struct {
    uint64_t messages_published;
    uint64_t messages_received;
    uint64_t errors;
    uint64_t p99_latency_micros;
    uint64_t in_flight;
} VapiMetrics;

int32_t vapi_get_metrics(void* process, VapiMetrics* out_metrics);
```

### 2.3 Basic Process Registration

```cpp
#include "vapi.h"
#include <cstring>
#include <iostream>

int main() {
    std::cout << "VIL version: " << vil_version() << "\n";

    // Create shared memory world
    void* world = vapi_world_new_shared();
    if (!world) {
        std::cerr << "Failed to initialize world: " << vil_last_error() << "\n";
        return 1;
    }

    // Register process
    void* proc = vapi_process_register(world, "cpp_worker");
    if (!proc) {
        std::cerr << "Failed to register: " << vil_last_error() << "\n";
        vapi_world_free(world);
        return 1;
    }

    std::cout << "Process registered successfully\n";

    // Cleanup
    vapi_process_free(proc);
    vapi_world_free(world);
    return 0;
}
```

### 2.4 Zero-Copy Data Publishing

```cpp
#include "vapi.h"
#include <cstring>
#include <iostream>

void publish_sensor_data(void* proc, const uint8_t* sensor_data, size_t data_len) {
    int32_t result = vapi_publish_bytes(proc,
                                        1,  // port_id
                                        sensor_data,
                                        data_len);

    if (result != 0) {
        std::cerr << "Publish failed (rc=" << result << "): "
                  << vil_last_error() << "\n";
    }
}
```

### 2.5 Message Reception with Memory Management

```cpp
#include "vapi.h"
#include <memory>
#include <vector>
#include <iostream>

// RAII wrapper for received buffers
struct MessageBuffer {
    uint8_t* data = nullptr;
    size_t length = 0;
    void* owner = nullptr;

    ~MessageBuffer() {
        if (data && owner) {
            vapi_release_buffer(owner, data, length);
        }
    }
};

std::unique_ptr<MessageBuffer> receive_message(void* proc, uint64_t port_id) {
    uint8_t* buffer = nullptr;
    size_t length = 0;

    int32_t result = vapi_recv_bytes(proc, port_id, &buffer, &length);

    if (result == -2) {
        return nullptr;  // No data available (non-blocking)
    }
    if (result != 0) {
        std::cerr << "Recv failed: " << vil_last_error() << "\n";
        return nullptr;
    }

    auto msg = std::make_unique<MessageBuffer>();
    msg->data = buffer;
    msg->length = length;
    msg->owner = proc;
    return msg;
}
```

### 2.6 Fault Handling and Control Signals

```cpp
#include "vapi.h"
#include <cstring>
#include <iostream>

// Send error signal via Control Lane
void signal_processing_error(void* proc, const char* error_msg) {
    VapiFaultSignal fault;
    fault.fault_type = 1;  // Application-defined fault type
    fault.error_code = 1;
    fault.message = error_msg;
    fault.message_length = std::strlen(error_msg);

    int32_t result = vapi_send_fault(proc, &fault);
    if (result != 0) {
        std::cerr << "Failed to send fault: " << vil_last_error() << "\n";
    }
}

// Send session abort signal
void abort_session(void* proc, uint64_t session_id) {
    int32_t result = vapi_control_abort(proc, session_id);
    if (result != 0) {
        std::cerr << "Failed to abort session: " << vil_last_error() << "\n";
    }
}
```

### 2.7 Performance Monitoring via VAPI

```cpp
#include "vapi.h"
#include <iostream>

void print_performance_metrics(void* proc) {
    VapiMetrics metrics;
    int32_t result = vapi_get_metrics(proc, &metrics);

    if (result == 0) {
        std::cout << "Messages Published: " << metrics.messages_published << "\n";
        std::cout << "Messages Received: " << metrics.messages_received << "\n";
        std::cout << "Errors: "             << metrics.errors << "\n";
        std::cout << "P99 Latency (us): "   << metrics.p99_latency_micros << "\n";
        std::cout << "In-Flight: "          << metrics.in_flight << "\n";
    }
}
```

---

## 3. Go (CGO Bridge)

Go integrates with VIL through CGO while maintaining memory safety and efficient pointer handling.

### 3.1 Setup and Package Structure

The Go binding lives in `sdk/go/vil.go` and uses cgo to call the 14 VAPI functions directly. The CGO declarations are embedded in the Go source file — no separate header needed.

```go
import "vil"  // package vil in sdk/go/

// Key types:
//   vil.Runtime   — wraps VapiWorld (world lifecycle)
//   vil.Process   — wraps VapiProcess (publish, recv, fault, abort, metrics)
//   vil.Metrics   — performance snapshot struct
//   vil.ErrNoData — sentinel error for non-blocking recv with no data
//   vil.Lane      — LaneTrigger, LaneData, LaneControl
```

### 3.2 Initialization and Process Management

```go
package main

import (
    "fmt"
    "log"
    "vil"
)

func main() {
    fmt.Println("VIL version:", vil.Version())

    rt, err := vil.NewRuntime()
    if err != nil {
        log.Fatal(err)
    }
    defer rt.Close()

    proc, err := rt.RegisterProcess("go_worker")
    if err != nil {
        log.Fatal(err)
    }
    defer proc.Close()

    fmt.Println("Process", proc.Name(), "registered successfully")
}
```

### 3.3 Publish and Receive

```go
func publishAndReceive(rt *vil.Runtime) {
    // Wire port 1 -> port 2
    rt.Connect(1, 2)

    proc, _ := rt.RegisterProcess("go_pub")
    defer proc.Close()

    // Publish bytes to port 1
    err := proc.Publish(1, []byte{0x01, 0x02, 0x03, 0x04})
    if err != nil {
        fmt.Println("Publish error:", err)
    }

    // Non-blocking receive from port 2
    data, err := proc.Recv(2)
    if err == vil.ErrNoData {
        fmt.Println("No data available yet")
    } else if err != nil {
        fmt.Println("Recv error:", err)
    } else {
        fmt.Printf("Received %d bytes\n", len(data))
    }
}
```

### 3.4 Fault, Abort, and Metrics

```go
func controlPlaneExample(proc *vil.Process) {
    // Send fault signal
    err := proc.SendFault(1, 42, "sensor overload")
    if err != nil {
        fmt.Println("SendFault error:", err)
    }

    // Abort a session
    err = proc.ControlAbort(12345)
    if err != nil {
        fmt.Println("ControlAbort error:", err)
    }

    // Get metrics
    metrics, err := proc.GetMetrics()
    if err != nil {
        fmt.Println("GetMetrics error:", err)
    } else {
        fmt.Printf("Published: %d, Received: %d, Errors: %d\n",
            metrics.MessagesPublished, metrics.MessagesReceived, metrics.Errors)
    }
}
```

### 3.5 Error Handling

```go
// vil.LastError() returns the last FFI error message (thread-local).
// All methods that can fail return (value, error) pairs.
// vil.ErrNoData is returned by Recv() when no data is available (non-blocking).

if err := proc.Publish(1, data); err != nil {
    fmt.Println("Error:", err, "| Last FFI error:", vil.LastError())
}
```

---

## 4. Java (JNI with Direct ByteBuffer)

Java leverages Direct ByteBuffer to achieve zero-copy semantics while maintaining type safety.

### 4.1 SDK Architecture

The Java SDK (`sdk/java/src/main/java/dev/vil/VilRuntime.java`) uses JNI to call
the 14 VAPI functions. Key classes:

- **`VilRuntime`** — world lifecycle, implements `AutoCloseable`
- **`VilRuntime.Process`** — process handle (publish, recv, sendFault, controlAbort, getMetrics), implements `AutoCloseable`
- **`VilRuntime.Metrics`** — performance snapshot (messagesPublished, messagesReceived, errors, p99LatencyMicros, inFlight)
- **`VilRuntime.FaultSignal`** — fault signal value object (faultType, errorCode, message)
- **`VilRuntime.Lane`** — enum: TRIGGER, DATA, CONTROL

### 4.2 Process Initialization and Lifecycle

```java
import dev.vil.VilRuntime;

public class App {
    public static void main(String[] args) {
        System.out.println("Version: " + new VilRuntime().version());

        try (var rt = new VilRuntime()) {
            try (var proc = rt.registerProcess("java_worker")) {
                System.out.println("Process: " + proc.getName());

                // Publish data
                proc.publish(1, new byte[]{0x01, 0x02, 0x03, 0x04});

                // Non-blocking receive (returns null if no data)
                byte[] data = proc.recv(2);
                if (data != null) {
                    System.out.println("Received " + data.length + " bytes");
                }
            }
        }
    }
}
```

### 4.3 Control Plane: Faults and Abort

```java
import dev.vil.VilRuntime;
import dev.vil.VilRuntime.FaultSignal;

try (var rt = new VilRuntime()) {
    try (var proc = rt.registerProcess("ctrl_worker")) {
        // Send fault signal
        proc.sendFault(new FaultSignal(1, 42, "sensor overload"));

        // Abort a session
        proc.controlAbort(12345);
    }
}
```

### 4.4 Metrics and Port Connection

```java
try (var rt = new VilRuntime()) {
    // Connect ports
    rt.connect(1, 2);

    try (var proc = rt.registerProcess("metrics_worker")) {
        VilRuntime.Metrics m = proc.getMetrics();
        if (m != null) {
            System.out.println(m); // Metrics{published=0, received=0, ...}
        }
    }
}
```

### 4.5 Error Handling

```java
// All errors throw RuntimeException or IllegalStateException.
// Use VilRuntime.lastError() for the last FFI-level error message.
try {
    proc.publish(1, data);
} catch (RuntimeException e) {
    System.err.println("FFI error: " + VilRuntime.lastError());
}
```

---

## 5. Python (ctypes Wrapper)

Python provides high-level abstractions over VAPI using ctypes for FFI without native compilation.

### 5.1 SDK Architecture

The Python SDK (`sdk/python/vil/runtime.py`) uses ctypes to call the 14 VAPI functions.
The library auto-discovers `libvil_ffi.so` (or `.dylib`) from standard locations.

Key classes:

- **`VilRuntime`** — world lifecycle, context manager (`with` support)
- **`VilProcess`** — process handle: `publish()`, `recv()`, `send_fault()`, `control_abort()`, `get_metrics()`, context manager
- **`Metrics`** — dataclass with `messages_published`, `messages_received`, `errors`, `p99_latency_micros`, `in_flight`
- **`VilError`** — base exception for all runtime errors

### 5.2 Basic Usage

```python
from vil.runtime import VilRuntime

with VilRuntime() as rt:
    print("Version:", rt.version())
    print("Healthy:", rt.health_check())

    with rt.register_process("py_worker") as proc:
        # Publish bytes to port 1
        proc.publish(1, b"\x01\x02\x03\x04")

        # Non-blocking receive from port 2 (returns None if no data)
        data = proc.recv(2)
        if data is not None:
            print(f"Received {len(data)} bytes")

        # Get performance metrics
        metrics = proc.get_metrics()
        print(f"Published: {metrics.messages_published}")
```

### 5.3 Control Plane: Faults and Abort

```python
from vil.runtime import VilRuntime

with VilRuntime() as rt:
    with rt.register_process("ctrl_worker") as proc:
        # Send fault signal
        proc.send_fault(fault_type=1, error_code=42, message="sensor overload")

        # Abort a session
        proc.control_abort(session_id=12345)

        # Port connections
        rt.connect(from_port=1, to_port=2)
```

### 5.4 Error Handling

```python
from vil.runtime import VilRuntime, VilError

rt = VilRuntime()
try:
    proc = rt.register_process("worker")
    proc.publish(1, b"hello")
except VilError as e:
    print(f"VIL error: {e}")
    print(f"Last FFI error: {rt.last_error()}")
finally:
    rt.close()
```

---

## 6. Comparison Matrix

| Language | Interface | Zero-Copy Level | Setup Complexity | Performance Class | Best Use Case |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Rust** | Native Trait | Physical (LoanWrite/Read) | Low | Extreme | Core Engine, Logic |
| **C++** | C-ABI (VAPI) | Pointer-Level | Medium | Extreme | Video/Image Processing, HPC |
| **Go** | CGO Bridge | Pointer-Level | Medium | High | Microservices, Cloud |
| **Java** | JNI + ByteBuffer | Direct Buffer | Medium | High | Enterprise Applications |
| **Python** | ctypes Wrapper | Buffered Copy | High | Medium | Scripting, Data Science |

---

## 7. Performance Optimization Strategies

### 7.1 Buffer Pooling and Reuse

Implement object pools to reduce allocation overhead:

```rust
// Rust example
use std::collections::VecDeque;

pub struct BufferPool {
    available: VecDeque<Vec<u8>>,
    buffer_size: usize,
}

impl BufferPool {
    pub fn new(capacity: usize, buffer_size: usize) -> Self {
        let mut available = VecDeque::with_capacity(capacity);
        for _ in 0..capacity {
            available.push_back(Vec::with_capacity(buffer_size));
        }
        Self { available, buffer_size }
    }
    
    pub fn acquire(&mut self) -> Vec<u8> {
        self.available.pop_front().unwrap_or_else(|| Vec::with_capacity(self.buffer_size))
    }
    
    pub fn release(&mut self, mut buffer: Vec<u8>) {
        buffer.clear();
        if self.available.len() < self.available.capacity() {
            self.available.push_back(buffer);
        }
    }
}
```

### 7.2 Batch Processing for Throughput

```go
// Go example - batch messages for improved throughput
func processBatch(proc *C.VapiProcess, portID C.uint32_t, batchSize int) {
    messages := make([][]byte, 0, batchSize)
    
    for i := 0; i < batchSize; i++ {
        var buffer *C.uint8_t
        var length C.size_t
        
        if C.vapi_recv_bytes(proc, portID, &buffer, &length) == 0 {
            msg := C.GoBytes(unsafe.Pointer(buffer), C.int(length))
            messages = append(messages, msg)
        }
    }
    
    // Process all messages in batch
    for _, msg := range messages {
        processMessage(msg)
    }
}
```

### 7.3 Memory Pinning for RDMA

```cpp
// C++ example - pin memory for RDMA transfer
#include <sys/mman.h>

void pin_memory_for_rdma(uint8_t* buffer, size_t length) {
    int result = mlock(buffer, length);
    if (result != 0) {
        std::cerr << "Failed to pin memory for RDMA\n";
    }
}

void unpin_memory(uint8_t* buffer, size_t length) {
    munlock(buffer, length);
}
```

---

## 8. Debugging and Troubleshooting

### 8.1 Enable Runtime Tracing

```rust
// Rust: enable verbose logging
#[vil_process]
#[trace_hop]
#[enable_debug_logging]
struct DebugProcessor;
```

### 8.2 Validate Message Contracts

```bash
# Check generated execution contract
cat vil_execution_contract.json | jq '.lanes'
```

### 8.3 Monitor System Metrics

```python
# Python: retrieve and display metrics
from vil.runtime import VilRuntime

with VilRuntime() as rt:
    with rt.register_process("monitor") as proc:
        m = proc.get_metrics()
        print(f"Messages Published: {m.messages_published}")
        print(f"Messages Received: {m.messages_received}")
        print(f"Errors: {m.errors}")
        print(f"P99 Latency: {m.p99_latency_micros} us")
        print(f"In-Flight: {m.in_flight}")
```

### 8.4 Blocking vs Non-Blocking Receive

`vapi_recv_bytes` is non-blocking: it returns `-2` (mapped to `None`/`null`/`ErrNoData`) when no data is available. For languages that need blocking I/O, a future `vapi_recv_bytes_blocking` function will be exposed. Until then, implement polling with backoff:

```go
// Go example: poll with backoff
for {
    data, err := proc.Recv(portID)
    if err == vil.ErrNoData {
        time.Sleep(1 * time.Millisecond)
        continue
    }
    if err != nil {
        log.Fatal(err)
    }
    process(data)
}
```

### 8.5 Running SDK Tests

Each language SDK includes test files. Build the FFI library first:

```bash
cargo build -p vil_ffi --release
```

Then run tests per language:

```bash
# Python
cd sdk/python
python -m pytest tests/test_runtime.py -v

# Go
cd sdk/go
CGO_LDFLAGS="-L../../target/release" go test -v

# Java (requires JUnit 5 on classpath)
cd sdk/java
mvn test   # or: gradle test

# Node.js (requires ffi-napi, ref-napi, ref-struct-napi)
cd sdk/nodejs
npm install
node test/test_runtime.js
```

---

## 9. Security Considerations

### 9.1 Buffer Overflow Protection

- Always validate buffer lengths before operations
- Use Safe abstractions (Rust, Java) where possible
- Implement bounds checking in C/C++

### 9.2 Memory Access Control

- Leverage Trust Zones for untrusted code (WASM Capsules)
- Validate all inter-process communication
- Use capability model for fine-grained access control

### 9.3 Capability-Based Security

```rust
// Rust: Declare execution zone
#[vil_process(zone = NativeTrusted)]
struct TrustedProcessor;

#[vil_process(zone = WasmCapsule)]
struct UntrustedPlugin;
```

---

## 10. Migration Guide: Adding New Language Support

To add support for a new language:

1. **Generate C Header** from VAPI via `vil_codegen_c`
2. **Create Language Bindings** (FFI/ctypes/JNI as appropriate)
3. **Implement Wrapper Classes** for ergonomic API
4. **Write Integration Tests** against the runtime substrate
5. **Document Patterns** specific to the language
6. **Benchmark Performance** against reference Rust implementation

---

## 10b. AI Plugin SDK Integration

AI plugins are accessible from all SDK languages via the same FFI bridge.

**From Rust (native):**
```rust
use vil_server::prelude::*;
use vil_llm::semantic::{LlmResponseEvent, LlmFault, LlmUsageState};

let content = SseCollect::post_to(url)
    .dialect(SseDialect::openai())
    .bearer_token(key)
    .body(body).collect_text().await?;
```

**From Python (Transpile SDK):**
```python
vil.sse_collect(url, dialect="openai", bearer_token=key, body=body)
```

51 AI crates available (all VIL Way): LLM, RAG, Agent, Embedder, VectorDB, and 46 more.
Each provides SSE pipeline builders + REST plugin endpoints.

---

## 11. Conclusion and Best Practices

### Summary

VIL's cross-language architecture provides two complementary modes:

- **Transpile SDK (Production)**: Write pipelines in Python/Go/Java/TypeScript using the VilPipeline DSL, then `vil compile` to native Rust binaries. Zero FFI overhead, Rust-native performance (~3,855 req/s).
- **FFI SDK (Development)**: Call the VIL runtime directly via VAPI bindings for rapid prototyping, debugging, and testing.
- **Type Safety**: All 9 semantic primitives available in every language
- **Operational Simplicity**: Automatic observability and contract generation
- **Deployment Flexibility**: Single static binary from any source language

### Recommended Patterns

1. Use **Rust** for core engine development and maximum control
2. Use **Python/Go/Java/TypeScript** with `vil compile` for application-level pipelines (same performance as Rust)
3. Use **C++** for hardware-accelerated operations (video, ML) via IDL-generated headers
4. Use **Sidecar SDK** for external ML models (Python/Go) with SHM zero-copy

### Additional Resources

- [Architecture Overview](../ARCHITECTURE_OVERVIEW.md) — layered system design
- [VIL Developer Guide](./VIL-Developer-Guide.md) — complete language reference
- [Quick Start](../QUICK_START.md) — first pipeline in 10 minutes
- [VIL Concept](./VIL_CONCEPT.md) — 10 immutable design principles
- **VAPI Reference**: Generated via `vil_codegen_c`

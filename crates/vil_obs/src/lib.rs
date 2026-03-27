// =============================================================================
// vil_obs — Built-In Observability
// =============================================================================
// Observability is not an afterthought in VIL — it is embedded in
// the runtime. Every publish/recv/crash produces observable events
// consumable by monitoring, debugging, and audit tools.
//
// Architecture:
//   ┌─────────────────────────────────────────────┐
//   │ RuntimeWorld                                │
//   │   ↓ publish()   ↓ recv()    ↓ crash()      │
//   │   TraceEvent    TraceEvent   TraceEvent     │
//   │   ↓             ↓            ↓              │
//   │ ┌─────────────────────────────────────────┐ │
//   │ │ RuntimeObserver (callback sink)         │ │
//   │ └─────────────────────────────────────────┘ │
//   │   ↓                                         │
//   │ RuntimeCounters (atomic live counters)       │
//   │ LatencyTracker (publish→recv histogram)      │
//   └─────────────────────────────────────────────┘
//
// Modules:
//   events.rs   — TraceEvent enum (all observable events)
//   counters.rs — RuntimeCounters (atomic u64 counters)
//   latency.rs  — LatencyTracker (histogram bucket-based)
//   observer.rs — RuntimeObserver (event callback sink)
//
// TASK LIST:
// [x] TraceEvent — publish/recv/crash/connect events
// [x] RuntimeCounters — atomic counters
// [x] LatencyTracker — bucket-based latency histogram
// [x] RuntimeObserver — callback-based event sink
// [x] Unit tests
// [ ] TODO(future): OpenTelemetry exporter
// [ ] TODO(future): structured logging integration
// [ ] TODO(future): live dashboard metrics endpoint
// =============================================================================

pub mod counters;
pub mod events;
pub mod hop_registry;
pub mod latency;
pub mod observer;
pub mod prometheus;

pub use counters::{CounterSnapshot, RuntimeCounters};
pub use events::TraceEvent;
pub use hop_registry::HopLatencyRegistry;
pub use latency::{LatencySnapshot, LatencyTracker};
pub use observer::{ObservabilityHub, RuntimeObserver};
pub use prometheus::VilMetrics;

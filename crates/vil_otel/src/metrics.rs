// =============================================================================
// vil_otel::metrics — MetricsBridge
// =============================================================================
//
// Bridges vil_obs CounterSnapshot values to OpenTelemetry metrics.
// Uses monotonic counters for cumulative values (publishes, receives, drops)
// and up-down counters/gauges for live state values.
//
// Usage:
//   let bridge = MetricsBridge::new(&config)?;
//   bridge.record_snapshot(&counters.snapshot());
// =============================================================================

use opentelemetry::{
    metrics::{Counter, Gauge, Meter},
    KeyValue,
};

use crate::{config::OtelConfig, error::OtelFault};

/// Metric names registered in the global dict for hash lookups.
const METRIC_PUBLISHES: &str = "vil.runtime.publishes";
const METRIC_RECEIVES: &str = "vil.runtime.receives";
const METRIC_DROPS: &str = "vil.runtime.drops";
const METRIC_CRASHES: &str = "vil.runtime.crashes";
const METRIC_ORPHANS: &str = "vil.runtime.orphans_reclaimed";
const METRIC_DESCRIPTORS: &str = "vil.runtime.descriptors_drained";
const METRIC_NET_PULLS: &str = "vil.runtime.net_pulls";
const METRIC_FAILOVER: &str = "vil.runtime.failover_events";
const METRIC_HOPS: &str = "vil.runtime.hops";

/// A point-in-time snapshot of vil_obs counter values.
/// This mirrors `vil_obs::CounterSnapshot` without a direct dependency.
#[derive(Debug, Clone, Copy, Default)]
pub struct CounterSnapshot {
    pub publishes: u64,
    pub receives: u64,
    pub drops: u64,
    pub crashes: u64,
    pub orphans_reclaimed: u64,
    pub descriptors_drained: u64,
    pub net_pulls: u64,
    pub failover_events: u64,
    pub hops: u64,
}

/// Bridge that exports vil_obs counters as OTel metrics.
pub struct MetricsBridge {
    /// Service label applied to every metric data point.
    service_name: String,
    /// OTel meter handle.
    meter: Meter,
    // Instruments
    publishes:    Counter<u64>,
    receives:     Counter<u64>,
    drops:        Counter<u64>,
    crashes:      Counter<u64>,
    orphans:      Counter<u64>,
    descriptors:  Counter<u64>,
    net_pulls:    Counter<u64>,
    failover:     Counter<u64>,
    hops_gauge:   Gauge<u64>,
}

impl MetricsBridge {
    /// Create a new MetricsBridge using a pre-built OTel `Meter`.
    pub fn with_meter(meter: Meter, config: &OtelConfig) -> Result<Self, OtelFault> {
        // Register all metric name strings in vil_log dict for hash lookup.
        vil_log::dict::register_str(METRIC_PUBLISHES);
        vil_log::dict::register_str(METRIC_RECEIVES);
        vil_log::dict::register_str(METRIC_DROPS);
        vil_log::dict::register_str(METRIC_CRASHES);
        vil_log::dict::register_str(METRIC_ORPHANS);
        vil_log::dict::register_str(METRIC_DESCRIPTORS);
        vil_log::dict::register_str(METRIC_NET_PULLS);
        vil_log::dict::register_str(METRIC_FAILOVER);
        vil_log::dict::register_str(METRIC_HOPS);

        let publishes   = meter.u64_counter(METRIC_PUBLISHES).build();
        let receives    = meter.u64_counter(METRIC_RECEIVES).build();
        let drops       = meter.u64_counter(METRIC_DROPS).build();
        let crashes     = meter.u64_counter(METRIC_CRASHES).build();
        let orphans     = meter.u64_counter(METRIC_ORPHANS).build();
        let descriptors = meter.u64_counter(METRIC_DESCRIPTORS).build();
        let net_pulls   = meter.u64_counter(METRIC_NET_PULLS).build();
        let failover    = meter.u64_counter(METRIC_FAILOVER).build();
        let hops_gauge  = meter.u64_gauge(METRIC_HOPS).build();

        Ok(Self {
            service_name: config.service_name.clone(),
            meter,
            publishes,
            receives,
            drops,
            crashes,
            orphans,
            descriptors,
            net_pulls,
            failover,
            hops_gauge,
        })
    }

    /// Export a CounterSnapshot as a batch of OTel metric observations.
    ///
    /// This should be called on each poll interval with a fresh snapshot
    /// derived from `vil_obs::RuntimeCounters::snapshot()`.
    pub fn record_snapshot(&self, snap: &CounterSnapshot) {
        let attrs = [KeyValue::new("service.name", self.service_name.clone())];

        // Use add(value) — OTel monotonic counters accumulate from last export.
        // The caller is responsible for passing delta values if needed.
        self.publishes.add(snap.publishes, &attrs);
        self.receives.add(snap.receives, &attrs);
        self.drops.add(snap.drops, &attrs);
        self.crashes.add(snap.crashes, &attrs);
        self.orphans.add(snap.orphans_reclaimed, &attrs);
        self.descriptors.add(snap.descriptors_drained, &attrs);
        self.net_pulls.add(snap.net_pulls, &attrs);
        self.failover.add(snap.failover_events, &attrs);
        self.hops_gauge.record(snap.hops, &attrs);
    }

    /// Returns a reference to the underlying meter.
    pub fn meter(&self) -> &Meter {
        &self.meter
    }
}

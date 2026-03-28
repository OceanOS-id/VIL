// =============================================================================
// vil_ir::contract — Execution Contract
// =============================================================================
// An `ExecutionContract` is a serialisable summary of a `WorkflowIR`,
// capturing all accumulated metadata:
//
//   - Trust zone and capability profile
//   - Lane/port roles and transfer modes
//   - Host topology
//   - HA failover strategies
//   - Observability annotations
//   - Memory class
//
// The generated JSON can be consumed by:
//   - Neutrino Runtime — scheduling & resource allocation
//   - Cluster Orchestrator — deployment & placement
//   - Monitoring Dashboard — pipeline topology auto-discovery
// =============================================================================

use serde::{Deserialize, Serialize};

/// Top-level Execution Contract for a VIL workflow.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ExecutionContract {
    /// Name of the workflow / pipeline.
    pub pipeline: String,

    /// Primary execution trust zone (from the first annotated process, or "Unset").
    pub execution_zone: String,

    /// Aggregated trust capability profile across all processes.
    pub trust_profile: TrustProfile,

    /// Summary of lanes used by this workflow (derived from routes + ports).
    pub lanes: Vec<LaneEntry>,

    /// Hosts declared in this workflow.
    pub hosts: Vec<String>,

    /// HA failover strategies declared in this workflow.
    pub failover: Vec<FailoverEntry>,

    /// Observability annotations aggregated across processes.
    pub observability: ObservabilityEntry,

    /// Process count and names for orchestration.
    pub processes: Vec<ProcessSummary>,
}

/// Trust capability profile: union of capabilities across all processes.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct TrustProfile {
    pub can_emit_lane: bool,
    pub can_read_state: bool,
    pub can_use_secret: bool,
    pub can_access_shm: bool,
    pub can_join_cluster: bool,
    pub can_use_remote: bool,
}

/// A lane entry: describes a route's role and transfer semantics.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LaneEntry {
    /// Format: "from_process.port -> to_process.port"
    pub route: String,
    /// Lane kind: "default", "trigger", "data", "control"
    pub lane_kind: String,
    /// Transfer mode: "loan_write", "loan_read", "copy", etc.
    pub transfer: String,
    /// Memory class of the message being routed.
    pub memory_class: String,
}

/// A failover entry: describes one FailoverIR record in human terms.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FailoverEntry {
    pub source: String,
    pub target: String,
    pub condition: String,
    pub strategy: String,
}

/// Aggregated observability settings across all processes.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ObservabilityEntry {
    /// True if any process has `#[trace_hop]`.
    pub trace_hops: bool,
    /// All latency labels from `#[latency_marker("label")]` across processes.
    pub latency_markers: Vec<String>,
}

/// Brief per-process summary for orchestration tooling.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProcessSummary {
    pub name: String,
    pub exec_class: String,
    pub trust_zone: Option<String>,
    pub host_affinity: Option<String>,
    pub trace_hop: bool,
    pub latency_label: Option<String>,
    pub memory_class: Option<String>,
}

impl ExecutionContract {
    /// Serialise to a pretty-printed JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Serialise to YAML — compatible with the .vwfd workflow file format style.
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_serializes_to_json() {
        let contract = ExecutionContract {
            pipeline: "TestPipeline".into(),
            execution_zone: "NativeTrusted".into(),
            trust_profile: TrustProfile {
                can_emit_lane: true,
                can_read_state: true,
                ..Default::default()
            },
            lanes: vec![LaneEntry {
                route: "Source.out -> Sink.in_port".into(),
                lane_kind: "data".into(),
                transfer: "loan_write".into(),
                memory_class: "paged_exchange".into(),
            }],
            hosts: vec!["192.168.1.10:9000".into()],
            failover: vec![FailoverEntry {
                source: "Source".into(),
                target: "BackupSink".into(),
                condition: "Crash".into(),
                strategy: "Immediate".into(),
            }],
            observability: ObservabilityEntry {
                trace_hops: true,
                latency_markers: vec!["inference".into()],
            },
            processes: vec![ProcessSummary {
                name: "Source".into(),
                exec_class: "thread".into(),
                trust_zone: Some("NativeTrusted".into()),
                host_affinity: None,
                trace_hop: true,
                latency_label: Some("inference".into()),
                memory_class: None,
            }],
        };

        let json = serde_json::to_string_pretty(&contract).unwrap();
        assert!(json.contains("TestPipeline"));
        assert!(json.contains("NativeTrusted"));
        assert!(json.contains("loan_write"));
        assert!(json.contains("inference"));
        assert!(json.contains("trace_hops"));

        // Round-trip
        let decoded: ExecutionContract = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, contract);
    }

    #[test]
    fn test_empty_contract_is_valid() {
        let contract = ExecutionContract {
            pipeline: "Empty".into(),
            execution_zone: "Unset".into(),
            trust_profile: TrustProfile::default(),
            lanes: vec![],
            hosts: vec![],
            failover: vec![],
            observability: ObservabilityEntry::default(),
            processes: vec![],
        };
        let json = serde_json::to_string(&contract).unwrap();
        assert!(json.contains("Empty"));
    }

    #[test]
    fn test_contract_yaml_roundtrip() {
        let contract = ExecutionContract {
            pipeline: "YamlPipeline".into(),
            execution_zone: "NativeCore".into(),
            trust_profile: TrustProfile {
                can_emit_lane: true,
                can_use_remote: true,
                ..Default::default()
            },
            lanes: vec![LaneEntry {
                route: "A.out -> B.in".into(),
                lane_kind: "data".into(),
                transfer: "loan_write".into(),
                memory_class: "paged_exchange".into(),
            }],
            hosts: vec!["10.0.1.11:9100".into()],
            failover: vec![FailoverEntry {
                source: "A".into(),
                target: "B".into(),
                condition: "Crash".into(),
                strategy: "Immediate".into(),
            }],
            observability: ObservabilityEntry {
                trace_hops: true,
                latency_markers: vec!["inference".into()],
            },
            processes: vec![],
        };

        let yaml = contract.to_yaml().expect("YAML serialization failed");
        // Must contain key fields
        assert!(yaml.contains("pipeline: YamlPipeline"));
        assert!(yaml.contains("execution_zone: NativeCore"));
        assert!(yaml.contains("loan_write"));
        assert!(yaml.contains("trace_hops: true"));
        assert!(yaml.contains("inference"));

        // Round-trip
        let decoded: ExecutionContract = serde_yaml::from_str(&yaml).expect("YAML decode failed");
        assert_eq!(decoded, contract);
    }
}

#![allow(dead_code)]
// =============================================================================
// VIL Operator — Kubernetes Resource Generation
// =============================================================================

use crate::crd::VilServerSpec;
use serde_json::json;

/// Generate Deployment JSON from VilServer spec.
pub fn generate_deployment(name: &str, namespace: &str, spec: &VilServerSpec) -> serde_json::Value {
    let mut volumes = Vec::<serde_json::Value>::new();
    let mut volume_mounts = Vec::<serde_json::Value>::new();

    if spec.shm.enabled {
        volumes.push(json!({
            "name": "shm",
            "emptyDir": { "medium": "Memory", "sizeLimit": spec.shm.size_limit }
        }));
        volume_mounts.push(json!({
            "name": "shm",
            "mountPath": "/dev/shm"
        }));
    }

    json!({
        "apiVersion": "apps/v1",
        "kind": "Deployment",
        "metadata": { "name": name, "namespace": namespace },
        "spec": {
            "replicas": spec.replicas,
            "selector": { "matchLabels": { "app": name } },
            "template": {
                "metadata": {
                    "labels": { "app": name },
                    "annotations": {
                        "prometheus.io/scrape": "true",
                        "prometheus.io/port": spec.metrics_port.to_string(),
                    }
                },
                "spec": {
                    "containers": [{
                        "name": "vil-server",
                        "image": spec.image,
                        "ports": [
                            { "name": "http", "containerPort": spec.port },
                            { "name": "metrics", "containerPort": spec.metrics_port }
                        ],
                        "env": [
                            { "name": "VIL_SERVER_PORT", "value": spec.port.to_string() },
                            { "name": "VIL_METRICS_PORT", "value": spec.metrics_port.to_string() },
                        ],
                        "livenessProbe": {
                            "httpGet": { "path": "/health", "port": "http" },
                            "initialDelaySeconds": 5,
                            "periodSeconds": 10
                        },
                        "readinessProbe": {
                            "httpGet": { "path": "/ready", "port": "http" },
                            "initialDelaySeconds": 3,
                            "periodSeconds": 5
                        },
                        "volumeMounts": volume_mounts,
                    }],
                    "volumes": volumes,
                }
            }
        }
    })
}

/// Generate Service JSON.
pub fn generate_service(name: &str, namespace: &str, spec: &VilServerSpec) -> serde_json::Value {
    json!({
        "apiVersion": "v1",
        "kind": "Service",
        "metadata": { "name": name, "namespace": namespace },
        "spec": {
            "type": "ClusterIP",
            "ports": [
                { "name": "http", "port": spec.port, "targetPort": "http" },
                { "name": "metrics", "port": spec.metrics_port, "targetPort": "metrics" }
            ],
            "selector": { "app": name }
        }
    })
}

// =============================================================================
// VIL Kubernetes Operator — Main Entry
// =============================================================================

fn main() {
    println!("vil-operator v0.1.0");
    println!("Kubernetes operator for VilServer CRD");
    println!();
    println!("Usage:");
    println!("  1. Install CRD:  kubectl apply -f manifests/crd.yaml");
    println!("  2. Install RBAC: kubectl apply -f manifests/rbac.yaml");
    println!("  3. Run operator: vil-operator");
    println!();
    println!("Note: Requires kube-rs runtime. Run with:");
    println!("  RUST_LOG=info vil-operator");
}

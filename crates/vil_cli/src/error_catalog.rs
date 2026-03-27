//! vil explain — VIL error code catalog
//!
//! Provides human-readable explanations for VIL error codes.

pub fn explain(code: &str) -> Result<(), String> {
    let entry = lookup(code);

    match entry {
        Some((title, explanation, fix)) => {
            println!();
            println!("  {} — {}", code, title);
            println!("  {}", "─".repeat(60));
            println!();
            for line in explanation.lines() {
                println!("  {}", line);
            }
            println!();
            println!("  Fix:");
            for line in fix.lines() {
                println!("    {}", line);
            }
            println!();
            Ok(())
        }
        None => {
            println!();
            println!("  Unknown error code: {}", code);
            println!();
            println!("  Available error codes:");
            for (code, title, _, _) in ERROR_CATALOG {
                println!("    {} — {}", code, title);
            }
            println!();
            Ok(())
        }
    }
}

fn lookup(code: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let normalized = code.to_uppercase();
    ERROR_CATALOG.iter()
        .find(|(c, _, _, _)| *c == normalized)
        .map(|(_, title, explanation, fix)| (*title, *explanation, *fix))
}

/// Error catalog: (code, title, explanation, fix)
const ERROR_CATALOG: &[(&str, &str, &str, &str)] = &[
    (
        "E-VIL-0001",
        "LayoutLegality violation",
        "Your message type contains a heap-allocated field (String, Vec, Box)\nin a zero-copy path. VIL requires VASI-compliant types for\nSHM transport — all fields must be fixed-size primitives.",
        "Replace heap types with VASI-compliant alternatives:\n  - String → VSlice<u8>\n  - Vec<T> → VSlice<T>\n  - Box<T> → VRef<T>\nSee: docs/vil/VIL-Developer-Guide.md section 3 (Memory Classes)",
    ),
    (
        "E-VIL-0002",
        "TransferCapability mismatch",
        "The transfer mode specified for this route is not compatible with\nthe message's layout profile. For example, LoanWrite requires\nVASI-compliant messages, but the message has External layout.",
        "Either:\n  1. Change the message layout to Flat or Relative\n  2. Change the transfer mode to Copy (for External layout)\nSee: docs/vil/VIL_CONCEPT.md section P9 (Ownership Transfer Model)",
    ),
    (
        "E-VIL-0003",
        "BoundaryLegality violation",
        "A message is crossing a boundary that doesn't support its\ntransfer mode. For example, zero-copy (LoanWrite) is not\navailable across host boundaries — use Copy instead.",
        "Check the boundary kind of your route:\n  - IntraProcess / InterThread: all modes OK\n  - InterProcess (SHM): LoanWrite, LoanRead, PublishOffset OK\n  - InterHost: Copy only\nSee: docs/vil/VIL_CONCEPT.md section 5 (Boundary Classification)",
    ),
    (
        "E-VIL-0004",
        "QueueCapability violation",
        "The queue kind specified doesn't match the port's requirements.\nSPSC queues only support single-producer single-consumer patterns.",
        "Use Mpmc queue kind for ports with multiple producers or consumers.\nSee: docs/vil/VIL-Developer-Guide.md section 6 (Building Pipelines)",
    ),
    (
        "E-VIL-0005",
        "OwnershipLegality violation",
        "A message with linear ownership semantics (ConsumeOnce) was\nused in a path that allows sharing. Linear resources must be\nconsumed exactly once.",
        "Ensure ConsumeOnce messages have exactly one consumer.\nDo not use ShareRead or broadcast patterns with linear resources.",
    ),
    (
        "E-VIL-0006",
        "Missing #[vil_endpoint] annotation",
        "A handler function is not annotated with #[vil_endpoint].\nThis is required for VLB compilation and vflow-server provisioning.",
        "Add #[vil_endpoint] to your handler:\n  #[vil_endpoint]\n  async fn my_handler(...) -> VilResult<T> { ... }",
    ),
    (
        "E-VIL-0007",
        "Missing #[derive(VilModel)]",
        "A message type used in endpoint input/output does not derive\nVilModel. This is required for schema export and SHM transport.",
        "Add VilModel derive:\n  #[derive(Clone, Debug, Serialize, Deserialize, VilModel)]\n  struct MyType { ... }",
    ),
    (
        "E-VIL-0008",
        "Missing #[vil_service_state]",
        "Service state is detected but not annotated with\n#[vil_service_state]. This is required for lifecycle management.",
        "Add the attribute:\n  #[vil_service_state]\n  struct MyState { db: DbPool }",
    ),
    (
        "E-VIL-0009",
        "Hidden HTTP dependency detected",
        "Your handler makes HTTP calls (reqwest, hyper::Client) without\ndeclaring the target as a mesh requirement. Inter-service\ncommunication should use Tri-Lane SHM, not HTTP.",
        "Declare the dependency in VxMeshConfig:\n  VxMeshConfig::new().route(\"my-service\", \"target\", VxLane::Data)\nOr use ServiceCtx.send() for Tri-Lane communication.",
    ),
    (
        "E-VIL-0010",
        "Legacy API usage (vil_server::new)",
        "Your code uses vil_server::new() which is the legacy API.\nFor VX Process-Oriented architecture, use VilApp::new().",
        "Replace:\n  vil_server::new(\"app\").route(...).run().await\nWith:\n  VilApp::new(\"app\").service(svc).run().await\nSee: docs/vil/VIL-Developer-Guide.md section 2b",
    ),
    (
        "E-VIL-0011",
        "CleanupObligation not met",
        "A process does not specify a cleanup policy. VIL requires\nexplicit cleanup behavior for crash recovery.",
        "Set cleanup policy on your process:\n  ExecClass::AsyncTask (default: ReclaimOrphans)\nOr configure CleanupConfig explicitly.",
    ),
    (
        "E-VIL-0012",
        "ObservabilityCompleteness warning",
        "A process does not have observability annotations. VIL\nrecommends #[trace_hop] on all processes for automatic\nlatency tracking.",
        "Add trace annotation:\n  #[vil_process]\n  #[trace_hop]\n  struct MyProcessor;",
    ),
];

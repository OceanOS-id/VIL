// =============================================================================
// VIL Error Code Registry — Legacy (replaced by error_catalog.rs)
// =============================================================================
#![allow(dead_code)]

use colored::*;

/// All known VIL error codes with explanations and fix suggestions.
pub struct ErrorEntry {
    pub code: &'static str,
    pub title: &'static str,
    pub explanation: &'static str,
    pub fix: &'static str,
    pub example: &'static str,
    pub doc_url: &'static str,
}

pub fn error_registry() -> Vec<ErrorEntry> {
    vec![
        ErrorEntry {
            code: "E-VIL-SEMANTIC-LANE-01",
            title: "Fault messages restricted to Control Lane",
            explanation: "Messages marked with #[vil_fault] can only be routed through the \
                Control Lane. Sending fault messages through the Data or Trigger lane \
                violates lane semantics and may cause head-of-line blocking.",
            fix: "Change the route to use the Control Lane, or change the message type \
                to #[vil_event] if it should flow through Data Lane.",
            example: r#"// Wrong: fault on Data Lane
routes: [ source.data_out -> sink.data_in (LoanWrite) ]  // if data_out carries #[vil_fault]

// Correct: fault on Control Lane
routes: [ source.ctrl_out -> sink.ctrl_in (Copy) ]"#,
            doc_url: "https://github.com/OceanOS-id/VIL/blob/main/docs/ARCHITECTURE_OVERVIEW.md#semantic-type-system--lane-classification",
        },
        ErrorEntry {
            code: "E-VIL-SEMANTIC-LANE-02",
            title: "Decision messages restricted to Trigger Lane",
            explanation: "Messages marked with #[vil_decision] can only be routed through the \
                Trigger Lane. Decision messages control routing logic and must arrive \
                before data processing begins.",
            fix: "Route decision messages through the Trigger Lane instead of Data or Control Lane.",
            example: r#"// Wrong: decision on Data Lane
routes: [ router.data_out -> worker.data_in (LoanWrite) ]

// Correct: decision on Trigger Lane
routes: [ router.trigger_out -> worker.trigger_in (LoanWrite) ]"#,
            doc_url: "https://github.com/OceanOS-id/VIL/blob/main/docs/ARCHITECTURE_OVERVIEW.md#semantic-type-system--lane-classification",
        },
        ErrorEntry {
            code: "E-VIL-TRANSFER-01",
            title: "ControlHeap message cannot use LoanWrite",
            explanation: "Messages with memory_class = ControlHeap are small control signals \
                designed for Copy transfer. LoanWrite requires PagedExchange or PinnedRemote \
                memory class for zero-copy SHM transfer.",
            fix: "Either change the transfer mode to Copy, or change the memory class \
                to PagedExchange.",
            example: r#"// Wrong: ControlHeap + LoanWrite
#[message(memory_class = ControlHeap)]
pub struct MySignal { ... }
routes: [ source.out -> sink.in (LoanWrite) ]

// Correct option 1: use Copy
routes: [ source.out -> sink.in (Copy) ]

// Correct option 2: use PagedExchange
#[message(memory_class = PagedExchange)]
pub struct MySignal { ... }"#,
            doc_url: "https://github.com/OceanOS-id/VIL/blob/main/docs/ARCHITECTURE_OVERVIEW.md#memory-class-semantics",
        },
        ErrorEntry {
            code: "E-VIL-TRANSFER-02",
            title: "Large message on Trigger Lane",
            explanation: "Messages larger than 64 bytes sent via Trigger Lane may cause \
                increased latency for session initiation. Trigger Lane is optimized for \
                small handoff signals.",
            fix: "Consider splitting: send a small trigger signal on Trigger Lane, \
                and the bulk payload on Data Lane with LoanWrite.",
            example: r#"// Warning: large struct on Trigger Lane
#[vil_state]
pub struct LargePayload { data: [u8; 4096] }
routes: [ source.trigger_out -> sink.trigger_in (Copy) ]

// Better: use Data Lane for bulk
routes: [
    source.trigger_out -> sink.trigger_in (Copy),      // small signal
    source.data_out -> sink.data_in (LoanWrite),       // bulk payload
]"#,
            doc_url: "https://github.com/OceanOS-id/VIL/blob/main/docs/VIL-Developer-Guide.md",
        },
        ErrorEntry {
            code: "E-VIL-TOPOLOGY-01",
            title: "Unresolved route target",
            explanation: "A route references a process or port that doesn't exist in the workflow. \
                This usually means a typo in the process name or port name.",
            fix: "Check that the process name and port name in the route match the \
                instances and their declared ports.",
            example: r#"// Wrong: typo in port name
routes: [ source.dta_out -> sink.data_in (LoanWrite) ]
                  ^^^^^^^  should be 'data_out'

// Correct:
routes: [ source.data_out -> sink.data_in (LoanWrite) ]"#,
            doc_url: "https://github.com/OceanOS-id/VIL/blob/main/docs/ARCHITECTURE_OVERVIEW.md#distributed-topology--host-aware-routes",
        },
        ErrorEntry {
            code: "E-VIL-TOPOLOGY-02",
            title: "Cross-host route without transport specification",
            explanation: "When routing between processes on different hosts, you must specify \
                the transport mechanism (e.g., RDMA, TCP). Without it, the runtime \
                cannot establish the network connection.",
            fix: "Add a transport specification to the cross-host route.",
            example: r#"// Wrong: cross-host without transport
instances: [ ingress @ node_a, processor @ node_b ]
routes: [ ingress.out -> processor.in (LoanWrite) ]

// Correct: specify transport
routes: [ ingress.out -> processor.in (LoanWrite, transport: RDMA) ]"#,
            doc_url: "https://github.com/OceanOS-id/VIL/blob/main/docs/ARCHITECTURE_OVERVIEW.md#distributed-topology--host-aware-routes",
        },
        ErrorEntry {
            code: "E-VIL-SHM-01",
            title: "Failed to initialize SHM runtime",
            explanation: "The VIL runtime could not create or attach to the shared memory region. \
                This typically happens when /dev/shm has insufficient space or incorrect permissions.",
            fix: "Check that /dev/shm has enough space and correct permissions.",
            example: r#"# Check /dev/shm space
df -h /dev/shm

# Increase if needed (Linux)
sudo mount -o remount,size=4G /dev/shm

# Check permissions
ls -la /dev/shm/vil_*"#,
            doc_url: "https://github.com/OceanOS-id/VIL/blob/main/docs/INSTALLATION.md",
        },
        ErrorEntry {
            code: "E-VIL-CAPSULE-01",
            title: "Capability violation in WASM Capsule",
            explanation: "A process running in WasmCapsule trust zone attempted to access \
                a capability it doesn't have (e.g., can_access_shm, can_use_secret). \
                WASM capsules are sandboxed with limited capabilities.",
            fix: "Either remove the offending capability usage from the WASM code, \
                or promote the process to NativeTrusted zone if it needs the capability.",
            example: r#"// WasmCapsule cannot access SHM directly
#[vil_process(zone = WasmCapsule)]
struct Plugin;
// Plugin cannot call: world.shm_stats()  // Capability denied

// If SHM access is needed:
#[vil_process(zone = NativeTrusted)]
struct Plugin;"#,
            doc_url: "https://github.com/OceanOS-id/VIL/blob/main/docs/ARCHITECTURE_OVERVIEW.md#trust-zone--capsule-system",
        },
    ]
}

/// Display a single error code explanation.
pub fn explain_error(code: &str) {
    let registry = error_registry();

    let found = registry.iter().find(|e| e.code.eq_ignore_ascii_case(code));

    match found {
        Some(entry) => {
            println!("{} {}", entry.code.red().bold(), entry.title.white().bold());
            println!();
            println!("{}", "Explanation:".yellow().bold());
            println!("  {}", entry.explanation);
            println!();
            println!("{}", "Fix:".green().bold());
            println!("  {}", entry.fix);
            println!();
            println!("{}", "Example:".cyan().bold());
            for line in entry.example.lines() {
                println!("  {}", line);
            }
            println!();
            println!("{} {}", "Docs:".blue().bold(), entry.doc_url);
        }
        None => {
            println!("{} Unknown error code: {}", "Error:".red().bold(), code);
            println!();
            println!("Available error codes:");
            for entry in &registry {
                println!("  {}  {}", entry.code.yellow(), entry.title);
            }
        }
    }
}

/// List all known error codes.
pub fn list_errors() {
    let registry = error_registry();
    println!("{}", "VIL Error Code Reference".green().bold());
    println!("{}", "=".repeat(60));
    println!();
    for entry in &registry {
        println!("  {}  {}", entry.code.yellow(), entry.title);
    }
    println!();
    println!("Use {} to see details for a specific error.",
        "vil explain <ERROR_CODE>".cyan());
}

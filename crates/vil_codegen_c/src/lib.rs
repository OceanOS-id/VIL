use vil_ir::{WorkflowIR, TypeRefIR, MessageIR};
use vil_types::SemanticKind;
use std::fmt::Write;

/// Convert CamelCase to snake_case for C-style naming.
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
        } else {
            result.push(ch);
        }
    }
    result
}

/// Generates a C header from VIL Workflow IR.
pub fn generate_header(ir: &WorkflowIR) -> String {
    let mut header = String::new();
    let guard = format!("__{}_H__", ir.name.to_uppercase());

    writeln!(header, "/*").unwrap();
    writeln!(header, " * VIL Generated Header: {}", ir.name).unwrap();
    writeln!(header, " * Canonical Semantic IR -> C99").unwrap();
    writeln!(header, " */").unwrap();
    writeln!(header, "#ifndef {}", guard).unwrap();
    writeln!(header, "#define {}", guard).unwrap();
    writeln!(header, "").unwrap();
    writeln!(header, "#include <stdint.h>").unwrap();
    writeln!(header, "#include <stdbool.h>").unwrap();
    writeln!(header, "").unwrap();

    // 1. Generate Constants for Ports
    writeln!(header, "/* Port ID Constants */").unwrap();
    for (_name, iface) in &ir.interfaces {
        for (port_name, _port) in &iface.ports {
            writeln!(header, "#define VIL_PORT_{} \"{}\"", port_name.to_uppercase(), port_name).unwrap();
        }
    }
    writeln!(header, "").unwrap();

    // 2. Group messages by SemanticKind and generate typed sections
    let mut messages: Vec<&MessageIR> = Vec::new();
    let mut states: Vec<&MessageIR> = Vec::new();
    let mut events: Vec<&MessageIR> = Vec::new();
    let mut faults: Vec<&MessageIR> = Vec::new();
    let mut decisions: Vec<&MessageIR> = Vec::new();

    for (_name, msg) in &ir.messages {
        match msg.semantic_kind {
            SemanticKind::Message => messages.push(msg),
            SemanticKind::State => states.push(msg),
            SemanticKind::Event => events.push(msg),
            SemanticKind::Fault => faults.push(msg),
            SemanticKind::Decision => decisions.push(msg),
        }
    }

    // 2a. Message types (general payload)
    if !messages.is_empty() {
        writeln!(header, "/* ── VIL Message Types ─────────────────────────── */").unwrap();
        for msg in &messages {
            writeln!(header, "/* semantic_kind: message */").unwrap();
            writeln!(header, "{}", generate_struct(msg)).unwrap();
        }
    }

    // 2b. State types
    if !states.is_empty() {
        writeln!(header, "/* ── VIL State Types ──────────────────────────── */").unwrap();
        for msg in &states {
            writeln!(header, "/* semantic_kind: state — mutable per-session, Data Lane only */").unwrap();
            writeln!(header, "{}", generate_struct(msg)).unwrap();
        }
    }

    // 2c. Event types
    if !events.is_empty() {
        writeln!(header, "/* ── VIL Event Types ──────────────────────────── */").unwrap();
        for msg in &events {
            writeln!(header, "/* semantic_kind: event — immutable log entry */").unwrap();
            writeln!(header, "{}", generate_struct(msg)).unwrap();
        }
    }

    // 2d. Fault types — represented as tagged unions in C
    if !faults.is_empty() {
        writeln!(header, "/* ── VIL Fault Types ──────────────────────────── */").unwrap();
        writeln!(header, "/* Note: Faults in VIL are enums — represented as tagged union in C */").unwrap();
        for msg in &faults {
            writeln!(header, "/* semantic_kind: fault — Control Lane only */").unwrap();
            writeln!(header, "{}", generate_fault_struct(msg)).unwrap();
        }
    }

    // 2e. Decision types
    if !decisions.is_empty() {
        writeln!(header, "/* ── VIL Decision Types ───────────────────────── */").unwrap();
        for msg in &decisions {
            writeln!(header, "/* semantic_kind: decision — Trigger Lane only */").unwrap();
            writeln!(header, "{}", generate_struct(msg)).unwrap();
        }
    }

    writeln!(header, "#endif /* {} */", guard).unwrap();

    header
}

/// Generate a fault type as a tagged union (discriminated union) in C.
fn generate_fault_struct(msg: &MessageIR) -> String {
    let mut s = String::new();
    writeln!(s, "typedef struct {{").unwrap();
    writeln!(s, "    uint32_t tag;  /* Variant discriminant */").unwrap();
    if !msg.fields.is_empty() {
        writeln!(s, "    union {{").unwrap();
        writeln!(s, "        struct {{").unwrap();
        for field in &msg.fields {
            let c_type = map_type(&field.ty);
            writeln!(s, "            {} {};", c_type, field.name).unwrap();
        }
        writeln!(s, "        }} data;").unwrap();
        writeln!(s, "    }} variants;").unwrap();
    }
    writeln!(s, "}} {}_t;", to_snake_case(&msg.name)).unwrap();
    s
}

fn generate_struct(msg: &MessageIR) -> String {
    let mut s = String::new();
    writeln!(s, "typedef struct {{").unwrap();
    for field in &msg.fields {
        let c_type = map_type(&field.ty);
        writeln!(s, "    {} {};", c_type, field.name).unwrap();
    }
    writeln!(s, "}} {}_t;", to_snake_case(&msg.name)).unwrap();
    s
}

fn map_type(ty: &TypeRefIR) -> String {
    match ty {
        TypeRefIR::Primitive(p) => match p.as_str() {
            "u8" => "uint8_t".to_string(),
            "u16" => "uint16_t".to_string(),
            "u32" => "uint32_t".to_string(),
            "u64" => "uint64_t".to_string(),
            "i8" => "int8_t".to_string(),
            "i16" => "int16_t".to_string(),
            "i32" => "int32_t".to_string(),
            "i64" => "int64_t".to_string(),
            "bool" => "bool".to_string(),
            "f32" => "float".to_string(),
            "f64" => "double".to_string(),
            _ => "void*".to_string(),
        },
        TypeRefIR::Named(n) => format!("{}_t", to_snake_case(n)),
        TypeRefIR::VSlice(_) => "void* /* VSlice pointer */".to_string(),
        TypeRefIR::VRef(_) => "void* /* VRef pointer */".to_string(),
        TypeRefIR::Unknown(_) => "void* /* Unknown type */".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vil_ir::builder::{WorkflowBuilder, MessageBuilder, InterfaceBuilder};
    use vil_types::QueueKind;

    #[test]
    fn test_c_header_generation() {
        let ir = WorkflowBuilder::new("TestSystem")
            .add_message(MessageBuilder::new("Frame")
                .add_field("id", TypeRefIR::Primitive("u64".to_string()))
                .add_field("active", TypeRefIR::Primitive("bool".to_string()))
                .build())
            .add_interface(InterfaceBuilder::new("Stream")
                .out_port("video", "Frame")
                .queue(QueueKind::Spsc, 1024)
                .done()
                .build())
            .build();

        let header = generate_header(&ir);
        println!("{}", header);

        assert!(header.contains("typedef struct {"));
        assert!(header.contains("uint64_t id;"));
        assert!(header.contains("bool active;"));
        assert!(header.contains("#define VIL_PORT_VIDEO \"video\""));
        // Default semantic_kind is Message
        assert!(header.contains("VIL Message Types"));
    }

    #[test]
    fn test_semantic_kind_grouping() {
        let ir = WorkflowBuilder::new("SemanticTest")
            .add_message(MessageBuilder::new("SessionProgress")
                .semantic_kind(SemanticKind::State)
                .add_field("session_id", TypeRefIR::Primitive("u64".to_string()))
                .add_field("progress", TypeRefIR::Primitive("u32".to_string()))
                .build())
            .add_message(MessageBuilder::new("AuditLog")
                .semantic_kind(SemanticKind::Event)
                .add_field("timestamp", TypeRefIR::Primitive("u64".to_string()))
                .add_field("level", TypeRefIR::Primitive("u8".to_string()))
                .build())
            .add_message(MessageBuilder::new("ProcessingError")
                .semantic_kind(SemanticKind::Fault)
                .add_field("error_code", TypeRefIR::Primitive("u32".to_string()))
                .add_field("retryable", TypeRefIR::Primitive("bool".to_string()))
                .build())
            .add_message(MessageBuilder::new("RoutingDecision")
                .semantic_kind(SemanticKind::Decision)
                .add_field("target_id", TypeRefIR::Primitive("u64".to_string()))
                .build())
            .add_message(MessageBuilder::new("DataPayload")
                .add_field("value", TypeRefIR::Primitive("f64".to_string()))
                .build())
            .build();

        let header = generate_header(&ir);
        println!("{}", header);

        // Check section headers
        assert!(header.contains("VIL Message Types"));
        assert!(header.contains("VIL State Types"));
        assert!(header.contains("VIL Event Types"));
        assert!(header.contains("VIL Fault Types"));
        assert!(header.contains("VIL Decision Types"));

        // Check semantic kind annotations
        assert!(header.contains("semantic_kind: state"));
        assert!(header.contains("semantic_kind: event"));
        assert!(header.contains("semantic_kind: fault"));
        assert!(header.contains("semantic_kind: decision"));
        assert!(header.contains("semantic_kind: message"));

        // Check fault is a tagged union
        assert!(header.contains("uint32_t tag;"));
        assert!(header.contains("Variant discriminant"));

        // Check struct names
        assert!(header.contains("session_progress_t;"));
        assert!(header.contains("audit_log_t;"));
        assert!(header.contains("processing_error_t;"));
        assert!(header.contains("routing_decision_t;"));
        assert!(header.contains("data_payload_t;"));
    }
}

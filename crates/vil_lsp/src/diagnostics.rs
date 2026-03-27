use tower_lsp::lsp_types::*;
use crate::parser::{VilUsage, UsageKind};

/// Generate diagnostics for a source file.
pub fn diagnose(source: &str, usages: &[VilUsage]) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    for usage in usages {
        match &usage.kind {
            UsageKind::SemanticMacro(name) => {
                // Check if Serialize/Deserialize is present near the macro
                check_semantic_derives(source, usage, name, &mut diags);
            }
            UsageKind::ExecClass(variant) => {
                check_exec_class(source, usage, variant, &mut diags);
            }
            _ => {}
        }
    }

    // Check for duplicate endpoint paths
    check_duplicate_endpoints(usages, &mut diags);

    diags
}

fn check_semantic_derives(source: &str, usage: &VilUsage, macro_name: &str, diags: &mut Vec<Diagnostic>) {
    // Look at the next few lines for derive macros
    let lines: Vec<&str> = source.lines().collect();
    let start = usage.line as usize;
    let end = (start + 5).min(lines.len());
    let context = lines[start..end].join("\n");

    // Check for common missing derives
    if !context.contains("Serialize") || !context.contains("Deserialize") {
        // Only warn for VilModel derives, semantic macros add their own derives
        if context.contains("VilModel") {
            diags.push(Diagnostic {
                range: Range {
                    start: Position { line: usage.line, character: usage.col },
                    end: Position { line: usage.line, character: usage.col + macro_name.len() as u32 },
                },
                severity: Some(DiagnosticSeverity::WARNING),
                source: Some("vil-lsp".into()),
                message: format!(
                    "#[{}] with VilModel requires Serialize and Deserialize derives",
                    macro_name
                ),
                ..Default::default()
            });
        }
    }
}

fn check_exec_class(_source: &str, usage: &VilUsage, variant: &str, diags: &mut Vec<Diagnostic>) {
    let valid = ["AsyncTask", "BlockingTask", "DedicatedThread", "PinnedWorker", "WasmFaaS", "SidecarProcess"];
    if !valid.contains(&variant) {
        diags.push(Diagnostic {
            range: Range {
                start: Position { line: usage.line, character: usage.col },
                end: Position { line: usage.line, character: usage.col + variant.len() as u32 },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            source: Some("vil-lsp".into()),
            message: format!(
                "Unknown ExecClass variant '{}'. Valid: {:?}",
                variant, valid
            ),
            ..Default::default()
        });
    }
}

fn check_duplicate_endpoints(usages: &[VilUsage], diags: &mut Vec<Diagnostic>) {
    let endpoints: Vec<_> = usages.iter()
        .filter(|u| u.kind == UsageKind::EndpointDef)
        .collect();

    for (i, ep1) in endpoints.iter().enumerate() {
        for ep2 in endpoints.iter().skip(i + 1) {
            if ep1.text == ep2.text {
                diags.push(Diagnostic {
                    range: Range {
                        start: Position { line: ep2.line, character: ep2.col },
                        end: Position { line: ep2.line, character: ep2.col + ep2.text.len() as u32 },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some("vil-lsp".into()),
                    message: format!("Duplicate endpoint: {} (first defined at line {})", ep2.text, ep1.line + 1),
                    ..Default::default()
                });
            }
        }
    }
}

use regex::Regex;
use std::sync::LazyLock;

/// Detected VIL macro usage in a source file.
#[derive(Debug, Clone)]
pub struct VilUsage {
    pub line: u32,
    pub col: u32,
    pub kind: UsageKind,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UsageKind {
    SemanticMacro(String), // vil_state, vil_event, vil_fault, vil_decision
    VilApp,
    ServiceProcess,
    ExecClass(String),
    WasmFaaSConfig,
    SidecarConfig,
    VilModel,
    VilError,
    EndpointDef,
}

static RE_SEMANTIC: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"#\[(vil_state|vil_event|vil_fault|vil_decision)\]").unwrap());

static RE_VIL_APP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"VilApp::new\(\s*"([^"]*)"\s*\)"#).unwrap());

static RE_SERVICE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"ServiceProcess::new\(\s*"([^"]*)"\s*\)"#).unwrap());

static RE_EXEC_CLASS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"ExecClass::(\w+)").unwrap());

static RE_DERIVE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"#\[derive\([^)]*\b(VilModel|VilError)\b[^)]*\)\]").unwrap());

static RE_ENDPOINT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\.endpoint\(\s*Method::(\w+)\s*,\s*"([^"]*)"\s*,"#).unwrap());

static RE_SIDECAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"SidecarConfig::new\(\s*"([^"]*)"\s*\)"#).unwrap());

static RE_WASM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"WasmFaaSConfig::new\(\s*"([^"]*)"\s*,"#).unwrap());

/// Parse a source file and return all detected VIL usages.
pub fn parse_vil_usages(source: &str) -> Vec<VilUsage> {
    let mut usages = Vec::new();

    for (line_num, line) in source.lines().enumerate() {
        let line_num = line_num as u32;

        // Semantic macros
        for cap in RE_SEMANTIC.captures_iter(line) {
            if let Some(m) = cap.get(1) {
                usages.push(VilUsage {
                    line: line_num,
                    col: m.start() as u32,
                    kind: UsageKind::SemanticMacro(m.as_str().to_string()),
                    text: m.as_str().to_string(),
                });
            }
        }

        // VilApp
        for cap in RE_VIL_APP.captures_iter(line) {
            usages.push(VilUsage {
                line: line_num,
                col: cap.get(0).map(|m| m.start() as u32).unwrap_or(0),
                kind: UsageKind::VilApp,
                text: cap
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default(),
            });
        }

        // ServiceProcess
        for cap in RE_SERVICE.captures_iter(line) {
            usages.push(VilUsage {
                line: line_num,
                col: cap.get(0).map(|m| m.start() as u32).unwrap_or(0),
                kind: UsageKind::ServiceProcess,
                text: cap
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default(),
            });
        }

        // ExecClass
        for cap in RE_EXEC_CLASS.captures_iter(line) {
            if let Some(m) = cap.get(1) {
                usages.push(VilUsage {
                    line: line_num,
                    col: m.start() as u32,
                    kind: UsageKind::ExecClass(m.as_str().to_string()),
                    text: m.as_str().to_string(),
                });
            }
        }

        // Derives
        for cap in RE_DERIVE.captures_iter(line) {
            if let Some(m) = cap.get(1) {
                let kind = if m.as_str() == "VilModel" {
                    UsageKind::VilModel
                } else {
                    UsageKind::VilError
                };
                usages.push(VilUsage {
                    line: line_num,
                    col: m.start() as u32,
                    kind,
                    text: m.as_str().to_string(),
                });
            }
        }

        // Endpoints
        for cap in RE_ENDPOINT.captures_iter(line) {
            usages.push(VilUsage {
                line: line_num,
                col: cap.get(0).map(|m| m.start() as u32).unwrap_or(0),
                kind: UsageKind::EndpointDef,
                text: format!(
                    "{} {}",
                    cap.get(1).map(|m| m.as_str()).unwrap_or("?"),
                    cap.get(2).map(|m| m.as_str()).unwrap_or("?")
                ),
            });
        }

        // Sidecar
        for cap in RE_SIDECAR.captures_iter(line) {
            usages.push(VilUsage {
                line: line_num,
                col: cap.get(0).map(|m| m.start() as u32).unwrap_or(0),
                kind: UsageKind::SidecarConfig,
                text: cap
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default(),
            });
        }

        // WASM
        for cap in RE_WASM.captures_iter(line) {
            usages.push(VilUsage {
                line: line_num,
                col: cap.get(0).map(|m| m.start() as u32).unwrap_or(0),
                kind: UsageKind::WasmFaaSConfig,
                text: cap
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default(),
            });
        }
    }

    usages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_semantic_macros() {
        let src = r#"
#[vil_state]
struct AppState { count: u32 }

#[vil_event]
struct OrderPlaced { id: u64 }
"#;
        let usages = parse_vil_usages(src);
        let semantics: Vec<_> = usages
            .iter()
            .filter(|u| matches!(&u.kind, UsageKind::SemanticMacro(_)))
            .collect();
        assert_eq!(semantics.len(), 2);
        assert_eq!(semantics[0].text, "vil_state");
        assert_eq!(semantics[1].text, "vil_event");
    }

    #[test]
    fn test_detect_vil_app() {
        let src = r#"VilApp::new("my-service").port(8080).run().await;"#;
        let usages = parse_vil_usages(src);
        assert!(usages
            .iter()
            .any(|u| u.kind == UsageKind::VilApp && u.text == "my-service"));
    }

    #[test]
    fn test_detect_endpoints() {
        let src = r#".endpoint(Method::GET, "/api/users", get(list_users))"#;
        let usages = parse_vil_usages(src);
        assert!(usages
            .iter()
            .any(|u| u.kind == UsageKind::EndpointDef && u.text == "GET /api/users"));
    }

    #[test]
    fn test_detect_exec_class() {
        let src = r#"ExecClass::WasmFaaS"#;
        let usages = parse_vil_usages(src);
        assert!(usages
            .iter()
            .any(|u| matches!(&u.kind, UsageKind::ExecClass(s) if s == "WasmFaaS")));
    }
}

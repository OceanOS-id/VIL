//! Codegen — generate VWFD YAML string + Rust module from parsed macro def.

use crate::parser::*;
use proc_macro2::TokenStream;
use quote::{quote, format_ident};

pub fn generate(def: &VwfdMacroDef) -> TokenStream {
    let yaml = generate_yaml(def);
    let mod_name = format_ident!("{}", def.id.replace('-', "_"));
    let id_str = &def.id;
    let route_str = &def.trigger.route;
    let trigger_type = &def.trigger.trigger_type;
    let node_count = def.activities.len() + 2; // + trigger + end

    quote! {
        pub mod #mod_name {
            /// VWFD YAML source — extractable via `vil export --vwfd`.
            pub const VWFD_YAML: &str = #yaml;

            /// Workflow metadata.
            pub fn metadata() -> vil_vwfd::handler::WorkflowMeta {
                vil_vwfd::handler::WorkflowMeta {
                    id: #id_str,
                    route: #route_str,
                    trigger_type: #trigger_type,
                    node_count: #node_count,
                }
            }

            /// Compile and return VilwGraph (call at startup).
            pub fn compile() -> Result<vil_vwfd::VilwGraph, String> {
                vil_vwfd::load_yaml(VWFD_YAML)
            }

            /// Register into WorkflowRegistry.
            pub fn register(registry: &mut vil_vwfd::WorkflowRegistry) {
                match compile() {
                    Ok(graph) => registry.register(graph),
                    Err(e) => eprintln!("Failed to compile workflow '{}': {}", #id_str, e),
                }
            }
        }
    }
}

fn generate_yaml(def: &VwfdMacroDef) -> String {
    let mut yaml = String::new();

    yaml.push_str("version: \"3.0\"\n");
    yaml.push_str("metadata:\n");
    yaml.push_str(&format!("  id: {}\n", def.id));
    if let Some(ref name) = def.name {
        yaml.push_str(&format!("  name: \"{}\"\n", name));
    }

    yaml.push_str("spec:\n");

    // Durability
    if let Some(ref dur) = def.durability {
        yaml.push_str("  durability:\n");
        yaml.push_str("    enabled: true\n");
        yaml.push_str(&format!("    default_mode: {}\n", dur));
    }

    // Activities
    yaml.push_str("  activities:\n");

    // Trigger activity
    yaml.push_str("    - id: trigger\n");
    yaml.push_str("      activity_type: Trigger\n");
    yaml.push_str("      trigger_config:\n");
    yaml.push_str(&format!("        trigger_type: {}\n", def.trigger.trigger_type));
    if !def.trigger.route.is_empty() {
        yaml.push_str(&format!("        route: {}\n", def.trigger.route));
        yaml.push_str("        webhook_config:\n");
        yaml.push_str(&format!("          path: {}\n", def.trigger.route));
    }
    if let Some(ref mode) = def.trigger.response_mode {
        yaml.push_str(&format!("        response_mode: {}\n", mode));
    }
    if let Some(ref ea) = def.trigger.end_activity {
        yaml.push_str(&format!("        end_activity: {}\n", ea));
    }
    if let Some(ref method) = def.trigger.method {
        yaml.push_str(&format!("          method: {}\n", method));
    }
    if let Some(ref cron) = def.trigger.cron_expr {
        yaml.push_str("        cron:\n");
        yaml.push_str(&format!("          expression: \"{}\"\n", cron));
    }
    yaml.push_str("      output_variable: trigger_payload\n");

    // User activities
    for act in &def.activities {
        yaml.push_str(&format!("    - id: {}\n", act.id));

        match &act.kind {
            ActivityKind::Connector => {
                yaml.push_str("      activity_type: Connector\n");
                yaml.push_str("      connector_config:\n");
                if let Some(ref cref) = act.connector_ref {
                    yaml.push_str(&format!("        connector_ref: {}\n", cref));
                }
                if let Some(ref op) = act.operation {
                    yaml.push_str(&format!("        operation: {}\n", op));
                }
                if !act.mappings.is_empty() {
                    yaml.push_str("      input_mappings:\n");
                    for m in &act.mappings {
                        yaml.push_str(&format!("        - target: {}\n", m.target));
                        yaml.push_str("          source:\n");
                        yaml.push_str(&format!("            language: \"{}\"\n", m.language));
                        yaml.push_str(&format!("            source: '{}'\n", escape_yaml_single(&m.source)));
                    }
                }
            }
            ActivityKind::EndTrigger => {
                yaml.push_str("      activity_type: EndTrigger\n");
                yaml.push_str("      end_trigger_config:\n");
                if let Some(ref tref) = act.trigger_ref {
                    yaml.push_str(&format!("        trigger_ref: {}\n", tref));
                }
                if let Some((ref lang, ref src)) = act.response_expr {
                    yaml.push_str("        final_response:\n");
                    yaml.push_str(&format!("          language: {}\n", lang));
                    yaml.push_str(&format!("          source: '{}'\n", escape_yaml_single(src)));
                }
            }
            ActivityKind::Transform => {
                yaml.push_str("      activity_type: transform\n");
                if !act.mappings.is_empty() {
                    yaml.push_str("      input_mappings:\n");
                    for m in &act.mappings {
                        yaml.push_str(&format!("        - target: {}\n", m.target));
                        yaml.push_str("          source:\n");
                        yaml.push_str(&format!("            language: \"{}\"\n", m.language));
                        yaml.push_str(&format!("            source: '{}'\n", escape_yaml_single(&m.source)));
                    }
                }
            }
            ActivityKind::VilRules(ref rule_set) => {
                yaml.push_str("      activity_type: Rule\n");
                yaml.push_str("      rule_config:\n");
                yaml.push_str(&format!("        rule_set_id: {}\n", rule_set));
            }
        }

        if let Some(ref out) = act.output {
            yaml.push_str(&format!("      output_variable: {}\n", out));
        }
        if let Some(ref dur) = act.durability {
            yaml.push_str(&format!("      durability: {}\n", dur));
        }
        if let Some(ref comp) = act.compensation {
            yaml.push_str("      compensation:\n");
            yaml.push_str(&format!("        connector_ref: {}\n", comp.connector_ref));
            yaml.push_str(&format!("        operation: {}\n", comp.operation));
        }
    }

    // End activity
    yaml.push_str("    - id: end\n");
    yaml.push_str("      activity_type: End\n");

    // Flows (from flow chain)
    yaml.push_str("  flows:\n");
    for (i, window) in def.flow.windows(2).enumerate() {
        yaml.push_str(&format!("    - id: f{}\n", i + 1));
        yaml.push_str(&format!("      from: {{ node: {} }}\n", window[0]));
        yaml.push_str(&format!("      to: {{ node: {} }}\n", window[1]));
    }

    // Variables
    yaml.push_str("  variables:\n");
    yaml.push_str("    - { name: trigger_payload, type: object }\n");
    for act in &def.activities {
        if let Some(ref out) = act.output {
            yaml.push_str(&format!("    - {{ name: {}, type: object }}\n", out));
        }
    }

    yaml
}

fn escape_yaml_single(s: &str) -> String {
    s.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::*;

    fn sample_def() -> VwfdMacroDef {
        VwfdMacroDef {
            id: "test-wf".into(),
            name: Some("Test Workflow".into()),
            trigger: TriggerDef {
                trigger_type: "webhook".into(),
                route: "/test".into(),
                method: Some("POST".into()),
                response_mode: Some("buffered".into()),
                end_activity: Some("respond".into()),
                cron_expr: None,
            },
            activities: vec![
                ActivityDef {
                    id: "step1".into(),
                    kind: ActivityKind::Connector,
                    connector_ref: Some("vastar.http".into()),
                    operation: Some("post".into()),
                    mappings: vec![
                        MappingDef { target: "url".into(), language: "literal".into(), source: "http://example.com".into() },
                        MappingDef { target: "body".into(), language: "vil-expr".into(), source: "trigger_payload".into() },
                    ],
                    output: Some("result".into()),
                    durability: None,
                    compensation: None,
                    response_expr: None,
                    trigger_ref: None,
                },
                ActivityDef {
                    id: "respond".into(),
                    kind: ActivityKind::EndTrigger,
                    connector_ref: None,
                    operation: None,
                    mappings: Vec::new(),
                    output: None,
                    durability: None,
                    compensation: None,
                    response_expr: Some(("vil-expr".into(), r#"{"result": result}"#.into())),
                    trigger_ref: Some("trigger".into()),
                },
            ],
            flow: vec!["trigger".into(), "step1".into(), "respond".into(), "end".into()],
            durability: Some("eventual".into()),
        }
    }

    #[test]
    fn test_yaml_generation() {
        let def = sample_def();
        let yaml = generate_yaml(&def);
        
        assert!(yaml.contains("id: test-wf"));
        assert!(yaml.contains("name: \"Test Workflow\""));
        assert!(yaml.contains("trigger_type: webhook"));
        assert!(yaml.contains("route: /test"));
        assert!(yaml.contains("connector_ref: vastar.http"));
        assert!(yaml.contains("operation: post"));
        assert!(yaml.contains("language: \"literal\""));
        assert!(yaml.contains("language: \"vil-expr\""));
        assert!(yaml.contains("activity_type: EndTrigger"));
        assert!(yaml.contains("trigger_ref: trigger"));
        assert!(yaml.contains("output_variable: result"));
        assert!(yaml.contains("default_mode: eventual"));
    }

    #[test]
    fn test_yaml_contains_flows() {
        let def = sample_def();
        let yaml = generate_yaml(&def);
        
        assert!(yaml.contains("from: { node: trigger }"));
        assert!(yaml.contains("to: { node: step1 }"));
        assert!(yaml.contains("from: { node: step1 }"));
        assert!(yaml.contains("to: { node: respond }"));
        assert!(yaml.contains("from: { node: respond }"));
        assert!(yaml.contains("to: { node: end }"));
    }

    #[test]
    fn test_yaml_contains_variables() {
        let def = sample_def();
        let yaml = generate_yaml(&def);
        
        assert!(yaml.contains("name: trigger_payload"));
        assert!(yaml.contains("name: result"));
    }

    // YAML parse test done in integration tests (Phase 10)
    // since proc-macro crates can't dep on serde_yaml
}

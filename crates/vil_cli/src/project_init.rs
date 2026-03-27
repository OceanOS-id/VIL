//! vil init — project initializer with templates and wizard.
//!
//! Generates: app.vil.yaml + src/main.rs + Cargo.toml + handlers/ + README.md
//!
//! Two modes:
//!   vil init my-app --template ai-gateway --port 3080    (arguments)
//!   vil init                                              (interactive wizard)

use crate::codegen;
use crate::manifest::WorkflowManifest;
use colored::*;
use std::io::{self, Write};
use std::path::Path;

pub struct InitArgs {
    pub name: Option<String>,
    pub template: Option<String>,
    pub token: Option<String>,
    pub port: Option<u16>,
    pub upstream: Option<String>,
    pub wizard: bool,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Templates
// ═══════════════════════════════════════════════════════════════════════════════

struct Template {
    id: &'static str,
    title: &'static str,
    description: &'static str,
    default_port: u16,
    default_upstream: &'static str,
    yaml: fn(&ProjectConfig) -> String,
    has_handler: bool,
    handler_name: &'static str,
}

struct ProjectConfig {
    name: String,
    port: u16,
    upstream: String,
    token: String,
}

const TEMPLATES: &[Template] = &[
    Template {
        id: "ai-gateway",
        title: "AI Gateway",
        description: "SSE streaming pipeline (webhook -> upstream SSE -> streaming response)",
        default_port: 3080,
        default_upstream: "http://localhost:18081/api/v1/credits/stream",
        yaml: yaml_ai_gateway,
        has_handler: false,
        handler_name: "",
    },
    Template {
        id: "rest-crud",
        title: "REST CRUD API",
        description: "REST API with GET/POST/PUT/DELETE endpoints",
        default_port: 8080,
        default_upstream: "",
        yaml: yaml_rest_crud,
        has_handler: true,
        handler_name: "handle_request",
    },
    Template {
        id: "multi-model-router",
        title: "Multi-Model Router",
        description: "Route requests to different upstream providers",
        default_port: 3080,
        default_upstream: "http://localhost:18081/api/v1/credits/stream",
        yaml: yaml_multi_model_router,
        has_handler: true,
        handler_name: "route_by_model",
    },
    Template {
        id: "rag-pipeline",
        title: "RAG Pipeline",
        description: "Retrieval-Augmented Generation: embed -> search -> generate",
        default_port: 3080,
        default_upstream: "http://localhost:18081/api/v1/credits/stream",
        yaml: yaml_rag_pipeline,
        has_handler: true,
        handler_name: "rag_query",
    },
    Template {
        id: "websocket-chat",
        title: "WebSocket Chat",
        description: "WebSocket broadcast chat room with fan-out",
        default_port: 8080,
        default_upstream: "",
        yaml: yaml_websocket_chat,
        has_handler: false,
        handler_name: "",
    },
    Template {
        id: "wasm-faas",
        title: "WASM FaaS",
        description: "WebAssembly functions with pre-warmed instance pool",
        default_port: 8080,
        default_upstream: "",
        yaml: yaml_wasm_faas,
        has_handler: false,
        handler_name: "",
    },
    Template {
        id: "agent",
        title: "AI Agent",
        description: "ReAct agent with tool calling (calculator, HTTP fetch, retrieval)",
        default_port: 8080,
        default_upstream: "http://localhost:18081/api/v1/credits/stream",
        yaml: yaml_agent,
        has_handler: true,
        handler_name: "agent_loop",
    },
    Template {
        id: "blank",
        title: "Blank Project",
        description: "Empty YAML skeleton — start from scratch",
        default_port: 8080,
        default_upstream: "",
        yaml: yaml_blank,
        has_handler: false,
        handler_name: "",
    },
];

// ═══════════════════════════════════════════════════════════════════════════════
// Entry point
// ═══════════════════════════════════════════════════════════════════════════════

pub fn run_init(args: InitArgs) -> Result<(), String> {
    println!("{}", "VIL Project Initializer".cyan().bold());
    println!();

    let (name, template_id, token, port, upstream) = if args.wizard {
        run_wizard(&args)?
    } else {
        let name = args
            .name
            .ok_or("Project name is required. Usage: vil init <name> --template <template>")?;
        let tmpl = args.template.unwrap_or("ai-gateway".into());
        let template = find_template(&tmpl)?;
        let token = args.token.unwrap_or("shm".into());
        let port = args.port.unwrap_or(template.default_port);
        let upstream = args.upstream.unwrap_or(template.default_upstream.into());
        (name, tmpl, token, port, upstream)
    };

    let template = find_template(&template_id)?;
    let project_dir = Path::new(&name);
    let project_name = project_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&name)
        .to_string();
    let config = ProjectConfig {
        name: project_name.clone(),
        port,
        upstream: upstream.clone(),
        token: token.clone(),
    };
    if project_dir.exists() {
        println!();
        println!(
            "  {} Directory '{}' already exists.",
            "WARN".yellow().bold(),
            name
        );
        println!("    1. {} — delete and recreate", "Replace".green());
        println!(
            "    2. {} — keep existing, rename new to {}-2",
            "Rename".green(),
            project_name
        );
        println!("    3. {} — abort", "Cancel".green());
        let choice = prompt("Choice", "1")?;
        match choice.as_str() {
            "1" | "replace" => {
                std::fs::remove_dir_all(project_dir)
                    .map_err(|e| format!("Failed to remove '{}': {}", name, e))?;
                println!("  {} Removed old directory", "OK".green());
            }
            "2" | "rename" => {
                // Find next available name
                let mut suffix = 2;
                let mut new_name = format!("{}-{}", name, suffix);
                while std::path::Path::new(&new_name).exists() {
                    suffix += 1;
                    new_name = format!("{}-{}", name, suffix);
                }
                // Update name and project_dir for the rest of the function
                let name = new_name;
                let project_dir = std::path::Path::new(&name);
                let project_name = project_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&name)
                    .to_string();
                println!("  {} Using '{}'", "OK".green(), name);
                // Re-create config with new name
                let config = ProjectConfig {
                    name: project_name,
                    port,
                    upstream: upstream.clone(),
                    token: token.clone(),
                };
                std::fs::create_dir_all(project_dir.join("src/handlers"))
                    .map_err(|e| format!("Failed to create directory: {}", e))?;
                println!("  {} Creating project: {}", "DIR".green(), name);
                return generate_project(project_dir, &config, template);
            }
            _ => {
                println!("  Aborted.");
                return Ok(());
            }
        }
    }
    std::fs::create_dir_all(project_dir.join("src/handlers"))
        .map_err(|e| format!("Failed to create directory: {}", e))?;
    println!("  {} Creating project: {}", "DIR".green(), project_name);

    generate_project(project_dir, &config, template)
}

fn generate_project(
    project_dir: &Path,
    config: &ProjectConfig,
    template: &Template,
) -> Result<(), String> {
    // 1. Generate YAML manifest
    let yaml_content = (template.yaml)(config);
    let yaml_path = project_dir.join("app.vil.yaml");
    std::fs::write(&yaml_path, &yaml_content)
        .map_err(|e| format!("Failed to write YAML: {}", e))?;
    println!("  {} {}", "YAML".green(), yaml_path.display());

    // 2. Generate Rust source from YAML
    let manifest = WorkflowManifest::from_yaml(&yaml_content)?;

    let crate_prefix = if crate::sdk_manager::is_sdk_installed() {
        crate::sdk_manager::sdk_current_path()
            .join("internal")
            .to_string_lossy()
            .to_string()
    } else {
        let ws = find_workspace_root_for_init();
        format!("{}/crates", ws)
    };

    let rust_source = if manifest.is_workflow() {
        codegen::generate_workflow_rust(&manifest)
    } else {
        codegen::generate_rust(&manifest)
    };

    let cargo_toml = if manifest.is_workflow() {
        codegen::generate_workflow_cargo_toml(&manifest, &crate_prefix)
    } else {
        codegen::generate_cargo_toml(&manifest, &crate_prefix)
    };

    std::fs::write(project_dir.join("src/main.rs"), &rust_source)
        .map_err(|e| format!("Failed to write main.rs: {}", e))?;
    println!(
        "  {} src/main.rs (auto-generated from YAML)",
        "RUST".green()
    );

    std::fs::write(project_dir.join("Cargo.toml"), &cargo_toml)
        .map_err(|e| format!("Failed to write Cargo.toml: {}", e))?;
    println!("  {} Cargo.toml", "TOML".green());

    // 3. Generate handler stubs
    if template.has_handler && !template.handler_name.is_empty() {
        let handler_content = generate_handler_stub(template.handler_name, config);
        let handler_path = project_dir.join(format!("src/handlers/{}.rs", template.handler_name));
        std::fs::write(&handler_path, &handler_content)
            .map_err(|e| format!("Failed to write handler: {}", e))?;
        std::fs::write(
            project_dir.join("src/handlers/mod.rs"),
            format!("pub mod {};", template.handler_name),
        )
        .map_err(|e| format!("Failed to write mod.rs: {}", e))?;
        println!(
            "  {} src/handlers/{}.rs",
            "HANDLER".green(),
            template.handler_name
        );
    }

    // 4. Generate README
    let readme = generate_readme(config, template);
    std::fs::write(project_dir.join("README.md"), &readme)
        .map_err(|e| format!("Failed to write README: {}", e))?;
    println!("  {} README.md", "DOC".green());

    // 5. Generate .gitignore
    std::fs::write(
        project_dir.join(".gitignore"),
        "target/\n*.wasm\nwasm-out/\n",
    )
    .map_err(|e| format!("Failed to write .gitignore: {}", e))?;

    // Summary
    println!();
    println!(
        "{} Project '{}' created!",
        "DONE".green().bold(),
        config.name
    );
    println!();
    println!("  Next steps:");
    println!("    cd {}", config.name);
    println!("    vil viz app.vil.yaml --open           # visualize");
    println!("    vil check app.vil.yaml                # validate");
    println!("    vil compile --from yaml --input app.vil.yaml --release  # build");
    println!("    vil run --file app.vil.yaml           # run");

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Wizard
// ═══════════════════════════════════════════════════════════════════════════════

fn run_wizard(args: &InitArgs) -> Result<(String, String, String, u16, String), String> {
    // Project name
    let name = if let Some(n) = &args.name {
        n.clone()
    } else {
        prompt("Project name", "my-vil-app")?
    };

    // Template selection
    println!();
    println!("  {} Available templates:", "TEMPLATES".cyan());
    for (i, t) in TEMPLATES.iter().enumerate() {
        println!("    {}. {:25} {}", i + 1, t.title.green(), t.description);
    }
    println!();
    let tmpl_input = prompt("Template (number or name)", "1")?;
    let template_id = resolve_template(&tmpl_input)?;

    let template = find_template(&template_id)?;

    // Token type
    println!();
    println!("  {} Token types:", "TOKEN".cyan());
    println!(
        "    1. {} — multi-pipeline, zero-copy SHM (recommended)",
        "shm".green()
    );
    println!("    2. {} — single pipeline, simpler", "generic".green());
    let token_input = prompt("Token", "shm")?;
    let token = if token_input == "2" || token_input == "generic" {
        "generic".into()
    } else {
        "shm".into()
    };

    // Port
    let port_str = prompt(&format!("Port"), &template.default_port.to_string())?;
    let port: u16 = port_str.parse().unwrap_or(template.default_port);

    // Upstream (only for pipeline templates)
    let upstream = if !template.default_upstream.is_empty() {
        prompt("Upstream URL", template.default_upstream)?
    } else {
        String::new()
    };

    Ok((name, template_id, token, port, upstream))
}

fn prompt(label: &str, default: &str) -> Result<String, String> {
    print!("  ? {} [{}]: ", label, default.dimmed());
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| e.to_string())?;
    let trimmed = input.trim();
    if trimmed.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

fn find_template(id: &str) -> Result<&'static Template, String> {
    TEMPLATES.iter().find(|t| t.id == id).ok_or_else(|| {
        format!(
            "Unknown template '{}'. Available: {}",
            id,
            TEMPLATES
                .iter()
                .map(|t| t.id)
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

fn resolve_template(input: &str) -> Result<String, String> {
    // Try as number
    if let Ok(n) = input.parse::<usize>() {
        if n >= 1 && n <= TEMPLATES.len() {
            return Ok(TEMPLATES[n - 1].id.to_string());
        }
    }
    // Try as name
    if TEMPLATES.iter().any(|t| t.id == input) {
        return Ok(input.to_string());
    }
    Err(format!("Invalid template: '{}'", input))
}

fn find_workspace_root_for_init() -> String {
    // Walk up to find Cargo.toml with [workspace]
    let mut dir = std::env::current_dir().unwrap_or_default();
    for _ in 0..5 {
        if dir.join("Cargo.toml").exists() {
            let content = std::fs::read_to_string(dir.join("Cargo.toml")).unwrap_or_default();
            if content.contains("[workspace]") {
                return dir.to_string_lossy().to_string();
            }
        }
        if !dir.pop() {
            break;
        }
    }
    ".".to_string()
}

// ═══════════════════════════════════════════════════════════════════════════════
// YAML Template Generators
// ═══════════════════════════════════════════════════════════════════════════════

fn yaml_ai_gateway(c: &ProjectConfig) -> String {
    format!(
        r#"# {name} — AI Gateway Pipeline
# Generated by: vil init {name} --template ai-gateway
#
# Build:  vil compile --from yaml --input app.vil.yaml --release
# Run:    vil run --file app.vil.yaml
# Viz:    vil viz app.vil.yaml --open

vil_version: "6.0.0"
name: {name}
port: {port}
token: {token}

semantic_types:
  - name: InferenceState
    kind: state
    fields:
      - {{ name: request_id, type: u64 }}
      - {{ name: tokens_received, type: u32 }}
      - {{ name: latency_us, type: u64 }}
      - {{ name: stream_active, type: bool }}

  - name: InferenceCompleted
    kind: event
    fields:
      - {{ name: request_id, type: u64 }}
      - {{ name: total_tokens, type: u32 }}
      - {{ name: duration_us, type: u64 }}
      - {{ name: status_code, type: u16 }}

  - name: InferenceFault
    kind: fault
    variants:
      - UpstreamTimeout
      - SseParseError
      - ShmWriteFailed
      - ConnectionRefused

nodes:
  webhook:
    type: http-sink
    port: {port}
    path: /trigger
    ports:
      trigger_out:      {{ direction: out, lane: trigger }}
      response_data_in: {{ direction: in,  lane: data }}
      response_ctrl_in: {{ direction: in,  lane: control }}

  inference:
    type: http-source
    url: \"{upstream}\"
    format: sse
    # ── SSE Dialect ─────────────────────────────────────────────────────
    # Determines how the SSE stream is parsed (done marker + json path).
    #
    #   openai     — done: \"data: [DONE]\"              tap: choices[0].delta.content
    #   anthropic  — done: \"event: message_stop\"       tap: delta.text
    #   ollama     — done: {{\"done\": true}} in JSON      tap: message.content
    #   cohere     — done: \"event: message-end\"         tap: text
    #   gemini     — done: TCP EOF                      tap: candidates[0].content.parts[0].text
    #   standard   — done: TCP EOF                      tap: (none, raw data)
    #   custom     — provide your own termination config:
    #                  dialect: custom
    #                  dialect_done_marker: \"data: [END]\"       # string in SSE data field
    #                  dialect_done_event: \"stream_end\"         # SSE event name
    #                  dialect_done_json: \"status=complete\"     # JSON field=value
    #
    dialect: standard
    ports:
      trigger_in:        {{ direction: in,  lane: trigger }}
      response_data_out: {{ direction: out, lane: data }}
      response_ctrl_out: {{ direction: out, lane: control }}

routes:
  - {{ from: webhook.trigger_out, to: inference.trigger_in, mode: LoanWrite }}
  - {{ from: inference.response_data_out, to: webhook.response_data_in, mode: LoanWrite }}
  - {{ from: inference.response_ctrl_out, to: webhook.response_ctrl_in, mode: Copy }}
"#,
        name = c.name,
        port = c.port,
        token = c.token,
        upstream = c.upstream
    )
}

fn yaml_rest_crud(c: &ProjectConfig) -> String {
    format!(
        r#"# {name} — REST CRUD API
# Generated by: vil init {name} --template rest-crud

vil_version: "6.0.0"
name: {name}
port: {port}
token: {token}

endpoints:
  - method: GET
    path: /items
    handler: list_items
    exec_class: AsyncTask
    output:
      type: json
      fields:
        - {{ name: items, type: array, items_type: object }}

  - method: POST
    path: /items
    handler: create_item
    exec_class: AsyncTask
    input:
      type: json
      fields:
        - {{ name: name, type: string, required: true }}
        - {{ name: description, type: string }}
    output:
      type: json
      fields:
        - {{ name: id, type: u64, required: true }}
        - {{ name: status, type: string }}

  - method: GET
    path: /items/:id
    handler: get_item
    exec_class: AsyncTask
    output:
      type: json
      fields:
        - {{ name: id, type: u64, required: true }}
        - {{ name: name, type: string, required: true }}

  - method: DELETE
    path: /items/:id
    handler: delete_item
    exec_class: AsyncTask
    output:
      type: json
      fields:
        - {{ name: deleted, type: bool, required: true }}

errors:
  - {{ name: not_found, status: 404, code: NOT_FOUND }}
  - {{ name: validation_error, status: 400, code: VALIDATION_ERROR }}
"#,
        name = c.name,
        port = c.port,
        token = c.token
    )
}

fn yaml_multi_model_router(c: &ProjectConfig) -> String {
    format!(
        r#"# {name} — Multi-Model Router
# Generated by: vil init {name} --template multi-model-router

vil_version: "6.0.0"
name: {name}
port: {port}
token: {token}

semantic_types:
  - name: RoutingDecision
    kind: decision
    fields:
      - {{ name: target_model, type: u32 }}
      - {{ name: priority, type: u8 }}
      - {{ name: confidence, type: u32 }}

nodes:
  gateway:
    type: http-sink
    port: {port}
    path: /infer
    ports:
      trigger_out:      {{ direction: out, lane: trigger }}
      response_data_in: {{ direction: in,  lane: data }}
      response_ctrl_in: {{ direction: in,  lane: control }}

  router:
    type: transform
    code:
      mode: handler
      handler: route_by_model
      async: true
    decision: RoutingDecision
    ports:
      in:        {{ direction: in,  lane: trigger }}
      openai:    {{ direction: out, lane: data }}
      anthropic: {{ direction: out, lane: data }}

  openai_source:
    type: http-source
    url: "{upstream}"
    format: sse
    dialect: standard          # openai | anthropic | ollama | cohere | gemini | standard
    ports:
      trigger_in:        {{ direction: in,  lane: trigger }}
      response_data_out: {{ direction: out, lane: data }}
      response_ctrl_out: {{ direction: out, lane: control }}

routes:
  - {{ from: gateway.trigger_out, to: router.in, mode: LoanWrite }}
  - {{ from: router.openai, to: openai_source.trigger_in, mode: LoanWrite }}
  - {{ from: openai_source.response_data_out, to: gateway.response_data_in, mode: LoanWrite }}
  - {{ from: openai_source.response_ctrl_out, to: gateway.response_ctrl_in, mode: Copy }}

failover:
  entries:
    - primary: openai_source
      backup: anthropic_source
      strategy: "retry:3"
"#,
        name = c.name,
        port = c.port,
        token = c.token,
        upstream = c.upstream
    )
}

fn yaml_rag_pipeline(c: &ProjectConfig) -> String {
    format!(
        r#"# {name} — RAG Pipeline
# Generated by: vil init {name} --template rag-pipeline

vil_version: "6.0.0"
name: {name}
port: {port}
token: {token}

nodes:
  gateway:
    type: http-sink
    port: {port}
    path: /query
    ports:
      trigger_out:      {{ direction: out, lane: trigger }}
      response_data_in: {{ direction: in,  lane: data }}
      response_ctrl_in: {{ direction: in,  lane: control }}

  llm:
    type: http-source
    url: "{upstream}"
    format: sse
    dialect: standard          # openai | anthropic | ollama | cohere | gemini | standard
    ports:
      trigger_in:        {{ direction: in,  lane: trigger }}
      response_data_out: {{ direction: out, lane: data }}
      response_ctrl_out: {{ direction: out, lane: control }}

routes:
  - {{ from: gateway.trigger_out, to: llm.trigger_in, mode: LoanWrite }}
  - {{ from: llm.response_data_out, to: gateway.response_data_in, mode: LoanWrite }}
  - {{ from: llm.response_ctrl_out, to: gateway.response_ctrl_in, mode: Copy }}

workflows:
  rag_query:
    trigger: gateway
    input: QueryRequest
    output: QueryResponse
    tasks:
      - id: embed
        name: "Embed query"
        type: Embed
        config: {{ model: "text-embedding-3-small", dimensions: 1536 }}
        timeout_ms: 5000
      - id: search
        name: "Vector search"
        type: Search
        deps: [embed]
        config: {{ index: "documents", top_k: 5 }}
        timeout_ms: 3000
      - id: generate
        name: "Generate answer"
        type: Generate
        deps: [search]
        config: {{ model: "gpt-4", max_tokens: 1024 }}
        timeout_ms: 30000
"#,
        name = c.name,
        port = c.port,
        token = c.token,
        upstream = c.upstream
    )
}

fn yaml_websocket_chat(c: &ProjectConfig) -> String {
    format!(
        r#"# {name} — WebSocket Chat
# Generated by: vil init {name} --template websocket-chat

vil_version: "6.0.0"
name: {name}
port: {port}
token: {token}

endpoints:
  - method: GET
    path: /health
    handler: health
    exec_class: AsyncTask
    output:
      type: json
      fields:
        - {{ name: status, type: string, required: true }}

ws_events:
  - name: ChatMessage
    topic: chat.room
    fields:
      - {{ name: sender, type: string }}
      - {{ name: content, type: string }}
      - {{ name: timestamp, type: u64 }}
"#,
        name = c.name,
        port = c.port,
        token = c.token
    )
}

fn yaml_wasm_faas(c: &ProjectConfig) -> String {
    format!(
        r#"# {name} — WASM FaaS
# Generated by: vil init {name} --template wasm-faas

vil_version: "6.0.0"
name: {name}
port: {port}
token: {token}

vil_wasm:
  - name: functions
    language: rust
    source_dir: wasm-src/functions/
    pool_size: 4
    sandbox:
      timeout_ms: 5000
      max_memory_mb: 16
    functions:
      - name: process
        input: {{ data: i32, len: i32 }}
        output: i32
        description: "Main processing function"

endpoints:
  - method: POST
    path: /invoke
    handler: invoke_wasm
    exec_class: AsyncTask
    input:
      type: json
      fields:
        - {{ name: function, type: string, required: true }}
        - {{ name: args, type: array }}
    output:
      type: json
      fields:
        - {{ name: result, type: number }}
"#,
        name = c.name,
        port = c.port,
        token = c.token
    )
}

fn yaml_agent(c: &ProjectConfig) -> String {
    format!(
        r#"# {name} — AI Agent
# Generated by: vil init {name} --template agent

vil_version: "6.0.0"
name: {name}
port: {port}
token: {token}

nodes:
  api:
    type: http-sink
    port: {port}
    path: /agent/run
    ports:
      trigger_out:      {{ direction: out, lane: trigger }}
      response_data_in: {{ direction: in,  lane: data }}
      response_ctrl_in: {{ direction: in,  lane: control }}

  llm:
    type: http-source
    url: "{upstream}"
    format: sse
    dialect: standard          # openai | anthropic | ollama | cohere | gemini | standard
    ports:
      trigger_in:        {{ direction: in,  lane: trigger }}
      response_data_out: {{ direction: out, lane: data }}
      response_ctrl_out: {{ direction: out, lane: control }}

routes:
  - {{ from: api.trigger_out, to: llm.trigger_in, mode: LoanWrite }}
  - {{ from: llm.response_data_out, to: api.response_data_in, mode: LoanWrite }}
  - {{ from: llm.response_ctrl_out, to: api.response_ctrl_in, mode: Copy }}

workflows:
  agent_loop:
    trigger: api
    tasks:
      - id: think
        name: "Analyze request"
        type: Transform
        code:
          mode: handler
          handler: agent_loop
        timeout_ms: 30000
"#,
        name = c.name,
        port = c.port,
        token = c.token,
        upstream = c.upstream
    )
}

fn yaml_blank(c: &ProjectConfig) -> String {
    format!(
        r#"# {name} — VIL Project
# Generated by: vil init {name} --template blank
#
# Edit this file, then:
#   vil compile --from yaml --input app.vil.yaml --release
#   vil run --file app.vil.yaml

vil_version: "6.0.0"
name: {name}
port: {port}
token: {token}

# Add your nodes here:
# nodes:
#   my_sink:
#     type: http-sink
#     port: {port}
#     path: /api
#   my_source:
#     type: http-source
#     url: "http://localhost:18081/api/v1/credits/stream"
#     format: sse

# Add routes:
# routes:
#   - from: my_sink.trigger_out
#     to: my_source.trigger_in
#     mode: LoanWrite
"#,
        name = c.name,
        port = c.port,
        token = c.token
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// Handler stub generator
// ═══════════════════════════════════════════════════════════════════════════════

fn generate_handler_stub(name: &str, config: &ProjectConfig) -> String {
    format!(
        r#"//! Handler: {name}
//! Generated by: vil init {project} --template ...
//!
//! This file is hand-edited. vil compile will NOT overwrite it.
//! Edit your business logic here.

use vil_server::prelude::*;

pub async fn {name}(
    input: serde_json::Value,
    _ctx: &HandlerContext,
) -> Result<serde_json::Value, VilError> {{
    // TODO: Implement your handler logic
    //
    // Available:
    //   input  — request payload (JSON)
    //   _ctx   — request context (trace_id, request_id, metrics)
    //
    // Return Ok(output) or Err(VilError::...)

    Ok(serde_json::json!({{
        "status": "ok",
        "handler": "{name}",
        "input_keys": input.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()),
    }}))
}}
"#,
        name = name,
        project = config.name
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// README generator
// ═══════════════════════════════════════════════════════════════════════════════

fn generate_readme(config: &ProjectConfig, template: &Template) -> String {
    format!(
        r#"# {name}

{desc}

Generated by `vil init {name} --template {tmpl}`.

## Quick Start

```bash
# Visualize
vil viz app.vil.yaml --open

# Validate
vil check app.vil.yaml

# Build native binary
vil compile --from yaml --input app.vil.yaml --release

# Run
vil run --file app.vil.yaml
```

## Test

```bash
curl -N -X POST http://localhost:{port}/trigger \
  -H "Content-Type: application/json" \
  -d '{{"prompt": "hello"}}'
```

## Project Structure

```
{name}/
├── app.vil.yaml          <- application manifest (edit this)
├── src/
│   ├── main.rs             <- auto-generated (don't edit)
│   └── handlers/           <- your custom logic (edit these)
├── Cargo.toml              <- auto-generated
└── README.md
```

## Regenerate Rust Code

After editing `app.vil.yaml`:

```bash
vil compile --from yaml --input app.vil.yaml --save-source
```

This regenerates `src/main.rs` and `Cargo.toml`. Your handler files in `src/handlers/` are NOT overwritten.
"#,
        name = config.name,
        desc = template.description,
        tmpl = template.id,
        port = config.port
    )
}

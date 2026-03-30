//! vil init — project initializer with templates and wizard.
//!
//! Templates are sourced from examples/ in the VIL repo. The CLI fetches
//! template-index.json from GitHub, then downloads the specific template files.
//! Falls back to local examples/ if available (for VIL developers).
//!
//! Two modes:
//!   vil init my-app --template ai-gateway --port 3080    (arguments)
//!   vil init                                              (interactive wizard)

use crate::codegen;
use crate::manifest::WorkflowManifest;
use colored::*;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const GITHUB_RAW_BASE: &str = "https://raw.githubusercontent.com/OceanOS-id/VIL/main";
const GITHUB_INDEX_URL: &str = "https://raw.githubusercontent.com/OceanOS-id/VIL/main/template-index.json";

// ═══════════════════════════════════════════════════════════════════════════════
// Example-based init: fetch from GitHub or local examples/
// ═══════════════════════════════════════════════════════════════════════════════

/// Try to init from example templates. Returns Some(result) if handled, None to fallback.
fn try_init_from_example(args: &InitArgs) -> Option<Result<(), String>> {
    let name = args.name.as_ref()?;
    let template_id = args.template.as_deref().unwrap_or("ai-gateway");

    // Fetch template index
    let index = match fetch_template_index() {
        Ok(idx) => idx,
        Err(e) => {
            println!("  {} Could not fetch template index: {}", "NOTE".yellow(), e);
            println!("  Falling back to built-in templates.");
            println!();
            return None;
        }
    };

    // Find matching template
    let tmpl = match index.templates.iter().find(|t| t.id == template_id) {
        Some(t) => t,
        None => {
            println!("  {} Template '{}' not found in remote index.", "NOTE".yellow(), template_id);
            println!("  Available: {}", index.templates.iter().map(|t| t.id.as_str()).collect::<Vec<_>>().join(", "));
            println!("  Falling back to built-in templates.");
            println!();
            return None;
        }
    };

    let port = args.port.unwrap_or(tmpl.default_port);
    let upstream = args.upstream.clone().unwrap_or(tmpl.default_upstream.clone());

    println!("  {} {} ({})", "TEMPLATE".cyan().bold(), tmpl.title, tmpl.id);
    println!("  {} {}", "DESC".dimmed(), tmpl.description);
    println!("  {} {} files from examples/{}", "SOURCE".dimmed(), tmpl.files.len(), tmpl.example_dir);
    println!();

    // Resolve output directory
    let vastar_home = std::env::var("VASTAR_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("vastar")
        });

    let is_vilapp = tmpl.id == "ai-gateway" || tmpl.id == "observer-demo";
    let project_dir = if is_vilapp {
        if !vastar_home.exists() {
            let _ = std::fs::create_dir_all(&vastar_home);
            println!("  {} VASTAR_HOME: {}", "HOME".green(), vastar_home.display());
        }
        vastar_home.join(name)
    } else {
        PathBuf::from(name)
    };

    if project_dir.exists() {
        println!("  {} Directory '{}' already exists. Remove it first.", "ERROR".red().bold(), project_dir.display());
        return Some(Err(format!("Directory {} exists", project_dir.display())));
    }

    // Download and write files
    println!("  {} Downloading template files...", "FETCH".cyan());
    for file_path in &tmpl.files {
        let url = format!("{}/examples/{}/{}", GITHUB_RAW_BASE, tmpl.example_dir, file_path);
        let dest = project_dir.join(file_path);

        // Create parent dirs
        if let Some(parent) = dest.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // GitHub first, VASTAR_HOME local as offline fallback
        let content = fetch_url(&url).ok()
            .or_else(|| try_read_local_example(&vastar_home, &tmpl.example_dir, file_path));

        match content {
            Some(mut text) => {
                // Extract just the directory name (not full path)
                let project_name = Path::new(name)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(name);

                // Apply replacements
                if let Some(pkg) = tmpl.replace.get("package_name") {
                    text = text.replace(pkg, project_name);
                }
                if let Some(old_port) = tmpl.replace.get("port") {
                    text = text.replace(old_port, &port.to_string());
                }
                if let Some(old_upstream) = tmpl.replace.get("upstream") {
                    if !upstream.is_empty() {
                        text = text.replace(old_upstream, &upstream);
                    }
                }
                std::fs::write(&dest, &text)
                    .map_err(|e| format!("Failed to write {}: {}", dest.display(), e))
                    .ok()?;
                println!("    {} {}", "+".green(), file_path);
            }
            None => {
                println!("    {} {} (download failed)", "!".yellow(), file_path);
            }
        }
    }

    // Write .gitignore
    let gitignore = "/target\n*.swp\n*.swo\n.DS_Store\n";
    let _ = std::fs::write(project_dir.join(".gitignore"), gitignore);

    // Summary
    println!();
    println!("{} Project '{}' created!", "DONE".green().bold(), name);
    println!();
    println!("  Next steps:");
    if is_vilapp {
        println!("    ai-endpoint-simulator &");
        println!("    cd {}", project_dir.display());
        println!("    cargo run --release");
        println!();
        println!("  Server will show curl/hey/vastar/dashboard instructions after startup.");
    } else {
        println!("    cd {}", name);
        println!("    cargo run --release");
    }
    println!();

    Some(Ok(()))
}

fn try_read_local_example(vastar_home: &Path, example_dir: &str, file_path: &str) -> Option<String> {
    // Check VASTAR_HOME/vil/examples/ (cloned VIL repo)
    let vil_examples = vastar_home.join("vil/examples").join(example_dir).join(file_path);
    if vil_examples.exists() {
        return std::fs::read_to_string(&vil_examples).ok();
    }
    // Check VASTAR_HOME/examples/ (flat layout)
    let flat = vastar_home.join("examples").join(example_dir).join(file_path);
    if flat.exists() {
        return std::fs::read_to_string(&flat).ok();
    }
    None
}

fn fetch_url(url: &str) -> Result<String, String> {
    reqwest::blocking::get(url)
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.text())
        .map_err(|e| format!("HTTP error: {}", e))
}

#[derive(serde::Deserialize)]
struct TemplateIndex {
    #[allow(dead_code)]
    version: u32,
    templates: Vec<TemplateEntry>,
}

#[derive(serde::Deserialize)]
struct TemplateEntry {
    id: String,
    title: String,
    description: String,
    default_port: u16,
    #[serde(default)]
    default_upstream: String,
    example_dir: String,
    #[serde(default)]
    replace: std::collections::HashMap<String, String>,
    files: Vec<String>,
}

/// `vil templates` — list available templates with sync status.
pub fn list_templates() -> Result<(), String> {
    println!("{}", "VIL Templates".cyan().bold());
    println!();

    let index = fetch_template_index()
        .map_err(|e| format!("Cannot fetch template list: {}", e))?;

    let vastar_home = std::env::var("VASTAR_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("vastar")
        });

    let mut synced = 0;
    println!("  {:<4} {:<22} {:<26} {}", "", "ID", "TITLE", "DESCRIPTION");
    println!("  {}", "-".repeat(85));
    for tmpl in &index.templates {
        let local_dir = vastar_home.join("vil/examples").join(&tmpl.example_dir);
        let is_synced = local_dir.exists() && local_dir.join("src/main.rs").exists();
        let marker = if is_synced { "OK".green().to_string() } else { "--".dimmed().to_string() };
        if is_synced { synced += 1; }
        println!("  {:<4} {:<22} {:<26} {}", marker, tmpl.id, tmpl.title, tmpl.description);
    }

    println!();
    println!("  {} synced / {} total", synced, index.templates.len());
    if synced < index.templates.len() {
        println!("  Run `vil sync` to download all templates for offline use.");
    }
    println!();
    println!("  Usage: vil init <name> --template <ID>");
    println!();

    Ok(())
}

/// `vil sync` — download all templates from GitHub to VASTAR_HOME for offline use.
pub fn sync_templates() -> Result<(), String> {
    println!("{}", "VIL Template Sync".cyan().bold());
    println!();

    let vastar_home = std::env::var("VASTAR_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("vastar")
        });

    // Fetch index from GitHub
    println!("  {} Fetching template index from GitHub...", "FETCH".cyan());
    let index_text = fetch_url(GITHUB_INDEX_URL)
        .map_err(|e| format!("Cannot reach GitHub: {}. Check your internet connection.", e))?;
    let index: TemplateIndex = serde_json::from_str(&index_text)
        .map_err(|e| format!("Invalid template index: {}", e))?;

    println!("  {} {} templates found", "OK".green(), index.templates.len());
    println!();

    // Save index locally
    let index_path = vastar_home.join("vil/template-index.json");
    if let Some(parent) = index_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&index_path, &index_text)
        .map_err(|e| format!("Failed to write {}: {}", index_path.display(), e))?;
    println!("  {} {}", "+".green(), index_path.display());

    // Download each template's files
    let mut total_files = 0;
    for tmpl in &index.templates {
        println!();
        println!("  {} {} ({})", "SYNC".cyan(), tmpl.title, tmpl.id);

        let example_dir = vastar_home.join("vil/examples").join(&tmpl.example_dir);

        // Save template.toml
        let toml_url = format!("{}/examples/{}/template.toml", GITHUB_RAW_BASE, tmpl.example_dir);
        if let Ok(toml_text) = fetch_url(&toml_url) {
            let _ = std::fs::create_dir_all(&example_dir);
            let _ = std::fs::write(example_dir.join("template.toml"), &toml_text);
        }

        for file_path in &tmpl.files {
            let url = format!("{}/examples/{}/{}", GITHUB_RAW_BASE, tmpl.example_dir, file_path);
            let dest = example_dir.join(file_path);

            if let Some(parent) = dest.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            match fetch_url(&url) {
                Ok(text) => {
                    std::fs::write(&dest, &text)
                        .map_err(|e| format!("Write error: {}", e))?;
                    println!("    {} {}", "+".green(), file_path);
                    total_files += 1;
                }
                Err(e) => {
                    println!("    {} {} ({})", "!".yellow(), file_path, e);
                }
            }
        }
    }

    println!();
    println!(
        "{} Synced {} templates, {} files to {}",
        "DONE".green().bold(),
        index.templates.len(),
        total_files,
        vastar_home.join("vil/examples").display()
    );
    println!();
    println!("  Templates are now available offline via `vil init`.");
    println!();

    Ok(())
}

fn fetch_template_index() -> Result<TemplateIndex, String> {
    // GitHub first (always up-to-date)
    if let Ok(text) = fetch_url(GITHUB_INDEX_URL) {
        if let Ok(index) = serde_json::from_str(&text) {
            return Ok(index);
        }
    }
    // Offline fallback: VASTAR_HOME/template-index.json or VASTAR_HOME/vil/template-index.json
    let vastar_home = std::env::var("VASTAR_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("vastar")
        });
    for path in &[
        vastar_home.join("vil/template-index.json"),
        vastar_home.join("template-index.json"),
    ] {
        if path.exists() {
            let text = std::fs::read_to_string(path)
                .map_err(|e| format!("Read error: {}", e))?;
            return serde_json::from_str(&text)
                .map_err(|e| format!("Parse error: {}", e));
        }
    }
    Err("Could not fetch template index from GitHub or local VASTAR_HOME".into())
}

pub struct InitArgs {
    pub name: Option<String>,
    pub template: Option<String>,
    pub lang: Option<String>,
    pub token: Option<String>,
    pub port: Option<u16>,
    pub upstream: Option<String>,
    pub wizard: bool,
}

const SUPPORTED_LANGS: &[(&str, &str)] = &[
    ("rust", "Rust (native — generates Cargo.toml + src/main.rs)"),
    ("python", "Python (generates VIL SDK pipeline script)"),
    ("go", "Go (generates VIL SDK Go module)"),
    ("java", "Java (generates VIL SDK Java source)"),
    ("typescript", "TypeScript (generates VIL SDK TS source)"),
    ("csharp", "C# (generates VIL SDK C# source)"),
    ("kotlin", "Kotlin (generates VIL SDK Kotlin source)"),
    ("swift", "Swift (generates VIL SDK Swift source)"),
    ("zig", "Zig (generates VIL SDK Zig source)"),
];

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
    lang: String,
    port: u16,
    upstream: String,
    token: String,
    observer: bool,
}

const TEMPLATES: &[Template] = &[
    Template {
        id: "ai-gateway",
        title: "AI Gateway",
        description: "SSE streaming pipeline (webhook -> upstream SSE -> streaming response)",
        default_port: 3080,
        default_upstream: "http://127.0.0.1:4545/v1/chat/completions",
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
        default_upstream: "http://127.0.0.1:4545/v1/chat/completions",
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
    Template {
        id: "data-pipeline",
        title: "Data Pipeline",
        description: "S3 ingest → transform → MongoDB store → ClickHouse analytics",
        default_port: 8080,
        default_upstream: "",
        yaml: yaml_data_pipeline,
        has_handler: false,
        handler_name: "",
    },
    Template {
        id: "event-driven",
        title: "Event-Driven",
        description: "RabbitMQ consume → process → publish result",
        default_port: 8080,
        default_upstream: "",
        yaml: yaml_event_driven,
        has_handler: false,
        handler_name: "",
    },
    Template {
        id: "iot-gateway",
        title: "IoT Gateway",
        description: "MQTT trigger → validate → TimeSeries store → alert",
        default_port: 8080,
        default_upstream: "",
        yaml: yaml_iot_gateway,
        has_handler: false,
        handler_name: "",
    },
    Template {
        id: "scheduled-etl",
        title: "Scheduled ETL",
        description: "Cron trigger → S3 fetch → transform → Elasticsearch index",
        default_port: 8080,
        default_upstream: "",
        yaml: yaml_scheduled_etl,
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

    // Try example-based init first (Rust only, if examples/ found)
    let lang = args.lang.clone().unwrap_or("rust".into());
    if lang == "rust" {
        if let Some(result) = try_init_from_example(&args) {
            return result;
        }
        // Fall through to legacy codegen if example not found
    }

    let (name, template_id, lang, token, port, upstream) = if args.wizard {
        run_wizard(&args)?
    } else {
        let name = args
            .name
            .ok_or("Project name is required. Usage: vil init <name> --template <template>")?;
        let tmpl = args.template.unwrap_or("ai-gateway".into());
        let template = find_template(&tmpl)?;
        let lang = validate_lang(&args.lang.unwrap_or("rust".into()))?;
        let token = args.token.unwrap_or("shm".into());
        let port = args.port.unwrap_or(template.default_port);
        let upstream = args.upstream.unwrap_or(template.default_upstream.into());
        (name, tmpl, lang, token, port, upstream)
    };

    let template = find_template(&template_id)?;

    // Resolve VASTAR_HOME workspace
    let vastar_home = std::env::var("VASTAR_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("vastar")
        });

    let is_vilapp = template.id == "ai-gateway";

    // Project lives inside VASTAR_HOME for VilApp templates
    let project_dir_owned = if is_vilapp {
        vastar_home.join(&name)
    } else {
        PathBuf::from(&name)
    };
    let project_dir = project_dir_owned.as_path();
    let project_name = project_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&name)
        .to_string();
    let config = ProjectConfig {
        name: project_name.clone(),
        lang: lang.clone(),
        port,
        upstream: upstream.clone(),
        token: token.clone(),
        observer: false,
    };

    // Setup VASTAR_HOME if VilApp template
    if is_vilapp && !vastar_home.exists() {
        std::fs::create_dir_all(&vastar_home)
            .map_err(|e| format!("Failed to create VASTAR_HOME at {}: {}", vastar_home.display(), e))?;
        println!("  {} VASTAR_HOME: {}", "HOME".green(), vastar_home.display());
    } else if is_vilapp {
        println!("  {} VASTAR_HOME: {}", "HOME".dimmed(), vastar_home.display());
    }

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
                    lang: lang.clone(),
                    port,
                    upstream: upstream.clone(),
                    token: token.clone(),
                    observer: false,
                };
                if config.lang == "rust" {
                    std::fs::create_dir_all(project_dir.join("src/handlers"))
                        .map_err(|e| format!("Failed to create directory: {}", e))?;
                } else {
                    std::fs::create_dir_all(project_dir)
                        .map_err(|e| format!("Failed to create directory: {}", e))?;
                }
                println!("  {} Creating project: {}", "DIR".green(), name);
                return generate_project(project_dir, &config, template);
            }
            _ => {
                println!("  Aborted.");
                return Ok(());
            }
        }
    }
    if config.lang == "rust" {
        std::fs::create_dir_all(project_dir.join("src/handlers"))
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    } else {
        std::fs::create_dir_all(project_dir)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }
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

    if config.lang == "rust" {
        generate_rust_project(project_dir, config, template, &yaml_content)?;
    } else {
        generate_sdk_project(project_dir, config, template)?;
    }

    // Generate README
    let readme = generate_readme(config, template);
    std::fs::write(project_dir.join("README.md"), &readme)
        .map_err(|e| format!("Failed to write README: {}", e))?;
    println!("  {} README.md", "DOC".green());

    // Generate .gitignore
    let gitignore = match config.lang.as_str() {
        "python" => "target/\n*.wasm\nwasm-out/\n__pycache__/\n*.pyc\n.venv/\n",
        "go" => "target/\n*.wasm\nwasm-out/\nvendor/\n",
        "java" => "target/\n*.wasm\nwasm-out/\n*.class\nbuild/\n.gradle/\n",
        "typescript" => "target/\n*.wasm\nwasm-out/\nnode_modules/\ndist/\n",
        "csharp" => "target/\n*.wasm\nwasm-out/\nbin/\nobj/\n.vs/\n",
        "kotlin" => "target/\n*.wasm\nwasm-out/\nbuild/\n.gradle/\n.kotlin/\n",
        "swift" => "target/\n*.wasm\nwasm-out/\n.build/\n.swiftpm/\nPackage.resolved\n",
        "zig" => "target/\n*.wasm\nwasm-out/\nzig-out/\nzig-cache/\n",
        _ => "target/\n*.wasm\nwasm-out/\n",
    };
    std::fs::write(project_dir.join(".gitignore"), gitignore)
        .map_err(|e| format!("Failed to write .gitignore: {}", e))?;

    // Summary
    println!();
    println!(
        "{} Project '{}' created! ({})",
        "DONE".green().bold(),
        config.name,
        config.lang
    );
    println!();
    println!("  Next steps:");
    match config.lang.as_str() {
        "rust" if template.id == "ai-gateway" => {
            println!("    ai-endpoint-simulator &");
            println!("    cd {}", project_dir.display());
            println!("    cargo run --release");
            println!();
            println!("  Server will show curl/hey/vastar/dashboard instructions after startup.");
        }
        "rust" => {
            println!("    cd {}", config.name);
            println!("    vil viz app.vil.yaml --open           # visualize");
            println!("    vil check app.vil.yaml                # validate");
            println!("    vil compile --from yaml --input app.vil.yaml --release  # build");
            println!("    vil run --file app.vil.yaml           # run");
        }
        "python" => {
            let src = format!("app.vil.py");
            println!(
                "    vil compile --from python --input {} --output {}  # compile to native binary",
                src, config.name
            );
            println!("    ./{}", config.name);
        }
        "go" => {
            println!(
                "    vil compile --from go --input main.go --output {}  # compile to native binary",
                config.name
            );
            println!("    ./{}", config.name);
        }
        "java" => {
            println!("    vil compile --from java --input App.java --output {}  # compile to native binary", config.name);
            println!("    ./{}", config.name);
        }
        "typescript" => {
            let src = format!("app.vil.ts");
            println!("    vil compile --from typescript --input {} --output {}  # compile to native binary", src, config.name);
            println!("    ./{}", config.name);
        }
        "csharp" => {
            println!("    vil compile --from csharp --input app.vil.cs --output {}  # compile to native binary", config.name);
            println!("    ./{}", config.name);
        }
        "kotlin" => {
            println!("    vil compile --from kotlin --input app.vil.kt --output {}  # compile to native binary", config.name);
            println!("    ./{}", config.name);
        }
        "swift" => {
            println!("    vil compile --from swift --input app.vil.swift --output {}  # compile to native binary", config.name);
            println!("    ./{}", config.name);
        }
        "zig" => {
            println!("    vil compile --from zig --input app.vil.zig --output {}  # compile to native binary", config.name);
            println!("    ./{}", config.name);
        }
        _ => {}
    }

    Ok(())
}

fn generate_rust_project(
    project_dir: &Path,
    config: &ProjectConfig,
    template: &Template,
    yaml_content: &str,
) -> Result<(), String> {
    let manifest = WorkflowManifest::from_yaml(yaml_content)?;

    let crate_prefix = if crate::sdk_manager::is_sdk_installed() {
        crate::sdk_manager::sdk_current_path()
            .join("internal")
            .to_string_lossy()
            .to_string()
    } else {
        let ws = find_workspace_root_for_init();
        format!("{}/crates", ws)
    };

    let is_vilapp_template = template.id == "ai-gateway";

    let (rust_source, cargo_toml) = if is_vilapp_template {
        (
            generate_vilapp_rust(&manifest, config),
            generate_vilapp_cargo_toml(&config.name, &crate_prefix),
        )
    } else if manifest.is_workflow() {
        (
            codegen::generate_workflow_rust(&manifest),
            codegen::generate_workflow_cargo_toml(&manifest, &crate_prefix),
        )
    } else {
        (
            codegen::generate_rust(&manifest),
            codegen::generate_cargo_toml(&manifest, &crate_prefix),
        )
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

    // Generate Dockerfile per project + shared docker-compose.yaml at VASTAR_HOME
    if is_vilapp_template {
        let dockerfile = generate_gateway_dockerfile(config);
        std::fs::write(project_dir.join("Dockerfile"), &dockerfile)
            .map_err(|e| format!("Failed to write Dockerfile: {}", e))?;
        println!("  {} Dockerfile", "DOCK".cyan());

        // Shared docker-compose.yaml at VASTAR_HOME
        let vastar_home = project_dir.parent().unwrap_or(project_dir);
        let compose_path = vastar_home.join("docker-compose.yaml");
        update_docker_compose(&compose_path, config)?;
        println!("  {} {}/docker-compose.yaml", "DOCK".cyan(), vastar_home.display());
    }

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

    Ok(())
}

fn generate_sdk_project(
    project_dir: &Path,
    config: &ProjectConfig,
    template: &Template,
) -> Result<(), String> {
    let sdk_source = generate_sdk_source(config, template);
    let (filename, lang_label) = match config.lang.as_str() {
        "python" => ("app.vil.py", "PYTHON"),
        "go" => ("main.go", "GO"),
        "java" => ("App.java", "JAVA"),
        "typescript" => ("app.vil.ts", "TS"),
        "csharp" => ("app.vil.cs", "CSHARP"),
        "kotlin" => ("app.vil.kt", "KOTLIN"),
        "swift" => ("app.vil.swift", "SWIFT"),
        "zig" => ("app.vil.zig", "ZIG"),
        _ => return Err(format!("Unsupported SDK language: {}", config.lang)),
    };

    std::fs::write(project_dir.join(filename), &sdk_source)
        .map_err(|e| format!("Failed to write {}: {}", filename, e))?;
    println!(
        "  {} {} (VIL SDK pipeline definition)",
        lang_label.green(),
        filename
    );

    // Language-specific project files
    match config.lang.as_str() {
        "python" => {
            std::fs::write(project_dir.join("requirements.txt"), "vil-sdk>=1.0.0\n")
                .map_err(|e| format!("Failed to write requirements.txt: {}", e))?;
            println!("  {} requirements.txt", "PIP".green());
        }
        "go" => {
            let go_mod = format!(
                "module {}\n\ngo 1.21\n\nrequire github.com/OceanOS-id/vil-sdk-go v1.0.0\n",
                config.name
            );
            std::fs::write(project_dir.join("go.mod"), &go_mod)
                .map_err(|e| format!("Failed to write go.mod: {}", e))?;
            println!("  {} go.mod", "GO".green());
        }
        "java" => {
            let pom = generate_java_pom(config);
            std::fs::write(project_dir.join("pom.xml"), &pom)
                .map_err(|e| format!("Failed to write pom.xml: {}", e))?;
            println!("  {} pom.xml", "MAVEN".green());
        }
        "typescript" => {
            let pkg = format!(
                r#"{{
  "name": "{}",
  "version": "1.0.0",
  "private": true,
  "dependencies": {{
    "@vastar/vil-sdk": "^1.0.0"
  }}
}}
"#,
                config.name
            );
            std::fs::write(project_dir.join("package.json"), &pkg)
                .map_err(|e| format!("Failed to write package.json: {}", e))?;
            println!("  {} package.json", "NPM".green());
        }
        "csharp" => {
            let csproj = generate_csharp_csproj(config);
            std::fs::write(project_dir.join(format!("{}.csproj", config.name)), &csproj)
                .map_err(|e| format!("Failed to write .csproj: {}", e))?;
            println!("  {} {}.csproj", "CSPROJ".green(), config.name);
        }
        "kotlin" => {
            let gradle = generate_kotlin_gradle(config);
            std::fs::write(project_dir.join("build.gradle.kts"), &gradle)
                .map_err(|e| format!("Failed to write build.gradle.kts: {}", e))?;
            println!("  {} build.gradle.kts", "GRADLE".green());
        }
        "swift" => {
            let pkg = generate_swift_package(config);
            std::fs::write(project_dir.join("Package.swift"), &pkg)
                .map_err(|e| format!("Failed to write Package.swift: {}", e))?;
            println!("  {} Package.swift", "SWIFT".green());
        }
        "zig" => {
            let build_zig = generate_zig_build(config);
            std::fs::write(project_dir.join("build.zig"), &build_zig)
                .map_err(|e| format!("Failed to write build.zig: {}", e))?;
            println!("  {} build.zig", "ZIG".green());
        }
        _ => {}
    }

    Ok(())
}

fn generate_sdk_source(config: &ProjectConfig, template: &Template) -> String {
    match config.lang.as_str() {
        "python" => generate_python_sdk(config, template),
        "go" => generate_go_sdk(config, template),
        "java" => generate_java_sdk(config, template),
        "typescript" => generate_ts_sdk(config, template),
        "csharp" => generate_csharp_sdk(config, template),
        "kotlin" => generate_kotlin_sdk(config, template),
        "swift" => generate_swift_sdk(config, template),
        "zig" => generate_zig_sdk(config, template),
        _ => String::new(),
    }
}

fn generate_python_sdk(config: &ProjectConfig, template: &Template) -> String {
    let steps = sdk_steps_for_template(template.id, "python");
    format!(
        r#"# {name} — VIL SDK Pipeline ({tmpl_title})
# Generated by: vil init {name} --lang python --template {tmpl_id}
#
# Compile: vil compile --from python --input app.vil.py --output {name}
# Run:     ./{name}

from vil_sdk import pipeline, http_trigger{imports}

p = pipeline("{name}")
p.trigger(http_trigger(port={port}, path="/api/{path}"{response_mode}))
{steps}

# Connectors (S3, MongoDB, etc.) are declared in app.vil.yaml, not in SDK code.
# See: vil init --template data-pipeline for a connector example.
"#,
        name = config.name,
        tmpl_title = template.title,
        tmpl_id = template.id,
        port = config.port,
        path = sdk_default_path(template.id),
        response_mode = sdk_response_mode(template.id, "python"),
        imports = sdk_imports(template.id, "python"),
        steps = steps,
    )
}

fn generate_go_sdk(config: &ProjectConfig, template: &Template) -> String {
    let steps = sdk_steps_for_template(template.id, "go");
    format!(
        r#"// {name} — VIL SDK Pipeline ({tmpl_title})
// Generated by: vil init {name} --lang go --template {tmpl_id}
//
// Compile: vil compile --from go --input main.go --output {name}
// Run:     ./{name}

package main

import vil "github.com/OceanOS-id/vil-sdk-go"

func main() {{
	p := vil.NewPipeline("{name}")
	p.Trigger(vil.HTTPTrigger{{Port: {port}, Path: "/api/{path}"{response_mode}}})
{steps}
}}

// Connectors (S3, MongoDB, etc.) are declared in app.vil.yaml, not in SDK code.
// See: vil init --template data-pipeline for a connector example.
"#,
        name = config.name,
        tmpl_title = template.title,
        tmpl_id = template.id,
        port = config.port,
        path = sdk_default_path(template.id),
        response_mode = sdk_response_mode(template.id, "go"),
        steps = steps,
    )
}

fn generate_java_sdk(config: &ProjectConfig, template: &Template) -> String {
    let class_name = config
        .name
        .chars()
        .enumerate()
        .map(|(i, c)| {
            if i == 0 || (i > 0 && config.name.as_bytes()[i - 1] == b'-') {
                c.to_uppercase().next().unwrap_or(c)
            } else if c == '-' {
                ' '
            } else {
                c
            }
        })
        .filter(|c| *c != ' ')
        .collect::<String>();
    let steps = sdk_steps_for_template(template.id, "java");
    format!(
        r#"// {name} — VIL SDK Pipeline ({tmpl_title})
// Generated by: vil init {name} --lang java --template {tmpl_id}
//
// Compile: vil compile --from java --input App.java --output {name}
// Run:     ./{name}

import id.vastar.vil.sdk.*;

public class App {{
    public static void main(String[] args) {{
        var p = VilPipeline.create("{name}");
        p.trigger(HTTPTrigger.builder().port({port}).path("/api/{path}"){response_mode}.build());
{steps}
    }}
}}

// Connectors (S3, MongoDB, etc.) are declared in app.vil.yaml, not in SDK code.
// See: vil init --template data-pipeline for a connector example.
"#,
        name = config.name,
        tmpl_title = template.title,
        tmpl_id = template.id,
        port = config.port,
        path = sdk_default_path(template.id),
        response_mode = sdk_response_mode(template.id, "java"),
        steps = steps,
    )
}

fn generate_ts_sdk(config: &ProjectConfig, template: &Template) -> String {
    let steps = sdk_steps_for_template(template.id, "typescript");
    format!(
        r#"// {name} — VIL SDK Pipeline ({tmpl_title})
// Generated by: vil init {name} --lang typescript --template {tmpl_id}
//
// Compile: vil compile --from typescript --input app.vil.ts --output {name}
// Run:     ./{name}

import {{ pipeline, httpTrigger{imports} }} from '@vastar/vil-sdk';

const p = pipeline('{name}');
p.trigger(httpTrigger({{ port: {port}, path: '/api/{path}'{response_mode} }}));
{steps}

// Connectors (S3, MongoDB, etc.) are declared in app.vil.yaml, not in SDK code.
// See: vil init --template data-pipeline for a connector example.
"#,
        name = config.name,
        tmpl_title = template.title,
        tmpl_id = template.id,
        port = config.port,
        path = sdk_default_path(template.id),
        response_mode = sdk_response_mode(template.id, "typescript"),
        imports = sdk_imports(template.id, "typescript"),
        steps = steps,
    )
}

fn generate_csharp_sdk(config: &ProjectConfig, _template: &Template) -> String {
    format!(
        r#"// {name} — VIL SDK Pipeline
// Generated by: vil init {name} --lang csharp
//
// Compile: vil compile --from csharp --input app.vil.cs --output {name}
// Run:     ./{name}

using Vil.Sdk;

var pipeline = new VilPipeline("{name}")
    .Port({port})
    .Source(new HttpSource("ingest")
        .Method(HttpMethod.Post)
        .Path("/trigger"))
    .Sink(new HttpSink("upstream")
        .Url("http://localhost:4545"))
    .Build();

VilRunner.Run(pipeline);

// Connectors (S3, MongoDB, etc.) are declared in app.vil.yaml, not in SDK code.
// See: vil init --template data-pipeline for a connector example.
"#,
        name = config.name,
        port = config.port,
    )
}

fn generate_kotlin_sdk(config: &ProjectConfig, _template: &Template) -> String {
    format!(
        r#"// {name} — VIL SDK Pipeline
// Generated by: vil init {name} --lang kotlin
//
// Compile: vil compile --from kotlin --input app.vil.kt --output {name}
// Run:     ./{name}

import id.vastar.vil.sdk.*

fun main() {{
    vilPipeline("{name}") {{
        port({port})
        source(httpSource("ingest") {{
            method(HttpMethod.POST)
            path("/trigger")
        }})
        sink(httpSink("upstream") {{
            url("http://localhost:4545")
        }})
    }}.run()
}}

// Connectors (S3, MongoDB, etc.) are declared in app.vil.yaml, not in SDK code.
// See: vil init --template data-pipeline for a connector example.
"#,
        name = config.name,
        port = config.port,
    )
}

fn generate_swift_sdk(config: &ProjectConfig, _template: &Template) -> String {
    format!(
        r#"// {name} — VIL SDK Pipeline
// Generated by: vil init {name} --lang swift
//
// Compile: vil compile --from swift --input app.vil.swift --output {name}
// Run:     ./{name}

import VilSDK

let pipeline = VilPipeline("{name}")
    .port({port})
    .source(HttpSource("ingest")
        .method(.post)
        .path("/trigger"))
    .sink(HttpSink("upstream")
        .url("http://localhost:4545"))

VilRunner.run(pipeline)

// Connectors (S3, MongoDB, etc.) are declared in app.vil.yaml, not in SDK code.
// See: vil init --template data-pipeline for a connector example.
"#,
        name = config.name,
        port = config.port,
    )
}

fn generate_zig_sdk(config: &ProjectConfig, _template: &Template) -> String {
    format!(
        r#"// {name} — VIL SDK Pipeline
// Generated by: vil init {name} --lang zig
//
// Compile: vil compile --from zig --input app.vil.zig --output {name}
// Run:     ./{name}

const vil = @import("vil-sdk");

pub fn main() !void {{
    var pipeline = vil.Pipeline.init("{name}")
        .port({port})
        .source(vil.HttpSource.init("ingest")
            .method(.post)
            .path("/trigger"))
        .sink(vil.HttpSink.init("upstream")
            .url("http://localhost:4545"));

    try vil.run(pipeline);
}}

// Connectors (S3, MongoDB, etc.) are declared in app.vil.yaml, not in SDK code.
// See: vil init --template data-pipeline for a connector example.
"#,
        name = config.name,
        port = config.port,
    )
}

fn sdk_default_path(template_id: &str) -> &str {
    match template_id {
        "ai-gateway" => "chat",
        "rest-crud" => "items",
        "multi-model-router" => "chat",
        "rag-pipeline" => "ask",
        "websocket-chat" => "ws",
        "wasm-faas" => "invoke",
        "agent" => "agent",
        _ => "trigger",
    }
}

fn sdk_response_mode(template_id: &str, lang: &str) -> String {
    let needs_sse = matches!(template_id, "ai-gateway" | "multi-model-router");
    if !needs_sse {
        return String::new();
    }
    match lang {
        "python" => ", response_mode=\"sse\"".into(),
        "go" => ", ResponseMode: \"sse\"".into(),
        "java" => ".responseMode(\"sse\")".into(),
        "typescript" => ", responseMode: 'sse'".into(),
        _ => String::new(),
    }
}

fn sdk_imports(template_id: &str, lang: &str) -> String {
    match lang {
        "python" => {
            let mut imps = Vec::new();
            match template_id {
                "ai-gateway" => imps.extend(["llm_call", "respond"]),
                "rest-crud" => imps.extend(["crud_handler", "respond"]),
                "multi-model-router" => imps.extend(["model_router", "respond"]),
                "rag-pipeline" => imps.extend(["rag_search", "llm_call", "respond"]),
                "websocket-chat" => imps.extend(["websocket_handler"]),
                "wasm-faas" => imps.extend(["wasm_function", "respond"]),
                "agent" => imps.extend(["react_agent", "respond"]),
                _ => imps.push("respond"),
            }
            if imps.is_empty() {
                String::new()
            } else {
                format!(", {}", imps.join(", "))
            }
        }
        "typescript" => {
            let mut imps = Vec::new();
            match template_id {
                "ai-gateway" => imps.extend(["llmCall", "respond"]),
                "rest-crud" => imps.extend(["crudHandler", "respond"]),
                "multi-model-router" => imps.extend(["modelRouter", "respond"]),
                "rag-pipeline" => imps.extend(["ragSearch", "llmCall", "respond"]),
                "websocket-chat" => imps.extend(["websocketHandler"]),
                "wasm-faas" => imps.extend(["wasmFunction", "respond"]),
                "agent" => imps.extend(["reactAgent", "respond"]),
                _ => imps.push("respond"),
            }
            if imps.is_empty() {
                String::new()
            } else {
                format!(", {}", imps.join(", "))
            }
        }
        _ => String::new(),
    }
}

fn sdk_steps_for_template(template_id: &str, lang: &str) -> String {
    match (template_id, lang) {
        // ── Python ──
        ("ai-gateway", "python") => r#"p.step(llm_call(model="gpt-4", temperature=0.7))
p.step(respond(format="sse"))"#.into(),
        ("rest-crud", "python") => r#"p.step(crud_handler(table="items", db="postgres://localhost/mydb"))
p.step(respond(format="json"))"#.into(),
        ("multi-model-router", "python") => r#"p.step(model_router(routes={
    "gpt-4": "https://api.openai.com/v1/chat/completions",
    "claude": "https://api.anthropic.com/v1/messages",
    "llama": "http://localhost:11434/api/chat",
}))
p.step(respond(format="sse"))"#.into(),
        ("rag-pipeline", "python") => r#"p.step(rag_search(collection="docs", top_k=5))
p.step(llm_call(model="gpt-4", temperature=0.3, system="Answer using the provided context."))
p.step(respond(format="json"))"#.into(),
        ("websocket-chat", "python") => r#"p.step(websocket_handler(broadcast=True))"#.into(),
        ("wasm-faas", "python") => r#"p.step(wasm_function(path="./functions/handler.wasm", memory_limit=64*1024*1024))
p.step(respond(format="json"))"#.into(),
        ("agent", "python") => r#"p.step(react_agent(model="gpt-4", tools=["calculator", "http_fetch", "retrieval"], max_steps=10))
p.step(respond(format="json"))"#.into(),
        ("blank", "python") => r#"# Add your pipeline steps here
p.step(respond(format="json"))"#.into(),

        // ── Go ──
        ("ai-gateway", "go") => "\tp.Step(vil.LLMCall{Model: \"gpt-4\", Temperature: 0.7})\n\tp.Step(vil.Respond{Format: \"sse\"})".into(),
        ("rest-crud", "go") => "\tp.Step(vil.CRUDHandler{Table: \"items\", DB: \"postgres://localhost/mydb\"})\n\tp.Step(vil.Respond{Format: \"json\"})".into(),
        ("multi-model-router", "go") => "\tp.Step(vil.ModelRouter{Routes: map[string]string{\n\t\t\"gpt-4\":  \"https://api.openai.com/v1/chat/completions\",\n\t\t\"claude\": \"https://api.anthropic.com/v1/messages\",\n\t\t\"llama\":  \"http://localhost:11434/api/chat\",\n\t}})\n\tp.Step(vil.Respond{Format: \"sse\"})".into(),
        ("rag-pipeline", "go") => "\tp.Step(vil.RAGSearch{Collection: \"docs\", TopK: 5})\n\tp.Step(vil.LLMCall{Model: \"gpt-4\", Temperature: 0.3, System: \"Answer using the provided context.\"})\n\tp.Step(vil.Respond{Format: \"json\"})".into(),
        ("websocket-chat", "go") => "\tp.Step(vil.WebSocketHandler{Broadcast: true})".into(),
        ("wasm-faas", "go") => "\tp.Step(vil.WASMFunction{Path: \"./functions/handler.wasm\", MemoryLimit: 64 * 1024 * 1024})\n\tp.Step(vil.Respond{Format: \"json\"})".into(),
        ("agent", "go") => "\tp.Step(vil.ReactAgent{Model: \"gpt-4\", Tools: []string{\"calculator\", \"http_fetch\", \"retrieval\"}, MaxSteps: 10})\n\tp.Step(vil.Respond{Format: \"json\"})".into(),
        ("blank", "go") => "\t// Add your pipeline steps here\n\tp.Step(vil.Respond{Format: \"json\"})".into(),

        // ── Java ──
        ("ai-gateway", "java") => "        p.step(LLMCall.builder().model(\"gpt-4\").temperature(0.7).build());\n        p.step(Respond.builder().format(\"sse\").build());".into(),
        ("rest-crud", "java") => "        p.step(CRUDHandler.builder().table(\"items\").db(\"postgres://localhost/mydb\").build());\n        p.step(Respond.builder().format(\"json\").build());".into(),
        ("multi-model-router", "java") => "        p.step(ModelRouter.builder()\n            .route(\"gpt-4\", \"https://api.openai.com/v1/chat/completions\")\n            .route(\"claude\", \"https://api.anthropic.com/v1/messages\")\n            .route(\"llama\", \"http://localhost:11434/api/chat\")\n            .build());\n        p.step(Respond.builder().format(\"sse\").build());".into(),
        ("rag-pipeline", "java") => "        p.step(RAGSearch.builder().collection(\"docs\").topK(5).build());\n        p.step(LLMCall.builder().model(\"gpt-4\").temperature(0.3).system(\"Answer using the provided context.\").build());\n        p.step(Respond.builder().format(\"json\").build());".into(),
        ("websocket-chat", "java") => "        p.step(WebSocketHandler.builder().broadcast(true).build());".into(),
        ("wasm-faas", "java") => "        p.step(WASMFunction.builder().path(\"./functions/handler.wasm\").memoryLimit(64 * 1024 * 1024).build());\n        p.step(Respond.builder().format(\"json\").build());".into(),
        ("agent", "java") => "        p.step(ReactAgent.builder().model(\"gpt-4\").tools(\"calculator\", \"http_fetch\", \"retrieval\").maxSteps(10).build());\n        p.step(Respond.builder().format(\"json\").build());".into(),
        ("blank", "java") => "        // Add your pipeline steps here\n        p.step(Respond.builder().format(\"json\").build());".into(),

        // ── TypeScript ──
        ("ai-gateway", "typescript") => "p.step(llmCall({ model: 'gpt-4', temperature: 0.7 }));\np.step(respond({ format: 'sse' }));".into(),
        ("rest-crud", "typescript") => "p.step(crudHandler({ table: 'items', db: 'postgres://localhost/mydb' }));\np.step(respond({ format: 'json' }));".into(),
        ("multi-model-router", "typescript") => "p.step(modelRouter({\n  routes: {\n    'gpt-4': 'https://api.openai.com/v1/chat/completions',\n    'claude': 'https://api.anthropic.com/v1/messages',\n    'llama': 'http://localhost:11434/api/chat',\n  }\n}));\np.step(respond({ format: 'sse' }));".into(),
        ("rag-pipeline", "typescript") => "p.step(ragSearch({ collection: 'docs', topK: 5 }));\np.step(llmCall({ model: 'gpt-4', temperature: 0.3, system: 'Answer using the provided context.' }));\np.step(respond({ format: 'json' }));".into(),
        ("websocket-chat", "typescript") => "p.step(websocketHandler({ broadcast: true }));".into(),
        ("wasm-faas", "typescript") => "p.step(wasmFunction({ path: './functions/handler.wasm', memoryLimit: 64 * 1024 * 1024 }));\np.step(respond({ format: 'json' }));".into(),
        ("agent", "typescript") => "p.step(reactAgent({ model: 'gpt-4', tools: ['calculator', 'http_fetch', 'retrieval'], maxSteps: 10 }));\np.step(respond({ format: 'json' }));".into(),
        ("blank", "typescript") => "// Add your pipeline steps here\np.step(respond({ format: 'json' }));".into(),

        _ => "// TODO: add pipeline steps".into(),
    }
}

fn generate_java_pom(config: &ProjectConfig) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
    <modelVersion>4.0.0</modelVersion>
    <groupId>id.vastar.vil</groupId>
    <artifactId>{name}</artifactId>
    <version>1.0.0</version>
    <properties>
        <maven.compiler.source>21</maven.compiler.source>
        <maven.compiler.target>21</maven.compiler.target>
    </properties>
    <dependencies>
        <dependency>
            <groupId>id.vastar</groupId>
            <artifactId>vil-sdk</artifactId>
            <version>1.0.0</version>
        </dependency>
    </dependencies>
</project>
"#,
        name = config.name
    )
}

fn generate_csharp_csproj(config: &ProjectConfig) -> String {
    format!(
        r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net8.0</TargetFramework>
    <AssemblyName>{name}</AssemblyName>
  </PropertyGroup>
  <ItemGroup>
    <PackageReference Include="vil-sdk" Version="1.0.0">
      <PackageName>vil-sdk</PackageName>
    </PackageReference>
  </ItemGroup>
</Project>
"#,
        name = config.name
    )
}

fn generate_kotlin_gradle(config: &ProjectConfig) -> String {
    format!(
        r#"plugins {{
    kotlin("jvm") version "1.9.0"
    application
}}

application {{
    mainClass.set("MainKt")
}}

group = "id.vastar.vil"
version = "1.0.0"
description = "{name}"

repositories {{
    mavenCentral()
    maven("https://repo.vastar.id/releases")
}}

dependencies {{
    implementation("id.vastar:vil-sdk:1.0.0")
}}
"#,
        name = config.name
    )
}

fn generate_swift_package(config: &ProjectConfig) -> String {
    format!(
        r#"// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "{name}",
    dependencies: [
        .package(url: "https://github.com/OceanOS-id/vil-sdk-swift.git", from: "1.0.0"),
    ],
    targets: [
        .executableTarget(
            name: "{name}",
            dependencies: [
                .product(name: "VilSDK", package: "vil-sdk-swift"),
            ]
        ),
    ]
)
"#,
        name = config.name
    )
}

fn generate_zig_build(config: &ProjectConfig) -> String {
    format!(
        r#"const std = @import("std");

pub fn build(b: *std.Build) void {{
    const target = b.standardTargetOptions(.{{}});
    const optimize = b.standardOptimizeOption(.{{}});

    const vil_sdk = b.dependency("vil-sdk", .{{
        .target = target,
        .optimize = optimize,
    }});

    const exe = b.addExecutable(.{{
        .name = "{name}",
        .root_source_file = b.path("app.vil.zig"),
        .target = target,
        .optimize = optimize,
    }});
    exe.root_module.addImport("vil-sdk", vil_sdk.module("vil-sdk"));

    b.installArtifact(exe);
}}
"#,
        name = config.name
    )
}

fn validate_lang(lang: &str) -> Result<String, String> {
    let normalized = lang.to_lowercase();
    let valid = match normalized.as_str() {
        "rust" | "rs" => "rust",
        "python" | "py" => "python",
        "go" | "golang" => "go",
        "java" => "java",
        "typescript" | "ts" => "typescript",
        "csharp" | "cs" | "c#" => "csharp",
        "kotlin" | "kt" => "kotlin",
        "swift" => "swift",
        "zig" => "zig",
        _ => {
            return Err(format!(
                "Unsupported language '{}'. Available: rust, python, go, java, typescript, csharp, kotlin, swift, zig",
                lang
            ))
        }
    };
    Ok(valid.to_string())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Wizard
// ═══════════════════════════════════════════════════════════════════════════════

fn run_wizard(args: &InitArgs) -> Result<(String, String, String, String, u16, String), String> {
    // Project name
    let name = if let Some(n) = &args.name {
        n.clone()
    } else {
        prompt("Project name", "my-vil-app")?
    };

    // Language selection
    println!();
    println!("  {} Available languages:", "LANGUAGE".cyan());
    for (i, (id, desc)) in SUPPORTED_LANGS.iter().enumerate() {
        println!("    {}. {:15} {}", i + 1, id.green(), desc);
    }
    println!();
    let lang_input = if let Some(l) = &args.lang {
        l.clone()
    } else {
        prompt("Language (number or name)", "1")?
    };
    let lang = resolve_lang(&lang_input)?;

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

    // Token type (only for Rust)
    let token = if lang == "rust" {
        println!();
        println!("  {} Token types:", "TOKEN".cyan());
        println!(
            "    1. {} — multi-pipeline, zero-copy SHM (recommended)",
            "shm".green()
        );
        println!("    2. {} — single pipeline, simpler", "generic".green());
        let token_input = prompt("Token", "shm")?;
        if token_input == "2" || token_input == "generic" {
            "generic".into()
        } else {
            "shm".into()
        }
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

    Ok((name, template_id, lang, token, port, upstream))
}

fn resolve_lang(input: &str) -> Result<String, String> {
    // Try as number
    if let Ok(n) = input.parse::<usize>() {
        if n >= 1 && n <= SUPPORTED_LANGS.len() {
            return Ok(SUPPORTED_LANGS[n - 1].0.to_string());
        }
    }
    // Try as name
    validate_lang(input)
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

/// Append optional YAML fields (observer, etc.) after the `token:` line.
fn yaml_optional_fields(c: &ProjectConfig) -> String {
    let mut s = String::new();
    if c.observer {
        s.push_str("observer: true\n");
    }
    s
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
{optional}
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
    url: {upstream}
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
        optional = yaml_optional_fields(c),
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
{optional}
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
        token = c.token,
        optional = yaml_optional_fields(c)
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
{optional}
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
        optional = yaml_optional_fields(c),
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
{optional}
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
        optional = yaml_optional_fields(c),
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
{optional}
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
        token = c.token,
        optional = yaml_optional_fields(c)
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
{optional}
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
        token = c.token,
        optional = yaml_optional_fields(c)
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
{optional}
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
        optional = yaml_optional_fields(c),
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
{optional}
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
        token = c.token,
        optional = yaml_optional_fields(c)
    )
}

fn yaml_data_pipeline(c: &ProjectConfig) -> String {
    format!(
        r#"# {name} — Data Pipeline
# Generated by: vil init {name} --template data-pipeline
#
# Pipeline: S3 ingest → transform → MongoDB store → ClickHouse analytics

vil_version: "6.0.0"
name: {name}
port: {port}
{optional}
connectors:
  storage:
    - name: ingest
      type: s3
      endpoint: ${{S3_ENDPOINT:-http://localhost:9000}}
      bucket: raw-data
      region: us-east-1
  databases:
    - name: store
      type: mongo
      uri: ${{MONGO_URI:-mongodb://localhost:27017}}
      database: processed
    - name: analytics
      type: clickhouse
      url: ${{CLICKHOUSE_URL:-http://localhost:8123}}
      database: analytics

logging:
  level: info
  threads: 4
  drains:
    - type: stdout
      format: resolved
"#,
        name = c.name,
        port = c.port,
        optional = yaml_optional_fields(c),
    )
}

fn yaml_event_driven(c: &ProjectConfig) -> String {
    format!(
        r#"# {name} — Event-Driven
# Generated by: vil init {name} --template event-driven
#
# Pipeline: RabbitMQ consume → process → publish result

vil_version: "6.0.0"
name: {name}
port: {port}
{optional}
connectors:
  queues:
    - name: input
      type: rabbitmq
      uri: ${{RABBITMQ_URI:-amqp://localhost:5672}}
      queue: tasks
    - name: output
      type: rabbitmq
      uri: ${{RABBITMQ_URI:-amqp://localhost:5672}}
      exchange: results

logging:
  level: info
  drains:
    - type: stdout
      format: resolved
"#,
        name = c.name,
        port = c.port,
        optional = yaml_optional_fields(c),
    )
}

fn yaml_iot_gateway(c: &ProjectConfig) -> String {
    format!(
        r#"# {name} — IoT Gateway
# Generated by: vil init {name} --template iot-gateway
#
# Pipeline: MQTT trigger → validate → TimeSeries store → alert

vil_version: "6.0.0"
name: {name}
port: {port}
{optional}
connectors:
  databases:
    - name: timeseries
      type: timeseries
      url: ${{INFLUXDB_URL:-http://localhost:8086}}

triggers:
  - name: devices
    type: iot
    topic: sensors/#
    url: ${{MQTT_HOST:-localhost:1883}}

logging:
  level: info
  drains:
    - type: stdout
      format: resolved
"#,
        name = c.name,
        port = c.port,
        optional = yaml_optional_fields(c),
    )
}

fn yaml_scheduled_etl(c: &ProjectConfig) -> String {
    format!(
        r#"# {name} — Scheduled ETL
# Generated by: vil init {name} --template scheduled-etl
#
# Pipeline: Cron trigger → S3 fetch → transform → Elasticsearch index

vil_version: "6.0.0"
name: {name}
port: {port}
{optional}
connectors:
  storage:
    - name: source
      type: s3
      endpoint: ${{S3_ENDPOINT:-http://localhost:9000}}
      bucket: raw-logs
      region: us-east-1
  databases:
    - name: search
      type: elastic
      url: ${{ELASTIC_URL:-http://localhost:9200}}

triggers:
  - name: hourly
    type: cron
    schedule: "0 0 * * * *"

logging:
  level: info
  drains:
    - type: stdout
      format: resolved
"#,
        name = c.name,
        port = c.port,
        optional = yaml_optional_fields(c),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// ═══════════════════════════════════════════════════════════════════════════════
// Docker Compose generator (upstream simulators with built-in Redis)
// ═══════════════════════════════════════════════════════════════════════════════

fn update_docker_compose(compose_path: &Path, config: &ProjectConfig) -> Result<(), String> {
    let project_service = format!(
        r#"
  {name}:
    build: ./{name}
    ports:
      - "{port}:{port}"
    depends_on:
      - ai-simulator
    environment:
      - UPSTREAM_URL=http://ai-simulator:4545/v1/chat/completions
      - RUST_LOG=info
"#,
        name = config.name,
        port = config.port,
    );

    if compose_path.exists() {
        // Append project to existing docker-compose.yaml
        let existing = std::fs::read_to_string(compose_path)
            .map_err(|e| format!("Failed to read docker-compose.yaml: {}", e))?;

        if existing.contains(&format!("  {}:", config.name)) {
            // Project already in compose — skip
            return Ok(());
        }

        let mut content = existing.trim_end().to_string();
        content.push_str(&project_service);
        std::fs::write(compose_path, content)
            .map_err(|e| format!("Failed to update docker-compose.yaml: {}", e))?;
    } else {
        // Create new docker-compose.yaml with shared infra + this project
        let content = format!(
            r#"# VASTAR_HOME — Shared infrastructure + applications
# Start:  docker compose up -d --build
# Stop:   docker compose down
# Logs:   docker compose logs -f
# Rebuild after code change:  docker compose up -d --build

services:
  redis:
    image: redis:7-alpine

  ai-simulator:
    image: cxlsilicondev/ai-endpoint-simulator
    ports:
      - "4545:4545"
    depends_on:
      - redis

  # credit-data-simulator:
  #   image: cxlsilicondev/credit-data-simulator
  #   ports:
  #     - "18081:18081"
{project}"#,
            project = project_service,
        );
        std::fs::write(compose_path, content)
            .map_err(|e| format!("Failed to write docker-compose.yaml: {}", e))?;
    }
    Ok(())
}

fn generate_gateway_dockerfile(config: &ProjectConfig) -> String {
    format!(
        r#"FROM rust:1.93-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
RUN cargo build --release

FROM rust:1.93-slim
WORKDIR /app
COPY --from=builder /app/target/release/{name} ./
EXPOSE {port}
CMD ["./{name}"]
"#,
        name = config.name,
        port = config.port,
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// VilApp pattern generator (ai-gateway template)
// ═══════════════════════════════════════════════════════════════════════════════

fn generate_vilapp_rust(manifest: &crate::manifest::WorkflowManifest, config: &ProjectConfig) -> String {
    let port = config.port;
    let upstream_url = manifest
        .nodes
        .values()
        .find(|n| n.node_type == "http-source")
        .and_then(|n| n.url.as_deref())
        .unwrap_or("http://127.0.0.1:4545/v1/chat/completions")
        .trim_matches(|c| c == '"' || c == '\\');
    let sink_path = manifest
        .nodes
        .values()
        .find(|n| n.node_type == "http-sink")
        .and_then(|n| n.path.as_deref())
        .unwrap_or("/trigger");

    format!(
        r#"// Auto-generated by: vil init {name} --template ai-gateway
// Pattern: VilApp (single service, observer enabled)

use vil_server::prelude::*;

fn upstream_url() -> String {{
    std::env::var("UPSTREAM_URL").unwrap_or_else(|_| "{upstream_url}".into())
}}

#[derive(Deserialize)]
struct TriggerRequest {{
    #[serde(default = "default_prompt")]
    prompt: String,
}}

fn default_prompt() -> String {{
    "Hello".to_string()
}}

async fn trigger(body: ShmSlice) -> impl IntoResponse {{
    let req: TriggerRequest = body.json().unwrap_or(TriggerRequest {{
        prompt: default_prompt(),
    }});

    let result = SseCollect::post_to(&upstream_url())
        .body(serde_json::json!({{
            "model": "gpt-4",
            "messages": [{{"role": "user", "content": req.prompt}}],
            "stream": true
        }}))
        .json_tap("choices[0].delta.content")
        .done_marker("[DONE]")
        .collect_text()
        .await;

    match result {{
        Ok(content) => (
            StatusCode::OK,
            Json(serde_json::json!({{ "content": content }})),
        ),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({{ "error": e.to_string() }})),
        ),
    }}
}}

#[tokio::main]
async fn main() {{
    let svc = ServiceProcess::new("gw")
        .endpoint(Method::POST, "{sink_path}", post(trigger));

    let app = VilApp::new("{name}")
        .port({port})
        .ensure_port_free()
        .observer(true)
        .service(svc);

    app.run().await;
}}
"#,
        name = config.name,
        upstream_url = upstream_url,
        sink_path = sink_path,
        port = port,
    )
}

fn generate_vilapp_cargo_toml(name: &str, crate_prefix: &str) -> String {
    let use_local = std::path::Path::new(crate_prefix).exists();
    let vil_server_dep = if use_local {
        let path = format!("{}/vil_server", crate_prefix);
        if std::path::Path::new(&path).join("Cargo.toml").exists() {
            format!("vil_server = {{ path = \"{}\" }}", path)
        } else {
            "vil_server = \"0.1\"".into()
        }
    } else {
        "vil_server = \"0.1\"".into()
    };

    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"
publish = false

[dependencies]
{vil_server_dep}
tokio = {{ version = "1", features = ["full"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
"#,
        name = name,
        vil_server_dep = vil_server_dep,
    )
}

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
    let lang_flag = if config.lang != "rust" {
        format!(" --lang {}", config.lang)
    } else {
        String::new()
    };

    let quick_start = match config.lang.as_str() {
        "python" => format!(
            r#"```bash
# Compile to native binary
vil compile --from python --input app.vil.py --output {name}

# Run
./{name}
```"#,
            name = config.name
        ),
        "go" => format!(
            r#"```bash
# Compile to native binary
vil compile --from go --input main.go --output {name}

# Run
./{name}
```"#,
            name = config.name
        ),
        "java" => format!(
            r#"```bash
# Compile to native binary
vil compile --from java --input App.java --output {name}

# Run
./{name}
```"#,
            name = config.name
        ),
        "typescript" => format!(
            r#"```bash
# Compile to native binary
vil compile --from typescript --input app.vil.ts --output {name}

# Run
./{name}
```"#,
            name = config.name
        ),
        "csharp" => format!(
            r#"```bash
# Compile to native binary
vil compile --from csharp --input app.vil.cs --output {name}

# Run
./{name}
```"#,
            name = config.name
        ),
        "kotlin" => format!(
            r#"```bash
# Compile to native binary
vil compile --from kotlin --input app.vil.kt --output {name}

# Run
./{name}
```"#,
            name = config.name
        ),
        "swift" => format!(
            r#"```bash
# Compile to native binary
vil compile --from swift --input app.vil.swift --output {name}

# Run
./{name}
```"#,
            name = config.name
        ),
        "zig" => format!(
            r#"```bash
# Compile to native binary
vil compile --from zig --input app.vil.zig --output {name}

# Run
./{name}
```"#,
            name = config.name
        ),
        _ => format!(
            r#"```bash
# Visualize
vil viz app.vil.yaml --open

# Validate
vil check app.vil.yaml

# Build native binary
vil compile --from yaml --input app.vil.yaml --release

# Run
vil run --file app.vil.yaml
```"#
        ),
    };

    let structure = match config.lang.as_str() {
        "python" => format!(
            r#"```
{name}/
├── app.vil.yaml          <- YAML manifest
├── app.vil.py            <- Python SDK pipeline (edit this)
├── requirements.txt
└── README.md
```"#,
            name = config.name
        ),
        "go" => format!(
            r#"```
{name}/
├── app.vil.yaml          <- YAML manifest
├── main.go               <- Go SDK pipeline (edit this)
├── go.mod
└── README.md
```"#,
            name = config.name
        ),
        "java" => format!(
            r#"```
{name}/
├── app.vil.yaml          <- YAML manifest
├── App.java              <- Java SDK pipeline (edit this)
├── pom.xml
└── README.md
```"#,
            name = config.name
        ),
        "typescript" => format!(
            r#"```
{name}/
├── app.vil.yaml          <- YAML manifest
├── app.vil.ts            <- TypeScript SDK pipeline (edit this)
├── package.json
└── README.md
```"#,
            name = config.name
        ),
        "csharp" => format!(
            r#"```
{name}/
├── app.vil.yaml          <- YAML manifest
├── app.vil.cs            <- C# SDK pipeline (edit this)
├── {name}.csproj
└── README.md
```"#,
            name = config.name
        ),
        "kotlin" => format!(
            r#"```
{name}/
├── app.vil.yaml          <- YAML manifest
├── app.vil.kt            <- Kotlin SDK pipeline (edit this)
├── build.gradle.kts
└── README.md
```"#,
            name = config.name
        ),
        "swift" => format!(
            r#"```
{name}/
├── app.vil.yaml          <- YAML manifest
├── app.vil.swift         <- Swift SDK pipeline (edit this)
├── Package.swift
└── README.md
```"#,
            name = config.name
        ),
        "zig" => format!(
            r#"```
{name}/
├── app.vil.yaml          <- YAML manifest
├── app.vil.zig           <- Zig SDK pipeline (edit this)
├── build.zig
└── README.md
```"#,
            name = config.name
        ),
        _ => format!(
            r#"```
{name}/
├── app.vil.yaml          <- application manifest (edit this)
├── src/
│   ├── main.rs             <- auto-generated (don't edit)
│   └── handlers/           <- your custom logic (edit these)
├── Cargo.toml              <- auto-generated
└── README.md
```"#,
            name = config.name
        ),
    };

    format!(
        r#"# {name}

{desc}

Generated by `vil init {name}{lang_flag} --template {tmpl}`.

## Quick Start

{quick_start}

## Test

```bash
curl -N -X POST http://localhost:{port}/trigger \
  -H "Content-Type: application/json" \
  -d '{{"prompt": "hello"}}'
```

## Project Structure

{structure}
"#,
        name = config.name,
        desc = template.description,
        tmpl = template.id,
        lang_flag = lang_flag,
        port = config.port,
        quick_start = quick_start,
        structure = structure,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(observer: bool) -> ProjectConfig {
        ProjectConfig {
            name: "test-app".into(),
            lang: "rust".into(),
            port: 8080,
            upstream: "http://localhost:3000".into(),
            token: "shm".into(),
            observer,
        }
    }

    #[test]
    fn yaml_optional_fields_observer_true() {
        let c = test_config(true);
        let fields = yaml_optional_fields(&c);
        assert!(
            fields.contains("observer: true"),
            "should emit observer: true"
        );
    }

    #[test]
    fn yaml_optional_fields_observer_false() {
        let c = test_config(false);
        let fields = yaml_optional_fields(&c);
        assert!(
            !fields.contains("observer"),
            "should emit nothing when observer=false"
        );
    }

    #[test]
    fn yaml_ai_gateway_includes_observer_placeholder() {
        let c = test_config(true);
        let yaml = yaml_ai_gateway(&c);
        assert!(
            yaml.contains("observer: true"),
            "ai-gateway YAML must include observer: true\n{}",
            yaml
        );
    }

    #[test]
    fn yaml_rest_crud_includes_observer_placeholder() {
        let c = test_config(true);
        let yaml = yaml_rest_crud(&c);
        assert!(
            yaml.contains("observer: true"),
            "rest-crud YAML must include observer: true\n{}",
            yaml
        );
    }

    #[test]
    fn yaml_blank_includes_observer_placeholder() {
        let c = test_config(true);
        let yaml = yaml_blank(&c);
        assert!(
            yaml.contains("observer: true"),
            "blank YAML must include observer: true\n{}",
            yaml
        );
    }

    #[test]
    fn yaml_data_pipeline_includes_observer_placeholder() {
        let c = test_config(true);
        let yaml = yaml_data_pipeline(&c);
        assert!(
            yaml.contains("observer: true"),
            "data-pipeline YAML must include observer: true\n{}",
            yaml
        );
    }

    #[test]
    fn yaml_templates_omit_observer_when_false() {
        let c = test_config(false);
        let yaml = yaml_ai_gateway(&c);
        assert!(
            !yaml.contains("observer: true"),
            "should NOT contain observer: true when disabled"
        );
        // Should not have stray blank line from empty optional
        let yaml2 = yaml_blank(&c);
        assert!(!yaml2.contains("observer: true"));
    }
}

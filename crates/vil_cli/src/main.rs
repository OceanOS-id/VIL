use clap::{Parser, Subcommand};
use colored::*;

#[derive(Parser)]
#[command(name = "vil")]
#[command(about = "VIL - Zero-copy streaming pipelines for Rust", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new VIL project from template
    New {
        /// Project name
        name: String,

        /// Template to use (ai-inference, webhook-forwarder, event-fanout, stream-filter, load-balancer)
        #[arg(short, long, default_value = "ai-inference")]
        template: String,
    },

    /// Run a VIL pipeline
    Run {
        /// Path to pipeline file (YAML or default to examples/)
        #[arg(short, long)]
        file: Option<String>,

        /// Port to listen on
        #[arg(short, long, default_value = "3080")]
        port: u16,

        /// Use built-in mock backend (no external dependencies)
        #[arg(short, long)]
        mock: bool,
    },

    /// Run benchmark suite
    Bench {
        /// Number of requests
        #[arg(short, long, default_value = "1000")]
        requests: usize,

        /// Concurrent connections
        #[arg(short, long, default_value = "10")]
        concurrency: usize,
    },

    /// Initialize VIL pipeline in current directory (use vil server init for server)
    #[command(name = "init-legacy", hide = true)]
    InitLegacy {
        /// Project name
        name: Option<String>,
    },

    /// Inspect the global SHM registry
    Registry {
        /// Show active processes
        #[arg(short, long)]
        processes: bool,

        /// Show active ports
        #[arg(short = 'P', long)]
        ports: bool,

        /// Show active samples
        #[arg(short, long)]
        samples: bool,
    },

    /// Inspect shared memory regions
    Shm {
        /// List all regions
        #[arg(short, long)]
        list: bool,
    },

    /// Show high-resolution performance metrics (latency)
    Metrics,

    /// Development mode with auto-rebuild on file changes
    Dev {
        /// Port to use
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Package name (reads from Cargo.toml if not specified)
        #[arg(short = 'P', long)]
        package: Option<String>,

        /// Watch interval in milliseconds
        #[arg(long, default_value = "1000")]
        interval: u64,
    },

    /// Explain a VIL error code
    Explain {
        /// Error code (e.g., E-VIL-0001)
        code: String,
    },

    /// Validate a YAML pipeline file without running it
    Validate {
        /// Path to pipeline YAML file
        file: String,
    },

    /// Visualize workflow topology and DAG
    Viz {
        /// Input YAML file
        input: String,
        /// Output format: html, svg, mermaid, dot, json, ascii
        #[arg(short, long, default_value = "html")]
        format: String,
        /// Output file (stdout if not specified)
        #[arg(short, long)]
        output: Option<String>,
        /// Show Tri-Lane types on edges
        #[arg(long)]
        show_lanes: bool,
        /// Show host placement
        #[arg(long)]
        show_topology: bool,
        /// Show port names
        #[arg(long)]
        show_ports: bool,
        /// Show message types on edges
        #[arg(long)]
        show_messages: bool,
        /// Expand task DAGs inside nodes
        #[arg(long)]
        show_workflows: bool,
        /// Show all details
        #[arg(long)]
        show_all: bool,
        /// Zoom level: topology, dag, full
        #[arg(long, default_value = "topology")]
        level: String,
        /// Open output in browser (html format)
        #[arg(long)]
        open: bool,
        /// Meta-view: scan directory for all workflow files, show file-level call graph
        #[arg(long)]
        call_graph: Option<String>,
        /// Inline-expand call: targets as nested subgraphs
        #[arg(long)]
        expand_calls: bool,
        /// Watch YAML file for changes and auto-re-render
        #[arg(long)]
        watch: bool,
        /// Color theme: light, dark, auto
        #[arg(long, default_value = "auto")]
        theme: String,
        /// Show failover pairs with dashed highlight
        #[arg(long)]
        show_failover: bool,
        /// Show transport type on cross-host edges (SHM/TCP/QUIC)
        #[arg(long)]
        show_transport: bool,
        /// Zoom into specific node's DAG (with --level dag)
        #[arg(long)]
        node: Option<String>,
    },

    /// Launch the real-time web dashboard (http://localhost:8081)
    Dashboard,

    /// Generate a new vil-server project
    Server {
        #[command(subcommand)]
        action: ServerAction,
    },

    /// Initialize VIL pipeline in current directory
    /// Initialize a new VIL project (YAML + Rust + handlers)
    Init {
        /// Project name (creates directory)
        name: Option<String>,

        /// Template: ai-gateway, rest-crud, multi-model-router, rag-pipeline,
        /// websocket-chat, grpc-service, wasm-faas, agent, blank
        #[arg(short, long)]
        template: Option<String>,

        /// Language: rust (default), python, go, java, typescript
        #[arg(short, long)]
        lang: Option<String>,

        /// Token type: shm (multi-pipeline, default) or generic (single pipeline)
        #[arg(long)]
        token: Option<String>,

        /// Listen port
        #[arg(short, long)]
        port: Option<u16>,

        /// Upstream URL (for pipeline templates)
        #[arg(long)]
        upstream: Option<String>,

        /// Interactive wizard mode (default if no --template given)
        #[arg(long)]
        wizard: bool,
    },

    /// List available project templates (--sync to download for offline use)
    Templates {
        /// Download all templates from GitHub to VASTAR_HOME
        #[arg(long)]
        sync: bool,
    },

    /// Generate VilORM code: model, service, migration, or full resource
    ///
    /// Examples:
    ///   vil gen model Profile username:string xp:integer
    ///   vil gen migration add_profiles
    ///   vil gen resource profiles username:string xp:integer
    #[command(name = "gen")]
    Gen {
        /// What to generate: model, service, migration, resource
        kind: String,
        /// Name of the model/service/migration
        name: String,
        /// Field definitions (name:type pairs)
        #[arg(trailing_var_arg = true)]
        fields: Vec<String>,
    },

    /// VilORM — Generate project from SQL schema
    ///
    /// Examples:
    ///   vil orm gen all --schema schema.sql --output my-app
    ///   vil orm gen all --schema schema.sql --name toefl-quiz
    ///   vil orm gen model --schema schema.sql --table profiles
    #[command(name = "orm")]
    Orm {
        #[command(subcommand)]
        action: OrmAction,
    },

    /// Export YAML manifest from Rust source (golden reference for SDK validation)
    ///
    /// Examples:
    ///   vil export-manifest --source examples/004/src/main.rs
    ///   vil export-manifest --source src/main.rs --output manifest.yaml
    #[command(name = "export-manifest")]
    ExportManifest {
        /// Rust source file path
        #[arg(long)]
        source: String,
        /// Output file (default: stdout)
        #[arg(long, short)]
        output: Option<String>,
    },

    /// Deploy to remote server: build release → scp → restart → health check
    ///
    /// Examples:
    ///   vil deploy --host 10.10.0.14 --user app --path /opt/my-app
    ///   vil deploy                  (uses .vil-deploy.toml)
    ///   vil deploy init             (create .vil-deploy.toml)
    ///   vil deploy status           (check remote health)
    ///   vil deploy rollback         (restore previous binary)
    Deploy {
        /// Subcommand: init, status, rollback (default: deploy)
        #[arg(default_value = "run")]
        action: String,
        /// Remote host IP or hostname
        #[arg(long)]
        host: Option<String>,
        /// Remote SSH user
        #[arg(long)]
        user: Option<String>,
        /// Remote binary path
        #[arg(long)]
        path: Option<String>,
        /// Systemd service name (default: package name)
        #[arg(long)]
        service: Option<String>,
    },

    /// Build a VIL service into a deployable artifact
    Build {
        /// Build target (vlb for vflow-server provisioning)
        #[arg(short, long, default_value = "binary")]
        target: String,

        /// Build in release mode
        #[arg(long)]
        release: bool,

        /// Output path for the artifact
        #[arg(short, long)]
        output: Option<String>,

        /// Service name (reads from Cargo.toml if not specified)
        #[arg(short, long)]
        name: Option<String>,

        /// Service version
        #[arg(short, long, default_value = "0.1.0")]
        version: String,
    },

    /// Check system readiness for vil-server
    Doctor,

    /// Trace request flow through VIL services
    Trace {
        /// Trace mode
        #[arg(long, default_value = "live")]
        mode: String,

        /// Target host (for remote tracing)
        #[arg(long, default_value = "http://localhost:8080")]
        host: String,

        /// Filter by service name
        #[arg(long)]
        service: Option<String>,

        /// Maximum events to show (0 = unlimited)
        #[arg(long, default_value = "0")]
        max_events: usize,
    },

    /// Export project topology as YAML
    Export {
        /// Output format
        #[arg(long, default_value = "yaml")]
        format: String,

        /// Output file (stdout if not specified)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Scaffold Rust code from YAML topology
    Scaffold {
        /// Input YAML file
        input: String,

        /// Output Rust file (stdout if not specified)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Compile a manifest or source file into a native Rust binary
    Compile {
        /// Source language (python, typescript, go, java, yaml)
        #[arg(long)]
        from: String,

        /// Input file
        #[arg(long)]
        input: String,

        /// Output binary name
        #[arg(long)]
        output: Option<String>,

        /// Build in release mode
        #[arg(long)]
        release: bool,

        /// Output target: "binary" (default) or "vlb" (also produces .vlb artifact)
        #[arg(long, default_value = "binary")]
        target: String,

        /// Save the generated YAML manifest next to the source file (as <file>.vil.yaml)
        #[arg(long)]
        save_manifest: bool,

        /// Compile inside Docker container (no local Rust toolchain needed)
        #[arg(long)]
        docker: bool,
    },

    /// Provision services to vflow-server
    Provision {
        #[command(subcommand)]
        action: ProvisionAction,
    },

    /// Inspect VIL service topology or VLB artifact
    Inspect {
        /// Path to .vlb file (if inspecting artifact)
        #[arg(short, long)]
        file: Option<String>,

        /// Show full contract JSON
        #[arg(long)]
        contract: bool,

        /// Show service routes
        #[arg(long)]
        routes: bool,

        /// Show process list
        #[arg(long)]
        processes: bool,

        /// Show message schemas
        #[arg(long)]
        schemas: bool,
    },

    /// Manage VIL sidecars (list, health, attach, drain)
    Sidecar {
        #[command(subcommand)]
        action: SidecarAction,
    },

    /// Generate code scaffolds from YAML manifest
    Generate {
        #[command(subcommand)]
        action: GenerateAction,
    },

    /// Manage WASM function modules
    Wasm {
        #[command(subcommand)]
        action: WasmAction,
    },

    /// List available built-in node types
    #[command(name = "node")]
    NodeCmd {
        /// Filter by category (ai, agent, knowledge, safety, database, etc.)
        #[arg(long)]
        category: Option<String>,
        /// Show default ports for each node type
        #[arg(long)]
        ports: bool,
    },

    /// Run workflow tests with fixture data
    Test {
        /// Path to YAML manifest
        manifest: String,
        /// Path to JSON fixture file
        #[arg(long)]
        input: String,
        /// Specific workflow to test (default: first workflow)
        #[arg(long)]
        workflow: Option<String>,
    },

    /// Comprehensive manifest validation (handlers, scripts, DAG cycles, types)
    Check {
        /// Path to YAML manifest
        manifest: String,
    },

    /// Manage VIL SDK (pre-compiled engine for closed-source distribution)
    Sdk {
        #[command(subcommand)]
        action: SdkAction,
    },
}

#[derive(Subcommand)]
enum OrmAction {
    /// Generate project/models/services from SQL schema
    Gen {
        /// What to generate: all, model, service
        #[arg(default_value = "all")]
        target: String,

        /// SQL schema file path
        #[arg(long)]
        schema: String,

        /// Output directory (default: current dir)
        #[arg(long, short)]
        output: Option<String>,

        /// Project name (default: derived from dir)
        #[arg(long)]
        name: Option<String>,

        /// Generate only this table (for model/service target)
        #[arg(long)]
        table: Option<String>,
    },
}

#[derive(Subcommand)]
enum SdkAction {
    /// Download and install SDK to ~/.vil/sdk/
    Install {
        /// SDK version (default: latest)
        #[arg(long, default_value = "0.1.0")]
        version: String,
    },
    /// Show installed SDK information
    Info,
    /// Print SDK path (for scripts)
    Path,
    /// List installed SDK versions
    List,
}

#[derive(Subcommand)]
enum WasmAction {
    /// Scaffold a new WASM module project
    Scaffold {
        /// Module name
        name: String,
        /// Language: rust, c, go, assemblyscript
        #[arg(long, default_value = "rust")]
        language: String,
        /// Output directory (default: wasm-src/)
        #[arg(short, long, default_value = "wasm-src")]
        output: String,
    },
    /// Build WASM modules declared in manifest
    Build {
        /// Path to YAML manifest
        manifest: String,
        /// Build only this module (default: all)
        #[arg(long)]
        module: Option<String>,
    },
    /// List WASM modules and functions declared in manifest
    List {
        /// Path to YAML manifest
        manifest: String,
    },
}

#[derive(Subcommand)]
enum GenerateAction {
    /// Scaffold a Rust handler function with typed signature from YAML
    Handler {
        /// Handler function name (must match a node's code.handler field)
        name: String,
        /// Path to YAML manifest
        #[arg(long)]
        from: String,
        /// Output directory (default: src/handlers/)
        #[arg(short, long, default_value = "src/handlers")]
        output: String,
    },
    /// Scaffold a script template (Lua/JS/WASM) from YAML
    Script {
        /// Script name
        name: String,
        /// Runtime: lua, js, wasm
        #[arg(long, default_value = "lua")]
        runtime: String,
        /// Path to YAML manifest
        #[arg(long)]
        from: String,
        /// Output directory (default: scripts/)
        #[arg(short, long, default_value = "scripts")]
        output: String,
    },
}

#[derive(Subcommand)]
enum ProvisionAction {
    /// Push a .vlb artifact to vflow-server
    Push {
        /// vflow-server host URL
        #[arg(long, default_value = "http://localhost:8080")]
        host: String,
        /// Path to .vlb artifact
        #[arg(long)]
        artifact: String,
    },
    /// Activate a provisioned service
    Activate {
        #[arg(long, default_value = "http://localhost:8080")]
        host: String,
        /// Service name
        #[arg(long)]
        service: String,
    },
    /// Drain a service (graceful shutdown)
    Drain {
        #[arg(long, default_value = "http://localhost:8080")]
        host: String,
        #[arg(long)]
        service: String,
    },
    /// Deactivate a service
    Deactivate {
        #[arg(long, default_value = "http://localhost:8080")]
        host: String,
        #[arg(long)]
        service: String,
    },
    /// List all services on vflow-server
    List {
        #[arg(long, default_value = "http://localhost:8080")]
        host: String,
    },
    /// Show vflow-server contract/topology
    Contract {
        #[arg(long, default_value = "http://localhost:8080")]
        host: String,
    },
    /// Check vflow-server health
    Health {
        #[arg(long, default_value = "http://localhost:8080")]
        host: String,
    },
}

#[derive(Subcommand)]
enum ServerAction {
    /// Create a new vil-server project from template
    New {
        /// Project name
        name: String,

        /// Template: hello, crud, grpc, nats, kafka, mqtt, multiservice, graphql, fullstack
        #[arg(short, long, default_value = "hello")]
        template: String,
    },

    /// Initialize vil-server in current directory
    Init {
        /// Template: hello, crud, grpc, nats, kafka, mqtt, multiservice, graphql, fullstack
        #[arg(short, long, default_value = "hello")]
        template: String,
    },

    /// Run in dev mode with auto-restart on file changes
    Dev {
        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
}

#[derive(Subcommand)]
enum SidecarAction {
    /// List all registered sidecars with health status
    List {
        /// VilApp host URL
        #[arg(long, default_value = "http://localhost:8080")]
        host: String,
    },
    /// Check health of a specific sidecar
    Health {
        /// Sidecar name
        name: String,
        /// VilApp host URL
        #[arg(long, default_value = "http://localhost:8080")]
        host: String,
    },
    /// Attach an external sidecar to a running VilApp
    Attach {
        /// Sidecar name
        name: String,
        /// Unix domain socket path
        #[arg(long)]
        socket: String,
        /// VilApp host URL
        #[arg(long, default_value = "http://localhost:8080")]
        host: String,
    },
    /// Gracefully drain a sidecar
    Drain {
        /// Sidecar name
        name: String,
        /// VilApp host URL
        #[arg(long, default_value = "http://localhost:8080")]
        host: String,
    },
    /// Show sidecar metrics
    Metrics {
        /// VilApp host URL
        #[arg(long, default_value = "http://localhost:8080")]
        host: String,
    },
}

mod call_resolver;
mod checker;
mod codegen;
mod compiler;
mod deploy;
mod dev_mode;
mod generate;
mod doctor;
mod error_catalog;
mod errors;
mod gen_scaffold;
mod hot_reload;
mod manifest;
mod mock_server;
mod node_types;
mod pipeline_init;
mod project_init;
mod provision;
mod runner;
mod sdk_manager;
mod server_dev;
mod server_scaffold;
mod templates;
mod test_runner;
mod tracer;
mod transform_builder;
mod viz_bridge;
mod vlb_builder;
mod vlb_inspector;
mod wasm_builder;
mod yaml_pipeline;
mod yaml_tools;
mod orm_cmd;

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::New { name, template } => {
            if let Err(e) = templates::create_project(name, template) {
                eprintln!("{} {}", "Error:".red().bold(), e);
                std::process::exit(1);
            }
            println!(
                "{} Created new VIL project '{}' from template '{}'",
                "✓".green().bold(),
                name,
                template
            );
            println!("\nTo get started:");
            println!("  cd {}", name);
            println!("  vil run");
        }

        Commands::Run { file, port, mock } => {
            // Check if file is a YAML pipeline
            if let Some(path) = file {
                if path.ends_with(".yaml") || path.ends_with(".yml") || path.ends_with(".vil.yaml")
                {
                    println!("{} Running YAML pipeline: {}", "✓".green().bold(), path);
                    if let Err(e) = yaml_pipeline::run_yaml_pipeline(path, Some(*port)) {
                        eprintln!("{} {}", "Error:".red().bold(), e);
                        std::process::exit(1);
                    }
                    return;
                }
            }

            // Check if we're in a VIL project directory
            let is_project = std::path::Path::new("Cargo.toml").exists();

            if is_project && file.is_none() && !*mock {
                // Run cargo run in current directory
                println!("{} Running project with cargo run", "✓".green().bold());
                std::process::Command::new("cargo")
                    .arg("run")
                    .args(std::env::args().skip(2))
                    .spawn()
                    .expect("Failed to run cargo")
                    .wait()
                    .expect("Failed to wait for cargo");
            } else if *mock {
                println!("{} Starting with built-in mock backend", "✓".green().bold());
                if let Err(e) = runner::run_with_mock(*port) {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    std::process::exit(1);
                }
            } else if let Some(path) = file {
                println!("{} Running pipeline from {}", "✓".green().bold(), path);
                if let Err(e) = runner::run_from_file(path, *port) {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    std::process::exit(1);
                }
            } else {
                println!("{} Running AI Gateway demo", "✓".green().bold());
                println!("  Listening on http://localhost:{}", *port);
                println!("  Press Ctrl+C to stop");

                if let Err(e) = runner::run_demo(*port) {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Bench {
            requests,
            concurrency,
        } => {
            println!(
                "{} Running benchmark ({} requests, {} concurrent)",
                "✓".green().bold(),
                requests,
                concurrency
            );
            if let Err(e) = runner::run_benchmark(*requests, *concurrency) {
                eprintln!("{} {}", "Error:".red().bold(), e);
                std::process::exit(1);
            }
        }

        Commands::InitLegacy { name } => {
            let project_name = name.clone().unwrap_or_else(|| "my-vil-project".to_string());
            if let Err(e) = templates::init_project(&project_name) {
                eprintln!("{} {}", "Error:".red().bold(), e);
                std::process::exit(1);
            }
            println!(
                "{} Initialized VIL project '{}'",
                "✓".green().bold(),
                project_name
            );
        }

        Commands::Init {
            name,
            template,
            lang,
            token,
            port,
            upstream,
            wizard,
        } => {
            if let Err(e) = project_init::run_init(project_init::InitArgs {
                name: name.clone(),
                template: template.clone(),
                lang: lang.clone(),
                token: token.clone(),
                port: *port,
                upstream: upstream.clone(),
                wizard: *wizard || template.is_none(),
            }) {
                eprintln!("{} {}", "Error:".red().bold(), e);
                std::process::exit(1);
            }
        }

        Commands::Gen { kind, name, fields } => {
            if let Err(e) = generate::run_generate(kind, name, fields) {
                eprintln!("{} {}", "Error:".red().bold(), e);
                std::process::exit(1);
            }
        }

        Commands::Orm { action } => {
            match action {
                OrmAction::Gen { target, schema, output, name, table } => {
                    if let Err(e) = orm_cmd::run_orm_gen(&target, &schema, output.as_deref(), name.as_deref(), table.as_deref()) {
                        eprintln!("{} {}", "Error:".red().bold(), e);
                        std::process::exit(1);
                    }
                }
            }
        }

        Commands::ExportManifest { source, output } => {
            if let Err(e) = orm_cmd::run_export_manifest(source, output.as_deref()) {
                eprintln!("{} {}", "Error:".red().bold(), e);
                std::process::exit(1);
            }
        }

        Commands::Deploy { action, host, user, path, service } => {
            if let Err(e) = deploy::run_deploy(
                action,
                host.as_deref(),
                user.as_deref(),
                path.as_deref(),
                service.as_deref(),
            ) {
                eprintln!("{} {}", "Error:".red().bold(), e);
                std::process::exit(1);
            }
        }

        Commands::Templates { sync } => {
            if *sync {
                if let Err(e) = project_init::sync_templates() {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    std::process::exit(1);
                }
            } else {
                if let Err(e) = project_init::list_templates() {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Registry {
            processes,
            ports,
            samples,
        } => {
            run_registry(*processes, *ports, *samples);
        }

        Commands::Shm { list } => {
            if *list {
                run_shm_list();
            }
        }

        Commands::Metrics => {
            run_metrics();
        }

        Commands::Dev {
            port,
            package,
            interval,
        } => {
            dev_mode::run_dev(dev_mode::DevConfig {
                port: *port,
                package: package.clone(),
                interval: *interval,
            })
            .unwrap_or_else(|e| eprintln!("Dev mode error: {}", e));
        }

        Commands::Explain { code } => {
            error_catalog::explain(code).unwrap_or_else(|e| eprintln!("Explain error: {}", e));
        }

        Commands::Validate { file } => {
            if let Err(e) = yaml_pipeline::validate_yaml_pipeline(file) {
                eprintln!("{} {}", "Error:".red().bold(), e);
                std::process::exit(1);
            }
        }

        Commands::Viz {
            input,
            format,
            output,
            show_lanes,
            show_topology,
            show_ports,
            show_messages,
            show_workflows,
            show_all,
            level,
            open,
            call_graph,
            expand_calls,
            watch,
            theme: _,
            show_failover: _,
            show_transport: _,
            node: _,
        } => {
            let viz_args = viz_bridge::VizArgs {
                input: input.clone(),
                format: format.clone(),
                output: output.clone(),
                show_lanes: *show_lanes || *show_all,
                show_topology: *show_topology || *show_all,
                show_ports: *show_ports || *show_all,
                show_messages: *show_messages || *show_all,
                show_workflows: *show_workflows || *show_all,
                level: level.clone(),
                open: *open,
                call_graph: call_graph.clone(),
                expand_calls: *expand_calls,
            };

            if *watch {
                // Watch mode: re-render on YAML change
                eprintln!(
                    "{} Watching {} for changes (Ctrl+C to stop)",
                    "WATCH".cyan().bold(),
                    input
                );
                let mut watcher = hot_reload::FileWatcher::new(1000);
                watcher.watch(input.as_str(), hot_reload::WatchKind::Yaml);
                let input_clone = input.clone();
                let args_format = format.clone();
                let args_output = output.clone();
                // Initial render
                if let Err(e) = viz_bridge::run_viz(viz_args) {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                }
                // Watch loop
                let handle = watcher.start(move |entry| {
                    eprintln!(
                        "{} {} changed, re-rendering...",
                        "RELOAD".yellow().bold(),
                        entry.path.display()
                    );
                    let args = viz_bridge::VizArgs {
                        input: input_clone.clone(),
                        format: args_format.clone(),
                        output: args_output.clone(),
                        show_lanes: false,
                        show_topology: false,
                        show_ports: false,
                        show_messages: false,
                        show_workflows: false,
                        level: "topology".into(),
                        open: false,
                        call_graph: None,
                        expand_calls: false,
                    };
                    if let Err(e) = viz_bridge::run_viz(args) {
                        eprintln!("{} {}", "Error:".red().bold(), e);
                    }
                });
                handle.join().expect("Watcher thread panicked");
            } else {
                if let Err(e) = viz_bridge::run_viz(viz_args) {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Dashboard => {
            println!(
                "{} Starting dashboard on http://localhost:8081",
                "✓".green().bold()
            );
            // Dashboard functionality requires runtime to be running
            println!(
                "{}",
                "Note: Dashboard requires a running pipeline.".yellow()
            );
        }

        Commands::Build {
            target,
            release,
            output,
            name,
            version,
        } => {
            if target == "vlb" {
                match vlb_builder::build_vlb(vlb_builder::VlbBuildConfig {
                    target: target.clone(),
                    release: *release,
                    output: output.clone(),
                    name: name.clone(),
                    version: version.clone(),
                }) {
                    Ok(path) => println!("Build complete: {}", path),
                    Err(e) => {
                        eprintln!("Build failed: {}", e);
                        std::process::exit(1);
                    }
                }
            } else if target == "binary" {
                // Standard cargo build
                let mut cmd = std::process::Command::new("cargo");
                cmd.arg("build");
                if *release {
                    cmd.arg("--release");
                }
                let status = cmd.status().expect("Failed to run cargo build");
                if !status.success() {
                    std::process::exit(1);
                }
            } else {
                eprintln!("Unknown build target: {}. Use 'vlb' or 'binary'", target);
                std::process::exit(1);
            }
        }

        Commands::Doctor => {
            doctor::run_doctor();
        }

        Commands::Inspect {
            file,
            contract,
            routes,
            processes,
            schemas,
        } => {
            if let Some(path) = file {
                if let Err(e) =
                    vlb_inspector::inspect_vlb(path, *contract, *routes, *processes, *schemas)
                {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            } else {
                if let Err(e) =
                    vlb_inspector::inspect_project(*contract, *routes, *processes, *schemas)
                {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Trace {
            mode,
            host,
            service,
            max_events,
        } => {
            if let Err(e) = tracer::trace_live(tracer::TraceConfig {
                mode: mode.clone(),
                host: host.clone(),
                service: service.clone(),
                max_events: *max_events,
            }) {
                eprintln!("{} {}", "Error:".red().bold(), e);
                std::process::exit(1);
            }
        }

        Commands::Export { format: _, output } => {
            if let Err(e) = yaml_tools::export_yaml(yaml_tools::ExportConfig {
                output: output.clone(),
            }) {
                eprintln!("{} {}", "Error:".red().bold(), e);
                std::process::exit(1);
            }
        }

        Commands::Scaffold { input, output } => {
            if let Err(e) = yaml_tools::scaffold_yaml(yaml_tools::ScaffoldConfig {
                input: input.clone(),
                output: output.clone(),
            }) {
                eprintln!("{} {}", "Error:".red().bold(), e);
                std::process::exit(1);
            }
        }

        Commands::Compile {
            from,
            input,
            output,
            release,
            target,
            save_manifest,
            docker,
        } => {
            if *docker {
                // Docker-based compilation
                println!(
                    "{} Compiling inside Docker container...",
                    "DOCKER".cyan().bold()
                );
                let status = std::process::Command::new("docker")
                    .args([
                        "run",
                        "--rm",
                        "-v",
                        &format!("{}:/workspace", std::env::current_dir().unwrap().display()),
                        "-w",
                        "/workspace",
                        "vil/compiler:latest",
                        "--from",
                        from,
                        "--input",
                        input,
                    ])
                    .status();
                match status {
                    Ok(s) if s.success() => {
                        return;
                    }
                    Ok(s) => {
                        eprintln!(
                            "{} Docker compile failed (exit {})",
                            "Error:".red().bold(),
                            s.code().unwrap_or(-1)
                        );
                        std::process::exit(1);
                    }
                    Err(e) => {
                        eprintln!("{} Docker not available: {}", "Error:".red().bold(), e);
                        eprintln!("  Install Docker or compile without --docker flag");
                        std::process::exit(1);
                    }
                }
            }
            if let Err(e) = compiler::run_compile(compiler::CompileConfig {
                from: from.clone(),
                input: input.clone(),
                output: output.clone(),
                release: *release,
                target_vlb: target == "vlb",
                save_manifest: *save_manifest,
            }) {
                eprintln!("{} {}", "Error:".red().bold(), e);
                std::process::exit(1);
            }
        }

        Commands::Provision { action } => {
            let paction = match action {
                ProvisionAction::Push { host, artifact } => provision::Action::Push {
                    host: host.clone(),
                    artifact: artifact.clone(),
                },
                ProvisionAction::Activate { host, service } => provision::Action::Activate {
                    host: host.clone(),
                    service: service.clone(),
                },
                ProvisionAction::Drain { host, service } => provision::Action::Drain {
                    host: host.clone(),
                    service: service.clone(),
                },
                ProvisionAction::Deactivate { host, service } => provision::Action::Deactivate {
                    host: host.clone(),
                    service: service.clone(),
                },
                ProvisionAction::List { host } => provision::Action::List { host: host.clone() },
                ProvisionAction::Contract { host } => {
                    provision::Action::Contract { host: host.clone() }
                }
                ProvisionAction::Health { host } => {
                    provision::Action::Health { host: host.clone() }
                }
            };
            if let Err(e) = provision::run_provision(paction) {
                eprintln!("{} {}", "Error:".red().bold(), e);
                std::process::exit(1);
            }
        }

        Commands::Server { action } => match action {
            ServerAction::New { name, template } => {
                if let Err(e) = server_scaffold::create_server_project(name, template) {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    std::process::exit(1);
                }
                println!(
                    "{} Created vil-server project '{}'",
                    "✓".green().bold(),
                    name
                );
                println!("\nTo get started:");
                println!("  cd {}", name);
                println!("  cargo run");
            }
            ServerAction::Init { template } => {
                println!(
                    "{} Initializing vil-server in current directory",
                    "✓".green().bold()
                );
                if let Err(e) = server_scaffold::init_server_in_current_dir(template) {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    std::process::exit(1);
                }
                println!("{} Server initialized. Run: cargo run", "✓".green().bold());
            }
            ServerAction::Dev { port } => {
                if let Err(e) = server_dev::run_dev_mode(*port) {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    std::process::exit(1);
                }
            }
        },

        Commands::Sidecar { action } => match action {
            SidecarAction::List { host } => {
                println!("{}", "=== VIL SIDECARS ===".green().bold());
                println!("  Querying {} ...", host);
                match reqwest::blocking::get(format!("{}/admin/sidecars", host)) {
                    Ok(resp) => {
                        if let Ok(text) = resp.text() {
                            println!("{}", text);
                        } else {
                            println!("  {}", "(no response body)".yellow());
                        }
                    }
                    Err(e) => {
                        eprintln!("  {} Could not reach host: {}", "✗".red(), e);
                        eprintln!("  Make sure VilApp is running with sidecars registered.");
                    }
                }
            }
            SidecarAction::Health { name, host } => {
                println!("{} Checking sidecar '{}' health...", "●".cyan(), name);
                match reqwest::blocking::get(format!("{}/admin/sidecars/{}", host, name)) {
                    Ok(resp) => {
                        if let Ok(text) = resp.text() {
                            println!("{}", text);
                        }
                    }
                    Err(e) => {
                        eprintln!("  {} {}", "✗".red(), e);
                    }
                }
            }
            SidecarAction::Attach { name, socket, host } => {
                println!(
                    "{} Attaching sidecar '{}' via {} ...",
                    "●".cyan(),
                    name,
                    socket
                );
                let client = reqwest::blocking::Client::new();
                let body = serde_json::json!({
                    "name": name,
                    "socket": socket,
                });
                match client
                    .post(format!("{}/admin/sidecars/{}/attach", host, name))
                    .json(&body)
                    .send()
                {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            println!(
                                "  {} Sidecar '{}' attached successfully",
                                "✓".green().bold(),
                                name
                            );
                        } else {
                            eprintln!("  {} Attach failed: {}", "✗".red(), resp.status());
                        }
                    }
                    Err(e) => eprintln!("  {} {}", "✗".red(), e),
                }
            }
            SidecarAction::Drain { name, host } => {
                println!("{} Draining sidecar '{}' ...", "●".cyan(), name);
                let client = reqwest::blocking::Client::new();
                match client
                    .post(format!("{}/admin/sidecars/{}/drain", host, name))
                    .send()
                {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            println!("  {} Sidecar '{}' drained", "✓".green().bold(), name);
                        } else {
                            eprintln!("  {} Drain failed: {}", "✗".red(), resp.status());
                        }
                    }
                    Err(e) => eprintln!("  {} {}", "✗".red(), e),
                }
            }
            SidecarAction::Metrics { host } => {
                println!("{}", "=== SIDECAR METRICS ===".green().bold());
                match reqwest::blocking::get(format!("{}/admin/sidecars/metrics", host)) {
                    Ok(resp) => {
                        if let Ok(text) = resp.text() {
                            println!("{}", text);
                        }
                    }
                    Err(e) => eprintln!("  {} {}", "✗".red(), e),
                }
            }
        },

        Commands::Generate { action } => match action {
            GenerateAction::Handler { name, from, output } => {
                if let Err(e) = gen_scaffold::generate_handler(name, from, output) {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    std::process::exit(1);
                }
            }
            GenerateAction::Script {
                name,
                runtime,
                from,
                output,
            } => {
                if let Err(e) = gen_scaffold::generate_script(name, runtime, from, output) {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    std::process::exit(1);
                }
            }
        },

        Commands::Wasm { action } => match action {
            WasmAction::Scaffold {
                name,
                language,
                output,
            } => {
                if let Err(e) = wasm_builder::scaffold_module(name, language, output) {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    std::process::exit(1);
                }
            }
            WasmAction::Build { manifest, module } => {
                if let Err(e) = wasm_builder::build_modules(manifest, module.as_deref()) {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    std::process::exit(1);
                }
            }
            WasmAction::List { manifest } => {
                if let Err(e) = wasm_builder::list_modules(manifest) {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    std::process::exit(1);
                }
            }
        },

        Commands::NodeCmd { category, ports } => {
            let types = node_types::list_node_types(category.as_deref());
            if types.is_empty() {
                println!(
                    "No node types found for category '{}'",
                    category.as_deref().unwrap_or("*")
                );
            } else {
                let mut current_cat = "";
                for entry in &types {
                    if entry.category != current_cat {
                        current_cat = entry.category;
                        println!(
                            "\n{} {}:",
                            "CATEGORY".cyan().bold(),
                            current_cat.to_uppercase()
                        );
                    }
                    println!(
                        "  {} {:20} {} ({})",
                        "type:".dimmed(),
                        entry.type_name.green(),
                        entry.description,
                        entry.crate_name.dimmed(),
                    );
                    if *ports {
                        for (pname, pdir, plane) in entry.default_ports {
                            let arrow = if *pdir == "in" { "◀" } else { "▶" };
                            println!("    {} {} {}", arrow, pname, plane.dimmed());
                        }
                    }
                }
                println!("\n{} types total", types.len());
            }
        }

        Commands::Test {
            manifest,
            input,
            workflow,
        } => {
            if let Err(e) = test_runner::run_test(manifest, input, workflow.as_deref()) {
                eprintln!("{} {}", "Error:".red().bold(), e);
                std::process::exit(1);
            }
        }

        Commands::Check { manifest } => {
            if let Err(_e) = checker::run_check(manifest) {
                std::process::exit(1);
            }
        }

        Commands::Sdk { action } => match action {
            SdkAction::Install { version } => {
                if let Err(e) = sdk_manager::install_sdk(version) {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    std::process::exit(1);
                }
            }
            SdkAction::Info => {
                if let Err(e) = sdk_manager::show_info() {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    std::process::exit(1);
                }
            }
            SdkAction::Path => {
                if let Err(e) = sdk_manager::show_path() {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    std::process::exit(1);
                }
            }
            SdkAction::List => {
                if let Err(e) = sdk_manager::list_versions() {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    std::process::exit(1);
                }
            }
        },
    }
}

fn run_registry(processes: bool, ports: bool, samples: bool) {
    println!("{}", "=== VIL SHM REGISTRY ===".green().bold());

    if !processes && !ports && !samples {
        println!("Use --processes, --ports, or --samples to view specific data");
        return;
    }

    let world = match vil_rt::VastarRuntimeWorld::new_shared() {
        Ok(w) => w,
        Err(_) => {
            println!(
                "{}",
                "No active VIL SHM runtime found. Start a pipeline first.".yellow()
            );
            return;
        }
    };

    if processes {
        println!("\n{}", "Processes:".yellow().bold());
        let procs = world.registry_processes();
        if procs.is_empty() {
            println!("  (none)");
        } else {
            for p in &procs {
                let status = if p.alive {
                    "alive".green()
                } else {
                    "dead".red()
                };
                println!("  {:>4}  {:<24} [{}]", p.id.0, p.name, status);
            }
        }
    }

    if ports {
        println!("\n{}", "Ports:".yellow().bold());
        let pts = world.registry_ports();
        if pts.is_empty() {
            println!("  (none)");
        } else {
            for p in &pts {
                println!(
                    "  {:>4}  proc={:<4} {:?}  {}",
                    p.id.0, p.process_id.0, p.direction, p.name
                );
            }
        }
    }

    if samples {
        println!("\n{}", "Samples:".yellow().bold());
        let samps = world.registry_samples();
        if samps.is_empty() {
            println!("  (none)");
        } else {
            println!("  {} active samples", samps.len());
            for s in samps.iter().take(20) {
                println!(
                    "  {:>6}  owner={:<4} host={:<4} published={}",
                    s.id.0, s.owner.0, s.origin_host.0, s.published
                );
            }
            if samps.len() > 20 {
                println!("  ... and {} more", samps.len() - 20);
            }
        }
    }
}

fn run_shm_list() {
    println!("{}", "=== VIL SHM REGIONS ===".green().bold());

    let world = match vil_rt::VastarRuntimeWorld::new_shared() {
        Ok(w) => w,
        Err(_) => {
            println!(
                "{}",
                "No active VIL SHM runtime found. Start a pipeline first.".yellow()
            );
            return;
        }
    };

    let stats = world.shm_stats();
    if stats.is_empty() {
        println!("  (no regions)");
    } else {
        println!(
            "  {:>6}  {:>12}  {:>12}  {:>12}",
            "Region", "Capacity", "Used", "Free"
        );
        for s in &stats {
            println!(
                "  {:>6}  {:>12}  {:>12}  {:>12}",
                s.region_id.0,
                format_bytes(s.capacity),
                format_bytes(s.used),
                format_bytes(s.remaining)
            );
        }
    }
}

fn run_metrics() {
    println!("{}", "=== VIL PERFORMANCE METRICS ===".green().bold());

    let world = match vil_rt::VastarRuntimeWorld::new_shared() {
        Ok(w) => w,
        Err(_) => {
            println!(
                "{}",
                "No active VIL SHM runtime found. Start a pipeline first.".yellow()
            );
            return;
        }
    };

    let counters = world.counters_snapshot();
    println!("\n{}", "Counters:".yellow().bold());
    println!("  Publishes:     {}", counters.publishes);
    println!("  Receives:      {}", counters.receives);
    println!("  Net Pulls:     {}", counters.net_pulls);
    println!("  Failovers:     {}", counters.failover_events);

    let latency = world.latency_snapshot();
    println!("\n{}", "Latency:".yellow().bold());
    println!("  Samples:       {}", latency.count);
    if latency.count > 0 {
        println!("  Min:           {:.3} us", latency.min_ns as f64 / 1000.0);
        println!("  Max:           {:.3} us", latency.max_ns as f64 / 1000.0);
        println!("  Mean:          {:.3} us", latency.mean_ns as f64 / 1000.0);
    }

    let metrics = world.metrics_snapshot();
    println!("\n{}", "Runtime:".yellow().bold());
    println!("  Queue depth:   {}", metrics.queue_depth_total);
    println!("  In-flight:     {}", metrics.in_flight_samples);
    println!("  Processes:     {}", metrics.registered_processes);
}

fn format_bytes(bytes: usize) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

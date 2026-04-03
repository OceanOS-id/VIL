// =============================================================================
// VIL CLI — Server Project Scaffolding
// =============================================================================
//
// `vil server new <name>` generates a ready-to-run vil-server project.

use std::fs;
use std::path::Path;

pub fn create_server_project(name: &str, template: &str) -> Result<(), String> {
    let project_dir = Path::new(name);
    if project_dir.exists() {
        return Err(format!("Directory '{}' already exists", name));
    }

    fs::create_dir_all(project_dir.join("src"))
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    // Write Cargo.toml
    let cargo_toml = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
vil_server = {{ git = "https://github.com/OceanOS-id/VIL.git" }}
tokio = {{ version = "1", features = ["full"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
"#
    );
    fs::write(project_dir.join("Cargo.toml"), cargo_toml)
        .map_err(|e| format!("Failed to write Cargo.toml: {}", e))?;

    // Write main.rs based on template
    let main_rs = match template {
        "crud" => generate_crud_template(name),
        "multiservice" => generate_multiservice_template(name),
        _ => generate_hello_template(name),
    };

    fs::write(project_dir.join("src/main.rs"), main_rs)
        .map_err(|e| format!("Failed to write src/main.rs: {}", e))?;

    // Write vil-server.yaml
    let yaml = generate_yaml(name, template);
    fs::write(project_dir.join("vil-server.yaml"), yaml)
        .map_err(|e| format!("Failed to write vil-server.yaml: {}", e))?;

    Ok(())
}

fn generate_hello_template(name: &str) -> String {
    format!(
        r#"use vil_server::prelude::*;

#[tokio::main]
async fn main() {{
    VilServer::new("{name}")
        .port(8080)
        .route("/", get(hello))
        .route("/greet/:name", get(greet))
        .run()
        .await;
}}

async fn hello() -> &'static str {{
    "Hello from {name}!"
}}

async fn greet(Path(name): Path<String>) -> Json<serde_json::Value> {{
    Json(serde_json::json!({{ "message": format!("Hello, {{}}!", name) }}))
}}
"#
    )
}

fn generate_crud_template(name: &str) -> String {
    format!(
        r#"use vil_server::prelude::*;
use std::sync::atomic::{{AtomicU64, Ordering}};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

#[tokio::main]
async fn main() {{
    let svc = ServiceProcess::new("items")
        .endpoint(Method::GET, "/items", get(list_items))
        .endpoint(Method::POST, "/items", post(create_item))
        .endpoint(Method::GET, "/items/:id", get(get_item))
        .endpoint(Method::DELETE, "/items/:id", delete(delete_item));

    VilApp::new("{name}")
        .port(8080)
        .service(svc)
        .run()
        .await;
}}

async fn list_items() -> VilResponse<Vec<Item>> {{
    VilResponse::ok(vec![
        Item {{ id: 1, name: "Example".into(), done: false }},
    ])
}}

async fn create_item(body: ShmSlice) -> VilResponse<Item> {{
    let input: CreateItem = body.json().expect("invalid JSON");
    let item = Item {{
        id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        name: input.name,
        done: false,
    }};
    VilResponse::created(item)
}}

async fn get_item(Path(id): Path<u64>) -> VilResponse<Item> {{
    VilResponse::ok(Item {{ id, name: "Example".into(), done: false }})
}}

async fn delete_item(Path(_id): Path<u64>) -> StatusCode {{
    StatusCode::NO_CONTENT
}}

#[derive(Serialize)]
struct Item {{
    id: u64,
    name: String,
    done: bool,
}}

#[derive(Deserialize)]
struct CreateItem {{
    name: String,
}}
"#
    )
}

fn generate_multiservice_template(name: &str) -> String {
    format!(
        r#"use vil_server::prelude::*;

#[tokio::main]
async fn main() {{
    VilServer::new("{name}")
        .port(8080)
        .metrics_port(9090)
        .service_def(api_service())
        .service_def(admin_service())
        .run()
        .await;
}}

fn api_service() -> ServiceDef {{
    let router = Router::new()
        .route("/hello", get(|| async {{ "Hello from API" }}));
    ServiceDef::new("api", router)
        .prefix("/api")
        .visibility(Visibility::Public)
}}

fn admin_service() -> ServiceDef {{
    let router = Router::new()
        .route("/status", get(|| async {{ "Admin OK" }}));
    ServiceDef::new("admin", router)
        .prefix("/admin")
        .visibility(Visibility::Internal)
}}
"#
    )
}

fn generate_yaml(name: &str, template: &str) -> String {
    match template {
        "multiservice" => format!(
            r#"server:
  name: {name}
  port: 8080
  metrics_port: 9090

services:
  - name: api
    visibility: public
    prefix: /api
  - name: admin
    visibility: internal
    prefix: /admin

mesh:
  mode: unified
  routes:
    - from: api
      to: admin
      lane: trigger
"#
        ),
        _ => format!(
            r#"server:
  name: {name}
  port: 8080

logging:
  level: info
  format: text
"#
        ),
    }
}

/// Initialize vil-server in the current directory (vil server init).
pub fn init_server_in_current_dir(template: &str) -> Result<(), String> {
    if !std::path::Path::new("Cargo.toml").exists() {
        return Err("No Cargo.toml found. Run 'cargo init' first, then 'vil server init'.".into());
    }

    // Generate main.rs based on template
    let main_rs = match template {
        "crud" => generate_crud_template("my-server"),
        "multiservice" => generate_multiservice_template("my-server"),
        "grpc" => generate_grpc_server_template(),
        "nats" => generate_nats_template(),
        "kafka" => generate_kafka_template(),
        "mqtt" => generate_mqtt_template(),
        "graphql" => generate_graphql_template(),
        "fullstack" => generate_fullstack_template(),
        _ => generate_hello_template("my-server"),
    };

    let main_path = std::path::Path::new("src/main.rs");
    if main_path.exists() {
        println!("  Warning: src/main.rs exists. Writing to src/vil_server_main.rs");
        fs::write("src/vil_server_main.rs", &main_rs).map_err(|e| e.to_string())?;
    } else {
        fs::create_dir_all("src").ok();
        fs::write(main_path, &main_rs).map_err(|e| e.to_string())?;
    }

    // Generate vil-server.yaml
    let yaml = generate_yaml("my-server", template);
    if !std::path::Path::new("vil-server.yaml").exists() {
        fs::write("vil-server.yaml", &yaml).map_err(|e| e.to_string())?;
    }

    println!("  Template: {}", template);
    println!(
        "  Add to Cargo.toml: vil_server = {{ git = \"https://github.com/OceanOS-id/VIL.git\" }}"
    );
    Ok(())
}

fn generate_grpc_server_template() -> String {
    r#"use vil_server::prelude::*;

#[tokio::main]
async fn main() {
    VilServer::new("grpc-server")
        .port(8080)
        .route("/", get(|| async { "gRPC + REST server" }))
        // TODO: Add gRPC service via vil_grpc::GrpcGatewayBuilder
        .run()
        .await;
}
"#
    .into()
}

fn generate_nats_template() -> String {
    r#"use vil_server::prelude::*;

#[tokio::main]
async fn main() {
    println!("Connecting to NATS...");
    // TODO: let nats = vil_mq_nats::NatsClient::connect(
    //     vil_mq_nats::NatsConfig::new("nats://localhost:4222")
    // ).await.unwrap();

    VilServer::new("nats-worker")
        .port(8080)
        .route("/", get(|| async { "NATS Worker" }))
        .route("/health", get(|| async { Json(serde_json::json!({"status": "ok"})) }))
        .run()
        .await;
}
"#
    .into()
}

fn generate_kafka_template() -> String {
    r#"use vil_server::prelude::*;

#[tokio::main]
async fn main() {
    println!("Connecting to Kafka...");
    // TODO: let producer = vil_mq_kafka::KafkaProducer::new(
    //     vil_mq_kafka::KafkaConfig::new("localhost:9092")
    // ).await.unwrap();

    VilServer::new("kafka-processor")
        .port(8080)
        .route("/", get(|| async { "Kafka Processor" }))
        .run()
        .await;
}
"#
    .into()
}

fn generate_mqtt_template() -> String {
    r#"use vil_server::prelude::*;

#[tokio::main]
async fn main() {
    println!("Connecting to MQTT broker...");
    // TODO: let mqtt = vil_mq_mqtt::MqttClient::new(
    //     vil_mq_mqtt::MqttConfig::new("mqtt://localhost:1883")
    // ).await.unwrap();

    VilServer::new("mqtt-gateway")
        .port(8080)
        .route("/", get(|| async { "MQTT IoT Gateway" }))
        .run()
        .await;
}
"#
    .into()
}

fn generate_graphql_template() -> String {
    r#"use vil_server::prelude::*;

#[tokio::main]
async fn main() {
    VilServer::new("graphql-api")
        .port(8080)
        .route("/", get(|| async { "GraphQL API — visit /graphql/playground" }))
        // TODO: Add GraphQL plugin via vil_graphql
        .run()
        .await;
}
"#
    .into()
}

fn generate_fullstack_template() -> String {
    r#"use vil_server::prelude::*;

#[tokio::main]
async fn main() {
    // Full-stack: HTTP + gRPC + NATS + DB + GraphQL
    VilServer::new("fullstack-app")
        .port(8080)
        .metrics_port(9090)
        .route("/", get(|| async { "VIL Fullstack — HTTP + gRPC + NATS + DB + GraphQL" }))
        // TODO: Add services, DB, NATS, Kafka, GraphQL
        .run()
        .await;
}
"#
    .into()
}

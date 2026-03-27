// =============================================================================
// VIL CLI — Pipeline Init (vil init --type <type>)
// =============================================================================
//
// Initialize a VIL pipeline in the current directory.
// Creates src/main.rs with appropriate boilerplate.

use std::path::Path;

pub fn init_pipeline(pipeline_type: &str) -> Result<(), String> {
    // Check Cargo.toml exists
    if !Path::new("Cargo.toml").exists() {
        return Err("No Cargo.toml found. Run 'cargo init' first, then 'vil init'.".into());
    }

    let main_rs = match pipeline_type {
        "sse" => generate_sse(),
        "websocket" => generate_websocket(),
        "grpc" => generate_grpc(),
        _ => generate_rest(), // default: rest
    };

    // Write src/main.rs (warn if exists)
    let main_path = Path::new("src/main.rs");
    if main_path.exists() {
        println!("  Warning: src/main.rs exists. Writing to src/vil_main.rs instead.");
        std::fs::write("src/vil_main.rs", &main_rs)
            .map_err(|e| format!("Failed to write: {}", e))?;
    } else {
        std::fs::create_dir_all("src").ok();
        std::fs::write(main_path, &main_rs).map_err(|e| format!("Failed to write: {}", e))?;
    }

    println!("  Type: {}", pipeline_type);
    println!("  Add to Cargo.toml: vil_sdk = {{ git = \"https://github.com/OceanOS-id/VIL.git\" }}");
    Ok(())
}

fn generate_rest() -> String {
    r#"use vil_sdk::prelude::*;

fn main() {
    vil_sdk::http_gateway()
        .listen(3080)
        .upstream("http://localhost:18081/api/v1/credits/stream")
        .run();
}
"#
    .into()
}

fn generate_sse() -> String {
    r#"use vil_sdk::prelude::*;

fn main() {
    vil_sdk::http_gateway()
        .listen(3080)
        .upstream("http://localhost:18081/api/v1/credits/stream?count=50&batch_size=10&delay_ms=100")
        .run();
}
"#.into()
}

fn generate_websocket() -> String {
    r#"// VIL WebSocket Pipeline
// Add vil_sdk to Cargo.toml dependencies

fn main() {
    println!("VIL WebSocket pipeline");
    println!("Configure your WebSocket relay in vil_sdk");
}
"#
    .into()
}

fn generate_grpc() -> String {
    r#"// VIL gRPC Pipeline
// Add vil_grpc to Cargo.toml dependencies

fn main() {
    println!("VIL gRPC pipeline");
    println!("Define your .proto service and use vil_grpc::GrpcGatewayBuilder");
}
"#
    .into()
}

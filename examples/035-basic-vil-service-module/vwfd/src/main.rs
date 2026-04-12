// 035 — Hospital Appointment System (Hybrid: Sidecar Lua for scheduling, NativeCode for registration)
use serde_json::json;

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/035-basic-vil-service-module/vwfd/workflows", 8080)
        // Patient registration — NativeCode (ID generation, simple)
        .native("register_patient", |input| {
            let body = &input["body"];
            let name = body["name"].as_str().unwrap_or("Patient");
            Ok(json!({
                "patient_id": format!("PAT-{:04}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() % 9999),
                "name": name,
                "registration_status": "active",
            }))
        })
        // Appointment scheduling — Sidecar Lua (external Lua runtime)
        .sidecar("scheduler", "lua5.4 examples/035-basic-vil-service-module/vwfd/sidecar/lua/scheduler.lua")
        .run().await;
}

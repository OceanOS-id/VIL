// 032 — Payment Gateway HA Failover (Hybrid: Sidecar C# for charge processing, NativeCode for health)
use serde_json::json;

#[tokio::main]
async fn main() {
    // dotnet-script needs DOTNET_ROOT to find the .NET runtime (snap install)
    if std::env::var("DOTNET_ROOT").is_err() {
        if std::path::Path::new("/snap/dotnet-sdk/current").exists() {
            std::env::set_var("DOTNET_ROOT", "/snap/dotnet-sdk/current");
        }
    }
    vil_vwfd::app("examples/032-basic-failover-ha/vwfd/workflows", 8080)
        // Health checks — NativeCode (trivial status)
        .native("primary_health_handler", |_| {
            Ok(json!({
                "provider": "stripe",
                "healthy": true,
                "role": "primary"
            }))
        })
        .native("backup_health_handler", |_| {
            Ok(json!({
                "provider": "adyen",
                "healthy": true,
                "role": "standby"
            }))
        })
        // Charge processing — Sidecar C# (external .NET runtime)
        .sidecar("payment_ha", "dotnet-script examples/032-basic-failover-ha/vwfd/sidecar/csharp/PaymentHA.cs")
        .run()
        .await;
}

//! vil provision — manage services on vflow-server

use std::process::Command;

pub enum Action {
    Push { host: String, artifact: String },
    Activate { host: String, service: String },
    Drain { host: String, service: String },
    Deactivate { host: String, service: String },
    List { host: String },
    Contract { host: String },
    Health { host: String },
}

pub fn run_provision(action: Action) -> Result<(), String> {
    match action {
        Action::Push { host, artifact } => {
            // Resolve absolute path
            let abs_path = std::fs::canonicalize(&artifact)
                .map_err(|e| format!("Cannot find artifact '{}': {}", artifact, e))?;
            let abs_str = abs_path.to_string_lossy();

            println!("  Provisioning {} to {}", abs_str, host);
            let body = format!(r#"{{"artifact":"{}"}}"#, abs_str);

            let output = curl_post(&format!("{}/internal/provision", host), &body)?;
            println!("  {}", output);
            Ok(())
        }
        Action::Activate { host, service } => {
            println!("  Activating {} on {}", service, host);
            let output = curl_post_empty(&format!("{}/internal/activate/{}", host, service))?;
            println!("  {}", output);
            Ok(())
        }
        Action::Drain { host, service } => {
            println!("  Draining {} on {}", service, host);
            let output = curl_post_empty(&format!("{}/internal/drain/{}", host, service))?;
            println!("  {}", output);
            Ok(())
        }
        Action::Deactivate { host, service } => {
            println!("  Deactivating {} on {}", service, host);
            let output = curl_post_empty(&format!("{}/internal/deactivate/{}", host, service))?;
            println!("  {}", output);
            Ok(())
        }
        Action::List { host } => {
            let output = curl_get(&format!("{}/internal/services", host))?;
            println!("{}", output);
            Ok(())
        }
        Action::Contract { host } => {
            let output = curl_get(&format!("{}/internal/contract", host))?;
            println!("{}", output);
            Ok(())
        }
        Action::Health { host } => {
            let output = curl_get(&format!("{}/health", host))?;
            println!("{}", output);
            Ok(())
        }
    }
}

fn curl_post(url: &str, body: &str) -> Result<String, String> {
    let output = Command::new("curl")
        .args(["-s", "-X", "POST", "-H", "Content-Type: application/json", "-d", body, url])
        .output()
        .map_err(|e| format!("Failed to run curl: {}", e))?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn curl_post_empty(url: &str) -> Result<String, String> {
    let output = Command::new("curl")
        .args(["-s", "-X", "POST", url])
        .output()
        .map_err(|e| format!("Failed to run curl: {}", e))?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn curl_get(url: &str) -> Result<String, String> {
    let output = Command::new("curl")
        .args(["-s", url])
        .output()
        .map_err(|e| format!("Failed to run curl: {}", e))?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

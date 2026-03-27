use std::sync::Arc;
use tiny_http::{Server, Response, Header};
use serde_json::json;
use vil_rt::world::VastarRuntimeWorld;

pub fn run(world: Arc<VastarRuntimeWorld>) {
    let port = 3081;
    let server = Server::http(format!("0.0.0.0:{}", port)).expect("Failed to start dashboard server");
    
    let addr = server.server_addr();
    println!("🚀 VIL Dashboard live at http://{}", addr);
    println!("Press Ctrl+C to stop.");

    for request in server.incoming_requests() {
        let url = request.url();
        
        let response = match url {
            "/" => {
                let html = include_str!("dashboard.html");
                Response::from_string(html)
                    .with_header(Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap())
            },
            "/api/metrics" => {
                let snap = world.latency_snapshot();
                let counters = world.counters_snapshot();
                
                let data = json!({
                    "latency": snap,
                    "counters": counters,
                    "timestamp": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64()
                });
                
                Response::from_string(data.to_string())
                    .with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
            },
            _ => {
                Response::from_string("Not Found").with_status_code(404)
            }
        };
        
        let _ = request.respond(response);
    }
}

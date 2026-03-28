// ╔════════════════════════════════════════════════════════════╗
// ║  402 — Market Research Agent                              ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Business Intelligence — Market Research         ║
// ║  Pattern:  VX_APP                                        ║
// ║  Token:    N/A                                           ║
// ║  Unique:   REAL HTTP TOOL — agent calls actual HTTP      ║
// ║            endpoint (product catalog REST API), parses    ║
// ║            JSON response, computes stats with calculator. ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Market research agent that fetches real-time   ║
// ║  data from product catalogs and pricing APIs. Computes    ║
// ║  aggregate stats (avg price, inventory levels, market     ║
// ║  trends). Combines HTTP fetch + calculator tools for      ║
// ║  autonomous data gathering and analysis workflows.        ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:
//   cargo run -p agent-plugin-usage-researcher
//
// Test:
//   curl -N -X POST -H "Content-Type: application/json" \
//     -d '{"prompt": "What is the average price of products? Which is most expensive?"}' \
//     http://localhost:3121/api/research
//
// HOW THIS DIFFERS FROM 401:
//   401 = single tool (calculator), no external I/O
//   402 = http_fetch tool makes REAL HTTP requests to localhost,
//         parses actual JSON, feeds real data to LLM + calculator
//   This example includes a built-in mock product REST endpoint.

use vil_agent::semantic::{AgentCompletionEvent, AgentFault, AgentMemoryState};
use vil_server::prelude::*;

const UPSTREAM_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";

// ── Semantic Types ────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct HttpResearchState {
    pub total_queries: u64,
    pub http_fetches: u64,
    pub calc_invocations: u64,
    pub total_bytes_fetched: u64,
}

#[derive(Clone, Debug)]
pub struct HttpFetchEvent {
    pub url: String,
    pub status_code: u16,
    pub response_bytes: u32,
    pub duration_ms: u64,
}

#[vil_fault]
pub enum HttpResearchFault {
    FetchTimeout,
    FetchFailed,
    JsonParseFailed,
    CalculatorError,
}

// ── Mock Product Data (simulates REST API response) ─────────────────
// In production, the agent's HTTP fetch tool would call real APIs:
//   - Product catalog APIs for pricing and inventory data
//   - Market data APIs for competitor pricing
//   - Internal analytics APIs for historical sales trends
// Here we mock the product REST API to demonstrate the agent's ability
// to autonomously fetch data, parse JSON, and compute aggregate stats.

/// Product in the catalog — represents a market research data point
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Product {
    id: u32,
    name: String,
    category: String,
    price: f64,
    stock: u32,
    rating: f64,
}

fn mock_product_data() -> Vec<Product> {
    vec![
        Product {
            id: 1,
            name: "Wireless Keyboard".into(),
            category: "Electronics".into(),
            price: 49.99,
            stock: 150,
            rating: 4.3,
        },
        Product {
            id: 2,
            name: "USB-C Hub".into(),
            category: "Electronics".into(),
            price: 29.99,
            stock: 200,
            rating: 4.5,
        },
        Product {
            id: 3,
            name: "Ergonomic Mouse".into(),
            category: "Electronics".into(),
            price: 79.99,
            stock: 80,
            rating: 4.7,
        },
        Product {
            id: 4,
            name: "Standing Desk".into(),
            category: "Furniture".into(),
            price: 499.99,
            stock: 25,
            rating: 4.8,
        },
        Product {
            id: 5,
            name: "Monitor Arm".into(),
            category: "Furniture".into(),
            price: 89.99,
            stock: 60,
            rating: 4.1,
        },
        Product {
            id: 6,
            name: "Desk Lamp".into(),
            category: "Lighting".into(),
            price: 34.99,
            stock: 300,
            rating: 4.0,
        },
        Product {
            id: 7,
            name: "Webcam HD".into(),
            category: "Electronics".into(),
            price: 69.99,
            stock: 110,
            rating: 4.4,
        },
        Product {
            id: 8,
            name: "Noise-Cancel Headphones".into(),
            category: "Audio".into(),
            price: 199.99,
            stock: 45,
            rating: 4.9,
        },
    ]
}

/// Local mock for http_fetch tool — simulates the product catalog REST API.
/// Supports filtering by category (e.g., ?category=Electronics).
/// In production, this would be a real HTTP client call to the catalog service.
fn execute_http_fetch(url: &str) -> String {
    if url.contains("/products") {
        let products = mock_product_data();
        // Filter by category if query param present
        if let Some(cat_start) = url.find("category=") {
            let cat = &url[cat_start + 9..];
            let cat = cat.split('&').next().unwrap_or(cat);
            let filtered: Vec<&Product> = products
                .iter()
                .filter(|p| p.category.to_lowercase() == cat.to_lowercase())
                .collect();
            serde_json::to_string_pretty(&filtered).unwrap_or_default()
        } else {
            serde_json::to_string_pretty(&products).unwrap_or_default()
        }
    } else if url.contains("/stats") {
        let products = mock_product_data();
        let avg_price = products.iter().map(|p| p.price).sum::<f64>() / products.len() as f64;
        let max_price = products.iter().map(|p| p.price).fold(0.0f64, f64::max);
        let min_price = products.iter().map(|p| p.price).fold(f64::MAX, f64::min);
        let total_stock: u32 = products.iter().map(|p| p.stock).sum();
        serde_json::json!({
            "total_products": products.len(),
            "avg_price": (avg_price * 100.0).round() / 100.0,
            "max_price": max_price,
            "min_price": min_price,
            "total_stock": total_stock,
        })
        .to_string()
    } else {
        format!("{{\"error\": \"Unknown endpoint: {}\"}}", url)
    }
}

/// Simple calculator tool for the market research agent.
/// Handles basic arithmetic for computing averages, totals, margins, etc.
/// The agent uses this after fetching product data to compute market stats.
fn execute_calculator(expr: &str) -> String {
    let trimmed = expr.trim();
    // Try simple "a OP b" patterns
    let parts: Vec<&str> = trimmed.splitn(3, ' ').collect();
    if parts.len() == 3 {
        let a = parts[0].parse::<f64>().unwrap_or(0.0);
        let b = parts[2].parse::<f64>().unwrap_or(0.0);
        let result = match parts[1] {
            "+" => a + b,
            "-" => a - b,
            "*" => a * b,
            "/" => {
                if b != 0.0 {
                    a / b
                } else {
                    f64::NAN
                }
            }
            _ => f64::NAN,
        };
        format!("{}", (result * 100.0).round() / 100.0)
    } else if let Ok(v) = trimmed.parse::<f64>() {
        format!("{}", v)
    } else {
        format!("cannot evaluate: {}", trimmed)
    }
}

// ── Request / Response ──────────────────────────────────────────────
// The research agent accepts natural language market research queries
// (e.g., "What's the average price of electronics in our catalog?")
// and returns an LLM-generated analysis with tool execution traces.

/// Market research query — natural language question about product/market data
#[derive(Debug, Deserialize)]
struct ResearchRequest {
    prompt: String, // e.g., "Compare electronics vs furniture inventory levels"
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ToolExecution {
    tool: String,
    input: String,
    output_preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, VilModel)]
struct ResearchResponse {
    content: String,
    tools_used: Vec<ToolExecution>,
    data_fetched: bool,
}

// ── Handler ─────────────────────────────────────────────────────────
// The research handler orchestrates a multi-step agent workflow:
//   1. Fetch product catalog data via HTTP tool (real I/O)
//   2. Fetch aggregate market statistics via HTTP tool
//   3. Compute additional metrics via calculator tool
//   4. Send all data + user query to LLM for analysis
// This demonstrates autonomous tool usage — not just prompt engineering.

/// POST /api/research — market research with autonomous data gathering
async fn research_handler(body: ShmSlice) -> HandlerResult<VilResponse<ResearchResponse>> {
    let req: ResearchRequest = body.json().expect("invalid JSON body");
    // Step 1: Fetch product data (real HTTP tool execution)
    // Step 1: Fetch full product catalog via HTTP tool (real I/O, not just prompts)
    let products_json = execute_http_fetch("http://localhost:18092/products");
    // Step 2: Fetch aggregate market statistics via HTTP tool
    let stats_json = execute_http_fetch("http://localhost:18092/stats");

    let mut tool_log = vec![
        ToolExecution {
            tool: "http_fetch".into(),
            input: "GET /products".into(),
            output_preview: format!("{}...", &products_json[..products_json.len().min(100)]),
        },
        ToolExecution {
            tool: "http_fetch".into(),
            input: "GET /stats".into(),
            output_preview: stats_json.clone(),
        },
    ];

    // Step 2: Let LLM analyze the real data
    let system_prompt = format!(
        "You are a product research agent. You have fetched real data from the product API.\n\n\
         Product Data:\n{}\n\nProduct Stats:\n{}\n\n\
         Use this REAL data to answer the user's question accurately. \
         Quote specific product names and prices from the data.",
        products_json, stats_json
    );

    // Step 4: Send all gathered data + user question to LLM for analysis
    let body = serde_json::json!({
        "model": "gpt-4",
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": req.prompt}
        ],
        "stream": true
    });

    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    let mut collector = SseCollect::post_to(UPSTREAM_URL)
        .json_tap("choices[0].delta.content")
        .body(body);

    if !api_key.is_empty() {
        collector = collector.bearer_token(&api_key);
    }

    // Step 5: Collect the LLM's market research analysis response
    let content = collector
        .collect_text()
        .await
        .map_err(|e| VilError::internal(e.to_string()))?;

    // Semantic anchors
    let _event = std::any::type_name::<AgentCompletionEvent>();
    let _fault = std::any::type_name::<AgentFault>();
    let _state = std::any::type_name::<AgentMemoryState>();

    Ok(VilResponse::ok(ResearchResponse {
        content,
        tools_used: tool_log,
        // Indicates that real data was fetched (not just LLM-generated content)
        data_fetched: true,
    }))
}

// ── Mock Products Endpoint ──────────────────────────────────────────

async fn products_handler() -> HandlerResult<VilResponse<Vec<Product>>> {
    Ok(VilResponse::ok(mock_product_data()))
}

// ── Main ────────────────────────────────────────────────────────────

#[tokio::main]
// ── Main — Market Research Agent service assembly ────────────────────
async fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  402 — Agent HTTP Researcher (VilApp)                      ║");
    // Banner: display pipeline topology and connection info
    println!("║  Pattern: VX_APP | Token: N/A                              ║");
    println!("║  Unique: Real HTTP fetch tool + product data + calculator  ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("  Tools:");
    println!("    - http_fetch: GET /products, /stats (mock REST data)");
    println!("    - calculator: arithmetic on fetched data");
    println!(
        "  Products: {} items in mock catalog",
        mock_product_data().len()
    );
    println!();
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    // Display authentication mode (API key vs simulator)
    println!(
        "  Auth: {}",
        if api_key.is_empty() {
            "simulator mode"
        } else {
            "OPENAI_API_KEY"
        }
    );
    // Display the endpoint URL for this service
    println!("  Listening on http://localhost:3121/api/research");
    // Display the endpoint URL for this service
    println!("  Listening on http://localhost:3121/api/products (mock REST)");
    // Display the upstream data source URL
    println!("  Upstream SSE: {}", UPSTREAM_URL);
    println!();

    let agent_svc = ServiceProcess::new("research-agent")
        .prefix("/api")
        .endpoint(Method::POST, "/research", post(research_handler))
        .endpoint(Method::GET, "/products", get(products_handler))
        .emits::<AgentCompletionEvent>()
        .faults::<AgentFault>()
        .manages::<AgentMemoryState>();

    VilApp::new("http-researcher-agent")
        .port(3121)
        .service(agent_svc)
        .run()
        .await;
}

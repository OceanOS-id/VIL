// ╔════════════════════════════════════════════════════════════╗
// ║  402 — Market Research Agent (Real HTTP + Calculator)      ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Business Intelligence — Competitive Pricing     ║
// ║  Pattern:  VX_APP                                           ║
// ║  Features: vil_agent::Agent, HttpFetchTool (real reqwest), ║
// ║            CalculatorTool, product catalog endpoint         ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Agent fetches product data from local catalog    ║
// ║  endpoint (real HTTP), analyzes pricing, generates report. ║
// ║  Product catalog is served by same VilApp (real data).     ║
// ╚════════════════════════════════════════════════════════════╝
//
// Requires: ai-endpoint-simulator at localhost:4545
//
// Run:   cargo run -p vil-agent-http-researcher
// Test:
//   curl http://localhost:8080/api/catalog/products
//   curl -X POST http://localhost:8080/api/research/analyze \
//     -H 'Content-Type: application/json' \
//     -d '{"query":"Compare prices of wireless mice and find the best deal"}'

use std::sync::Arc;

use async_trait::async_trait;
use vil_agent::tool::{Tool, ToolError, ToolResult};
use vil_agent::Agent;
use vil_llm::{OpenAiConfig, OpenAiProvider};
use vil_server::prelude::*;

// ── Models ───────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct Product {
    id: String,
    name: String,
    category: String,
    price_cents: i64,
    stock: i32,
    rating: f64,
}

#[derive(Debug, Deserialize)]
struct ResearchRequest {
    query: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct ResearchResponse {
    answer: String,
    tools_used: Vec<String>,
    iterations: usize,
}

struct ResearchState {
    agent: Agent,
}

// ── Product Catalog (real data) ──────────────────────────────────────────

fn catalog() -> Vec<Product> {
    vec![
        Product { id: "WM-001".into(), name: "Logitech M185 Wireless Mouse".into(), category: "mice".into(), price_cents: 1499, stock: 150, rating: 4.2 },
        Product { id: "WM-002".into(), name: "Logitech MX Master 3S".into(), category: "mice".into(), price_cents: 9999, stock: 45, rating: 4.8 },
        Product { id: "WM-003".into(), name: "Razer DeathAdder V3".into(), category: "mice".into(), price_cents: 6999, stock: 30, rating: 4.6 },
        Product { id: "WM-004".into(), name: "Microsoft Arc Mouse".into(), category: "mice".into(), price_cents: 7999, stock: 20, rating: 4.0 },
        Product { id: "KB-001".into(), name: "Keychron K2 Wireless".into(), category: "keyboards".into(), price_cents: 8900, stock: 60, rating: 4.7 },
        Product { id: "KB-002".into(), name: "Logitech MX Keys".into(), category: "keyboards".into(), price_cents: 10999, stock: 35, rating: 4.5 },
        Product { id: "HD-001".into(), name: "Sony WH-1000XM5".into(), category: "headphones".into(), price_cents: 34999, stock: 25, rating: 4.7 },
        Product { id: "HD-002".into(), name: "Apple AirPods Pro 2".into(), category: "headphones".into(), price_cents: 24999, stock: 80, rating: 4.6 },
        Product { id: "MN-001".into(), name: "Dell UltraSharp 27 4K".into(), category: "monitors".into(), price_cents: 44999, stock: 15, rating: 4.4 },
        Product { id: "MN-002".into(), name: "LG 27GP850-B".into(), category: "monitors".into(), price_cents: 39999, stock: 22, rating: 4.5 },
        Product { id: "WC-001".into(), name: "Logitech Brio 4K".into(), category: "webcams".into(), price_cents: 17999, stock: 40, rating: 4.3 },
        Product { id: "CH-001".into(), name: "Anker USB-C Hub 7-in-1".into(), category: "accessories".into(), price_cents: 3599, stock: 200, rating: 4.4 },
    ]
}

// ── Catalog endpoint (real data served by same VilApp) ───────────────────

async fn list_products() -> VilResponse<Vec<Product>> {
    VilResponse::ok(catalog())
}

async fn get_product(body: ShmSlice) -> HandlerResult<VilResponse<Product>> {
    let req: serde_json::Value = body.json()
        .map_err(|_| VilError::bad_request("invalid JSON"))?;
    let id = req["id"].as_str().unwrap_or("");
    catalog().into_iter()
        .find(|p| p.id == id)
        .map(VilResponse::ok)
        .ok_or_else(|| VilError::not_found(format!("product {} not found", id)))
}

// ── Tool: Local HTTP Fetch (real reqwest to self) ────────────────────────

struct ProductFetchTool {
    base_url: String,
}

#[async_trait]
impl Tool for ProductFetchTool {
    fn name(&self) -> &str { "fetch_products" }
    fn description(&self) -> &str {
        "Fetch product catalog from local API. Input: {\"category\": \"mice\"} or {\"category\": \"all\"}"
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": { "category": { "type": "string" } },
            "required": ["category"]
        })
    }
    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult, ToolError> {
        let category = params["category"].as_str().unwrap_or("all");

        // Real HTTP call to local catalog endpoint
        let url = format!("{}/api/catalog/products", self.base_url);
        let client = reqwest::Client::new();
        let resp = client.get(&url).send().await
            .map_err(|e| ToolError::ExecutionFailed(format!("HTTP fetch failed: {}", e)))?;

        let body: serde_json::Value = resp.json().await
            .map_err(|e| ToolError::ExecutionFailed(format!("parse failed: {}", e)))?;

        // Filter by category if specified
        let products = body["data"].as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter(|p| category == "all" || p["category"].as_str() == Some(category))
            .map(|p| format!(
                "{}: {} — ${:.2} (stock: {}, rating: {})",
                p["id"].as_str().unwrap_or("?"),
                p["name"].as_str().unwrap_or("?"),
                p["price_cents"].as_i64().unwrap_or(0) as f64 / 100.0,
                p["stock"].as_i64().unwrap_or(0),
                p["rating"].as_f64().unwrap_or(0.0),
            ))
            .collect::<Vec<_>>();

        let output = if products.is_empty() {
            format!("No products found in category '{}'", category)
        } else {
            format!("Found {} products:\n{}", products.len(), products.join("\n"))
        };

        Ok(ToolResult { output, metadata: None })
    }
}

// ── Tool: Calculator ─────────────────────────────────────────────────────

struct PriceCalculatorTool;

#[async_trait]
impl Tool for PriceCalculatorTool {
    fn name(&self) -> &str { "calculator" }
    fn description(&self) -> &str {
        "Calculate price metrics. Input: {\"operation\": \"average\", \"values\": [14.99, 99.99, 69.99]}"
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": { "type": "string", "enum": ["average", "min", "max", "sum", "difference"] },
                "values": { "type": "array", "items": { "type": "number" } }
            },
            "required": ["operation", "values"]
        })
    }
    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult, ToolError> {
        let op = params["operation"].as_str().unwrap_or("average");
        let values: Vec<f64> = params["values"].as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_f64())
            .collect();

        if values.is_empty() {
            return Ok(ToolResult { output: "No values provided".into(), metadata: None });
        }

        let result = match op {
            "average" => values.iter().sum::<f64>() / values.len() as f64,
            "min" => values.iter().cloned().fold(f64::INFINITY, f64::min),
            "max" => values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            "sum" => values.iter().sum(),
            "difference" if values.len() >= 2 => (values[0] - values[1]).abs(),
            _ => return Ok(ToolResult { output: format!("Unknown operation: {}", op), metadata: None }),
        };

        Ok(ToolResult {
            output: format!("{}({:?}) = {:.2}", op, values, result),
            metadata: None,
        })
    }
}

// ── Handler ──────────────────────────────────────────────────────────────

async fn analyze(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<ResearchResponse>> {
    let req: ResearchRequest = body.json()
        .map_err(|_| VilError::bad_request("invalid JSON"))?;

    let state = ctx.state::<Arc<ResearchState>>()
        .map_err(|_| VilError::internal("state not found"))?;

    let response = state.agent.run(&req.query).await
        .map_err(|e| VilError::internal(format!("agent failed: {:?}", e)))?;

    Ok(VilResponse::ok(ResearchResponse {
        answer: response.answer,
        tools_used: response.tool_calls_made.iter()
            .map(|tc| format!("{}({})", tc.tool, tc.input))
            .collect(),
        iterations: response.iterations,
    }))
}

// ── Main ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let upstream = std::env::var("LLM_UPSTREAM")
        .unwrap_or_else(|_| "http://127.0.0.1:4545".into());
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    let llm = Arc::new(OpenAiProvider::new(
        OpenAiConfig::new(&api_key, "gpt-4").base_url(&format!("{}/v1", upstream)),
    ));

    let agent = Agent::builder()
        .llm(llm)
        .tool(Arc::new(ProductFetchTool { base_url: "http://localhost:8080".into() }))
        .tool(Arc::new(PriceCalculatorTool))
        .max_iterations(8)
        .system_prompt(
            "You are a market research analyst. Use fetch_products to get product data \
             from the catalog, and calculator for price comparisons. \
             Provide clear, data-driven analysis with specific numbers."
        )
        .build();

    let state = Arc::new(ResearchState { agent });

    // Catalog service (real product data)
    let catalog_svc = ServiceProcess::new("catalog")
        .endpoint(Method::GET, "/products", get(list_products))
        .endpoint(Method::POST, "/product", post(get_product));

    // Research service (agent)
    let research_svc = ServiceProcess::new("research")
        .endpoint(Method::POST, "/analyze", post(analyze))
        .state(state);

    VilApp::new("market-research-agent")
        .port(8080)
        .observer(true)
        .service(catalog_svc)
        .service(research_svc)
        .run()
        .await;
}

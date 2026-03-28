// ╔════════════════════════════════════════════════════════════╗
// ║  404 — Business Intelligence Analyst                     ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Pattern:  VX_APP                                        ║
// ║  Token:    N/A                                           ║
// ║  Unique:   STRUCTURED DATA TOOLS — CSV parsing, stats    ║
// ║            computation (mean/median/stddev/growth), and  ║
// ║            chart-friendly JSON output for frontend.      ║
// ║            Input is raw CSV, not natural language.       ║
// ║  Domain:   Parses CSV reports, computes statistics,      ║
// ║            generates chart-ready data for dashboards      ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:
//   cargo run -p agent-plugin-usage-data-analyst
//
// Test:
//   curl -N -X POST -H "Content-Type: application/json" \
//     -d '{"csv_data": "month,revenue,users\nJan,10000,500\nFeb,12000,600\nMar,15000,750\nApr,14000,720\nMay,18000,900\nJun,22000,1100", "question": "What is the revenue growth trend? Which month had best user acquisition?"}' \
//     http://localhost:3123/api/csv-analyze
//
// BUSINESS CONTEXT:
//   Business intelligence analyst agent for executive dashboards. Sales
//   managers upload monthly CSV reports (revenue, user acquisition, churn),
//   and the agent automatically computes KPIs: growth rate, mean/median
//   revenue, standard deviation (volatility), and generates Chart.js-ready
//   JSON for the frontend dashboard. This replaces manual Excel analysis
//   with an AI-powered pipeline that also provides narrative insights
//   ("March showed 25% growth driven by user acquisition surge").
//
// HOW THIS DIFFERS FROM 402:
//   402 = HTTP fetch tool (structured REST API)
//   404 = CSV parsing tool + statistics tool + chart data generator
//   Input is raw CSV data, tools parse it into records, compute stats,
//   and produce chart-ready JSON. Different domain (tabular data analysis).

use vil_agent::semantic::{AgentCompletionEvent, AgentFault, AgentMemoryState};
use vil_server::prelude::*;

const UPSTREAM_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";

// ── Semantic Types ────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct CsvAnalystState {
    pub datasets_analyzed: u64,
    pub total_rows_processed: u64,
    pub total_columns_processed: u64,
    pub charts_generated: u64,
}

#[derive(Clone, Debug)]
pub struct CsvToolEvent {
    pub tool: String,
    pub rows_processed: u32,
    pub columns: Vec<String>,
    pub result_summary: String,
}

#[vil_fault]
pub enum CsvAnalystFault {
    CsvParseFailed,
    EmptyDataset,
    NonNumericColumn,
    StatComputeFailed,
    LlmUpstreamError,
}

// ── CSV Parser Tool ─────────────────────────────────────────────────
// Parses raw CSV text into structured records. Business: sales teams
// export monthly reports from CRM/ERP systems as CSV — this tool is
// the first step in the analysis pipeline.

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CsvRecord {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

fn tool_parse_csv(csv_data: &str) -> (CsvRecord, String) {
    let mut lines = csv_data.lines();
    let headers: Vec<String> = match lines.next() {
        Some(h) => h.split(',').map(|s| s.trim().to_string()).collect(),
        None => {
            return (
                CsvRecord {
                    headers: vec![],
                    rows: vec![],
                },
                "Empty CSV".into(),
            )
        }
    };

    let rows: Vec<Vec<String>> = lines
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.split(',').map(|s| s.trim().to_string()).collect())
        .collect();

    let summary = format!(
        "{} columns x {} rows. Headers: {:?}",
        headers.len(),
        rows.len(),
        headers
    );
    (CsvRecord { headers, rows }, summary)
}

// ── Statistics Tool ─────────────────────────────────────────────────
// Computes descriptive statistics for any numeric column.
// Business KPIs derived from these: revenue growth (board metric),
// user acquisition rate (marketing KPI), volatility (risk indicator).

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ColumnStats {
    column: String,
    count: usize,
    mean: f64,
    median: f64,
    std_dev: f64,
    min: f64,
    max: f64,
    growth_rate_pct: f64,
}

/// Compute descriptive statistics for a numeric column.
/// Business KPIs: growth_rate_pct is the headline metric for board reports,
/// std_dev indicates revenue volatility (risk signal for investors).
fn tool_compute_stats(record: &CsvRecord, col_name: &str) -> Option<ColumnStats> {
    let col_idx = record.headers.iter().position(|h| h == col_name)?;
    let mut values: Vec<f64> = record
        .rows
        .iter()
        .filter_map(|row| row.get(col_idx).and_then(|v| v.parse::<f64>().ok()))
        .collect();

    if values.is_empty() {
        return None;
    }

    let count = values.len();
    let sum: f64 = values.iter().sum();
    let mean = sum / count as f64;

    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = if count % 2 == 0 {
        (values[count / 2 - 1] + values[count / 2]) / 2.0
    } else {
        values[count / 2]
    };

    let variance: f64 = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / count as f64;
    let std_dev = variance.sqrt();

    let min = values.first().copied().unwrap_or(0.0);
    let max = values.last().copied().unwrap_or(0.0);

    // Growth rate: (last - first) / first * 100
    let first = values.first().copied().unwrap_or(1.0);
    let last = values.last().copied().unwrap_or(1.0);
    let growth_rate_pct = if first != 0.0 {
        ((last - first) / first) * 100.0
    } else {
        0.0
    };

    Some(ColumnStats {
        column: col_name.into(),
        count,
        mean: (mean * 100.0).round() / 100.0,
        median: (median * 100.0).round() / 100.0,
        std_dev: (std_dev * 100.0).round() / 100.0,
        min,
        max,
        growth_rate_pct: (growth_rate_pct * 100.0).round() / 100.0,
    })
}

// ── Chart Data Tool ─────────────────────────────────────────────────
// Generates JSON compatible with Chart.js / Recharts / D3.
// Business: the dashboard frontend renders this directly without
// any client-side data transformation.

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ChartData {
    chart_type: String,
    title: String,
    labels: Vec<String>,
    datasets: Vec<ChartDataset>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ChartDataset {
    label: String,
    data: Vec<f64>,
}

/// Generate chart-ready JSON data compatible with Chart.js / Recharts.
/// Business: the frontend dashboard renders this directly without transformation.
fn tool_chart_data(record: &CsvRecord, label_col: &str, data_cols: &[&str]) -> ChartData {
    let label_idx = record.headers.iter().position(|h| h == label_col);
    let labels: Vec<String> = if let Some(idx) = label_idx {
        record
            .rows
            .iter()
            .filter_map(|r| r.get(idx).cloned())
            .collect()
    } else {
        (1..=record.rows.len())
            .map(|i| format!("Row {}", i))
            .collect()
    };

    let datasets: Vec<ChartDataset> = data_cols
        .iter()
        .filter_map(|col| {
            let idx = record.headers.iter().position(|h| h == *col)?;
            let data: Vec<f64> = record
                .rows
                .iter()
                .filter_map(|r| r.get(idx).and_then(|v| v.parse().ok()))
                .collect();
            Some(ChartDataset {
                label: col.to_string(),
                data,
            })
        })
        .collect();

    ChartData {
        chart_type: "line".into(),
        title: format!("{} over {}", data_cols.join(" & "), label_col),
        labels,
        datasets,
    }
}

// ── Request / Response ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CsvAnalyzeRequest {
    csv_data: String,
    question: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, VilModel)]
struct CsvAnalyzeResponse {
    analysis: String,
    stats: Vec<ColumnStats>,
    chart_data: ChartData,
    rows_processed: usize,
}

// ── Handler ─────────────────────────────────────────────────────────

async fn csv_analyze_handler(body: ShmSlice) -> HandlerResult<VilResponse<CsvAnalyzeResponse>> {
    let req: CsvAnalyzeRequest = body.json().expect("invalid JSON body");
    // Step 1: Parse CSV
    let (record, parse_summary) = tool_parse_csv(&req.csv_data);
    if record.rows.is_empty() {
        return Err(VilError::bad_request("CSV data is empty or invalid"));
    }

    // Step 2: Compute stats for all numeric columns
    let mut all_stats = Vec::new();
    let mut numeric_cols = Vec::new();
    let label_col = record
        .headers
        .first()
        .map(|s| s.as_str())
        .unwrap_or("index");

    for header in &record.headers[1..] {
        // Skip first column (usually labels/dates)
        if let Some(stats) = tool_compute_stats(&record, header) {
            all_stats.push(stats);
            numeric_cols.push(header.as_str());
        }
    }

    // Step 3: Generate chart data
    let chart = tool_chart_data(&record, label_col, &numeric_cols);

    // Step 4: Build context for LLM
    let stats_text: String = all_stats
        .iter()
        .map(|s| {
            format!(
                "Column '{}': mean={}, median={}, std_dev={}, min={}, max={}, growth={}%",
                s.column, s.mean, s.median, s.std_dev, s.min, s.max, s.growth_rate_pct
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let system_prompt = format!(
        "You are a data analyst agent. The following CSV data has been parsed and analyzed.\n\n\
         Parse Summary: {}\n\n\
         Raw Data:\n{}\n\n\
         Computed Statistics:\n{}\n\n\
         Provide insightful analysis answering the user's question. \
         Reference specific numbers from the statistics. \
         Identify trends, outliers, and actionable insights.",
        parse_summary, req.csv_data, stats_text
    );

    let body = serde_json::json!({
        "model": "gpt-4",
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": req.question}
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

    let analysis = collector
        .collect_text()
        .await
        .map_err(|e| VilError::internal(e.to_string()))?;

    // Semantic anchors
    let _event = std::any::type_name::<AgentCompletionEvent>();
    let _fault = std::any::type_name::<AgentFault>();
    let _state = std::any::type_name::<AgentMemoryState>();

    Ok(VilResponse::ok(CsvAnalyzeResponse {
        analysis,
        stats: all_stats,
        chart_data: chart,
        rows_processed: record.rows.len(),
    }))
}

// ── Main ────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  404 — Agent Data CSV Analyst (VilApp)                     ║");
    println!("║  Pattern: VX_APP | Token: N/A                              ║");
    println!("║  Unique: CSV parsing + stats + chart-ready JSON output     ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("  Tools:");
    println!("    - parse_csv : parse CSV into structured records");
    println!("    - calculator: mean, median, std_dev, growth_rate");
    println!("    - chart_data: generate chart-friendly JSON");
    println!();
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    println!(
        "  Auth: {}",
        if api_key.is_empty() {
            "simulator mode"
        } else {
            "OPENAI_API_KEY"
        }
    );
    println!("  Listening on http://localhost:3123/api/csv-analyze");
    println!("  Upstream SSE: {}", UPSTREAM_URL);
    println!();

    let svc = ServiceProcess::new("csv-analyst-agent")
        .prefix("/api")
        .endpoint(Method::POST, "/csv-analyze", post(csv_analyze_handler))
        .emits::<AgentCompletionEvent>()
        .faults::<AgentFault>()
        .manages::<AgentMemoryState>();

    VilApp::new("csv-analyst-agent")
        .port(3123)
        .service(svc)
        .run()
        .await;
}

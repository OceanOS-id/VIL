// 902 — Data Pipeline (Workflow Pattern)
// Demonstrates: parse_csv, parse_xml, validate_schema, reshape, stats in V-CEL
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/902-faas-data-pipeline/vwfd/workflows", 8080)
        .run()
        .await;
}

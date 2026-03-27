# 404 — Agent Data CSV Analyst

Agent with structured data tools: CSV parsing, statistics computation (mean/median/stddev/growth), and chart-friendly JSON output.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A (HTTP server) |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## Tool Pattern

CSV-focused tools -- parse_csv, compute_stats (mean/median/stddev/growth), generate_chart_data -- input is raw CSV data, not natural language; produces chart-ready JSON

## Architecture

```
POST /api/csv-analyze (:3123)
  -> System prompt with tool descriptions
  -> SseCollect -> LLM upstream :4545
  -> Parse tool calls from LLM output
  -> Execute tools locally
  -> (Optional: feed results back for multi-turn)
  -> VilResponse with tool trace
```

## Key VIL Features Used

- `CSV parsing tool (raw CSV -> records)`
- `Statistics computation (mean, median, stddev, growth rate)`
- `Chart data generator (frontend-ready JSON)`
- `ShmSlice for CSV + question body`
- `#[vil_fault] CsvAnalystFault (InvalidCsv, EmptyDataset)`

## Run

```bash
cargo run -p agent-plugin-usage-data-analyst
```

## Test

```bash
curl -X POST -H 'Content-Type: application/json' -d '{"csv_data": "month,revenue\nJan,10000\nFeb,12000", "question": "What is the growth trend?"}' http://localhost:3123/api/csv-analyze
```

## System Specs

| Spec | Value |
|------|-------|
| **CPU** | Intel i9-11900F @ 2.50GHz (8C/16T, turbo 5.2GHz) |
| **RAM** | 32GB DDR4 |
| **OS** | Ubuntu 22.04 LTS (kernel 6.8.0) |
| **Rust** | 1.93.1 |

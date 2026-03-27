# 402 — Agent HTTP Researcher

Agent with REAL HTTP tool that fetches actual data from localhost product REST endpoint, parses JSON, and uses calculator for stats.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A (HTTP server) |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## Tool Pattern

Real HTTP I/O -- agent calls actual HTTP endpoint (product REST simulator), parses JSON response, computes statistics with calculator tool

## Architecture

```
POST /api/research (:3121)
  -> System prompt with tool descriptions
  -> SseCollect -> LLM upstream :4545
  -> Parse tool calls from LLM output
  -> Execute tools locally
  -> (Optional: feed results back for multi-turn)
  -> VilResponse with tool trace
```

## Key VIL Features Used

- `http_fetch tool with real HTTP GET requests`
- `Calculator tool for statistical computation`
- `ShmSlice for research query body`
- `#[vil_fault] HttpResearchFault (FetchTimeout, InvalidUrl)`
- `HttpFetchEvent semantic audit with status_code + response_bytes`

## Run

```bash
cargo run -p agent-plugin-usage-researcher
```

## Test

```bash
curl -X POST -H 'Content-Type: application/json' -d '{"prompt": "What is the average price of products?"}' http://localhost:3121/api/research
```

## System Specs

| Spec | Value |
|------|-------|
| **CPU** | Intel i9-11900F @ 2.50GHz (8C/16T, turbo 5.2GHz) |
| **RAM** | 32GB DDR4 |
| **OS** | Ubuntu 22.04 LTS (kernel 6.8.0) |
| **Rust** | 1.93.1 |

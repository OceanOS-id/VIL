# 203 — LLM Code Review with Tools

Multi-turn LLM conversation with LOCAL tool execution: parses <tool>name:input</tool> from LLM output, executes calculator/analyzer tools, feeds results back.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## What Makes This Unique

Multi-turn conversation with tool execution loop -- LLM outputs tool calls, tools run locally, results fed back as new messages (max 3 turns)

## Architecture

```
POST /api/code/review (:3102)
```

## Key VIL Features Used

- `Multi-turn SseCollect calls in a loop`
- `Local tool execution (calculator: lines/complexity/math, analyzer: static analysis)`
- `ShmSlice + ServiceCtx extractors`
- `#[vil_fault] CodeReviewFault`
- `VilResponse<CodeReviewResponse> with tools_executed trace`

## Run

```bash
cargo run -p llm-plugin-usage-code-assistant
```

## Test

```bash
curl -X POST -H 'Content-Type: application/json' -d '{"code": "fn fib(n: u64) -> u64 { if n < 2 { n } else { fib(n-1) + fib(n-2) } }"}' http://localhost:3102/api/code/review
```

## System Specs

| Spec | Value |
|------|-------|
| **CPU** | Intel i9-11900F @ 2.50GHz (8C/16T, turbo 5.2GHz) |
| **RAM** | 32GB DDR4 |
| **OS** | Ubuntu 22.04 LTS (kernel 6.8.0) |
| **Rust** | 1.93.1 |

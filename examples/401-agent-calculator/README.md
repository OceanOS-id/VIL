# 401 — Agent Calculator

Simplest agent pattern: single calculator tool with local expression evaluation, single LLM turn.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A (HTTP server) |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## Tool Pattern

Single tool (calculator) with local math expression evaluation -- no multi-turn, no external I/O, simplest possible agent

## Architecture

```
POST /api/calc (:3120)
  -> System prompt with tool descriptions
  -> SseCollect -> LLM upstream :4545
  -> Parse tool calls from LLM output
  -> Execute tools locally
  -> (Optional: feed results back for multi-turn)
  -> VilResponse with tool trace
```

## Key VIL Features Used

- `Local calculator tool (expression evaluation)`
- `SseCollect with tool-augmented system prompt`
- `ShmSlice for agent request body`
- `#[vil_fault] CalcAgentFault (InvalidExpression, DivisionByZero)`
- `AgentCompletionEvent / AgentFault / AgentMemoryState semantic types`

## Run

```bash
cargo run -p agent-plugin-usage-calculator
```

## Test

```bash
curl -X POST -H 'Content-Type: application/json' -d '{"prompt": "What is 42 * 13 + 7?"}' http://localhost:3120/api/calc
```

## System Specs

| Spec | Value |
|------|-------|
| **CPU** | Intel i9-11900F @ 2.50GHz (8C/16T, turbo 5.2GHz) |
| **RAM** | 32GB DDR4 |
| **OS** | Ubuntu 22.04 LTS (kernel 6.8.0) |
| **Rust** | 1.93.1 |

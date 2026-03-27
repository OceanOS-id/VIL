# 405 — Agent ReAct Multi-Tool

Iterative ReAct loop agent: Think -> Act -> Observe -> Repeat with multi-step stateful conversation, explicit reasoning trace, max 5 iterations.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A (HTTP server) |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## Tool Pattern

ReAct (Reasoning + Acting) loop -- fundamentally different from single-turn agents: LLM thinks, chooses tool, tool executes, result fed back as observation, repeats until FINAL_ANSWER or max 5 iterations

## Architecture

```
POST /api/react (:3124)
  -> System prompt with tool descriptions
  -> SseCollect -> LLM upstream :4545
  -> Parse tool calls from LLM output
  -> Execute tools locally
  -> (Optional: feed results back for multi-turn)
  -> VilResponse with tool trace
```

## Key VIL Features Used

- `ReAct loop: Think -> Act -> Observe -> Repeat (max 5 iterations)`
- `Multiple tools (search, calculator, http_fetch)`
- `Full reasoning trace in response`
- `ShmSlice for complex query body`
- `#[vil_fault] ReactAgentFault with max iteration tracking`
- `ReactStepEvent per-iteration semantic audit`

## Run

```bash
cargo run -p agent-plugin-usage-multi-tool
```

## Test

```bash
curl -X POST -H 'Content-Type: application/json' -d '{"prompt": "What is the total value of all electronics products in stock?"}' http://localhost:3124/api/react
```

## System Specs

| Spec | Value |
|------|-------|
| **CPU** | Intel i9-11900F @ 2.50GHz (8C/16T, turbo 5.2GHz) |
| **RAM** | 32GB DDR4 |
| **OS** | Ubuntu 22.04 LTS (kernel 6.8.0) |
| **Rust** | 1.93.1 |

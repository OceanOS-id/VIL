# 026 — AI Agent (Basic)

VX_APP agent endpoint with tool descriptions (calculator, search) and semantic AgentCompletionEvent audit.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## Architecture

```
POST /api/agent (:3092)
```

## Key VIL Features Used

- `SseCollect with json_tap extraction`
- `ShmSlice for agent request body`
- `.emits::<AgentCompletionEvent>(), .faults::<AgentFault>()`
- `Tool descriptions in system prompt`
- `VilResponse typed output`

## Run

```bash
cargo run -p basic-usage-ai-agent
```

## Test

```bash
curl -X POST -H 'Content-Type: application/json' -d '{"prompt": "What is 42 * 13?"}' http://localhost:3092/api/agent
```

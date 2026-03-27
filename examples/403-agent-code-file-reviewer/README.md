# 403 — Agent Code File Reviewer

Agent with file system tools (read_file, count_lines, find_pattern) operating on mock file system for code review analysis.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A (HTTP server) |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## Tool Pattern

File system tools -- read_file, count_lines, find_pattern on mock file system -- completely different tool set from HTTP researcher (file I/O vs REST API)

## Architecture

```
POST /api/code-review (:3122)
  -> System prompt with tool descriptions
  -> SseCollect -> LLM upstream :4545
  -> Parse tool calls from LLM output
  -> Execute tools locally
  -> (Optional: feed results back for multi-turn)
  -> VilResponse with tool trace
```

## Key VIL Features Used

- `File system tools (read_file, count_lines, find_pattern)`
- `Mock file system with sample Rust code`
- `ShmSlice for review request body`
- `#[vil_fault] CodeReviewAgentFault (FileNotFound, PatternInvalid)`
- `FileToolEvent semantic audit`

## Run

```bash
cargo run -p agent-plugin-usage-code-review
```

## Test

```bash
curl -X POST -H 'Content-Type: application/json' -d '{"prompt": "Review main.rs -- check for unwrap() usage"}' http://localhost:3122/api/code-review
```

## System Specs

| Spec | Value |
|------|-------|
| **CPU** | Intel i9-11900F @ 2.50GHz (8C/16T, turbo 5.2GHz) |
| **RAM** | 32GB DDR4 |
| **OS** | Ubuntu 22.04 LTS (kernel 6.8.0) |
| **Rust** | 1.93.1 |

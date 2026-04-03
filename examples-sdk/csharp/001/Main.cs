// 001 — AI Gateway (SSE Pipeline)
// Equivalent to: examples/001-basic-ai-gw-demo (Rust)
// Compile: vil compile --from csharp --input 001/Main.cs --release
#load "../../../sdk/csharp/Vil.cs"

var pipeline = new VilPipeline("ai-gateway", 3080)
    .Sink("webhook", 3080, "/trigger")
    .Source("inference", "http://localhost:4545/v1/chat/completions",
        format: "sse", dialect: "openai", jsonTap: "choices[0].delta.content")
    .Route("webhook.trigger_out", "inference.trigger_in", "LoanWrite")
    .Route("inference.data_out", "webhook.data_in", "LoanWrite")
    .Route("inference.ctrl_out", "webhook.ctrl_in", "Copy");

pipeline.Compile();

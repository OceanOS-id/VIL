// 001 — AI Gateway (SSE Pipeline)
// Equivalent to: examples/001-basic-ai-gw-demo (Rust)
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	pipeline := vil.NewPipeline("DecomposedPipeline", 3080)

	pipeline.Sink(vil.SinkOpts{Port: 3080, Path: "/trigger", Name: "webhook_trigger"})
	pipeline.Source(vil.SourceOpts{
		URL:     "http://127.0.0.1:4545/v1/chat/completions",
		Format:  "sse",
		JSONTap: "choices[0].delta.content",
		Dialect: "openai",
		Name:    "sse_inference",
	})

	pipeline.Route("sink.trigger_out", "source.trigger_in", "LoanWrite")
	pipeline.Route("source.response_data_out", "sink.response_data_in", "LoanWrite")
	pipeline.Route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy")

	pipeline.Compile()
}

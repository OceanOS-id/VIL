// 016 — AI RAG Gateway (SSE Pipeline)
// Equivalent to: examples/016-basic-ai-rag-gateway (Rust)
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	pipeline := vil.NewPipeline("RagPipeline", 3084)

	pipeline.Sink(vil.SinkOpts{Port: 3084, Path: "/rag", Name: "rag_webhook"})
	pipeline.Source(vil.SourceOpts{
		URL:     "http://127.0.0.1:4545/v1/chat/completions",
		Format:  "sse",
		JSONTap: "choices[0].delta.content",
		Dialect: "openai",
		Name:    "rag_sse_inference",
	})

	pipeline.Route("sink.trigger_out", "source.trigger_in", "LoanWrite")
	pipeline.Route("source.response_data_out", "sink.response_data_in", "LoanWrite")
	pipeline.Route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy")

	pipeline.Compile()
}

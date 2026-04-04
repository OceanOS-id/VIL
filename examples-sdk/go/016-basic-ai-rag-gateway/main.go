// 016-basic-ai-rag-gateway — Go SDK equivalent
// Compile: vil compile --from go --input 016-basic-ai-rag-gateway/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	p := vil.NewPipeline("RagPipeline", 3084)

	p.Sink(vil.SinkOpts{Name: "rag_webhook", Port: 3084, Path: "/rag"})
	p.Source(vil.SourceOpts{Name: "rag_sse_inference", URL: "http://127.0.0.1:4545/v1/chat/completions", Format: "sse", JSONTap: "choices[0].delta.content", Dialect: "openai"})

	p.Route("sink.trigger_out", "source.trigger_in", "LoanWrite")
	p.Route("source.response_data_out", "sink.response_data_in", "LoanWrite")
	p.Route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy")

	p.Compile()
}

// 001-basic-ai-gw-demo — Go SDK equivalent
// Compile: vil compile --from go --input 001-basic-ai-gw-demo/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	p := vil.NewPipeline("DecomposedPipeline", 3080)

	p.Sink(vil.SinkOpts{Name: "webhook_trigger", Port: 3080, Path: "/trigger"})
	p.Source(vil.SourceOpts{Name: "sse_inference", URL: "http://127.0.0.1:4545/v1/chat/completions", Format: "sse", JSONTap: "choices[0].delta.content", Dialect: "openai"})

	p.Route("sink.trigger_out", "source.trigger_in", "LoanWrite")
	p.Route("source.response_data_out", "sink.response_data_in", "LoanWrite")
	p.Route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy")

	p.Compile()
}

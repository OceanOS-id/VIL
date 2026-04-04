// 019-basic-ai-multi-model-advanced — Go SDK equivalent
// Compile: vil compile --from go --input 019-basic-ai-multi-model-advanced/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	p := vil.NewPipeline("AdvancedMultiModelRouterPipeline", 3086)

	p.Sink(vil.SinkOpts{Name: "advanced_router_sink", Port: 3086, Path: "/route-advanced"})
	p.Source(vil.SourceOpts{Name: "advanced_router_source", URL: "http://127.0.0.1:4545/v1/chat/completions", Format: "sse", JSONTap: "choices[0].delta.content", Dialect: "openai"})

	p.Route("sink.trigger_out", "source.trigger_in", "LoanWrite")
	p.Route("source.response_data_out", "sink.response_data_in", "LoanWrite")
	p.Route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy")

	p.Compile()
}

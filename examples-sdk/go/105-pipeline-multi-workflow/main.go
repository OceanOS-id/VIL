// 105-pipeline-multi-workflow — Go SDK equivalent
// Compile: vil compile --from go --input 105-pipeline-multi-workflow/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	p := vil.NewPipeline("AiGatewayWorkflow", 3097)

	p.Sink(vil.SinkOpts{Name: "ai_gateway_sink", Port: 3097, Path: "/ai"})
	p.Source(vil.SourceOpts{Name: "ai_sse_source", URL: "http://127.0.0.1:4545/v1/chat/completions", Format: "sse", JSONTap: "choices[0].delta.content", Dialect: "openai"})
	p.Sink(vil.SinkOpts{Name: "credit_sink", Port: 3098, Path: "/credit"})
	p.Source(vil.SourceOpts{Name: "credit_ndjson_source", URL: "http://localhost:18081/api/v1/credits/ndjson?count=100", Format: "json"})
	p.Sink(vil.SinkOpts{Name: "inventory_sink", Port: 3099, Path: "/inventory"})
	p.Source(vil.SourceOpts{Name: "inventory_rest_source", URL: "http://localhost:18092/api/v1/products"})

	p.Route("ai_sink.trigger_out", "ai_source.trigger_in", "LoanWrite")
	p.Route("ai_source.response_data_out", "ai_sink.response_data_in", "LoanWrite")
	p.Route("ai_source.response_ctrl_out", "ai_sink.response_ctrl_in", "Copy")
	p.Route("credit_sink.trigger_out", "credit_source.trigger_in", "LoanWrite")
	p.Route("credit_source.response_data_out", "credit_sink.response_data_in", "LoanWrite")
	p.Route("credit_source.response_ctrl_out", "credit_sink.response_ctrl_in", "Copy")
	p.Route("inventory_sink.trigger_out", "inventory_source.trigger_in", "LoanWrite")
	p.Route("inventory_source.response_data_out", "inventory_sink.response_data_in", "LoanWrite")
	p.Route("inventory_source.response_ctrl_out", "inventory_sink.response_ctrl_in", "Copy")

	p.Compile()
}

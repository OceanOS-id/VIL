// 103-pipeline-fanin-gather — Go SDK equivalent
// Compile: vil compile --from go --input 103-pipeline-fanin-gather/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	p := vil.NewPipeline("CreditGatherPipeline", 3093)

	p.Sink(vil.SinkOpts{Name: "credit_gather_sink", Port: 3093, Path: "/gather"})
	p.Source(vil.SourceOpts{Name: "credit_source", URL: "http://localhost:18081/api/v1/credits/ndjson?count=100", Format: "json"})
	p.Sink(vil.SinkOpts{Name: "inventory_gather_sink", Port: 3094, Path: "/inventory"})
	p.Source(vil.SourceOpts{Name: "inventory_source", URL: "http://localhost:18092/api/v1/products"})

	p.Route("credit_sink.trigger_out", "credit_source.trigger_in", "LoanWrite")
	p.Route("credit_source.response_data_out", "credit_sink.response_data_in", "LoanWrite")
	p.Route("credit_source.response_ctrl_out", "credit_sink.response_ctrl_in", "Copy")
	p.Route("inventory_sink.trigger_out", "inventory_source.trigger_in", "LoanWrite")
	p.Route("inventory_source.response_data_out", "inventory_sink.response_data_in", "LoanWrite")
	p.Route("inventory_source.response_ctrl_out", "inventory_sink.response_ctrl_in", "Copy")

	p.Compile()
}

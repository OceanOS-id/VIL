// 009-basic-credit-regulatory-slik — Go SDK equivalent
// Compile: vil compile --from go --input 009-basic-credit-regulatory-slik/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	p := vil.NewPipeline("RegulatoryStreamPipeline", 3083)

	p.Sink(vil.SinkOpts{Name: "regulatory_sink", Port: 3083, Path: "/regulatory-stream"})
	p.Source(vil.SourceOpts{Name: "regulatory_source", URL: "http://localhost:18081/api/v1/credits/ndjson?count=1000", Format: "json"})

	p.Route("sink.trigger_out", "source.trigger_in", "LoanWrite")
	p.Route("source.response_data_out", "sink.response_data_in", "LoanWrite")
	p.Route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy")

	p.Compile()
}

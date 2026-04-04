// 107-pipeline-process-traced — Go SDK equivalent
// Compile: vil compile --from go --input 107-pipeline-process-traced/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	p := vil.NewPipeline("SupplyChainTrackedPipeline", 3107)

	p.Sink(vil.SinkOpts{Name: "tracking_sink", Port: 3107, Path: "/traced"})
	p.Source(vil.SourceOpts{Name: "supply_chain_source", URL: "http://localhost:18081/api/v1/credits/stream", Format: "sse"})

	p.Route("sink.trigger_out", "source.trigger_in", "LoanWrite")
	p.Route("source.tracking_data_out", "sink.tracking_data_in", "LoanWrite")
	p.Route("source.delivery_ctrl_out", "sink.delivery_ctrl_in", "Copy")

	p.Compile()
}

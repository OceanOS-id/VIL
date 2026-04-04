// 101-pipeline-3node-transform-chain — Go SDK equivalent
// Compile: vil compile --from go --input 101-pipeline-3node-transform-chain/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	p := vil.NewPipeline("TransformChainPipeline", 3090)

	p.Sink(vil.SinkOpts{Name: "transform_gateway", Port: 3090, Path: "/transform"})
	p.Source(vil.SourceOpts{Name: "chained_transform_source", URL: "http://localhost:18081/api/v1/credits/ndjson?count=100", Format: "json"})

	p.Route("sink.trigger_out", "source.trigger_in", "LoanWrite")
	p.Route("source.response_data_out", "sink.response_data_in", "LoanWrite")
	p.Route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy")

	p.Compile()
}

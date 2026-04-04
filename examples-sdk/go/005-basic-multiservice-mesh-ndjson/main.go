// 005-basic-multiservice-mesh-ndjson — Go SDK equivalent
// Compile: vil compile --from go --input 005-basic-multiservice-mesh-ndjson/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	p := vil.NewPipeline("MultiServiceMesh", 3084)

	p.Sink(vil.SinkOpts{Name: "gateway", Port: 3084, Path: "/ingest"})
	p.Source(vil.SourceOpts{Name: "credit_ingest", Format: "json"})

	p.Route("gateway.trigger_out", "ingest.trigger_in", "LoanWrite")
	p.Route("ingest.response_data_out", "gateway.response_data_in", "LoanWrite")
	p.Route("ingest.response_ctrl_out", "gateway.response_ctrl_in", "Copy")

	p.Compile()
}

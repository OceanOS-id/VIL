// 005 — Multiservice Mesh (NDJSON + Transform)
// Equivalent to: examples/005-basic-multiservice-mesh-ndjson (Rust)
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	pipeline := vil.NewPipeline("MultiServiceMesh", 3084)

	pipeline.Sink(vil.SinkOpts{Port: 3084, Path: "/ingest", Name: "gateway"})
	pipeline.Source(vil.SourceOpts{
		Format: "json",
		Name:   "credit_ingest",
	})

	pipeline.Route("gateway.trigger_out", "ingest.trigger_in", "LoanWrite")
	pipeline.Route("ingest.response_data_out", "gateway.response_data_in", "LoanWrite")
	pipeline.Route("ingest.response_ctrl_out", "gateway.response_ctrl_in", "Copy")

	pipeline.Compile()
}

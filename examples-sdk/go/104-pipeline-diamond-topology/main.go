// 104-pipeline-diamond-topology — Go SDK equivalent
// Compile: vil compile --from go --input 104-pipeline-diamond-topology/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	p := vil.NewPipeline("DiamondSummary", 3095)

	p.Sink(vil.SinkOpts{Name: "summary_sink", Port: 3095, Path: "/diamond"})
	p.Source(vil.SourceOpts{Name: "summary_source", URL: "http://localhost:18081/api/v1/credits/ndjson?count=100", Format: "json"})
	p.Sink(vil.SinkOpts{Name: "detail_sink", Port: 3096, Path: "/diamond-detail"})
	p.Source(vil.SourceOpts{Name: "detail_source", URL: "http://localhost:18081/api/v1/credits/ndjson?count=100", Format: "json"})

	p.Route("summary_sink.trigger_out", "summary_source.trigger_in", "LoanWrite")
	p.Route("summary_source.response_data_out", "summary_sink.response_data_in", "LoanWrite")
	p.Route("summary_source.response_ctrl_out", "summary_sink.response_ctrl_in", "Copy")
	p.Route("detail_sink.trigger_out", "detail_source.trigger_in", "LoanWrite")
	p.Route("detail_source.response_data_out", "detail_sink.response_data_in", "LoanWrite")
	p.Route("detail_source.response_ctrl_out", "detail_sink.response_ctrl_in", "Copy")

	p.Compile()
}

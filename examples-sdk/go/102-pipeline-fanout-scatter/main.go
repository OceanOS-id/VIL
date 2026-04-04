// 102-pipeline-fanout-scatter — Go SDK equivalent
// Compile: vil compile --from go --input 102-pipeline-fanout-scatter/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	p := vil.NewPipeline("NplPipeline", 3091)

	p.Sink(vil.SinkOpts{Name: "npl_sink", Port: 3091, Path: "/npl"})
	p.Source(vil.SourceOpts{Name: "npl_source", URL: "http://localhost:18081/api/v1/credits/ndjson?count=100", Format: "json"})
	p.Sink(vil.SinkOpts{Name: "healthy_sink", Port: 3092, Path: "/healthy"})
	p.Source(vil.SourceOpts{Name: "healthy_source", URL: "http://localhost:18081/api/v1/credits/ndjson?count=100", Format: "json"})

	p.Route("npl_sink.trigger_out", "npl_source.trigger_in", "LoanWrite")
	p.Route("npl_source.response_data_out", "npl_sink.response_data_in", "LoanWrite")
	p.Route("npl_source.response_ctrl_out", "npl_sink.response_ctrl_in", "Copy")
	p.Route("healthy_sink.trigger_out", "healthy_source.trigger_in", "LoanWrite")
	p.Route("healthy_source.response_data_out", "healthy_sink.response_data_in", "LoanWrite")
	p.Route("healthy_source.response_ctrl_out", "healthy_sink.response_ctrl_in", "Copy")

	p.Compile()
}

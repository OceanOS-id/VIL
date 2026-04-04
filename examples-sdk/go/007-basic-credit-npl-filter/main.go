// 007 — Credit NPL Filter (.transform)
// Equivalent to: examples/007-basic-credit-npl-filter (Rust)
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	pipeline := vil.NewPipeline("NplFilterPipeline", 3081)

	pipeline.Sink(vil.SinkOpts{Port: 3081, Path: "/filter-npl", Name: "npl_filter_sink"})
	pipeline.Source(vil.SourceOpts{
		Format: "json",
		Name:   "npl_credit_source",
	})

	pipeline.Route("sink.trigger_out", "source.trigger_in", "LoanWrite")
	pipeline.Route("source.response_data_out", "sink.response_data_in", "LoanWrite")
	pipeline.Route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy")

	pipeline.Compile()
}

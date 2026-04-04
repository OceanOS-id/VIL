// 007-basic-credit-npl-filter — Go SDK equivalent
// Compile: vil compile --from go --input 007-basic-credit-npl-filter/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	p := vil.NewPipeline("NplFilterPipeline", 3081)

	p.Sink(vil.SinkOpts{Name: "npl_filter_sink", Port: 3081, Path: "/filter-npl"})
	p.Source(vil.SourceOpts{Name: "npl_credit_source", Format: "json"})

	p.Route("sink.trigger_out", "source.trigger_in", "LoanWrite")
	p.Route("source.response_data_out", "sink.response_data_in", "LoanWrite")
	p.Route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy")

	p.Compile()
}

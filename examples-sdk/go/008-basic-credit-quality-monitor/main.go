// 008-basic-credit-quality-monitor — Go SDK equivalent
// Compile: vil compile --from go --input 008-basic-credit-quality-monitor/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	p := vil.NewPipeline("CreditQualityMonitorPipeline", 3082)

	p.Sink(vil.SinkOpts{Name: "quality_monitor_sink", Port: 3082, Path: "/quality-check"})
	p.Source(vil.SourceOpts{Name: "quality_credit_source", Format: "json"})

	p.Route("sink.trigger_out", "source.trigger_in", "LoanWrite")
	p.Route("source.response_data_out", "sink.response_data_in", "LoanWrite")
	p.Route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy")

	p.Compile()
}

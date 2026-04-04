// 101b-multi-pipeline-benchmark — Go SDK equivalent
// Compile: vil compile --from go --input 101b-multi-pipeline-benchmark/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	p := vil.NewPipeline("MultiPipelineBench", 3090)

	p.Sink(vil.SinkOpts{Name: "gateway", Port: 3090, Path: "/trigger"})
	p.Source(vil.SourceOpts{Name: "l_l_m_upstream", URL: "http://127.0.0.1:4545/v1/chat/completions", JSONTap: "choices[0].delta.content"})

	p.Route("sink.trigger_out", "source.trigger_in", "LoanWrite")
	p.Route("source.response_data_out", "sink.response_data_in", "LoanWrite")
	p.Route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy")

	p.Compile()
}

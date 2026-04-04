// 205-llm-chunked-summarizer — Go SDK equivalent
// Compile: vil compile --from go --input 205-llm-chunked-summarizer/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	p := vil.NewPipeline("ChunkedSummarizerPipeline", 8080)

	p.Route("sink.trigger_out", "source_summarize.trigger_in", "LoanWrite")
	p.Route("source_summarize.response_data_out", "sink.response_data_in", "LoanWrite")
	p.Route("source_summarize.response_ctrl_out", "sink.response_ctrl_in", "Copy")

	p.Compile()
}

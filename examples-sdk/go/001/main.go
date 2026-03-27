// 001 — AI Gateway (SSE Pipeline)
// Equivalent to: examples/001-basic-ai-gw-demo (Rust)
// Compile: vil compile --from go --input 001/main.go --release
package main

import (
	"fmt"
	"os"

	vil "github.com/OceanOS-id/vil-go"
)

func main() {
	pipeline := vil.NewPipeline("ai-gateway", 3080)

	// -- Semantic types -------------------------------------------------------
	pipeline.SemanticType("InferenceState", "state", map[string]string{
		"request_id": "u64",
		"tokens":     "u32",
	})
	pipeline.Fault("InferenceFault", []string{"UpstreamTimeout", "ParseError"})

	// -- Nodes ----------------------------------------------------------------
	pipeline.Sink(vil.SinkOpts{Port: 3080, Path: "/trigger", Name: "webhook"})
	pipeline.Source(vil.SourceOpts{
		URL:     "http://localhost:4545/v1/chat/completions",
		Format:  "sse",
		Dialect: "openai",
		JsonTap: "choices[0].delta.content",
		Name:    "inference",
	})

	// -- Tri-Lane routes ------------------------------------------------------
	pipeline.Route("webhook.trigger_out", "inference.trigger_in", "LoanWrite")
	pipeline.Route("inference.data_out", "webhook.data_in", "LoanWrite")
	pipeline.Route("inference.ctrl_out", "webhook.ctrl_in", "Copy")

	// -- Emit / compile -------------------------------------------------------
	if os.Getenv("VIL_COMPILE_MODE") == "manifest" {
		fmt.Println(pipeline.ToYAML())
	} else {
		pipeline.Compile()
	}
}

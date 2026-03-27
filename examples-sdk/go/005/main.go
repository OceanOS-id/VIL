// 005 — Multiservice Mesh (NDJSON + Transform)
// Equivalent to: examples/005-basic-multiservice-mesh-ndjson (Rust)
// Compile: vil compile --from go --input 005/main.go --release
package main

import (
	"fmt"
	"os"

	vil "github.com/OceanOS-id/vil-go"
)

func main() {
	pipeline := vil.NewPipeline("multiservice-mesh", 3084, vil.WithToken("shm"))

	// -- Semantic types -------------------------------------------------------
	pipeline.SemanticType("MeshState", "state", map[string]string{
		"request_id":         "u64",
		"messages_forwarded": "u32",
		"active_pipelines":   "u8",
	})
	pipeline.Fault("MeshFault", []string{
		"ProcessorTimeout", "AnalyticsTimeout", "ShmWriteFailed", "RouteNotFound",
	})

	// -- Nodes ----------------------------------------------------------------
	pipeline.Sink(vil.SinkOpts{Port: 3084, Path: "/ingest", Name: "gateway"})
	pipeline.Source(vil.SourceOpts{
		URL: "http://127.0.0.1:4545/v1/chat/completions", Format: "sse",
		Dialect: "openai", JsonTap: "choices[0].delta.content", Name: "processor",
	})

	// -- Transform: NDJSON enrichment -----------------------------------------
	pipeline.Transform("enrich", `
		let mut obj = serde_json::from_slice(line).ok()?;
		obj["processed"] = serde_json::json!(true);
		Some(serde_json::to_vec(&obj).ok()?)
	`)

	// -- Tri-Lane routes ------------------------------------------------------
	pipeline.Route("gateway.trigger_out", "processor.trigger_in", "LoanWrite")
	pipeline.Route("processor.data_out", "enrich.data_in", "LoanWrite")
	pipeline.Route("enrich.data_out", "gateway.data_in", "LoanWrite")
	pipeline.Route("processor.ctrl_out", "gateway.ctrl_in", "Copy")

	// -- Emit / compile -------------------------------------------------------
	if os.Getenv("VIL_COMPILE_MODE") == "manifest" {
		fmt.Println(pipeline.ToYAML())
	} else {
		pipeline.Compile()
	}
}

// 016 — AI RAG Gateway (SSE Pipeline)
// Equivalent to: examples/016-basic-ai-rag-gateway (Rust)
// Compile: vil compile --from go --input 016/main.go --release
package main

import (
	"fmt"
	"os"

	vil "github.com/OceanOS-id/vil-go"
)

func main() {
	pipeline := vil.NewPipeline("ai-rag-gateway", 3084)

	// -- Semantic types -------------------------------------------------------
	pipeline.SemanticType("RagState", "state", map[string]string{
		"query_id":         "u64",
		"chunks_retrieved": "u32",
		"tokens_generated": "u32",
	})
	pipeline.Fault("RagFault", []string{
		"VectorDbTimeout", "EmbeddingError", "UpstreamTimeout", "ParseError",
	})

	// -- Nodes ----------------------------------------------------------------
	pipeline.Sink(vil.SinkOpts{Port: 3084, Path: "/rag", Name: "gateway"})
	pipeline.Source(vil.SourceOpts{
		URL: "http://localhost:4545/v1/chat/completions", Format: "sse",
		Dialect: "openai", JsonTap: "choices[0].delta.content", Name: "inference",
	})

	// -- Transform: inject retrieved context before LLM call ------------------
	pipeline.Transform("context_inject", `
		let mut req = serde_json::from_slice(line).ok()?;
		let ctx = retrieve_context(&req["query"].as_str()?);
		req["messages"][0]["content"] = serde_json::json!(ctx);
		Some(serde_json::to_vec(&req).ok()?)
	`)

	// -- Tri-Lane routes ------------------------------------------------------
	pipeline.Route("gateway.trigger_out", "context_inject.trigger_in", "LoanWrite")
	pipeline.Route("context_inject.data_out", "inference.trigger_in", "LoanWrite")
	pipeline.Route("inference.data_out", "gateway.data_in", "LoanWrite")
	pipeline.Route("inference.ctrl_out", "gateway.ctrl_in", "Copy")

	// -- Emit / compile -------------------------------------------------------
	if os.Getenv("VIL_COMPILE_MODE") == "manifest" {
		fmt.Println(pipeline.ToYAML())
	} else {
		pipeline.Compile()
	}
}

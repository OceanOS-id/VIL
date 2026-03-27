// 007 — Credit NPL Filter (.transform)
// Equivalent to: examples/007-basic-credit-npl-filter (Rust)
// Compile: vil compile --from go --input 007/main.go --release
package main

import (
	"fmt"
	"os"

	vil "github.com/OceanOS-id/vil-go"
)

func main() {
	pipeline := vil.NewPipeline("credit-npl-filter", 3081)

	// -- Semantic types -------------------------------------------------------
	pipeline.SemanticType("CreditRecord", "event", map[string]string{
		"loan_id":         "u64",
		"kolektabilitas":  "u8",
		"outstanding":     "f64",
	})
	pipeline.Fault("FilterFault", []string{"ParseError", "UpstreamTimeout"})

	// -- Nodes ----------------------------------------------------------------
	pipeline.Source(vil.SourceOpts{
		URL:    "http://localhost:18081/api/v1/credits/ndjson?count=1000",
		Format: "ndjson",
		Name:   "credits",
	})
	pipeline.Sink(vil.SinkOpts{Port: 3081, Path: "/filter", Name: "output"})

	// -- Transform: keep only NPL (kolektabilitas >= 3) -----------------------
	pipeline.Transform("npl_filter", `
		let record = serde_json::from_slice(line).ok()?;
		if record["kolektabilitas"].as_u64()? >= 3 { Some(line.to_vec()) } else { None }
	`)

	// -- Tri-Lane routes ------------------------------------------------------
	pipeline.Route("credits.data_out", "npl_filter.data_in", "LoanWrite")
	pipeline.Route("npl_filter.data_out", "output.data_in", "LoanWrite")
	pipeline.Route("credits.ctrl_out", "output.ctrl_in", "Copy")

	// -- Emit / compile -------------------------------------------------------
	if os.Getenv("VIL_COMPILE_MODE") == "manifest" {
		fmt.Println(pipeline.ToYAML())
	} else {
		pipeline.Compile()
	}
}

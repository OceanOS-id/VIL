// 023-basic-hybrid-wasm-sidecar — Go SDK equivalent
// Compile: vil compile --from go --input 023-basic-hybrid-wasm-sidecar/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("hybrid-pipeline", 8080)

	pipeline := vil.NewService("pipeline")
	pipeline.Endpoint("GET", "/", "index")
	pipeline.Endpoint("POST", "/validate", "validate_order")
	pipeline.Endpoint("POST", "/price", "calculate_price")
	pipeline.Endpoint("POST", "/fraud", "fraud_check")
	pipeline.Endpoint("POST", "/order", "process_order")
	s.Service(pipeline)

	s.Compile()
}

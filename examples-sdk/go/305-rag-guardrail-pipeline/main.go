// 305-rag-guardrail-pipeline — Go SDK equivalent
// Compile: vil compile --from go --input 305-rag-guardrail-pipeline/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("rag-guardrail-pipeline", 3114)

	rag_guardrail := vil.NewService("rag-guardrail")
	rag_guardrail.Endpoint("POST", "/safe-rag", "safe_rag_handler")
	s.Service(rag_guardrail)

	s.Compile()
}

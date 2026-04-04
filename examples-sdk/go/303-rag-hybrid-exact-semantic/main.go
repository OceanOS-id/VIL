// 303-rag-hybrid-exact-semantic — Go SDK equivalent
// Compile: vil compile --from go --input 303-rag-hybrid-exact-semantic/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("rag-hybrid-exact-semantic", 3112)

	rag_hybrid := vil.NewService("rag-hybrid")
	rag_hybrid.Endpoint("POST", "/hybrid", "hybrid_handler")
	s.Service(rag_hybrid)

	s.Compile()
}

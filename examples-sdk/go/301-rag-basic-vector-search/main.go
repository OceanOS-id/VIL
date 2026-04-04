// 301-rag-basic-vector-search — Go SDK equivalent
// Compile: vil compile --from go --input 301-rag-basic-vector-search/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("rag-basic-vector-search", 3110)

	rag_basic := vil.NewService("rag-basic")
	rag_basic.Endpoint("POST", "/rag", "rag_handler")
	s.Service(rag_basic)

	s.Compile()
}

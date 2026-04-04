// 025-basic-rag-service — Go SDK equivalent
// Compile: vil compile --from go --input 025-basic-rag-service/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("rag-service", 3091)

	rag := vil.NewService("rag")
	rag.Endpoint("POST", "/rag", "rag_handler")
	s.Service(rag)

	s.Compile()
}

// 304-rag-citation-extraction — Go SDK equivalent
// Compile: vil compile --from go --input 304-rag-citation-extraction/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("rag-citation-extraction", 3113)

	rag_citation := vil.NewService("rag-citation")
	rag_citation.Endpoint("POST", "/cited-rag", "cited_rag_handler")
	s.Service(rag_citation)

	s.Compile()
}

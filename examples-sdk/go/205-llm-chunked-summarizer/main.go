// 205-llm-chunked-summarizer — Go SDK equivalent
// Compile: vil compile --from go --input 205-llm-chunked-summarizer/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("ChunkedSummarizerPipeline", 8080)
	s.Compile()
}

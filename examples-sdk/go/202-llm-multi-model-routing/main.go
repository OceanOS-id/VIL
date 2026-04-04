// 202-llm-multi-model-routing — Go SDK equivalent
// Compile: vil compile --from go --input 202-llm-multi-model-routing/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("MultiModelPipeline_GPT4", 8080)
	s.Compile()
}

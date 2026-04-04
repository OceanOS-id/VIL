// 101c-vilapp-multi-pipeline-benchmark — Go SDK equivalent
// Compile: vil compile --from go --input 101c-vilapp-multi-pipeline-benchmark/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("multi-pipeline-bench", 3090)

	pipeline := vil.NewService("pipeline")
	s.Service(pipeline)

	s.Compile()
}

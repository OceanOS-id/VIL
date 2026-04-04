// 204-llm-streaming-translator — Go SDK equivalent
// Compile: vil compile --from go --input 204-llm-streaming-translator/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("llm-streaming-translator", 3103)

	translator := vil.NewService("translator")
	s.Service(translator)

	s.Compile()
}

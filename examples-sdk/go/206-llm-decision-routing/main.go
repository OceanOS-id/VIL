// 206-llm-decision-routing — Go SDK equivalent
// Compile: vil compile --from go --input 206-llm-decision-routing/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("insurance-underwriting-ai", 3116)

	underwriter := vil.NewService("underwriter")
	s.Service(underwriter)

	s.Compile()
}

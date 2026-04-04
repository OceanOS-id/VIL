// 020-basic-ai-ab-testing — Go SDK equivalent
// Compile: vil compile --from go --input 020-basic-ai-ab-testing/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("ai-ab-testing-gateway", 8080)

	ab := vil.NewService("ab")
	ab.Endpoint("POST", "/infer", "infer")
	ab.Endpoint("GET", "/metrics", "metrics")
	ab.Endpoint("POST", "/config", "update_config")
	s.Service(ab)

	root := vil.NewService("root")
	root.Endpoint("GET", "/", "index")
	s.Service(root)

	s.Compile()
}

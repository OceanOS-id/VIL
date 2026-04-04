// 018-basic-ai-multi-model-router — Go SDK equivalent
// Compile: vil compile --from go --input 018-basic-ai-multi-model-router/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("ai-multi-model-router", 3085)

	router := vil.NewService("router")
	router.Endpoint("POST", "/route", "route_handler")
	s.Service(router)

	s.Compile()
}

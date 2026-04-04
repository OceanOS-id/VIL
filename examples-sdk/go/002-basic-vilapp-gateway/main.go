// 002-basic-vilapp-gateway — Go SDK equivalent
// Compile: vil compile --from go --input 002-basic-vilapp-gateway/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("vil-app-gateway", 3081)

	gw := vil.NewService("gw")
	gw.Endpoint("POST", "/trigger", "trigger_handler")
	s.Service(gw)

	s.Compile()
}

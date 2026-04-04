// 406-agent-vil-handler-shm — Go SDK equivalent
// Compile: vil compile --from go --input 406-agent-vil-handler-shm/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("fraud-detection-agent", 3126)

	fraud_agent := vil.NewService("fraud-agent")
	fraud_agent.Endpoint("POST", "/detect", "detect_fraud")
	fraud_agent.Endpoint("GET", "/health", "health")
	s.Service(fraud_agent)

	s.Compile()
}

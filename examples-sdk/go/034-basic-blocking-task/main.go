// 034-basic-blocking-task — Go SDK equivalent
// Compile: vil compile --from go --input 034-basic-blocking-task/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("credit-risk-scoring-engine", 8080)

	risk_engine := vil.NewService("risk-engine")
	risk_engine.Endpoint("POST", "/risk/assess", "assess_risk")
	risk_engine.Endpoint("GET", "/risk/health", "risk_health")
	s.Service(risk_engine)

	s.Compile()
}

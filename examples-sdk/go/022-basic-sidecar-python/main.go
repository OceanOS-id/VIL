// 022-basic-sidecar-python — Go SDK equivalent
// Compile: vil compile --from go --input 022-basic-sidecar-python/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("sidecar-python-example", 8080)

	fraud := vil.NewService("fraud")
	fraud.Endpoint("GET", "/status", "fraud_status")
	fraud.Endpoint("POST", "/check", "fraud_check")
	s.Service(fraud)

	root := vil.NewService("root")
	root.Endpoint("GET", "/", "index")
	s.Service(root)

	s.Compile()
}

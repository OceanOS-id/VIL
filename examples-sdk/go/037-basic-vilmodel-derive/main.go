// 037-basic-vilmodel-derive — Go SDK equivalent
// Compile: vil compile --from go --input 037-basic-vilmodel-derive/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("insurance-claim-processing", 8080)

	claims := vil.NewService("claims")
	claims.Endpoint("POST", "/claims/submit", "submit_claim")
	claims.Endpoint("GET", "/claims/sample", "sample_claim")
	s.Service(claims)

	s.Compile()
}

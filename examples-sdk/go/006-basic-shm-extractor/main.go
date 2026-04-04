// 006-basic-shm-extractor — Go SDK equivalent
// Compile: vil compile --from go --input 006-basic-shm-extractor/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("shm-extractor-demo", 8080)

	shm_demo := vil.NewService("shm-demo")
	shm_demo.Endpoint("POST", "/ingest", "ingest")
	shm_demo.Endpoint("POST", "/compute", "compute")
	shm_demo.Endpoint("GET", "/shm-stats", "shm_stats")
	shm_demo.Endpoint("GET", "/benchmark", "benchmark")
	s.Service(shm_demo)

	s.Compile()
}

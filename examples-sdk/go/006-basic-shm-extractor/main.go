// 006-basic-shm-extractor — Go SDK equivalent
// Domain: Capital Markets — HFT Data Ingestion
// Compile: vil compile --from go --input 006-basic-shm-extractor/main.go --release
//
// Business: Processes high-frequency market data via SHM zero-copy.
//
// VIL handles all endpoints (HTTP, routing, SHM). Custom business logic
// runs as activities within each endpoint via sidecar or wasm.
//
// Switch mode: VIL_MODE=sidecar (default) / VIL_MODE=wasm

package main

import (
	"example/006-basic-shm-extractor/handlers"

	vil "github.com/OceanOS-id/vil-go"
)

// ── Mode switch — visible, explicit ──

var mode = vil.ModeFromEnv()

// ── Register activities (pure business logic) ──

var Ingest = vil.Activity("HandleIngest", mode, "shm", handlers.HandleIngest)
var Compute = vil.Activity("HandleCompute", mode, "shm", handlers.HandleCompute)
var ShmStats = vil.Activity("HandleShmStats", mode, "shm", handlers.HandleShmStats)
var Benchmark = vil.Activity("HandleBenchmark", mode, "shm", handlers.HandleBenchmark)

func main() {
	// Sidecar dispatch: VIL_HANDLER=<name> → run activity + exit
	vil.Run(Ingest, Compute, ShmStats, Benchmark)

	// SDK mode: VIL handles endpoints, activities handle business logic
	s := vil.NewServer("shm-extractor-demo", 8080)
	shmDemo := vil.NewService("shm-demo")

	shmDemo.Endpoint("POST", "/ingest", "ingest", vil.WithActivity(Ingest))
	shmDemo.Endpoint("POST", "/compute", "compute", vil.WithActivity(Compute))
	shmDemo.Endpoint("GET", "/shm-stats", "shm_stats", vil.WithActivity(ShmStats))
	shmDemo.Endpoint("GET", "/benchmark", "benchmark", vil.WithActivity(Benchmark))

	s.Service(shmDemo)
	s.Compile()
}

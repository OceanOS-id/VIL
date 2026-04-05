package handlers

import (
	"encoding/json"
	"time"
)

// HandleCompute — POST /compute
// CPU-bound: N iterations of wrapping multiply+add hash loop.
func HandleCompute(body []byte) interface{} {
	var req struct {
		Iterations uint64 `json:"iterations"`
	}
	json.Unmarshal(body, &req)
	iterations := req.Iterations
	if iterations > 100_000_000 {
		iterations = 100_000_000
	}

	start := time.Now()
	var hash uint64
	for i := uint64(0); i < iterations; i++ {
		hash += i*17 + 31
	}
	elapsed := time.Since(start)

	return map[string]interface{}{
		"status":      "computed",
		"iterations":  iterations,
		"result_hash": hash,
		"elapsed_ms":  float64(elapsed.Nanoseconds()) / 1_000_000,
		"thread":      "sidecar_process",
		"note":        "CPU-bound work runs in sidecar process, freeing async threads for I/O",
	}
}

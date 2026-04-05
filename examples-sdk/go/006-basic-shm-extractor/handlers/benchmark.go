package handlers

import "time"

// HandleBenchmark — GET /benchmark
// Returns nanosecond timestamp.
func HandleBenchmark(body []byte) interface{} {
	return map[string]interface{}{
		"ok":           true,
		"timestamp_ns": time.Now().UnixNano(),
	}
}

// ML Engine Sidecar — Go ML inference sidecar example.
//
// This sidecar connects to a VIL host and provides ML inference
// via a simple scoring model (replace with real ML in production).
//
// Architecture:
//
//	VlangApp (Rust) → UDS + SHM → ml_engine (Go)
//	- Zero-copy: request/response data in /dev/shm
//	- Transport: Unix Domain Socket (descriptors only)
//
// Usage:
//
//	go run ml_engine.go
//
//	Or auto-spawned by VlangApp with:
//	  .sidecar(SidecarConfig::new("ml-engine")
//	      .command("go run examples-sdk/sidecar/go/ml_engine.go"))
package main

import (
	"fmt"
	"math"

	vil "github.com/oceanos-id/vil-sidecar-go"
)

func main() {
	fmt.Println("==================================================")
	fmt.Println("  ML Engine Sidecar (Go)")
	fmt.Println("  VIL WASM FaaS + Sidecar Hybrid")
	fmt.Println("==================================================")
	fmt.Println()

	app := vil.NewSidecar("ml-engine")
	app.Version = "1.0.0"

	// Handler: predict — single inference
	app.Handle("predict", func(req vil.Request) vil.Response {
		data := req.JSON()

		// Simple sigmoid-based scoring (replace with real ML)
		features, ok := data["features"].([]interface{})
		if !ok {
			return vil.Err("missing 'features' array")
		}

		// Compute weighted sum
		weights := []float64{0.3, -0.5, 0.8, 0.2, -0.1}
		sum := 0.0
		for i, f := range features {
			val, ok := f.(float64)
			if !ok {
				continue
			}
			if i < len(weights) {
				sum += val * weights[i]
			}
		}

		// Sigmoid
		score := 1.0 / (1.0 + math.Exp(-sum))

		return vil.OK(map[string]interface{}{
			"score":         math.Round(score*1000) / 1000,
			"prediction":    boolToLabel(score > 0.5),
			"confidence":    math.Round(math.Abs(score-0.5)*2*1000) / 1000,
			"model_version": "sigmoid-v1.0",
		})
	})

	// Handler: batch_predict — batch inference
	app.Handle("batch_predict", func(req vil.Request) vil.Response {
		data := req.JSON()
		items, ok := data["items"].([]interface{})
		if !ok {
			return vil.Err("missing 'items' array")
		}

		results := make([]map[string]interface{}, 0, len(items))
		for _, item := range items {
			itemMap, ok := item.(map[string]interface{})
			if !ok {
				continue
			}
			// Re-invoke predict logic
			innerReq := vil.Request{Method: "predict", Data: itemMap}
			resp := predictSingle(innerReq)
			results = append(results, resp.Data)
		}

		return vil.OK(map[string]interface{}{
			"results":    results,
			"batch_size": len(results),
		})
	})

	// Handler: health_info — extended health info
	app.Handle("health_info", func(req vil.Request) vil.Response {
		return vil.OK(map[string]interface{}{
			"model":   "sigmoid-v1.0",
			"status":  "ready",
			"gpu":     false,
			"runtime": "go",
		})
	})

	app.Run()
}

func predictSingle(req vil.Request) vil.Response {
	data := req.JSON()
	features, ok := data["features"].([]interface{})
	if !ok {
		return vil.Err("missing features")
	}

	weights := []float64{0.3, -0.5, 0.8, 0.2, -0.1}
	sum := 0.0
	for i, f := range features {
		if val, ok := f.(float64); ok && i < len(weights) {
			sum += val * weights[i]
		}
	}

	score := 1.0 / (1.0 + math.Exp(-sum))
	return vil.OK(map[string]interface{}{
		"score":      math.Round(score*1000) / 1000,
		"prediction": boolToLabel(score > 0.5),
	})
}

func boolToLabel(b bool) string {
	if b {
		return "positive"
	}
	return "negative"
}

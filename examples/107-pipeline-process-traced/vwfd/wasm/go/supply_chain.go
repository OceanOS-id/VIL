// Supply Chain Tracker — Go WASM Module
// Adds hop metadata (timestamp, location) for supply chain tracing.
package main

import (
	"encoding/json"
	"fmt"
	"os"
	"time"
)

func main() {
	var input map[string]interface{}
	if err := json.NewDecoder(os.Stdin).Decode(&input); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}

	input["hop_timestamp"] = time.Now().UnixMilli()
	input["hop_processor"] = "go-wasm-supply-chain"
	input["traced"] = true

	json.NewEncoder(os.Stdout).Encode(input)
}

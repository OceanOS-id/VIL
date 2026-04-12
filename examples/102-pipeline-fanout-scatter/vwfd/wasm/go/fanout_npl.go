// NPL Fan-Out Filter — Go WASM Module
// Classifies loans as NPL (kolektabilitas >= 3) or healthy.
package main

import (
	"encoding/json"
	"fmt"
	"os"
)

type Loan struct {
	NIK            string  `json:"nik"`
	Kolektabilitas int     `json:"kolektabilitas"`
	Outstanding    float64 `json:"outstanding"`
}

func main() {
	var input Loan
	if err := json.NewDecoder(os.Stdin).Decode(&input); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}

	isNPL := input.Kolektabilitas >= 3
	category := "healthy"
	if isNPL {
		category = "npl"
	}

	result := map[string]interface{}{
		"nik":            input.NIK,
		"kolektabilitas": input.Kolektabilitas,
		"outstanding":    input.Outstanding,
		"is_npl":         isNPL,
		"category":       category,
	}
	json.NewEncoder(os.Stdout).Encode(result)
}

package handlers

import (
	"encoding/json"
	"strings"
	"time"
)

// HandleTransform — POST /transform
// Receive JSON, apply transformation (uppercase + double value), return result.
func HandleTransform(body []byte) interface{} {
	var req struct {
		Data  string  `json:"data"`
		Value float64 `json:"value"`
	}
	json.Unmarshal(body, &req)

	return map[string]interface{}{
		"original":      req.Data,
		"transformed":   strings.ToUpper(req.Data),
		"value_doubled": req.Value * 2.0,
		"timestamp":     time.Now().Unix(),
	}
}

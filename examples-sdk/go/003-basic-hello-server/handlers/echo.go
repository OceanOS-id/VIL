package handlers

import "encoding/json"

// HandleEcho — POST /echo
// Echo request body (useful for integration testing).
func HandleEcho(body []byte) interface{} {
	var parsed interface{}
	if err := json.Unmarshal(body, &parsed); err != nil {
		parsed = nil
	}

	return map[string]interface{}{
		"received_bytes": len(body),
		"body":           parsed,
		"zero_copy":      true,
	}
}

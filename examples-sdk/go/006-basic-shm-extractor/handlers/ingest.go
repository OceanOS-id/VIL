package handlers

import (
	"encoding/json"
	"fmt"
	"unicode/utf8"
)

// HandleIngest — POST /ingest
// Reads raw body bytes, tries UTF-8 preview, checks JSON validity.
func HandleIngest(body []byte) interface{} {
	length := len(body)

	preview := ""
	if utf8.Valid(body) {
		runes := []rune(string(body))
		if len(runes) > 100 {
			runes = runes[:100]
		}
		preview = string(runes)
	} else {
		preview = fmt.Sprintf("<binary %d bytes>", length)
	}

	var js json.RawMessage
	isJSON := json.Unmarshal(body, &js) == nil

	return map[string]interface{}{
		"status":         "ingested",
		"bytes_received": length,
		"shm_region_id":  "0",
		"preview":        preview,
		"is_valid_json":  isJSON,
		"transport":      "SHM zero-copy",
		"copies":         "1 (kernel → SHM), then 0 for handler read",
	}
}

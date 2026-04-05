package handlers

// HandleHealth — GET /health
// Service health check.
func HandleHealth(body []byte) interface{} {
	return map[string]interface{}{
		"status":  "healthy",
		"service": "vil-api",
		"shm":     true,
	}
}

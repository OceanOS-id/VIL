// 404-agent-data-csv-analyst — Go SDK equivalent
// Compile: vil compile --from go --input 404-agent-data-csv-analyst/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("csv-analyst-agent", 3123)

	csv_analyst_agent := vil.NewService("csv-analyst-agent")
	csv_analyst_agent.Endpoint("POST", "/csv-analyze", "csv_analyze_handler")
	s.Service(csv_analyst_agent)

	s.Compile()
}

// 608-db-vilorm-analytics — Go SDK equivalent
// Compile: vil compile --from go --input 608-db-vilorm-analytics/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("vilorm-analytics", 8088)

	analytics := vil.NewService("analytics")
	analytics.Endpoint("POST", "/events", "log_event")
	analytics.Endpoint("GET", "/events/recent", "recent_events")
	analytics.Endpoint("GET", "/events/by-type", "events_by_type")
	analytics.Endpoint("GET", "/stats/daily", "daily_stats")
	analytics.Endpoint("GET", "/stats/unique-users", "unique_users")
	analytics.Endpoint("GET", "/stats/summary", "stats_summary")
	s.Service(analytics)

	s.Compile()
}

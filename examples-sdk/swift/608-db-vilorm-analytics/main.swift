// 608-db-vilorm-analytics — Swift SDK equivalent
// Compile: vil compile --from swift --input 608-db-vilorm-analytics/main.swift --release

let server = VilServer(name: "vilorm-analytics", port: 8088)
let analytics = ServiceProcess(name: "analytics")
analytics.endpoint(method: "POST", path: "/events", handler: "log_event")
analytics.endpoint(method: "GET", path: "/events/recent", handler: "recent_events")
analytics.endpoint(method: "GET", path: "/events/by-type", handler: "events_by_type")
analytics.endpoint(method: "GET", path: "/stats/daily", handler: "daily_stats")
analytics.endpoint(method: "GET", path: "/stats/unique-users", handler: "unique_users")
analytics.endpoint(method: "GET", path: "/stats/summary", handler: "stats_summary")
server.service(analytics)
server.compile()

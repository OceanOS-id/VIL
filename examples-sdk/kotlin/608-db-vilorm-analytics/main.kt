// 608-db-vilorm-analytics — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 608-db-vilorm-analytics/main.kt --release

fun main() {
    val server = VilServer("vilorm-analytics", 8088)
    val analytics = ServiceProcess("analytics")
    analytics.endpoint("POST", "/events", "log_event")
    analytics.endpoint("GET", "/events/recent", "recent_events")
    analytics.endpoint("GET", "/events/by-type", "events_by_type")
    analytics.endpoint("GET", "/stats/daily", "daily_stats")
    analytics.endpoint("GET", "/stats/unique-users", "unique_users")
    analytics.endpoint("GET", "/stats/summary", "stats_summary")
    server.service(analytics)
    server.compile()
}

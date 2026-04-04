// 608-db-vilorm-analytics — Zig SDK equivalent
// Compile: vil compile --from zig --input 608-db-vilorm-analytics/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("vilorm-analytics", 8088);
    var analytics = vil.Service.init("analytics");
    analytics.endpoint("POST", "/events", "log_event");
    analytics.endpoint("GET", "/events/recent", "recent_events");
    analytics.endpoint("GET", "/events/by-type", "events_by_type");
    analytics.endpoint("GET", "/stats/daily", "daily_stats");
    analytics.endpoint("GET", "/stats/unique-users", "unique_users");
    analytics.endpoint("GET", "/stats/summary", "stats_summary");
    server.service(&analytics);
    server.compile();
}

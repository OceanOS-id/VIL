// 608-db-vilorm-analytics — C# SDK equivalent
// Compile: vil compile --from csharp --input 608-db-vilorm-analytics/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("vilorm-analytics", 8088);
var analytics = new ServiceProcess("analytics");
analytics.Endpoint("POST", "/events", "log_event");
analytics.Endpoint("GET", "/events/recent", "recent_events");
analytics.Endpoint("GET", "/events/by-type", "events_by_type");
analytics.Endpoint("GET", "/stats/daily", "daily_stats");
analytics.Endpoint("GET", "/stats/unique-users", "unique_users");
analytics.Endpoint("GET", "/stats/summary", "stats_summary");
server.Service(analytics);
server.Compile();

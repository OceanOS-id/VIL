// 608-db-vilorm-analytics — Java SDK equivalent
// Compile: vil compile --from java --input 608-db-vilorm-analytics/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("vilorm-analytics", 8088);
        ServiceProcess analytics = new ServiceProcess("analytics");
        analytics.endpoint("POST", "/events", "log_event");
        analytics.endpoint("GET", "/events/recent", "recent_events");
        analytics.endpoint("GET", "/events/by-type", "events_by_type");
        analytics.endpoint("GET", "/stats/daily", "daily_stats");
        analytics.endpoint("GET", "/stats/unique-users", "unique_users");
        analytics.endpoint("GET", "/stats/summary", "stats_summary");
        server.service(analytics);
        server.compile();
    }
}

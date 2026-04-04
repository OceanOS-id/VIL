// 012-basic-plugin-database — Java SDK equivalent
// Compile: vil compile --from java --input 012-basic-plugin-database/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("plugin-database", 8080);
        ServiceProcess plugin_db = new ServiceProcess("plugin-db");
        plugin_db.endpoint("GET", "/", "index");
        plugin_db.endpoint("GET", "/plugins", "list_plugins");
        plugin_db.endpoint("GET", "/config", "show_config");
        plugin_db.endpoint("GET", "/products", "list_products");
        plugin_db.endpoint("POST", "/tasks", "create_task");
        plugin_db.endpoint("GET", "/tasks", "list_tasks");
        plugin_db.endpoint("GET", "/pool-stats", "pool_stats");
        plugin_db.endpoint("GET", "/redis-ping", "redis_ping");
        server.service(plugin_db);
        server.compile();
    }
}

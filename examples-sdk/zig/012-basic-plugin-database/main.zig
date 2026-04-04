// 012-basic-plugin-database — Zig SDK equivalent
// Compile: vil compile --from zig --input 012-basic-plugin-database/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("plugin-database", 8080);
    var plugin_db = vil.Service.init("plugin-db");
    plugin_db.endpoint("GET", "/", "index");
    plugin_db.endpoint("GET", "/plugins", "list_plugins");
    plugin_db.endpoint("GET", "/config", "show_config");
    plugin_db.endpoint("GET", "/products", "list_products");
    plugin_db.endpoint("POST", "/tasks", "create_task");
    plugin_db.endpoint("GET", "/tasks", "list_tasks");
    plugin_db.endpoint("GET", "/pool-stats", "pool_stats");
    plugin_db.endpoint("GET", "/redis-ping", "redis_ping");
    server.service(&plugin_db);
    server.compile();
}

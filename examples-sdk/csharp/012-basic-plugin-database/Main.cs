// 012-basic-plugin-database — C# SDK equivalent
// Compile: vil compile --from csharp --input 012-basic-plugin-database/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("plugin-database", 8080);
var plugin_db = new ServiceProcess("plugin-db");
plugin_db.Endpoint("GET", "/", "index");
plugin_db.Endpoint("GET", "/plugins", "list_plugins");
plugin_db.Endpoint("GET", "/config", "show_config");
plugin_db.Endpoint("GET", "/products", "list_products");
plugin_db.Endpoint("POST", "/tasks", "create_task");
plugin_db.Endpoint("GET", "/tasks", "list_tasks");
plugin_db.Endpoint("GET", "/pool-stats", "pool_stats");
plugin_db.Endpoint("GET", "/redis-ping", "redis_ping");
server.Service(plugin_db);
server.Compile();

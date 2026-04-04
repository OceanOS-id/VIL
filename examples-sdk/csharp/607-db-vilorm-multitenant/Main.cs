// 607-db-vilorm-multitenant — C# SDK equivalent
// Compile: vil compile --from csharp --input 607-db-vilorm-multitenant/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("vilorm-multitenant", 8087);
var saas = new ServiceProcess("saas");
saas.Endpoint("POST", "/tenants", "create_tenant");
saas.Endpoint("GET", "/tenants/:id", "get_tenant");
saas.Endpoint("PUT", "/tenants/:id", "update_tenant");
saas.Endpoint("POST", "/tenants/:id/users", "add_user");
saas.Endpoint("GET", "/tenants/:id/users", "list_users");
saas.Endpoint("POST", "/tenants/:id/settings", "upsert_setting");
saas.Endpoint("GET", "/tenants/:id/settings", "list_settings");
saas.Endpoint("GET", "/tenants/:id/stats", "tenant_stats");
server.Service(saas);
server.Compile();

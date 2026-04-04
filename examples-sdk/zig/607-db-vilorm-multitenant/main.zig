// 607-db-vilorm-multitenant — Zig SDK equivalent
// Compile: vil compile --from zig --input 607-db-vilorm-multitenant/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("vilorm-multitenant", 8087);
    var saas = vil.Service.init("saas");
    saas.endpoint("POST", "/tenants", "create_tenant");
    saas.endpoint("GET", "/tenants/:id", "get_tenant");
    saas.endpoint("PUT", "/tenants/:id", "update_tenant");
    saas.endpoint("POST", "/tenants/:id/users", "add_user");
    saas.endpoint("GET", "/tenants/:id/users", "list_users");
    saas.endpoint("POST", "/tenants/:id/settings", "upsert_setting");
    saas.endpoint("GET", "/tenants/:id/settings", "list_settings");
    saas.endpoint("GET", "/tenants/:id/stats", "tenant_stats");
    server.service(&saas);
    server.compile();
}

// 607-db-vilorm-multitenant — Java SDK equivalent
// Compile: vil compile --from java --input 607-db-vilorm-multitenant/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("vilorm-multitenant", 8087);
        ServiceProcess saas = new ServiceProcess("saas");
        saas.endpoint("POST", "/tenants", "create_tenant");
        saas.endpoint("GET", "/tenants/:id", "get_tenant");
        saas.endpoint("PUT", "/tenants/:id", "update_tenant");
        saas.endpoint("POST", "/tenants/:id/users", "add_user");
        saas.endpoint("GET", "/tenants/:id/users", "list_users");
        saas.endpoint("POST", "/tenants/:id/settings", "upsert_setting");
        saas.endpoint("GET", "/tenants/:id/settings", "list_settings");
        saas.endpoint("GET", "/tenants/:id/stats", "tenant_stats");
        server.service(saas);
        server.compile();
    }
}

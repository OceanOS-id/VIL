// 032-basic-failover-ha — Zig SDK equivalent
// Compile: vil compile --from zig --input 032-basic-failover-ha/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("payment-gateway-ha", 8080);
    var primary = vil.Service.init("primary");
    primary.endpoint("GET", "/health", "primary_health");
    primary.endpoint("POST", "/charge", "primary_charge");
    server.service(&primary);
    var backup = vil.Service.init("backup");
    backup.endpoint("GET", "/health", "backup_health");
    backup.endpoint("POST", "/charge", "backup_charge");
    server.service(&backup);
    server.compile();
}

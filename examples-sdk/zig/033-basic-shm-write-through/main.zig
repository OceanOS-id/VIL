// 033-basic-shm-write-through — Zig SDK equivalent
// Compile: vil compile --from zig --input 033-basic-shm-write-through/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("realtime-analytics-dashboard", 8080);
    var catalog = vil.Service.init("catalog");
    catalog.endpoint("POST", "/catalog/search", "catalog_search");
    catalog.endpoint("GET", "/catalog/health", "catalog_health");
    server.service(&catalog);
    server.compile();
}

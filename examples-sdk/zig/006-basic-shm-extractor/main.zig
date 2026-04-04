// 006-basic-shm-extractor — Zig SDK equivalent
// Compile: vil compile --from zig --input 006-basic-shm-extractor/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("shm-extractor-demo", 8080);
    var shm_demo = vil.Service.init("shm-demo");
    shm_demo.endpoint("POST", "/ingest", "ingest");
    shm_demo.endpoint("POST", "/compute", "compute");
    shm_demo.endpoint("GET", "/shm-stats", "shm_stats");
    shm_demo.endpoint("GET", "/benchmark", "benchmark");
    server.service(&shm_demo);
    server.compile();
}

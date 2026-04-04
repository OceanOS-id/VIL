// 013-basic-nats-worker — Zig SDK equivalent
// Compile: vil compile --from zig --input 013-basic-nats-worker/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("nats-worker", 8080);
    var nats = vil.Service.init("nats");
    nats.endpoint("GET", "/nats/config", "nats_config");
    nats.endpoint("POST", "/nats/publish", "nats_publish");
    nats.endpoint("GET", "/nats/jetstream", "jetstream_info");
    nats.endpoint("GET", "/nats/kv", "kv_demo");
    server.service(&nats);
    var root = vil.Service.init("root");
    server.service(&root);
    server.compile();
}

// 028-basic-sse-hub-streaming — Zig SDK equivalent
// Compile: vil compile --from zig --input 028-basic-sse-hub-streaming/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("sse-hub-demo", 8080);
    var events = vil.Service.init("events");
    events.endpoint("POST", "/publish", "publish");
    events.endpoint("GET", "/stream", "stream");
    events.endpoint("GET", "/stats", "stats");
    server.service(&events);
    server.compile();
}

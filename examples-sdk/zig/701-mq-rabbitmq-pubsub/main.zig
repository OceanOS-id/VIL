// 701-mq-rabbitmq-pubsub — Zig SDK equivalent
// Compile: vil compile --from zig --input 701-mq-rabbitmq-pubsub/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("app", 8080);
    server.compile();
}

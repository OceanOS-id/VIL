// 702-mq-sqs-send-receive — Zig SDK equivalent
// Compile: vil compile --from zig --input 702-mq-sqs-send-receive/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("app", 8080);
    server.compile();
}

// 010-basic-websocket-chat — Zig SDK equivalent
// Compile: vil compile --from zig --input 010-basic-websocket-chat/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("websocket-chat", 8080);
    var chat = vil.Service.init("chat");
    chat.endpoint("GET", "/", "index");
    chat.endpoint("GET", "/ws", "ws_handler");
    chat.endpoint("GET", "/stats", "stats");
    server.service(&chat);
    server.compile();
}

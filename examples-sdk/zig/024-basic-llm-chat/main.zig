// 024-basic-llm-chat — Zig SDK equivalent
// Compile: vil compile --from zig --input 024-basic-llm-chat/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("llm-chat", 8080);
    var chat = vil.Service.init("chat");
    chat.endpoint("POST", "/chat", "chat_handler");
    server.service(&chat);
    server.compile();
}

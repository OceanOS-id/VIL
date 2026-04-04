// 201-llm-basic-chat — Zig SDK equivalent
// Compile: vil compile --from zig --input 201-llm-basic-chat/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("llm-basic-chat", 3100);
    var chat = vil.Service.init("chat");
    chat.endpoint("POST", "/chat", "chat_handler");
    server.service(&chat);
    server.compile();
}

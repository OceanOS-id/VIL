// 306-rag-ai-event-tracking — Zig SDK equivalent
// Compile: vil compile --from zig --input 306-rag-ai-event-tracking/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("customer-support-rag", 3116);
    var support = vil.Service.init("support");
    support.endpoint("POST", "/support/ask", "answer_question");
    server.service(&support);
    server.compile();
}

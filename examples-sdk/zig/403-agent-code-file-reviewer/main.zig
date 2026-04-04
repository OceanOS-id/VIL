// 403-agent-code-file-reviewer — Zig SDK equivalent
// Compile: vil compile --from zig --input 403-agent-code-file-reviewer/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("code-file-reviewer-agent", 3122);
    var code_review_agent = vil.Service.init("code-review-agent");
    code_review_agent.endpoint("POST", "/code-review", "code_review_handler");
    server.service(&code_review_agent);
    server.compile();
}

// 203-llm-code-review-with-tools — Zig SDK equivalent
// Compile: vil compile --from zig --input 203-llm-code-review-with-tools/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("llm-code-review-tools", 3102);
    var code_review = vil.Service.init("code-review");
    code_review.endpoint("POST", "/code/review", "code_review_handler");
    server.service(&code_review);
    server.compile();
}

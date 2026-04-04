// 205-llm-chunked-summarizer — Zig SDK equivalent
const vil = @import("vil");
pub fn main() void {
    var p = vil.Pipeline.init("ChunkedSummarizerPipeline", 8080);
    p.route("sink.trigger_out", "source_summarize.trigger_in", "LoanWrite");
    p.route("source_summarize.response_data_out", "sink.response_data_in", "LoanWrite");
    p.route("source_summarize.response_ctrl_out", "sink.response_ctrl_in", "Copy");
    p.compile();
}

// 202-llm-multi-model-routing — Zig SDK equivalent
const vil = @import("vil");
pub fn main() void {
    var p = vil.Pipeline.init("MultiModelPipeline_GPT4", 8080);
    p.route("sink.trigger_out", "source_gpt4.trigger_in", "LoanWrite");
    p.route("source_gpt4.response_data_out", "sink.response_data_in", "LoanWrite");
    p.route("source_gpt4.response_ctrl_out", "sink.response_ctrl_in", "Copy");
    p.compile();
}

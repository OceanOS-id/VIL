// 019-basic-ai-multi-model-advanced — Java SDK equivalent
// Compile: vil compile --from java --input 019-basic-ai-multi-model-advanced/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilPipeline p = new VilPipeline("AdvancedMultiModelRouterPipeline", 3086);
        p.sink(3086, "/route-advanced", "advanced_router_sink");
        p.source("http://127.0.0.1:4545/v1/chat/completions", "sse", "advanced_router_source");
        p.route("sink.trigger_out", "source.trigger_in", "LoanWrite");
        p.route("source.response_data_out", "sink.response_data_in", "LoanWrite");
        p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
        p.compile();
    }
}

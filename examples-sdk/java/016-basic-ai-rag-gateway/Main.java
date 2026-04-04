// 016-basic-ai-rag-gateway — Java SDK equivalent
// Compile: vil compile --from java --input 016-basic-ai-rag-gateway/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilPipeline p = new VilPipeline("RagPipeline", 3084);
        p.sink(3084, "/rag", "rag_webhook");
        p.source("http://127.0.0.1:4545/v1/chat/completions", "sse", "rag_sse_inference");
        p.route("sink.trigger_out", "source.trigger_in", "LoanWrite");
        p.route("source.response_data_out", "sink.response_data_in", "LoanWrite");
        p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
        p.compile();
    }
}

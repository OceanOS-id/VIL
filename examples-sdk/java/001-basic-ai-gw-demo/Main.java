// 001-basic-ai-gw-demo — Java SDK equivalent
// Compile: vil compile --from java --input 001-basic-ai-gw-demo/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilPipeline p = new VilPipeline("DecomposedPipeline", 3080);
        p.sink(3080, "/trigger", "webhook_trigger");
        p.source("http://127.0.0.1:4545/v1/chat/completions", "sse", "sse_inference");
        p.route("sink.trigger_out", "source.trigger_in", "LoanWrite");
        p.route("source.response_data_out", "sink.response_data_in", "LoanWrite");
        p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
        p.compile();
    }
}

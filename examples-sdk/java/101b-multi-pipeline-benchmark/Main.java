// 101b-multi-pipeline-benchmark — Java SDK equivalent
// Compile: vil compile --from java --input 101b-multi-pipeline-benchmark/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilPipeline p = new VilPipeline("MultiPipelineBench", 3090);
        p.sink(3090, "/trigger", "gateway");
        p.source("http://127.0.0.1:4545/v1/chat/completions", "", "l_l_m_upstream");
        p.route("sink.trigger_out", "source.trigger_in", "LoanWrite");
        p.route("source.response_data_out", "sink.response_data_in", "LoanWrite");
        p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
        p.compile();
    }
}

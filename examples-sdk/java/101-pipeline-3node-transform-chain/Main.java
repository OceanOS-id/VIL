// 101-pipeline-3node-transform-chain — Java SDK equivalent
// Compile: vil compile --from java --input 101-pipeline-3node-transform-chain/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilPipeline p = new VilPipeline("TransformChainPipeline", 3090);
        p.sink(3090, "/transform", "transform_gateway");
        p.source("http://localhost:18081/api/v1/credits/ndjson?count=100", "json", "chained_transform_source");
        p.route("sink.trigger_out", "source.trigger_in", "LoanWrite");
        p.route("source.response_data_out", "sink.response_data_in", "LoanWrite");
        p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
        p.compile();
    }
}

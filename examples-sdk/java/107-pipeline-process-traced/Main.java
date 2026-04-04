// 107-pipeline-process-traced — Java SDK equivalent
// Compile: vil compile --from java --input 107-pipeline-process-traced/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilPipeline p = new VilPipeline("SupplyChainTrackedPipeline", 3107);
        p.sink(3107, "/traced", "tracking_sink");
        p.source("http://localhost:18081/api/v1/credits/stream", "sse", "supply_chain_source");
        p.route("sink.trigger_out", "source.trigger_in", "LoanWrite");
        p.route("source.tracking_data_out", "sink.tracking_data_in", "LoanWrite");
        p.route("source.delivery_ctrl_out", "sink.delivery_ctrl_in", "Copy");
        p.compile();
    }
}

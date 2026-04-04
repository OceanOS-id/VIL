// 009-basic-credit-regulatory-slik — Java SDK equivalent
// Compile: vil compile --from java --input 009-basic-credit-regulatory-slik/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilPipeline p = new VilPipeline("RegulatoryStreamPipeline", 3083);
        p.sink(3083, "/regulatory-stream", "regulatory_sink");
        p.source("http://localhost:18081/api/v1/credits/ndjson?count=1000", "json", "regulatory_source");
        p.route("sink.trigger_out", "source.trigger_in", "LoanWrite");
        p.route("source.response_data_out", "sink.response_data_in", "LoanWrite");
        p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
        p.compile();
    }
}

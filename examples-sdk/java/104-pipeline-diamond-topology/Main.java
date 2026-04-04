// 104-pipeline-diamond-topology — Java SDK equivalent
// Compile: vil compile --from java --input 104-pipeline-diamond-topology/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilPipeline p = new VilPipeline("DiamondSummary", 3095);
        p.sink(3095, "/diamond", "summary_sink");
        p.source("http://localhost:18081/api/v1/credits/ndjson?count=100", "json", "summary_source");
        p.sink(3096, "/diamond-detail", "detail_sink");
        p.source("http://localhost:18081/api/v1/credits/ndjson?count=100", "json", "detail_source");
        p.route("summary_sink.trigger_out", "summary_source.trigger_in", "LoanWrite");
        p.route("summary_source.response_data_out", "summary_sink.response_data_in", "LoanWrite");
        p.route("summary_source.response_ctrl_out", "summary_sink.response_ctrl_in", "Copy");
        p.route("detail_sink.trigger_out", "detail_source.trigger_in", "LoanWrite");
        p.route("detail_source.response_data_out", "detail_sink.response_data_in", "LoanWrite");
        p.route("detail_source.response_ctrl_out", "detail_sink.response_ctrl_in", "Copy");
        p.compile();
    }
}

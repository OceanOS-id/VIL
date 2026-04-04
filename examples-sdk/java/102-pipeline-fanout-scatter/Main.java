// 102-pipeline-fanout-scatter — Java SDK equivalent
// Compile: vil compile --from java --input 102-pipeline-fanout-scatter/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilPipeline p = new VilPipeline("NplPipeline", 3091);
        p.sink(3091, "/npl", "npl_sink");
        p.source("http://localhost:18081/api/v1/credits/ndjson?count=100", "json", "npl_source");
        p.sink(3092, "/healthy", "healthy_sink");
        p.source("http://localhost:18081/api/v1/credits/ndjson?count=100", "json", "healthy_source");
        p.route("npl_sink.trigger_out", "npl_source.trigger_in", "LoanWrite");
        p.route("npl_source.response_data_out", "npl_sink.response_data_in", "LoanWrite");
        p.route("npl_source.response_ctrl_out", "npl_sink.response_ctrl_in", "Copy");
        p.route("healthy_sink.trigger_out", "healthy_source.trigger_in", "LoanWrite");
        p.route("healthy_source.response_data_out", "healthy_sink.response_data_in", "LoanWrite");
        p.route("healthy_source.response_ctrl_out", "healthy_sink.response_ctrl_in", "Copy");
        p.compile();
    }
}

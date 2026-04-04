// 103-pipeline-fanin-gather — Java SDK equivalent
// Compile: vil compile --from java --input 103-pipeline-fanin-gather/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilPipeline p = new VilPipeline("CreditGatherPipeline", 3093);
        p.sink(3093, "/gather", "credit_gather_sink");
        p.source("http://localhost:18081/api/v1/credits/ndjson?count=100", "json", "credit_source");
        p.sink(3094, "/inventory", "inventory_gather_sink");
        p.source("http://localhost:18092/api/v1/products", "", "inventory_source");
        p.route("credit_sink.trigger_out", "credit_source.trigger_in", "LoanWrite");
        p.route("credit_source.response_data_out", "credit_sink.response_data_in", "LoanWrite");
        p.route("credit_source.response_ctrl_out", "credit_sink.response_ctrl_in", "Copy");
        p.route("inventory_sink.trigger_out", "inventory_source.trigger_in", "LoanWrite");
        p.route("inventory_source.response_data_out", "inventory_sink.response_data_in", "LoanWrite");
        p.route("inventory_source.response_ctrl_out", "inventory_sink.response_ctrl_in", "Copy");
        p.compile();
    }
}

// 105-pipeline-multi-workflow — Java SDK equivalent
// Compile: vil compile --from java --input 105-pipeline-multi-workflow/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilPipeline p = new VilPipeline("AiGatewayWorkflow", 3097);
        p.sink(3097, "/ai", "ai_gateway_sink");
        p.source("http://127.0.0.1:4545/v1/chat/completions", "sse", "ai_sse_source");
        p.sink(3098, "/credit", "credit_sink");
        p.source("http://localhost:18081/api/v1/credits/ndjson?count=100", "json", "credit_ndjson_source");
        p.sink(3099, "/inventory", "inventory_sink");
        p.source("http://localhost:18092/api/v1/products", "", "inventory_rest_source");
        p.route("ai_sink.trigger_out", "ai_source.trigger_in", "LoanWrite");
        p.route("ai_source.response_data_out", "ai_sink.response_data_in", "LoanWrite");
        p.route("ai_source.response_ctrl_out", "ai_sink.response_ctrl_in", "Copy");
        p.route("credit_sink.trigger_out", "credit_source.trigger_in", "LoanWrite");
        p.route("credit_source.response_data_out", "credit_sink.response_data_in", "LoanWrite");
        p.route("credit_source.response_ctrl_out", "credit_sink.response_ctrl_in", "Copy");
        p.route("inventory_sink.trigger_out", "inventory_source.trigger_in", "LoanWrite");
        p.route("inventory_source.response_data_out", "inventory_sink.response_data_in", "LoanWrite");
        p.route("inventory_source.response_ctrl_out", "inventory_sink.response_ctrl_in", "Copy");
        p.compile();
    }
}

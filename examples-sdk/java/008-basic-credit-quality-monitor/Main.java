// 008-basic-credit-quality-monitor — Java SDK equivalent
// Compile: vil compile --from java --input 008-basic-credit-quality-monitor/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilPipeline p = new VilPipeline("CreditQualityMonitorPipeline", 3082);
        p.sink(3082, "/quality-check", "quality_monitor_sink");
        p.source("", "json", "quality_credit_source");
        p.route("sink.trigger_out", "source.trigger_in", "LoanWrite");
        p.route("source.response_data_out", "sink.response_data_in", "LoanWrite");
        p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
        p.compile();
    }
}

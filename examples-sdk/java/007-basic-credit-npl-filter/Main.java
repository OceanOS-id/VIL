// 007-basic-credit-npl-filter — Java SDK equivalent
// Compile: vil compile --from java --input 007-basic-credit-npl-filter/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilPipeline p = new VilPipeline("NplFilterPipeline", 3081);
        p.sink(3081, "/filter-npl", "npl_filter_sink");
        p.source("", "json", "npl_credit_source");
        p.route("sink.trigger_out", "source.trigger_in", "LoanWrite");
        p.route("source.response_data_out", "sink.response_data_in", "LoanWrite");
        p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
        p.compile();
    }
}

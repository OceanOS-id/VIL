// 005-basic-multiservice-mesh-ndjson — Java SDK equivalent
// Compile: vil compile --from java --input 005-basic-multiservice-mesh-ndjson/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilPipeline p = new VilPipeline("MultiServiceMesh", 3084);
        p.sink(3084, "/ingest", "gateway");
        p.source("", "json", "credit_ingest");
        p.route("gateway.trigger_out", "ingest.trigger_in", "LoanWrite");
        p.route("ingest.response_data_out", "gateway.response_data_in", "LoanWrite");
        p.route("ingest.response_ctrl_out", "gateway.response_ctrl_in", "Copy");
        p.compile();
    }
}

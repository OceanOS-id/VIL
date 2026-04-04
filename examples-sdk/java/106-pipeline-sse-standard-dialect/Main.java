// 106-pipeline-sse-standard-dialect — Java SDK equivalent
// Compile: vil compile --from java --input 106-pipeline-sse-standard-dialect/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilPipeline p = new VilPipeline("IoTSensorPipeline", 3106);
        p.sink(3106, "/stream", "io_t_dashboard_sink");
        p.source("http://localhost:18081/api/v1/credits/stream", "sse", "io_t_sensor_source");
        p.route("sink.trigger_out", "source.trigger_in", "LoanWrite");
        p.route("source.sensor_data_out", "sink.sensor_data_in", "LoanWrite");
        p.route("source.batch_ctrl_out", "sink.batch_ctrl_in", "Copy");
        p.compile();
    }
}

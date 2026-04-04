// 106-pipeline-sse-standard-dialect — C# SDK equivalent
// Compile: vil compile --from csharp --input 106-pipeline-sse-standard-dialect/Main.cs --release

#r "sdk/csharp/Vil.cs"

var p = new VilPipeline("IoTSensorPipeline", 3106);
p.Sink("io_t_dashboard_sink", 3106, "/stream");
p.Source("io_t_sensor_source", "http://localhost:18081/api/v1/credits/stream", "sse");
p.Route("sink.trigger_out", "source.trigger_in", "LoanWrite");
p.Route("source.sensor_data_out", "sink.sensor_data_in", "LoanWrite");
p.Route("source.batch_ctrl_out", "sink.batch_ctrl_in", "Copy");
p.Compile();

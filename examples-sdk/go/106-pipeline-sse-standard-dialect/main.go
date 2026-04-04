// 106-pipeline-sse-standard-dialect — Go SDK equivalent
// Compile: vil compile --from go --input 106-pipeline-sse-standard-dialect/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	p := vil.NewPipeline("IoTSensorPipeline", 3106)

	p.Sink(vil.SinkOpts{Name: "io_t_dashboard_sink", Port: 3106, Path: "/stream"})
	p.Source(vil.SourceOpts{Name: "io_t_sensor_source", URL: "http://localhost:18081/api/v1/credits/stream", Format: "sse"})

	p.Route("sink.trigger_out", "source.trigger_in", "LoanWrite")
	p.Route("source.sensor_data_out", "sink.sensor_data_in", "LoanWrite")
	p.Route("source.batch_ctrl_out", "sink.batch_ctrl_in", "Copy")

	p.Compile()
}

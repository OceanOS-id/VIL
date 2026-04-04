// 005-basic-multiservice-mesh-ndjson — C# SDK equivalent
// Compile: vil compile --from csharp --input 005-basic-multiservice-mesh-ndjson/Main.cs --release

#r "sdk/csharp/Vil.cs"

var p = new VilPipeline("MultiServiceMesh", 3084);
p.Sink("gateway", 3084, "/ingest");
p.Source("credit_ingest", "json");
p.Route("gateway.trigger_out", "ingest.trigger_in", "LoanWrite");
p.Route("ingest.response_data_out", "gateway.response_data_in", "LoanWrite");
p.Route("ingest.response_ctrl_out", "gateway.response_ctrl_in", "Copy");
p.Compile();

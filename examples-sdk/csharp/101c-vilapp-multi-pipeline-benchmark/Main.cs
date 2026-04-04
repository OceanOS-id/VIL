// 101c-vilapp-multi-pipeline-benchmark — C# SDK equivalent
// Compile: vil compile --from csharp --input 101c-vilapp-multi-pipeline-benchmark/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("multi-pipeline-bench", 3090);
var pipeline = new ServiceProcess("pipeline");
server.Service(pipeline);
server.Compile();

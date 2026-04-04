// 202-llm-multi-model-routing — C# SDK equivalent
// Compile: vil compile --from csharp --input 202-llm-multi-model-routing/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("MultiModelPipeline_GPT4", 8080);
server.Compile();

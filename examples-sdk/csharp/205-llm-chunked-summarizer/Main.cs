// 205-llm-chunked-summarizer — C# SDK equivalent
// Compile: vil compile --from csharp --input 205-llm-chunked-summarizer/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("ChunkedSummarizerPipeline", 8080);
server.Compile();

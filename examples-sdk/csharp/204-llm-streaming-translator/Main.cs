// 204-llm-streaming-translator — C# SDK equivalent
// Compile: vil compile --from csharp --input 204-llm-streaming-translator/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("llm-streaming-translator", 3103);
var translator = new ServiceProcess("translator");
server.Service(translator);
server.Compile();

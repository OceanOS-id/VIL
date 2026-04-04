// 302-rag-multi-source-fanin — C# SDK equivalent
// Compile: vil compile --from csharp --input 302-rag-multi-source-fanin/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("rag-multi-source-fanin", 3111);
server.Compile();

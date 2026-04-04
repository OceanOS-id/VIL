// 024-basic-llm-chat — C# SDK equivalent
// Compile: vil compile --from csharp --input 024-basic-llm-chat/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("llm-chat", 8080);
var chat = new ServiceProcess("chat");
chat.Endpoint("POST", "/chat", "chat_handler");
server.Service(chat);
server.Compile();

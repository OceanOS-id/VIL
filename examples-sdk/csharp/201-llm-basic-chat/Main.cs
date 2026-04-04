// 201-llm-basic-chat — C# SDK equivalent
// Compile: vil compile --from csharp --input 201-llm-basic-chat/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("llm-basic-chat", 3100);
var chat = new ServiceProcess("chat");
chat.Endpoint("POST", "/chat", "chat_handler");
server.Service(chat);
server.Compile();

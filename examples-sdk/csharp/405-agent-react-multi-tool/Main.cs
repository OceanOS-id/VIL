// 405-agent-react-multi-tool — C# SDK equivalent
// Compile: vil compile --from csharp --input 405-agent-react-multi-tool/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("react-multi-tool-agent", 3124);
var react_agent = new ServiceProcess("react-agent");
react_agent.Endpoint("POST", "/react", "react_handler");
server.Service(react_agent);
server.Compile();

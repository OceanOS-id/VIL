// 017-basic-production-fullstack — C# SDK equivalent
// Compile: vil compile --from csharp --input 017-basic-production-fullstack/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("production-fullstack", 8080);
var fullstack = new ServiceProcess("fullstack");
fullstack.Endpoint("GET", "/stack", "stack_info");
fullstack.Endpoint("GET", "/config", "full_config");
fullstack.Endpoint("GET", "/sprints", "sprints");
fullstack.Endpoint("GET", "/middleware", "middleware_info");
server.Service(fullstack);
var admin = new ServiceProcess("admin");
admin.Endpoint("GET", "/config", "full_config");
server.Service(admin);
var root = new ServiceProcess("root");
server.Service(root);
server.Compile();

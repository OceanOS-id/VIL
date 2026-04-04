// 203-llm-code-review-with-tools — C# SDK equivalent
// Compile: vil compile --from csharp --input 203-llm-code-review-with-tools/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("llm-code-review-tools", 3102);
var code_review = new ServiceProcess("code-review");
code_review.Endpoint("POST", "/code/review", "code_review_handler");
server.Service(code_review);
server.Compile();

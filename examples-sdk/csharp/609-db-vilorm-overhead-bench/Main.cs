// 609-db-vilorm-overhead-bench — C# SDK equivalent
// Compile: vil compile --from csharp --input 609-db-vilorm-overhead-bench/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("overhead-bench", 8099);
var bench = new ServiceProcess("bench");
bench.Endpoint("GET", "/raw/items/:id", "raw_find_by_id");
bench.Endpoint("GET", "/raw/items", "raw_list");
bench.Endpoint("GET", "/raw/count", "raw_count");
bench.Endpoint("GET", "/raw/cols", "raw_select_cols");
bench.Endpoint("GET", "/orm/items/:id", "orm_find_by_id");
bench.Endpoint("GET", "/orm/items", "orm_list");
bench.Endpoint("GET", "/orm/count", "orm_count");
bench.Endpoint("GET", "/orm/cols", "orm_select_cols");
server.Service(bench);
server.Compile();

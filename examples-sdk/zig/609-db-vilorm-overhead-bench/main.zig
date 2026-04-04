// 609-db-vilorm-overhead-bench — Zig SDK equivalent
// Compile: vil compile --from zig --input 609-db-vilorm-overhead-bench/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("overhead-bench", 8099);
    var bench = vil.Service.init("bench");
    bench.endpoint("GET", "/raw/items/:id", "raw_find_by_id");
    bench.endpoint("GET", "/raw/items", "raw_list");
    bench.endpoint("GET", "/raw/count", "raw_count");
    bench.endpoint("GET", "/raw/cols", "raw_select_cols");
    bench.endpoint("GET", "/orm/items/:id", "orm_find_by_id");
    bench.endpoint("GET", "/orm/items", "orm_list");
    bench.endpoint("GET", "/orm/count", "orm_count");
    bench.endpoint("GET", "/orm/cols", "orm_select_cols");
    server.service(&bench);
    server.compile();
}

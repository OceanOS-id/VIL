// 603-db-clickhouse-batch — Zig SDK equivalent
// Compile: vil compile --from zig --input 603-db-clickhouse-batch/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("app", 8080);
    server.compile();
}

// 609-db-vilorm-overhead-bench — Java SDK equivalent
// Compile: vil compile --from java --input 609-db-vilorm-overhead-bench/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("overhead-bench", 8099);
        ServiceProcess bench = new ServiceProcess("bench");
        bench.endpoint("GET", "/raw/items/:id", "raw_find_by_id");
        bench.endpoint("GET", "/raw/items", "raw_list");
        bench.endpoint("GET", "/raw/count", "raw_count");
        bench.endpoint("GET", "/raw/cols", "raw_select_cols");
        bench.endpoint("GET", "/orm/items/:id", "orm_find_by_id");
        bench.endpoint("GET", "/orm/items", "orm_list");
        bench.endpoint("GET", "/orm/count", "orm_count");
        bench.endpoint("GET", "/orm/cols", "orm_select_cols");
        server.service(bench);
        server.compile();
    }
}

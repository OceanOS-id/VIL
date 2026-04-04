// 606-db-vilorm-ecommerce — Java SDK equivalent
// Compile: vil compile --from java --input 606-db-vilorm-ecommerce/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("vilorm-ecommerce", 8086);
        ServiceProcess shop = new ServiceProcess("shop");
        shop.endpoint("POST", "/products", "create_product");
        shop.endpoint("GET", "/products", "list_products");
        shop.endpoint("GET", "/products/:id", "get_product");
        shop.endpoint("POST", "/orders", "create_order");
        shop.endpoint("GET", "/orders", "list_orders");
        shop.endpoint("GET", "/orders/:id/total", "order_total");
        shop.endpoint("DELETE", "/orders/:id", "cancel_order");
        server.service(shop);
        server.compile();
    }
}

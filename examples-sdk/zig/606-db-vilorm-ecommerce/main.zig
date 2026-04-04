// 606-db-vilorm-ecommerce — Zig SDK equivalent
// Compile: vil compile --from zig --input 606-db-vilorm-ecommerce/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("vilorm-ecommerce", 8086);
    var shop = vil.Service.init("shop");
    shop.endpoint("POST", "/products", "create_product");
    shop.endpoint("GET", "/products", "list_products");
    shop.endpoint("GET", "/products/:id", "get_product");
    shop.endpoint("POST", "/orders", "create_order");
    shop.endpoint("GET", "/orders", "list_orders");
    shop.endpoint("GET", "/orders/:id/total", "order_total");
    shop.endpoint("DELETE", "/orders/:id", "cancel_order");
    server.service(&shop);
    server.compile();
}

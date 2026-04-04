// 606-db-vilorm-ecommerce — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 606-db-vilorm-ecommerce/main.kt --release

fun main() {
    val server = VilServer("vilorm-ecommerce", 8086)
    val shop = ServiceProcess("shop")
    shop.endpoint("POST", "/products", "create_product")
    shop.endpoint("GET", "/products", "list_products")
    shop.endpoint("GET", "/products/:id", "get_product")
    shop.endpoint("POST", "/orders", "create_order")
    shop.endpoint("GET", "/orders", "list_orders")
    shop.endpoint("GET", "/orders/:id/total", "order_total")
    shop.endpoint("DELETE", "/orders/:id", "cancel_order")
    server.service(shop)
    server.compile()
}

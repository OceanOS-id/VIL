// 606-db-vilorm-ecommerce — Swift SDK equivalent
// Compile: vil compile --from swift --input 606-db-vilorm-ecommerce/main.swift --release

let server = VilServer(name: "vilorm-ecommerce", port: 8086)
let shop = ServiceProcess(name: "shop")
shop.endpoint(method: "POST", path: "/products", handler: "create_product")
shop.endpoint(method: "GET", path: "/products", handler: "list_products")
shop.endpoint(method: "GET", path: "/products/:id", handler: "get_product")
shop.endpoint(method: "POST", path: "/orders", handler: "create_order")
shop.endpoint(method: "GET", path: "/orders", handler: "list_orders")
shop.endpoint(method: "GET", path: "/orders/:id/total", handler: "order_total")
shop.endpoint(method: "DELETE", path: "/orders/:id", handler: "cancel_order")
server.service(shop)
server.compile()

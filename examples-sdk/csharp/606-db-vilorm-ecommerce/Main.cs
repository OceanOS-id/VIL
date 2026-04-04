// 606-db-vilorm-ecommerce — C# SDK equivalent
// Compile: vil compile --from csharp --input 606-db-vilorm-ecommerce/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("vilorm-ecommerce", 8086);
var shop = new ServiceProcess("shop");
shop.Endpoint("POST", "/products", "create_product");
shop.Endpoint("GET", "/products", "list_products");
shop.Endpoint("GET", "/products/:id", "get_product");
shop.Endpoint("POST", "/orders", "create_order");
shop.Endpoint("GET", "/orders", "list_orders");
shop.Endpoint("GET", "/orders/:id/total", "order_total");
shop.Endpoint("DELETE", "/orders/:id", "cancel_order");
server.Service(shop);
server.Compile();

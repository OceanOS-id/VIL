// 606-db-vilorm-ecommerce — Go SDK equivalent
// Compile: vil compile --from go --input 606-db-vilorm-ecommerce/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("vilorm-ecommerce", 8086)

	shop := vil.NewService("shop")
	shop.Endpoint("POST", "/products", "create_product")
	shop.Endpoint("GET", "/products", "list_products")
	shop.Endpoint("GET", "/products/:id", "get_product")
	shop.Endpoint("POST", "/orders", "create_order")
	shop.Endpoint("GET", "/orders", "list_orders")
	shop.Endpoint("GET", "/orders/:id/total", "order_total")
	shop.Endpoint("DELETE", "/orders/:id", "cancel_order")
	s.Service(shop)

	s.Compile()
}

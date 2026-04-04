// 031-basic-mesh-routing — Go SDK equivalent
// Compile: vil compile --from go --input 031-basic-mesh-routing/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("banking-transaction-mesh", 8080)

	teller := vil.NewService("teller")
	teller.Endpoint("GET", "/ping", "teller_ping")
	teller.Endpoint("POST", "/submit", "teller_submit")
	s.Service(teller)

	fraud_check := vil.NewService("fraud_check")
	fraud_check.Endpoint("POST", "/analyze", "fraud_process")
	s.Service(fraud_check)

	core_banking := vil.NewService("core_banking")
	core_banking.Endpoint("POST", "/post", "core_banking_post")
	s.Service(core_banking)

	notification := vil.NewService("notification")
	notification.Endpoint("GET", "/send", "notification_send")
	s.Service(notification)

	s.Compile()
}

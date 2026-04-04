// 017-basic-production-fullstack — Go SDK equivalent
// Compile: vil compile --from go --input 017-basic-production-fullstack/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("production-fullstack", 8080)

	fullstack := vil.NewService("fullstack")
	fullstack.Endpoint("GET", "/stack", "stack_info")
	fullstack.Endpoint("GET", "/config", "full_config")
	fullstack.Endpoint("GET", "/sprints", "sprints")
	fullstack.Endpoint("GET", "/middleware", "middleware_info")
	s.Service(fullstack)

	admin := vil.NewService("admin")
	admin.Endpoint("GET", "/config", "full_config")
	s.Service(admin)

	root := vil.NewService("root")
	s.Service(root)

	s.Compile()
}

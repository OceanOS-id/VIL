// 405-agent-react-multi-tool — Go SDK equivalent
// Compile: vil compile --from go --input 405-agent-react-multi-tool/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("react-multi-tool-agent", 3124)

	react_agent := vil.NewService("react-agent")
	react_agent.Endpoint("POST", "/react", "react_handler")
	s.Service(react_agent)

	s.Compile()
}

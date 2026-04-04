// 403-agent-code-file-reviewer — Go SDK equivalent
// Compile: vil compile --from go --input 403-agent-code-file-reviewer/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("code-file-reviewer-agent", 3122)

	code_review_agent := vil.NewService("code-review-agent")
	code_review_agent.Endpoint("POST", "/code-review", "code_review_handler")
	s.Service(code_review_agent)

	s.Compile()
}

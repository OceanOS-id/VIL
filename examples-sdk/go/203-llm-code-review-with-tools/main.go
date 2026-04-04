// 203-llm-code-review-with-tools — Go SDK equivalent
// Compile: vil compile --from go --input 203-llm-code-review-with-tools/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("llm-code-review-tools", 3102)

	code_review := vil.NewService("code-review")
	code_review.Endpoint("POST", "/code/review", "code_review_handler")
	s.Service(code_review)

	s.Compile()
}

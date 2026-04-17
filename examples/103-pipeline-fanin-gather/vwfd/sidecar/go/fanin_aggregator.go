package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"os"
)

func main() {
	scanner := bufio.NewScanner(os.Stdin)
	for scanner.Scan() {
		line := scanner.Text()
		if line == "" {
			continue
		}
		var input map[string]interface{}
		if err := json.Unmarshal([]byte(line), &input); err != nil {
			fmt.Println(`{"error":"invalid JSON"}`)
			continue
		}
		sources, _ := input["sources"].([]interface{})
		total := 0
		for _, s := range sources {
			if m, ok := s.(map[string]interface{}); ok {
				if c, ok := m["count"].(float64); ok {
					total += int(c)
				}
			}
		}
		result := map[string]interface{}{
			"total_records":  total,
			"sources_merged": len(sources),
			"strategy":       "fanin_append",
		}
		out, _ := json.Marshal(result)
		fmt.Println(string(out))
	}
}

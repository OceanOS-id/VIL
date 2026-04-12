package main
import ("encoding/json"; "fmt"; "os")
func main() {
    var input map[string]interface{}
    json.NewDecoder(os.Stdin).Decode(&input)
    sources, _ := input["sources"].([]interface{})
    total := 0
    for _, s := range sources { if m, ok := s.(map[string]interface{}); ok { if c, ok := m["count"].(float64); ok { total += int(c) } } }
    result := map[string]interface{}{"total_records": total, "sources_merged": len(sources), "strategy": "fanin_append"}
    json.NewEncoder(os.Stdout).Encode(result)
}

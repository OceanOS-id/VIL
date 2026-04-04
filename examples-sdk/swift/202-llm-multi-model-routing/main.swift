// 202-llm-multi-model-routing — Swift SDK equivalent
// Compile: vil compile --from swift --input 202-llm-multi-model-routing/main.swift --release

let server = VilServer(name: "MultiModelPipeline_GPT4", port: 8080)
server.compile()
